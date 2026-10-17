use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Resources the player reads, with the sizes and SHA-256 hashes from the
/// known `PANIC.EXE` release. These are hashes, not original data.
const FILES: [(&str, u64, &str); 19] = [
    (
        "P.P",
        593106,
        "5e0fc5b58683d98a401d2a76e8c203a4a6e516841b9eb6704932f52e384ac61b",
    ),
    (
        "MAP.U",
        64000,
        "8ce740dbec09200195db49020900b837e6c194a26699f33bd9e6928461411a76",
    ),
    (
        "MAP.PAL",
        768,
        "a9031dbd91652240602be7142bf01989841fc882d07287e4d748157d134a97c0",
    ),
    (
        "DEP.U",
        64000,
        "818edbd1b09e5c528cff69564f8b39c737599a5b10bc60eabb374d3c32caa756",
    ),
    (
        "WINDOW.PAL",
        768,
        "e2c0c1550517de64e0e68b104266115dc52a78b30a302656cb3bd59c1a02b7c5",
    ),
    (
        "WELLPAL.PAL",
        768,
        "c84e06bca2c95c22dc94971b91846e5cffb5f62fe08f1d363e4a2ab35bbaf97c",
    ),
    (
        "M.SIN",
        512,
        "2265737ddfc9757160447e3375b5ff167ef580a80ca3f5d34e9c9d30aff933bc",
    ),
    (
        "M.SUM",
        32604,
        "a9631553834ff0e7850e4666780bdd3e9fe3702fa828295ab96cc3cbdf160e79",
    ),
    (
        "M.DAT",
        65000,
        "0decb8adb9244a77a28e04a98a4256979ad1a5afdecd77f99a34778c7e59db7f",
    ),
    (
        "PSTART.U",
        64000,
        "877cfa9174d4aec857a290cddf237f92356031a03aa5c3b7715ce6981928cfdd",
    ),
    (
        "PSTART.PAL",
        768,
        "28de1c2c2d26fbbbe734de926b195de63aabcab0fbfadbe70b3591ebf7026cbe",
    ),
    (
        "PBUF1.PRE",
        55000,
        "5e2132b3b22f66773ab5527f51ba14b46b182011372929444c672cf4d7c65cb8",
    ),
    (
        "PBUF2.PRE",
        55000,
        "09be89a3ffaf400a7838906d39440d1386f115748a687c9b015838b1c177393a",
    ),
    (
        "TRUG_0.MAP",
        64000,
        "03c9fad9e107eb3b23666f4063c983987092439f942fc9cb43aa31adb4dd65d0",
    ),
    (
        "TRUG_1.MAP",
        64000,
        "748081315ed0faf262e97c34937b8084914761fe08cfe341c34abfff32c4bfe2",
    ),
    (
        "TRUG_2.MAP",
        64000,
        "d6d6192bc7466abe2332fa43d8e608cb750777924f67bbdc6e6a0a340dfd927f",
    ),
    (
        "TRUG_3.MAP",
        64000,
        "67e1059bd567ae7958d03cf4576058dfbbd7ecbe792d9eb46c56c470390893e9",
    ),
    (
        "MONSTER.U",
        64000,
        "47ea1ec93a1e78fc0b36aae1ea6180cdcbd2564cb37f685ff1898bcf0c3304d6",
    ),
    (
        "MONSTER.PAL",
        768,
        "78ee356861fc8f97c3aff43cbd96286e5c3dcc12d18dd024fb9a1b8c0edb3fac",
    ),
];

/// Unpacked program images, as produced by `tools/import_panic.py`.
const MODULES: [(&str, u64, &str); 9] = [
    (
        "PANIC.loader.exe",
        75456,
        "d34aa1db30f1372802acd413f4ba7cdfa080d616ee1d4c35179a2060e1367e45",
    ),
    (
        "1.000.exe",
        376029,
        "9d1763f0ec66a84ec9fefa728cd76639356a3308ee8820f78ca179715053530d",
    ),
    (
        "2.000.exe",
        275344,
        "96b86fa00989d50075be4b9c73fff7cd7d95d1584af9902b9c6fd8fd47978d8c",
    ),
    (
        "3.000.exe",
        368601,
        "b779a89d01ac98416e5b7526d60d5cb57138975c99a537cd7534b3110a018821",
    ),
    (
        "4.000.exe",
        94216,
        "843a11e7a0f4a2106e445bc4e7cde457252e3efd2fdcf327cab1c377d5d7dc42",
    ),
    (
        "5.final.000.exe",
        71568,
        "fb8daf80eb1905e6b2871aca333a800039c2b1c0379981a4cbd0aa7f8b09d1eb",
    ),
    (
        "6.final.000.exe",
        12480,
        "178d457f7883305d53d8f2be66c70764cf8f962ab1c8e018a36b58c093868706",
    ),
    (
        "7.000.exe",
        54777,
        "d009f0d26c8c9ddcd986461cdc25c46292b1afd16861e173834b556fb68e419d",
    ),
    (
        "8.000.exe",
        99204,
        "5da80539e5ce30a575c41981c7fb194c1555dc868c738901ff5318547b215f03",
    ),
];

/// Directory containing `files/` and `unpacked/`, or the research project root.
#[derive(Clone, Debug)]
pub struct Data {
    root: PathBuf,
    project: bool,
}

impl Data {
    /// Opens a data directory and checks that every file the player reads
    /// matches the imported release. Scenes rely on this: they index into
    /// resources assuming the original sizes and contents.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, String> {
        let data = Self::unverified(root)?;
        data.verify()?;
        Ok(data)
    }
    /// Opens a data directory without checking its contents. Only for tests
    /// that use small synthetic files; the scenes may panic on anything else.
    pub fn unverified(root: impl AsRef<Path>) -> Result<Self, String> {
        let root = root.as_ref().to_path_buf();
        let project = root.join("artifacts/derived/assets/files/P.P").is_file();
        let data = Self { root, project };
        if !data.file_path("P.P").is_file() {
            return Err(format!(
                "Panic data not found at {}. Pass --data <project-or-data-directory>.",
                data.root.display()
            ));
        }
        Ok(data)
    }
    fn verify(&self) -> Result<(), String> {
        let expected = FILES
            .iter()
            .map(|&(name, bytes, hash)| (self.file_path(name), bytes, hash))
            .chain(
                MODULES
                    .iter()
                    .map(|&(name, bytes, hash)| (self.module_path(name), bytes, hash)),
            );
        let mut problems = Vec::new();
        for (path, bytes, hash) in expected {
            match fs::read(&path) {
                Err(e) => problems.push(format!("{}: {e}", path.display())),
                Ok(blob) if blob.len() as u64 != bytes => problems.push(format!(
                    "{}: expected {bytes} bytes, found {}",
                    path.display(),
                    blob.len()
                )),
                Ok(blob) if format!("{:x}", Sha256::digest(&blob)) != hash => {
                    problems.push(format!("{}: contents don't match", path.display()))
                }
                Ok(_) => {}
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "The data in {} doesn't match the supported Panic release. \
                 Re-run tools/import_panic.py on an unchanged PANIC.EXE.\n  {}",
                self.root.display(),
                problems.join("\n  ")
            ))
        }
    }
    fn file_path(&self, name: &str) -> PathBuf {
        self.root
            .join(if self.project {
                "artifacts/derived/assets/files"
            } else {
                "files"
            })
            .join(name)
    }
    fn module_path(&self, name: &str) -> PathBuf {
        self.root
            .join(if self.project {
                "artifacts/derived/unpacked"
            } else {
                "unpacked"
            })
            .join(name)
    }
    pub fn file(&self, name: &str) -> Result<Vec<u8>, String> {
        let path = self.file_path(name);
        fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
    }
    /// Original load-module layout, including relocations and zero-filled BSS.
    /// Preserves addresses used in the evidence; this is data loading, not a CPU.
    pub fn memory(&self, part: u8) -> Result<Vec<u8>, String> {
        let name = match part {
            0 => "PANIC.loader.exe".to_owned(),
            5 | 6 => format!("{part}.final.000.exe"),
            1..=8 => format!("{part}.000.exe"),
            _ => return Err(format!("Invalid part {part}")),
        };
        let path = self.module_path(&name);
        let blob = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if blob.len() < 28 || &blob[..2] != b"MZ" {
            return Err(format!("Invalid MZ data: {}", path.display()));
        }
        let word = |o| u16::from_le_bytes([blob[o], blob[o + 1]]) as usize;
        let pages = word(4);
        if pages == 0 {
            return Err(format!("Invalid MZ extent: {}", path.display()));
        }
        let end = (pages - 1) * 512 + if word(2) == 0 { 512 } else { word(2) };
        let start = word(8) * 16;
        if start > end || end > blob.len() || end - start > 0x100000 {
            return Err(format!("Invalid MZ data extent: {}", path.display()));
        }
        let mut mem = vec![0; 0x110000];
        mem[0x10000..0x10000 + end - start].copy_from_slice(&blob[start..end]);
        for i in 0..word(6) {
            let at = word(0x18) + i * 4;
            if at + 4 > start {
                return Err(format!("Invalid MZ relocation table: {}", path.display()));
            }
            let dest = 0x10000 + word(at + 2) * 16 + word(at);
            if dest + 2 > 0x10000 + end - start {
                return Err(format!("Relocation outside module: {}", path.display()));
            }
            let value = u16::from_le_bytes([mem[dest], mem[dest + 1]]).wrapping_add(0x1000);
            mem[dest..dest + 2].copy_from_slice(&value.to_le_bytes());
        }
        Ok(mem)
    }
}

pub fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
pub fn signed(bytes: &[u8], offset: usize) -> i32 {
    word(bytes, offset) as i16 as i32
}
pub fn words(bytes: &[u8], offset: usize, count: usize) -> Vec<i32> {
    (0..count).map(|i| signed(bytes, offset + i * 2)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Scratch;

    fn put_word(bytes: &mut [u8], at: usize, value: u16) {
        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn invented_mz() -> Vec<u8> {
        // A 48-byte MZ header followed by 16 bytes of invented data, not code.
        let mut bytes = vec![0; 64];
        for (index, value) in [0x5a4d, 64, 1, 2, 3, 0, 0xffff, 0, 0, 0, 0, 0, 0x1c, 0]
            .into_iter()
            .enumerate()
        {
            put_word(&mut bytes, index * 2, value);
        }
        put_word(&mut bytes, 0x1c, 2);
        put_word(&mut bytes, 0x20, 8);
        put_word(&mut bytes, 48, 0xabcd);
        put_word(&mut bytes, 50, 0xfff8);
        put_word(&mut bytes, 56, 0x0110);
        bytes
    }

    #[test]
    fn portable_and_research_layouts_read_only_the_selected_files() {
        let portable = Scratch::new();
        let data = portable.player_data();
        fs::write(portable.0.join("files/INVENTED.DAT"), [4, 9, 16]).unwrap();
        assert_eq!(data.file("INVENTED.DAT").unwrap(), [4, 9, 16]);
        assert!(data.file("ABSENT.DAT").unwrap_err().contains("ABSENT.DAT"));

        let project = Scratch::new();
        let files = project.0.join("artifacts/derived/assets/files");
        fs::create_dir_all(&files).unwrap();
        fs::write(files.join("P.P"), b"another invented marker").unwrap();
        assert_eq!(
            Data::unverified(&project.0).unwrap().file("P.P").unwrap(),
            b"another invented marker"
        );
        assert!(Data::unverified(project.0.join("missing")).is_err());
    }

    #[test]
    fn mz_loading_relocates_words_with_wraparound_and_zero_fills_memory() {
        let scratch = Scratch::new();
        let data = scratch.player_data();
        let path = scratch.0.join("unpacked/1.000.exe");
        let source = invented_mz();
        fs::write(&path, &source).unwrap();
        let memory = data.memory(1).unwrap();
        assert_eq!(memory.len(), 0x110000);
        assert_eq!(word(&memory, 0x10000), 0xabcd);
        assert_eq!(word(&memory, 0x10002), 0x0ff8);
        assert_eq!(word(&memory, 0x10008), 0x1110);
        assert!(memory[..0x10000].iter().all(|&value| value == 0));
        assert!(memory[0x10010..].iter().all(|&value| value == 0));
        assert_eq!(fs::read(&path).unwrap(), source);
    }

    #[test]
    fn mz_loading_rejects_invalid_headers_and_relocations() {
        let scratch = Scratch::new();
        let data = scratch.player_data();
        let path = scratch.0.join("unpacked/1.000.exe");
        for invalid in [b"MZ".to_vec(), vec![0; 64]] {
            fs::write(&path, invalid).unwrap();
            assert!(data.memory(1).is_err());
        }
        for (offset, value, message) in [
            (4, 0, "MZ extent"),
            (2, 65, "data extent"),
            (8, 5, "data extent"),
            (0x18, 48, "relocation table"),
            (0x1c, 15, "Relocation outside module"),
        ] {
            let mut source = invented_mz();
            put_word(&mut source, offset, value);
            fs::write(&path, source).unwrap();
            assert!(data.memory(1).unwrap_err().contains(message), "{message}");
        }
        assert!(data.memory(9).unwrap_err().contains("Invalid part"));
    }

    #[test]
    fn little_endian_word_helpers_preserve_signed_boundaries() {
        let values = [0, 0, 0xff, 0x7f, 0, 0x80, 0xff, 0xff];
        assert_eq!(word(&values, 4), 32768);
        assert_eq!(signed(&values, 6), -1);
        assert_eq!(words(&values, 0, 4), [0, 32767, -32768, -1]);
    }

    #[test]
    fn verification_lists_missing_resized_and_changed_files() {
        let scratch = Scratch::new();
        scratch.player_data();
        // Right size, wrong contents.
        fs::write(scratch.0.join("files/P.P"), vec![0; 593106]).unwrap();
        // Wrong size.
        fs::write(scratch.0.join("files/MAP.PAL"), [0; 100]).unwrap();
        let error = Data::new(&scratch.0).unwrap_err();
        assert!(error.contains("P.P: contents don't match"), "{error}");
        assert!(
            error.contains("MAP.PAL: expected 768 bytes, found 100"),
            "{error}"
        );
        assert!(error.contains("MONSTER.U"), "{error}");
        assert!(error.contains("PANIC.loader.exe"), "{error}");
        assert!(error.contains("import_panic.py"), "{error}");
    }
}
