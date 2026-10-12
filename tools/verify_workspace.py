"""Verify retained original/reference fixtures before rebuilding."""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parents[1]
for folder in ['assets/original','data/reference']:
    base=root/folder
    manifest=json.loads((base/'manifest.json').read_text(encoding='utf8'))
    for name,expected in manifest['files'].items():
        path=base/name
        assert path.is_file(),f'Missing retained input: {path}'
        assert hashlib.sha256(path.read_bytes()).hexdigest()==expected,f'Changed retained input: {path}'
print('PASS: original assets and retained research fixtures match their manifests.')
