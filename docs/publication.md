# Publication scope

This repository starts with a fresh history and a deliberately limited scope.
It publishes authored resource-inspection code, synthetic tests, original prose
and factual release metadata. The parser is written from the observed directory
layout; it contains no recovered demo instructions or payload byte arrays.

The private research work is not imported as a Git parent or remote. Its native
players, protection-loader emulation, disassembly, decompiler output, exact
machine-code tests, graphics and music fixtures, recordings and packaged builds
are outside this snapshot. The existing preview and music-export scripts are
also outside its scope.

The metadata is an explicit selection of source/member hashes, byte counts and
offsets. It includes public source references, with no local filesystem paths,
private repository URLs or backup locations. It cannot reconstruct the original
content without a separately supplied copy.

## Original release terms

The original `PANIC.NFO` permits copying the complete, unchanged release and
restricts modifying the demonstration or using its contents in other programs.
The embedded README repeats those terms. The source is identified in the
[metadata](../metadata/panic-release.json); a public transcription is available
on [Pouët](https://www.pouet.net/prod_nfo.php?font=1&which=479).

This repository's MIT license applies to its authored tools, tests and prose.
It grants no rights over user-supplied or extracted Future Crew material.
Keeping that material out of Git reduces what this project distributes, but
does not settle all rights associated with extraction, adaptation or reuse.

No later permission, current ownership determination or legal clearance is
asserted. A faithful port using original code, artwork or music needs its own
rights review; it should not be added merely because its assets can be imported
locally. Research accuracy and permission to publish are separate questions.

## Verification boundary

Default tests create synthetic payloads in memory and cover valid extraction,
hostile filenames and malformed extents, output preservation and cleanup.
The optional original-release test compares all 31 member hashes against the
metadata. Neither suite executes the demo or validates its rendered behaviour.

`tools/check_publication.py` checks staged files and reachable Git history
against an explicit list, requires small UTF-8 text files, and rejects selected
private-path markers. This is a structural check. Human review is still needed
for copied text, translated code, encoded data or additions to the allowed list.
