# Panic research and resource tools

This repository is private. It contains the Rust research port and local data
importer alongside a previously curated resource-tooling snapshot. The user
requested the port here after the tooling snapshot was pushed. Source history
remains separate from the larger research archive.

The Rust port and full loader/import tools are authorised in this private
repository, with public redistribution rights unresolved. Keep original or
extracted demo material, disassembly, decompiler output, binary fixtures,
screenshots, recordings, fonts, samples and private archive locations out of Git.
Generate player data locally from a user-supplied original. Do not treat local
extraction or private visibility as copyright clearance.

Use synthetic data in the default tests. Python original-release tests are
opted into with PANIC_EXE. Rust original-data tests are ignored by default and
use PANIC_DATA and PANIC_FIXTURES. Read caller-supplied files without changing them.
Keep local output under ignored artifacts/. Never execute DOS code on the host.

Distinguish observed bytes, static interpretation, runtime observation and
unresolved hypotheses. An exact directory inventory does not establish a
complete reverse engineering or permission to reuse the payloads.

Before a commit, run python3 -m unittest discover -s tests -v and
cargo test --release --locked --no-default-features. After staging, run
python3 tools/check_publication.py --private. Its default public mode must keep
rejecting port/importer additions until a separate rights review clears them.
Review changes to either explicit file list. The checker tests scope and text
constraints, not legal clearance. Keep remote visibility private for pushes.
