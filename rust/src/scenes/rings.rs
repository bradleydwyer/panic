//! Part 6: bitplane interference rings and the PSTART transition.
//! Original 0x1161c control, 0x1130e mask decoder, 0x11372 window copier.
use crate::{
    data::{Data, word},
    vga::Vga,
};

/// Decodes the M.SUM bitplane mask into planes 2 and 3. After a leading
/// word the file holds 16 word offsets to streams: entries 2k and 2k + 1 fill
/// 8 KiB page k of plane 2 and plane 3 respectively. Each stream is a list of
/// (skip, count, count literal words) records ending with a zero skip; skipped
/// words are cleared and writing starts two bytes into the page.
fn decode_mask(mask: &[u8], planes: &mut [Vec<u8>; 4]) -> Result<(), String> {
    if mask.len() < 34 {
        return Err("Truncated M.SUM header".into());
    }
    for n in 0..16 {
        let mut at = word(mask, 2 + n * 2) as usize;
        let page = (n / 2) * 8192;
        let plane = &mut planes[2 + n % 2];
        let mut dest = page + 2;
        loop {
            if at + 2 > mask.len() {
                return Err("Truncated M.SUM skip".into());
            }
            let skip = word(mask, at) as usize;
            at += 2;
            if skip == 0 {
                break;
            }
            if at + 2 > mask.len() || dest + skip * 2 > page + 8192 {
                return Err("Invalid M.SUM skip".into());
            }
            plane[dest..dest + skip * 2].fill(0);
            dest += skip * 2;
            let count = word(mask, at) as usize;
            at += 2;
            if at + count * 2 > mask.len() || dest + count * 2 > page + 8192 {
                return Err("Invalid M.SUM literal".into());
            }
            plane[dest..dest + count * 2].copy_from_slice(&mask[at..at + count * 2]);
            at += count * 2;
            dest += count * 2;
        }
    }
    Ok(())
}

pub struct Rings {
    source: Vec<u8>,
    sine: Vec<i32>,
    image: Vec<u8>,
    image_palette: Vec<u8>,
    frame: usize,
    intensity: i32,
    selector: bool,
    page: u16,
    phase1: u16,
    phase2: u16,
    previous_row: u8,
    rng: u32,
    started: bool,
    transition: Option<usize>,
}
impl Rings {
    pub fn new(data: &Data, vga: &mut Vga) -> Result<Self, String> {
        let source = data.file("M.DAT")?;
        let mask = data.file("M.SUM")?;
        let sine = data.file("M.SIN")?;
        if source.len() < 64448 || sine.len() != 512 || mask.len() < 34 {
            return Err("Invalid ring resources".into());
        }
        for p in &mut vga.planes {
            p.fill(0);
        }
        vga.palette.fill(0);
        decode_mask(&mask, &mut vga.planes)?;
        vga.ega(0, 40, 320, 200);
        Ok(Self {
            source,
            sine: sine
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]) as i32)
                .collect(),
            image: data.file("PSTART.U")?,
            image_palette: data.file("PSTART.PAL")?,
            frame: 0,
            intensity: -100,
            selector: false,
            page: 0x2000,
            phase1: 0,
            phase2: 0,
            previous_row: 0,
            rng: 1,
            started: false,
            transition: None,
        })
    }
    pub fn step(&mut self, vga: &mut Vga, music: u16) -> bool {
        if let Some(level) = self.transition {
            if level >= 64 {
                return true;
            }
            for (out, &input) in vga.palette[3..].iter_mut().zip(&self.image_palette[3..]) {
                *out = (input as usize * level / 64) as u8 & 63;
            }
            self.transition = Some(level + 1);
            return false;
        }
        if !self.started {
            if music < 0x600 {
                return false;
            }
            self.started = true;
        }
        if self.frame >= 4464
            || (music == 0x7fff && self.frame > 1750)
            || (music != 0x7fff && music > 0x1000)
        {
            for p in &mut vga.planes {
                p.fill(0);
            }
            vga.palette.fill(0);
            vga.packed(&self.image, 320, 200);
            self.transition = Some(0);
            return self.step(vga, music);
        }
        let change = if music == 0x7fff {
            if self.frame < 700 && self.intensity < 64 {
                self.intensity += 1;
            }
            if self.frame > 1400 && self.intensity > 0 {
                self.intensity -= 1;
            }
            self.frame.is_multiple_of(8)
        } else {
            if music > 0xe00 {
                if self.intensity != 0 {
                    self.intensity -= 1;
                }
            } else if music > 0x71e && self.intensity < 64 {
                self.intensity += 1;
            }
            let row = music as u8;
            let change = row != self.previous_row && row & 1 == 0;
            self.previous_row = row;
            change
        };
        if change {
            self.selector = !self.selector;
            if !self.selector {
                self.page = self.page.wrapping_add(0x2000);
            }
        }
        let draw = self.page as usize;
        self.page ^= 0x2000;
        let active = if self.frame & 1 != 0 {
            64
        } else if self.selector {
            8
        } else {
            4
        };
        for index in 0..16 {
            let shade = if self.intensity <= 0 {
                0
            } else {
                [0, 48, 32, 0][index & 3] * self.intensity / 64
            };
            let rgb = if index & active != 0 {
                self.rng = self.rng.wrapping_mul(214013).wrapping_add(2531011);
                [63, 63, ((self.rng >> 16) & 31) as u8]
            } else {
                [shade as u8; 3]
            };
            vga.palette[index * 3..index * 3 + 3].copy_from_slice(&rgb);
        }
        self.phase1 = (self.phase1 & 4095).wrapping_add(35);
        self.phase2 = (self.phase2 & 4095).wrapping_sub(28);
        for (plane, phase, numerator, denominator) in
            [(1, self.phase1, 1, 1), (0, self.phase2, 9, 10)]
        {
            let i = (phase / 16) as usize;
            let x = self.sine[i & 255] * numerator / denominator + 50;
            let y = self.sine[(i + 64) & 255] * numerator / denominator + 50;
            let bank = (x & 7) as usize;
            for row in 0..184 {
                let r = y as usize + row;
                let mirror = if r < 152 { r } else { 303 - r };
                let src = bank * 8056 + mirror * 53 + (x / 8) as usize + 4;
                let dest = (draw + 324 + row * 40) & 65535;
                vga.planes[plane][dest..dest + 32].copy_from_slice(&self.source[src..src + 32]);
            }
        }
        self.frame += 1;
        vga.ega(self.page as usize, 40, 320, 200);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// Builds an invented mask whose entry 0 points at `stream` and whose
    /// other 15 entries point at a shared zero terminator after it.
    fn mask(stream: &[u8]) -> Vec<u8> {
        let terminator = 34 + stream.len();
        let mut bytes = vec![0; 34];
        bytes[2..4].copy_from_slice(&34u16.to_le_bytes());
        for n in 1..16 {
            bytes[2 + n * 2..4 + n * 2].copy_from_slice(&(terminator as u16).to_le_bytes());
        }
        bytes.extend_from_slice(stream);
        bytes.extend_from_slice(&[0, 0]);
        bytes
    }
    fn planes() -> [Vec<u8>; 4] {
        std::array::from_fn(|_| vec![0x11; 65536])
    }
    #[test]
    fn mask_decoder_clears_skips_and_copies_literals() {
        let mut out = planes();
        decode_mask(&mask(&[1, 0, 2, 0, 0xaa, 0xbb, 0xcc, 0xdd]), &mut out).unwrap();
        let mut expected = vec![0x11; 65536];
        expected[2..8].copy_from_slice(&[0, 0, 0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(out[2], expected);
        for plane in [0, 1, 3] {
            assert!(out[plane].iter().all(|&b| b == 0x11), "plane {plane}");
        }
    }
    #[test]
    fn mask_decoder_odd_entries_fill_plane_three_pages() {
        let mut bytes = mask(&[]);
        // Entry 3 is page 1 of plane 3; give it a one-word literal.
        let offset = bytes.len() as u16;
        bytes[8..10].copy_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&[1, 0, 1, 0, 0x5a, 0xa5, 0, 0]);
        let mut out = planes();
        decode_mask(&bytes, &mut out).unwrap();
        assert_eq!(out[3][8192 + 2..8192 + 6], [0, 0, 0x5a, 0xa5]);
        assert_eq!(out[3].iter().filter(|&&b| b != 0x11).count(), 4);
        assert!(out[2].iter().all(|&b| b == 0x11));
    }
    #[test]
    fn mask_decoder_rejects_truncated_and_overflowing_streams() {
        let cases: [(Vec<u8>, &str); 7] = [
            (vec![0; 33], "Truncated M.SUM header"),
            (
                {
                    let mut bytes = mask(&[]);
                    bytes[2..4].copy_from_slice(&0xfff0u16.to_le_bytes());
                    bytes
                },
                "Truncated M.SUM skip",
            ),
            (
                {
                    let mut bytes = mask(&[1, 0]);
                    bytes.truncate(36);
                    bytes[4..34].fill(0);
                    bytes
                },
                "Invalid M.SUM skip",
            ),
            (mask(&[0, 16, 0, 0]), "Invalid M.SUM skip"),
            (mask(&[1, 0, 4, 0, 1, 2, 3, 4]), "Invalid M.SUM literal"),
            (
                mask(&[0xfe, 0x0f, 2, 0, 1, 2, 3, 4]),
                "Invalid M.SUM literal",
            ),
            (
                {
                    let mut bytes = mask(&[1, 0, 0, 0]);
                    bytes.truncate(38);
                    bytes[4..34].fill(0);
                    bytes
                },
                "Truncated M.SUM skip",
            ),
        ];
        for (n, (bytes, error)) in cases.iter().enumerate() {
            assert_eq!(
                decode_mask(bytes, &mut planes()),
                Err(error.to_string()),
                "case {n}"
            );
        }
    }
    #[test]
    #[ignore = "requires locally supplied PANIC_DATA; run with --ignored"]
    fn music_wait_and_strict_exit_threshold() {
        let data = crate::test_support::data();
        let mut vga = Vga::new();
        let mut scene = Rings::new(&data, &mut vga).unwrap();
        assert!(!scene.step(&mut vga, 0x5ff));
        assert_eq!(scene.frame, 0);
        for position in [0x600, 0x71e, 0x71f, 0xe00, 0xe01, 0x1000] {
            assert!(!scene.step(&mut vga, position));
        }
        assert_eq!(scene.frame, 6);
        assert_eq!(scene.intensity, -100);
        for _ in 0..64 {
            assert!(!scene.step(&mut vga, 0x1001));
        }
        assert!(scene.step(&mut vga, 0x1001));
        assert_eq!(scene.frame, 6);
    }
    #[test]
    #[ignore = "requires locally supplied PANIC_DATA and PANIC_FIXTURES; run with --ignored"]
    fn all_original_fallback_planes_and_transition() {
        let data = crate::test_support::data();
        let mut vga = Vga::new();
        let mut scene = Rings::new(&data, &mut vga).unwrap();
        let palette_text =
            crate::test_support::fixture_text("testdata/567/rings-palette-sha256.txt");
        let mut palettes = palette_text.lines();
        for line in crate::test_support::fixture_text("testdata/567/rings-sha256.txt").lines() {
            assert!(!scene.step(&mut vga, 0x7fff));
            assert_eq!(
                format!("{:x}", Sha256::digest(&vga.palette)),
                palettes.next().unwrap(),
                "palette frame"
            );
            let mut hash = Sha256::new();
            for plane in &vga.planes {
                hash.update(plane);
            }
            assert_eq!(
                format!("{:x}", hash.finalize()),
                line,
                "rings frame {}",
                scene.frame
            );
        }
        for _ in 0..64 {
            assert!(!scene.step(&mut vga, 0x7fff));
            assert_eq!(
                format!("{:x}", Sha256::digest(&vga.palette)),
                palettes.next().unwrap(),
                "palette frame"
            );
        }
        assert!(scene.step(&mut vga, 0x7fff));
        assert_eq!(vga.indices, data.file("PSTART.U").unwrap());
        let mut expected = data.file("PSTART.PAL").unwrap();
        for value in &mut expected {
            *value = (*value as u16 * 63 / 64) as u8;
        }
        expected[..3].fill(0);
        assert_eq!(vga.palette, expected);
    }
}
