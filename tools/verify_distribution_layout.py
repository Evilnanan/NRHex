"""Check report routing without rebuilding or modifying game resources."""
import ast
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from distribution_paths import distribution_file


def verify_record_writer(root):
    with tempfile.TemporaryDirectory(prefix='nrhex-report-layout-') as temporary:
        fixture = Path(temporary)
        for folder in ['tools', 'src', 'dist/reports', 'dist/mod/natives']:
            (fixture / folder).mkdir(parents=True, exist_ok=True)
        shutil.copy2(root / 'tools/record_build.py', fixture / 'tools/record_build.py')
        (fixture / 'Cargo.toml').write_text('[package]\nversion = "0.0.0"\n', encoding='utf-8')
        (fixture / 'src/lib.rs').write_text("fn signatures()->[(&'static str,u32);198] {}", encoding='utf-8')
        reports = fixture / 'dist/reports'
        (reports / 'validation.json').write_text('{"hexes":[]}', encoding='utf-8')
        (reports / 'fixture-validation.json').write_text('{}', encoding='utf-8')
        dll = fixture / 'dist/mod/natives/NRHex.dll'
        dll.write_bytes(b'layout-fixture')
        command = [sys.executable, str(fixture / 'tools/record_build.py'), '--dll', str(dll)]
        subprocess.run(command, check=True, capture_output=True, text=True)
        assert not list((fixture / 'dist').glob('*.json'))
        assert {'build-validation.json', 'runtime-validation.json', 'icon-validation.json', 'integration-validation.json'}.issubset(path.name for path in reports.iterdir())
        build = json.loads((reports / 'build-validation.json').read_text())
        assert 'fixture-validation.json' in build['functionChecks']
        before = {path.name: path.read_bytes() for path in reports.iterdir()}
        subprocess.run(command + ['--dll-only', '--no-install'], check=True, capture_output=True, text=True)
        assert before == {path.name: path.read_bytes() for path in reports.iterdir()}
        assert (fixture / 'work/build/dll-validation.json').is_file()


def verify(root):
    dist = root / 'dist'
    reports = dist / 'reports'
    assert reports.is_dir(), 'Missing report directory'
    assert not list(dist.glob('*.json')), 'JSON reports leaked into the distribution root'
    for path in reports.glob('*.json'):
        json.loads(path.read_text(encoding='utf-8-sig'))
    for name in ['启动测试版.ps1', '强制结束游戏.cmd', 'Stop-Nightreign.ps1', 'NRHex.me3', 'README.md']:
        assert (dist / name).is_file(), name
    for name in ['validation.json', 'arts-pool.json', 'wraith-inventory-validation.json']:
        assert distribution_file(dist, name) == reports / name
        assert distribution_file(dist, Path(name)) == reports / name
        assert distribution_file(dist, 'reports/' + name) == reports / name
    for name in ['mod/natives/NRHex.dll', 'mod/regulation.bin', 'testmods/example/config.json', 'NRHex.me3', 'README.md']:
        assert distribution_file(dist, name) == dist / name
    absolute = root / 'work' / 'validation.json'
    assert distribution_file(dist, absolute) == absolute
    sources = [path for path in (root / 'tools').rglob('*') if path.suffix in ['.py', '.cs', '.ps1'] and not {'obj', 'bin', '__pycache__'}.intersection(path.parts)]
    sources += list(root.glob('build*.ps1'))
    for path in sources:
        source = path.read_text(encoding='utf-8-sig')
        assert not re.search(r'dist/[\w*.-]+\.json', source), f'Legacy report path: {path}'
        if path.suffix != '.py':
            continue
        tree = ast.parse(source, filename=str(path))
        for node in ast.walk(tree):
            if not isinstance(node, ast.BinOp) or not isinstance(node.op, ast.Div):
                continue
            if not isinstance(node.left, ast.Name) or node.left.id != 'dist':
                continue
            if isinstance(node.right, ast.Constant) and isinstance(node.right.value, str):
                value = node.right.value
                assert not (value.endswith('.json') and '/' not in value), f'Unrouted report: {path}:{node.lineno}'
    verify_record_writer(root)
    print(f'PASS: {len(list(reports.glob("*.json")))} reports, legacy-name routing, runtime paths, full/DLL-only report writers and {len(sources)} tool sources')


if __name__ == '__main__':
    verify(Path(__file__).resolve().parents[1])
