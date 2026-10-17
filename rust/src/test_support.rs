//! Test helpers: locally supplied data and fixtures, and scratch directories
//! for synthetic tests. No original data is embedded.
use crate::data::Data;
use std::{
    env, fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn configured_root(variable: &str, description: &str) -> PathBuf {
    env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("Set {variable} to {description} before running ignored tests"))
}

pub fn data() -> Data {
    let root = configured_root(
        "PANIC_DATA",
        "a local imported data directory containing files/ and unpacked/, or the research root",
    );
    Data::new(&root)
        .unwrap_or_else(|error| panic!("Invalid PANIC_DATA at {}: {error}", root.display()))
}

pub fn fixture(relative: &str) -> Vec<u8> {
    let root = configured_root(
        "PANIC_FIXTURES",
        "the local research rust/ directory containing tests/fixtures/ and testdata/567/",
    );
    let path = root.join(relative);
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "Cannot read PANIC_FIXTURES input {}: {error}. Oracle fixtures are not bundled.",
            path.display()
        )
    })
}

pub fn fixture_text(relative: &str) -> String {
    String::from_utf8(fixture(relative))
        .unwrap_or_else(|error| panic!("PANIC_FIXTURES input {relative} is not UTF-8: {error}"))
}

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

/// A temporary directory removed on drop, for tests with invented files.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new() -> Self {
        loop {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!("panic-rust-test-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("Cannot create synthetic test directory: {error}"),
            }
        }
    }

    /// An unverified data directory containing only an invented `P.P`.
    pub fn player_data(&self) -> Data {
        fs::create_dir_all(self.0.join("files")).unwrap();
        fs::create_dir_all(self.0.join("unpacked")).unwrap();
        fs::write(self.0.join("files/P.P"), b"invented test marker").unwrap();
        Data::unverified(&self.0).unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
