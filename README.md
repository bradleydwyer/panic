# Panic in Rust

A native Rust research reconstruction of Future Crew's **Panic**, with all
eight scenes, the recovered STMIK music sequencer and sample mixer. Desktop
playback uses SDL2; the headless player needs only Rust. The player runs native
Rust code, not a DOS emulator or captured video.

This repository is **private while publication rights are reviewed**. Original
assets, music, unpacked executables and binary test fixtures stay outside Git.
You generate the player's data locally from your own `PANIC.EXE`.

## Import and run

The player requires Rust 1.88 or newer. Desktop builds also need CMake and a
C/C++ compiler. Data import needs Python with the pinned packages below and
a Deark executable. See [the full setup instructions](rust/README.md).

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-import.txt
.venv/bin/python tools/import_panic.py /path/to/PANIC.EXE \
  --deark /path/to/deark --out artifacts/local-data
cargo run --release --locked -- --data artifacts/local-data
```

The importer accepts the known 1,693,874-byte release and verifies its SHA-256.
It extracts 23 resources, decodes the eight embedded programs with Deark, and
recovers the resident loader in bounded Unicorn emulation. All nine resulting
module hashes must match the recorded identities. DOS code never runs directly
on the host. The importer fetches nothing and refuses existing destinations.

For a complete run without a window or audio device:

```sh
cargo run --release --locked --no-default-features -- \
  --headless --data artifacts/local-data
```

Escape quits, Space pauses and F toggles fullscreen. `--seek 180` fast-forwards
to three minutes. `--mute` keeps music timing while silencing output.

## Resource inspection

The standalone directory tool uses only Python's standard library:

```sh
python3 tools/panic_resources.py inspect /path/to/PANIC.EXE
python3 tools/panic_resources.py extract /path/to/PANIC.EXE \
  --out artifacts/raw-resources
```

It inventories all 31 directory entries and extracts the 23 non-program
resources by default. `--include-programs` also copies the packed programs
unchanged. This static extraction does not recover the tables the Rust player
needs; use `import_panic.py` for playable data.

[Release metadata](metadata/panic-release.json) records hashes and offsets.
The [container notes](docs/container-format.md) describe the observed layout.
The original release is catalogued by
[scene.org](https://files.scene.org/view/parties/1992/theparty92/demo/panic.zip).

## Tests and source checks

Default tests use synthetic data and need no original release:

```sh
python3 -m unittest discover -s tests -v
cargo test --release --locked --no-default-features
```

Python integration checks are enabled with `PANIC_EXE=/path/to/PANIC.EXE`.
Rust original-data and reference tests are ignored by default and read local
data/fixtures at runtime. [Rust testing instructions](rust/README.md#tests)
explain how to run them without committing fixtures.

After staging changes, run `python3 tools/check_publication.py --private`.
The checker permits only the listed text sources and inspects reachable Git
history. Its default public mode intentionally rejects the port and full
importer. Keep GitHub visibility private before pushing this source.

## Rights and scope

The [MIT license](LICENSE) covers the standalone resource tools, their synthetic
tests and prose. The Rust reconstruction and full import/recovery helpers are
outside that license while rights review remains unresolved; see
[NOTICE](NOTICE), [rust/NOTICE](rust/NOTICE) and the
[publication scope](docs/publication.md).

Future Crew's material retains its original rights. Local extraction and private
repository visibility do not grant permission to reuse or redistribute it.
This is a working research port, not a claim of complete reverse engineering
or public-release clearance.
