# Panic resource tools

Inspect and extract the appended resources in Future Crew's **Panic** demo.
You supply the original `PANIC.EXE`; the tools read its directory and copy exact
byte ranges. They use Python's standard library and do not execute DOS code.

This repository contains authored tooling, synthetic tests, format notes and
hashes. It contains no demo assets or player implementation.

## Inspect your copy

Requires Python 3.10 or newer. There are no packages to install.

```sh
python3 tools/panic_resources.py inspect /path/to/PANIC.EXE
```

The command prints JSON with all 31 entries, including names, offsets, sizes and
SHA-256 hashes. It accepts only the known 1,693,874-byte public release:

```text
a7ceeae290a7132880cd6ba616c2b0bb4001b2f7c83694f04052314fa26df6e7
```

The [release metadata](metadata/panic-release.json) provides expected hashes for
each member. The [container notes](docs/container-format.md) describe the layout
and distinguish observations from interpretation. The original release is
catalogued by [scene.org](https://files.scene.org/view/parties/1992/theparty92/demo/panic.zip);
the tools never download it.

## Extract local resources

```sh
python3 tools/panic_resources.py extract /path/to/PANIC.EXE \
  --out artifacts/local-data
```

This writes 23 non-program resources into `files/` and a metadata inventory into
`inventory.json`. The embedded `files/README.TXT` retains the original notice.
The destination must be new. Output hashes are checked before completion.

To also copy the eight packed program members unchanged, add
`--include-programs`. They are never decompressed or executed. The resulting
files alone cannot run the research player: its tables also depend on unpacked
modules, which this tool does not recover.

Extracted files belong under ignored `artifacts/`. A generated `.gitignore`
also excludes the contents when another destination is chosen.

## Rights and scope

The [MIT license](LICENSE) covers this repository's authored code and prose.
Future Crew's material retains its original rights; see [NOTICE](NOTICE).
The original release allows sharing the complete, unchanged package but
restricts modification and reuse in other programs. Extracting your own copy
does not itself grant permission to reuse or redistribute its contents.

The [publication scope](docs/publication.md) records what is included, what
remains outside this repository, and the limits of this review.

## Tests

The default suite uses synthetic containers created in memory:

```sh
python3 -m unittest discover -s tests -v
```

An optional integration test compares a locally supplied original with every
recorded member hash. It reads the original and extracts into temporary storage:

```sh
PANIC_EXE=/path/to/PANIC.EXE python3 -m unittest discover -s tests -v
```

Before committing, stage the intended files and run:

```sh
python3 tools/check_publication.py
```

That checks the staged snapshot and every reachable commit against the reviewed
file list, text-only constraints and local/private-path exclusions. It cannot
determine copyright ownership or recognise every possible embedded asset.
