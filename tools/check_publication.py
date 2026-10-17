#!/usr/bin/env python3
"""Check the staged snapshot and reachable history against the public scope.

This checks file boundaries and text constraints, not copyright ownership.
Review additions to APPROVED_FILES and the contents themselves before release.
"""
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
APPROVED_FILES = {
    '.gitignore', 'AGENTS.md', 'LICENSE', 'NOTICE', 'README.md',
    'docs/container-format.md', 'docs/publication.md', 'metadata/panic-release.json',
    'tests/test_resources.py', 'tests/test_local_release.py',
    'tools/panic_resources.py', 'tools/check_publication.py',
}


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def check_rows(records, label, *, staged=False):
    names = set()
    blobs = []
    for record in records.split(b'\0'):
        if not record:
            continue
        fields, name = record.split(b'\t', 1)
        name = name.decode('utf-8')
        mode, second, third = fields.decode('ascii').split()
        if mode not in ('100644', '100755'):
            raise ValueError(f'{label}: symlink or non-file entry: {name}')
        if staged:
            if third != '0':
                raise ValueError(f'{label}: unresolved index entry: {name}')
            oid = second
        else:
            if second != 'blob':
                raise ValueError(f'{label}: unexpected object type: {name}')
            oid = third
        if name not in APPROVED_FILES:
            raise ValueError(f'{label}: file outside reviewed scope: {name}')
        names.add(name)
        blobs.append((name, oid))
    if staged and names != APPROVED_FILES:
        raise ValueError(f'Missing staged files: {sorted(APPROVED_FILES - names)}')
    return blobs


def main():
    try:
        blobs = check_rows(git('ls-files', '--stage', '-z'), 'index', staged=True)
        commits = git('rev-list', '--all').decode('ascii').splitlines()
        for commit in commits:
            blobs.extend(check_rows(git('ls-tree', '-rz', '--full-tree', commit), commit))
        seen = set()
        for name, oid in blobs:
            if oid in seen:
                continue
            seen.add(oid)
            if int(git('cat-file', '-s', oid)) > 128 * 1024:
                raise ValueError(f'Unexpectedly large text file: {name}')
            content = git('cat-file', 'blob', oid).decode('utf-8')
            if '\0' in content:
                raise ValueError(f'Binary/NUL content: {name}')
            # Split literals so this checker does not match its own source.
            forbidden = ['/' + 'Users/', '/' + 'home/', 'research' + '-private',
                         'divoom-minitoo' + '-research', 'gh' + 'p_']
            if any(marker in content for marker in forbidden):
                raise ValueError(f'Local/private-path or token marker: {name}')
        print(f'Checked {len(APPROVED_FILES)} staged files, {len(commits)} commits, '
              f'{len(seen)} distinct text blobs. No files outside the reviewed scope.')
    except (OSError, ValueError, UnicodeError, subprocess.CalledProcessError) as error:
        print(f'Publication check failed: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
