# Appended resource directory

These observations apply to the release identified in
[panic-release.json](../metadata/panic-release.json). Offsets are absolute file
positions and ranges below exclude their upper endpoint. All integers are
little-endian. No runtime behaviour is inferred from this inventory.

| Range | Bytes | Observed role |
|---|---:|---|
| `0x000000..0x006f91` | 28,561 | Declared DOS MZ extent |
| `0x006f91..0x006f92` | 1 | Gap before resource payloads |
| `0x006f92..0x19d5ba` | 1,664,552 | Contiguous resource payloads |
| `0x19d5ba..0x19d5c6` | 12 | Directory header |
| `0x19d5c6..0x19d8ae` | 744 | 31 directory records |
| `0x19d8ae..0x19d8b2` | 4 | Absolute directory pointer |

The footer points to `0x19d5ba`. The header consists of three unsigned 32-bit
fields: an opaque marker (`0x00c82fc0`), the record count (`31`), and the first
payload offset (`0x6f92`). The marker's meaning remains unresolved.

Each 24-byte directory record contains:

| Record offset | Type | Meaning |
|---|---|---|
| 0 | 16 bytes | ASCII name, NUL terminated and zero padded |
| 16 | unsigned 32-bit integer | Absolute payload offset |
| 20 | unsigned 32-bit integer | Payload length in bytes |

The payloads occur in directory order, with no intervening gaps or overlaps.
The last payload ends exactly where the directory begins. Eight names have an
`.EXE` suffix; the remaining 23 include a README, graphics-related data and a
music resource. Names alone do not establish a payload's full format or purpose.

The CLI first verifies the whole-file size and SHA-256. The parser then checks
the MZ extent, directory extent, count, payload bounds, name termination,
contiguity and uniqueness. It rejects path separators, case-insensitive name
collisions and reserved device names so extraction works across common desktop
filesystems. These filename restrictions are tool choices, not a claim about
every historical Future Crew container.

Extraction copies the original slices and verifies their hashes. It does not
decode images, render fonts, interpret tracker patterns, unpack executables,
emulate the loader or claim complete source-level understanding.

Evidence basis: the known carrier's bytes and a previously preserved directory
inventory, cross-checked against the local original when preparing this
repository. The metadata contains names, sizes, offsets and hashes only.
