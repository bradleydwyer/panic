# Public Panic tools

This is the public tooling repository, with a separate history from the private
research workspace. Keep its scope to authored inspection/extraction tools,
synthetic tests, factual metadata and original documentation.

Do not add original or extracted demo material, recovered disassembly,
decompiler output, translated player code, machine-code fixtures, screenshots,
recordings, palettes, fonts, samples or private archive metadata. New scope
requires a separate provenance and rights review; changing file extensions or
encoding binary data as text does not change that requirement.

Use synthetic data in the default tests. Original-release tests are explicitly
opted into with PANIC_EXE and must read the caller's file without changing it.
Keep local output under ignored artifacts/. Never execute DOS code on the host.

Distinguish observed bytes, static interpretation, runtime observation and
unresolved hypotheses. An exact directory inventory does not establish a
complete reverse engineering or permission to reuse the payloads.

Before a commit, run python3 -m unittest discover -s tests -v and, after staging,
python3 tools/check_publication.py. Review additions to its explicit file list.
The checker tests the publication boundary, not legal clearance.
