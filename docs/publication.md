# Publication scope

This repository is currently private. It contains two distinct scopes:

| Scope | Contents | Status |
|---|---|---|
| Resource tooling | Static directory inspector/extractor, synthetic tests, factual metadata and original format prose | The original curated snapshot; MIT applies as stated in LICENSE. |
| Research reconstruction | Rust player, its tests and build files, full importer and bounded loader recovery | Added for private development. Public redistribution and licensing remain unresolved. |

The initial commit `3c350aa` contains the resource-tooling snapshot alone.
The research source was subsequently added at the user's request after the
GitHub repository was made private. No original research history was merged.

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
asserted. Private visibility limits access but does not itself resolve copyright
or usage rights. Before making this repository public, review the reconstructed
implementation, loader/importer, original-content usage and licensing scope.
Permission from the relevant rights holders or an applicable legal basis has
not been established here.

## Checks

The default Python/Rust suites use synthetic data. Original-release and reference
tests opt into local inputs; Rust oracle tests are ignored by default and load
fixtures at runtime. The importer is checked against the recorded source and
unpacked-module hashes. These are technical checks, not copyright conclusions.

`tools/check_publication.py --private` checks the staged snapshot and every
reachable commit against the explicit private source list, with text-only and
private-path constraints. Its default public mode intentionally rejects the
port and importer, including their presence in history. A future removal from
HEAD alone would not make that history suitable for a public push.

The checker does not determine ownership, detect every encoded asset or inspect
GitHub visibility. Keep the remote private and review its visibility separately
before pushing changes in the research scope.
