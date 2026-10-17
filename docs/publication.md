# What's published here

This repository has two parts with different licensing:

| Part | Contents | Licence |
|---|---|---|
| Resource tools | Resource directory inspector/extractor, synthetic tests, release metadata and documentation | MIT, see [LICENSE](../LICENSE) |
| Rust port | Rust player and its tests and build files, `tools/import_panic.py`, `tools/unpack_panic.py` | No licence granted while rights are reviewed; see [rust/NOTICE](../rust/NOTICE) |

The first commit (`3c350aa`) contains only the resource tools. The Rust port
and importer were added afterwards. The repository was started fresh and
contains none of the history of the original private research project.

## What's left out

Nothing from the demo itself is in the repository: no assets, music or
recordings, no recovered assembly or decompiler output, no binary test
fixtures and no packaged builds. The importer recreates the 23 resources and
nine unpacked programs on your machine from your own copy of `PANIC.EXE`. The
unpacked programs contain original code as well as data, so they stay out of
Git too.

The Rust port was written from analysis of the original code and tested
against it, so it isn't a clean-room implementation. The importer runs the
original loader, but only inside Unicorn. Leaving out the demo's files doesn't
settle whether the port itself can be freely shared, which is why it isn't
covered by the MIT License.

## The original release terms

Panic's `PANIC.NFO` allows copying the complete, unchanged release. It doesn't
allow modifying the demo or using its contents in other programs, and the
README embedded in `PANIC.EXE` says the same. You can read the NFO on
[Pouët](https://www.pouet.net/prod_nfo.php?font=1&which=479). The release's
hashes are in [the metadata](../metadata/panic-release.json).

We haven't found any later permission, and we haven't determined who holds the
rights today. Publishing this code doesn't grant any rights to the port or to
Future Crew's material.

## Checks

The default Python and Rust tests use synthetic data. Tests that need the
original release or reference output have to be enabled explicitly and read
those files at run time. The importer checks its input and every output file
against known hashes.

`tools/check_publication.py --research` checks the staged files and every
reachable commit against the list of allowed source files. It rejects binary
content, large files and local paths. Without `--research` it checks the
tooling-only list and fails if the port or importer is present anywhere in
history.

The checker can't detect every way an asset might be encoded into text, and
it doesn't check the GitHub repository's visibility or anyone's rights.
