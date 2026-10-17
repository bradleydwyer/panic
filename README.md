# Panic in Rust

![The PANIC logo, rendered by the Rust port](docs/panic-logo.png)

A Rust port of Future Crew's 1992 demo **Panic**. All eight scenes are
implemented, along with the STMIK music player (sequencer and sample mixer).
It's native code, not a DOS emulator or a video capture. Desktop playback uses
SDL2; the headless player needs only Rust.

Apart from the screenshot above, the repository contains nothing from the
original demo. You generate the player's data from your own copy of
`PANIC.EXE`.

For a modern take on the intro and the first scroller, see
[panic-playground](https://github.com/bradleydwyer/panic-playground), which
builds on this port. It keeps the original sequence and music; its 3D models,
materials and effects are new work, not recovered from the demo.

## Quick start

You need Rust 1.88 or newer. Desktop builds also need CMake and a C/C++
compiler. The data importer needs Python, the packages in
`requirements-import.txt` and a [Deark](https://github.com/jsummers/deark)
executable. [rust/README.md](rust/README.md) has the full setup, including how
to build the tested Deark revision.

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-import.txt
.venv/bin/python tools/import_panic.py /path/to/PANIC.EXE \
  --deark /path/to/deark --out artifacts/local-data
cargo run --release --locked -- --data artifacts/local-data
```

The importer only accepts the 1,693,874-byte release (it checks the SHA-256).
It extracts the 23 data resources, unpacks the eight embedded programs with
Deark, and recovers the resident loader by running it under Unicorn. DOS code
is never run directly on your machine. Every output file is checked against a
known hash. The importer doesn't download anything and won't overwrite an
existing directory.

To play without a window or audio device:

```sh
cargo run --release --locked --no-default-features -- \
  --headless --data artifacts/local-data
```

Controls: Escape quits, Space pauses, F toggles fullscreen, and the arrow keys
seek (Right/Left 10 seconds, Up/Down 60 seconds, Shift+Right/Left by scene). `--seek 180` skips
to the three-minute mark and `--mute` turns the sound off. Run with `--help`
for the other options.

## Inspecting the resource file

`tools/panic_resources.py` reads the resource directory appended to
`PANIC.EXE`. It needs only the Python standard library:

```sh
python3 tools/panic_resources.py inspect /path/to/PANIC.EXE
python3 tools/panic_resources.py extract /path/to/PANIC.EXE \
  --out artifacts/raw-resources
```

`inspect` lists all 31 entries. `extract` copies out the 23 non-program
resources, or the packed programs as well with `--include-programs`. This isn't
enough to run the player, which also needs tables from the unpacked programs.
Use `import_panic.py` for that.

[metadata/panic-release.json](metadata/panic-release.json) lists every entry's
offset, size and hash, and [docs/container-format.md](docs/container-format.md)
describes the layout. The original release is on
[scene.org](https://files.scene.org/view/parties/1992/theparty92/demo/panic.zip).

## Tests

The default tests use synthetic data, so you don't need the original release:

```sh
python3 -m unittest discover -s tests -v
cargo test --release --locked --no-default-features
```

Set `PANIC_EXE=/path/to/PANIC.EXE` to include the Python tests that use the
real file. The Rust tests that need original data are ignored by default; see
[rust/README.md](rust/README.md#tests) for how to run them.

Before committing, stage your changes and run
`python3 tools/check_publication.py --research`. It checks that the index and
every reachable commit contain only the listed text files, with no binary data
or local paths. Without `--research` it checks the narrower tooling-only list.

## Licence and rights

The [MIT License](LICENSE) covers the resource tools, their tests and the
documentation, but not the screenshot. It does not cover the Rust port (`rust/`) or the
import/unpacking scripts (`tools/import_panic.py`, `tools/unpack_panic.py`),
whose rights status hasn't been settled. See [NOTICE](NOTICE),
[rust/NOTICE](rust/NOTICE) and [docs/publication.md](docs/publication.md).

Panic and everything in it belong to Future Crew, including the logo in the
screenshot above. Extracting the files yourself
doesn't give you any extra rights to use or share them.
