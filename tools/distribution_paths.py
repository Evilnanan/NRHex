"""Resolve distribution files and legacy root-level report names."""
from pathlib import Path


def distribution_file(directory, relative):
    relative = Path(relative)
    if relative.parent == Path('.') and relative.suffix.lower() == '.json':
        return Path(directory) / 'reports' / relative
    return Path(directory) / relative
