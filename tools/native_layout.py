"""Move legacy native payloads beside mod/, preserving every file and setting."""
import argparse
import hashlib
import json
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def migrate(directory: Path):
    directory = directory.resolve()
    old, new = directory / 'mod/natives', directory / 'natives'
    if not old.exists():
        return {}
    if new.exists():
        raise ValueError(f'Both native directories exist; resolve their contents first: {directory}')
    for path in [old, *old.rglob('*')]:
        if path.resolve() != path.absolute():
            raise ValueError(f'Linked native path cannot be migrated: {path}')
    if not old.resolve().is_relative_to(directory):
        raise ValueError('Native directory escaped destination')
    hashes = {p.relative_to(old).as_posix(): digest(p) for p in old.rglob('*') if p.is_file()}
    for relative in hashes:
        if Path(relative).suffix.lower() == '.dll':
            with (old / relative).open('r+b'):
                pass
    old.rename(new)
    assert all(digest(new / relative) == value for relative, value in hashes.items())
    return hashes


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    print(json.dumps(migrate(args.directory), indent=2))
