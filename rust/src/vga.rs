/// Only the logical video memory and scanout operations used by Panic.
/// DAC components remain six-bit until desktop presentation.
#[derive(Clone)]
pub struct Vga {
    pub planes: [Vec<u8>; 4],
    pub palette: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub indices: Vec<u8>,
}
impl Default for Vga {
    fn default() -> Self {
        Self::new()
    }
}
impl Vga {
    pub fn new() -> Self {
        Self {
            planes: std::array::from_fn(|_| vec![0; 65536]),
            palette: vec![0; 768],
            width: 320,
            height: 200,
            indices: vec![0; 64000],
        }
    }
    fn size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.indices.resize(width * height, 0);
    }
    pub fn mode_x(&mut self, start: usize, stride: usize, width: usize, height: usize) {
        self.size(width, height);
        for y in 0..height {
            for x in 0..width {
                self.indices[y * width + x] =
                    self.planes[x & 3][(start + y * stride + (x >> 2)) & 65535];
            }
        }
    }
    pub fn packed(&mut self, pixels: &[u8], width: usize, height: usize) {
        assert!(
            pixels.len() >= width * height,
            "packed image has {} bytes, needs {}",
            pixels.len(),
            width * height
        );
        self.size(width, height);
        self.indices.copy_from_slice(&pixels[..width * height]);
        for (i, &v) in pixels[..width * height].iter().enumerate() {
            self.planes[i & 3][(i >> 2) & 65535] = v;
        }
    }
    pub fn ega(&mut self, start: usize, stride: usize, width: usize, height: usize) {
        self.size(width, height);
        for y in 0..height {
            for x in 0..width {
                let at = (start + y * stride + (x >> 3)) & 65535;
                let bit = 7 - (x & 7);
                self.indices[y * width + x] = (0..4)
                    .map(|p| ((self.planes[p][at] >> bit) & 1) << p)
                    .fold(0, |a, b| a | b);
            }
        }
    }
    pub fn rgb(&self) -> Vec<u8> {
        self.indices
            .iter()
            .flat_map(|&index| {
                self.palette[index as usize * 3..index as usize * 3 + 3]
                    .iter()
                    .map(|&v| {
                        let v = v & 63;
                        (v << 2) | (v >> 4)
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_x_scanout_interleaves_planes_across_address_wraparound() {
        let mut video = Vga::new();
        for (plane, values) in video.planes.iter_mut().enumerate() {
            values[65535] = 10 + plane as u8;
            values[0] = 20 + plane as u8;
            values[1] = 30 + plane as u8;
        }
        video.mode_x(65535, 1, 8, 2);
        assert_eq!(
            video.indices,
            [
                10, 11, 12, 13, 20, 21, 22, 23, 20, 21, 22, 23, 30, 31, 32, 33
            ]
        );
        assert_eq!((video.width, video.height), (8, 2));
    }

    #[test]
    fn ega_scanout_combines_plane_bits_in_most_significant_bit_order() {
        let mut video = Vga::new();
        for (plane, byte) in [0x55, 0x33, 0x0f, 0].into_iter().enumerate() {
            video.planes[plane][42] = byte;
            video.planes[plane][43] = byte | if plane == 3 { 0xff } else { 0 };
        }
        video.ega(42, 1, 8, 2);
        assert_eq!(video.indices, (0..16).collect::<Vec<_>>());
    }

    #[test]
    fn packed_pixels_roundtrip_through_planes_and_six_bit_dac() {
        let mut video = Vga::new();
        let pixels = [2, 0, 1, 2, 1, 2, 0, 1];
        video.palette[..9].copy_from_slice(&[0, 16, 63, 64, 95, 127, 1, 32, 62]);
        video.packed(&pixels, 4, 2);
        assert_eq!(video.indices, pixels);
        video.indices.fill(255);
        video.mode_x(0, 1, 4, 2);
        assert_eq!(video.indices, pixels);
        assert_eq!(&video.rgb()[..9], &[4, 130, 251, 0, 65, 255, 0, 125, 255]);
    }
}
