# Native Rust player

The port implements all eight effects in the order 1, 2, 6, 5, 3, 4, 7, 8,
plus the STMIK sequencer and stereo sample mixer. The runtime consumes original
assets and unpacked module data without executing DOS instructions. Its data
must be generated locally; the original resource files are not bundled.

This repository publishes the reconstruction source for research and development.
Its licensing status and original-material restrictions are described in [NOTICE](NOTICE).

## Setup

Use Rust 1.88 or newer. Desktop playback builds SDL2 0.38.0 from its pinned
bundled source and requires CMake plus a C/C++ compiler. On Linux, also install
the platform development libraries SDL needs for your window/audio system.
The pure Rust headless build does not require SDL or CMake.

The full data importer needs Python, Unicorn 2.1.4, Capstone 5.0.7 and Deark.
The tested Deark revision is `8997059c4f32af79978fb6c83be7c1c26c259afa`
(1.7.3-1). A local source build can be prepared with Git, make and a C compiler:

```sh
git clone https://github.com/jsummers/deark.git artifacts/tools/deark
git -C artifacts/tools/deark checkout 8997059c4f32af79978fb6c83be7c1c26c259afa
make -C artifacts/tools/deark
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-import.txt
.venv/bin/python tools/import_panic.py /path/to/PANIC.EXE \
  --deark artifacts/tools/deark/deark --out artifacts/local-data
```

Run these commands from the repository root. On Windows, use the virtual
environment's `Scripts/python.exe` and a compatible Deark build instead.
The build tools have their own licenses; their source and binaries remain local.

The original carrier must have SHA-256
`a7ceeae290a7132880cd6ba616c2b0bb4001b2f7c83694f04052314fa26df6e7`.
The importer verifies all extracted and unpacked output identities before
completion. It preserves the original embedded notice in `files/README.TXT`.
The output contains original code as well as data and must remain outside Git.
Importing does not grant usage or redistribution rights.

## Run

```sh
cargo run --release --locked -- --data artifacts/local-data
cargo run --release --locked --no-default-features -- \
  --headless --data artifacts/local-data
```

Escape quits, Space pauses and F toggles fullscreen. `--seek 180` starts after
a deterministic fast-forward to three minutes. `--mute` silences the speakers
but keeps music timing. `--fallback` selects the no-music schedules.
`--seconds 10` limits playback; `--out artifacts/export` saves locally generated
audio/images and diagnostics. Those outputs remain excluded from Git.

`PANIC_DATA` can supply the data directory instead of `--data`. The runtime
also recognises `data/` beside the executable and the original research layout.
The lightweight `panic_resources.py extract` output is insufficient; use the
full importer above.

## Tests

The normal suite compiles with no original files or binary fixtures:

```sh
cargo test --release --locked --no-default-features
```

Tests comparing original data and machine-code oracle results are explicitly
ignored by default. They read files at runtime instead of embedding fixtures.
To run them, supply imported player data and your separately preserved reference
fixtures. `PANIC_FIXTURES` points to a directory containing `tests/fixtures/`
and `testdata/567/`, matching the research project's `rust/` layout:

```sh
PANIC_DATA=/path/to/imported-data PANIC_FIXTURES=/path/to/reference-rust \
  cargo test --release --locked --no-default-features -- --ignored
```

The importer generates player inputs, not oracle fixtures. The full historical
oracle suite requires those separately preserved local fixtures. No binary
fixture, including an audio recording, belongs in this repository.

## Behaviour and limits

The native timing model uses a deterministic 70 Hz scene timer, 16 kHz stereo
audio and a fixed 512-byte refill contract. This preserves the recovered
arithmetic and sequencing choices but does not emulate physical VGA/PIT timing
or promise the same frame selection as every original DOS machine.

The research baseline's full headless run takes 17,442 simulated timer steps,
or 249.171429 seconds. The reconstruction has been exercised on Apple Silicon;
other desktop platforms still require runtime validation. A successful build
alone does not establish their display/audio behaviour.
