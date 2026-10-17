# Publication scope

This repository is public at the maintainer's explicit request. It contains two
distinct scopes:

| Scope | Contents | Status |
|---|---|---|
| Resource tooling | Static directory inspector/extractor, synthetic tests, factual metadata and original format prose | The original curated snapshot; MIT applies as stated in LICENSE. |
| Research reconstruction | Rust player, its tests and build files, full importer and bounded loader recovery | Published as research source. Licensing and original-material restrictions are recorded in rust/NOTICE. |

The initial commit `3c350aa` contains the resource-tooling snapshot alone.
The research source was subsequently added at the user's request after the
GitHub repository was made private. The maintainer then requested public
visibility for the combined source. No original research history was merged.

Original/extracted assets, music recordings, binary oracle fixtures, recovered
assembly, decompiler output and packaged builds remain excluded. The importer
generates 23 resources and nine unpacked module files locally from a supplied
carrier. Those modules contain original code as well as tables; they remain
ignored data and are not a licence workaround.

The Rust source is a research reconstruction informed by original-code analysis
and differential tests. It is not represented as a clean-room implementation.
The full loader helper executes the known carrier only inside bounded Unicorn
emulation. Neither addition is cleared merely because the source contains no
bundled media. See [rust/NOTICE](../rust/NOTICE).

## Original release terms

The original PANIC.NFO permits copying the complete, unchanged release and
restricts modifying the demonstration or using its contents in other programs.
The embedded README repeats those terms. Source identity is recorded in
[the metadata](../metadata/panic-release.json); a public transcription is
available on [Pouët](https://www.pouet.net/prod_nfo.php?font=1&which=479).

No later permission, current ownership determination or legal clearance is
asserted. Publishing source does not itself resolve copyright or usage rights
over the reconstructed implementation or the original content it uses. No new
permission over Future Crew material is claimed.

## Checks

The default Python/Rust suites use synthetic data. Original-release and reference
tests opt into local inputs; Rust oracle tests are ignored by default and load
fixtures at runtime. The importer is checked against the recorded source and
unpacked-module hashes. These are technical checks, not copyright conclusions.

`tools/check_publication.py --research` checks the staged snapshot and every
reachable commit against the explicit research source list, with text-only and
private-path constraints. Its default mode checks the smaller standalone-tooling
scope and rejects the port/importer, including their presence in history.

The checker does not determine ownership, detect every encoded asset or inspect
GitHub visibility. It does not certify publication rights.
