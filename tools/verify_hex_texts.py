"""Exercise bilingual editing and FMG round trips in an isolated workspace.

Run after: dotnet build tools/HexBuild/HexBuild.csproj
Only the temporary workspace under .work is modified.
"""
from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
tool = root / 'tools/HexBuild/bin/Debug/net10.0/HexBuild.dll'
assert tool.is_file(), 'Build HexBuild first.'
env = os.environ.copy()
env.setdefault('NRHEX_SMITHBOX', str(Path(root.anchor) / 'Program Files/Smithbox'))
env.setdefault('NRHEX_GAME', str(Path(root.anchor) / 'Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game'))


def run(work, option, error=None):
    result = subprocess.run(['dotnet', str(tool), str(work), option], env=env,
                            capture_output=True, text=True, encoding='utf-8', errors='replace')
    output = result.stdout + result.stderr
    if error is None:
        assert result.returncode == 0, output
    else:
        assert result.returncode != 0 and error in output, output


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def install_stage(work):
    for language in ('zhocn', 'engus'):
        for kind in ('item', 'menu'):
            relative = Path(language) / f'{kind}_dlc01.msgbnd.dcx'
            shutil.copy2(work / '.work/texts' / relative, work / 'dist/mod/msg' / relative)


(root / '.work').mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix='text-check-', dir=root / '.work') as folder:
    work = Path(folder)
    for name in ('docs/hexes.md', 'docs/hexes.en.md', 'data/text/status-effects.json'):
        target = work / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(root / name, target)
    shutil.copytree(root / 'dist/mod/msg', work / 'dist/mod/msg')
    status_path = work / 'data/text/status-effects.json'
    statuses = json.loads(status_path.read_text(encoding='utf-8'))
    collision = next(status for status in statuses if status['textId'] == 95000800)
    collision['enName'] = 'Dedicated invisibility status'
    collision['enInfo'] = 'Dedicated status description, not the ability description.'
    status_path.write_text(json.dumps(statuses, ensure_ascii=False), encoding='utf-8')
    run(work, '--stage-translations')
    install_stage(work)
    run(work, '--check-descriptions')

    english = work / 'docs/hexes.en.md'
    original = english.read_text(encoding='utf-8')
    blocks = re.split(r'(?=<!-- hex:\d+ -->)', original)
    first_name = re.search(r'^## .+$', blocks[1], re.M).group()
    second_name = re.search(r'^## .+$', blocks[2], re.M).group()
    first_description = re.search(r'^## [^\n]+\n(.*)', blocks[1], re.M | re.S).group(1).strip()
    invalid = [
        ('missing ID', ''.join(blocks[:-1]), 'missing [' + re.search(r'hex:(\d+)', blocks[-1]).group(1) + ']'),
        ('extra ID', original + '\n<!-- hex:95009900 -->\n## Extra\n\nExtra text.\n', 'extra [95009900]'),
        ('duplicate ID', original + blocks[-1], 'Duplicate hex text ID'),
        ('duplicate name', original.replace(second_name, first_name), 'Duplicate hex names'),
        ('empty description', original.replace(first_description, ''), 'Missing or malformed hex text'),
        ('missing marker', original.replace('<!-- hex:95000100 -->\n', ''), 'stable hex ID comment'),
        ('orphan marker', original + '\n<!-- hex:95009900 -->\n', 'stable hex ID comment'),
    ]
    for name, text, error in invalid:
        english.write_text(text, encoding='utf-8')
        before = {p: digest(p) for p in (work / 'dist/mod/msg').rglob('*.dcx')}
        run(work, '--stage-translations', error)
        assert all(digest(p) == value for p, value in before.items()), name

    # Names are associated by ID, not by order; Windows BOM and line endings work.
    reordered = blocks[0] + ''.join(reversed(blocks[1:]))
    english.write_bytes(b'\xef\xbb\xbf' + reordered.replace('\n', '\r\n').encode('utf-8'))
    run(work, '--check-descriptions')

    # An English-only edit must reach both reward names and inherited status names.
    name = 'Translator\'s "Crystal" Shell'
    info = 'Blocks an attack.\nSecond line: 50% & <value>.'
    shell = next(block for block in blocks if block.startswith('<!-- hex:95000700 -->'))
    edited = original.replace(shell, f'<!-- hex:95000700 -->\n## {name}\n\n{info}\n\n')
    english.write_text(edited, encoding='utf-8')
    status_path = work / 'data/text/status-effects.json'
    statuses = json.loads(status_path.read_text(encoding='utf-8'))
    statuses[0]['enInfo'] = 'Edited shield status.'
    status_path.write_text(json.dumps(statuses, ensure_ascii=False), encoding='utf-8')
    run(work, '--check-descriptions', 'engus/')
    run(work, '--stage-translations')
    report = json.loads((work / '.work/texts/validation.json').read_text(encoding='utf-8'))
    assert report['englishSourceSha256'].lower() == digest(english)
    assert all(r['unrelatedEntriesAndMembersUnchanged'] for r in report['reports'])
    assert all(not r['changes'] for r in report['reports'] if r['language'] == 'zhocn')
    item = next(r for r in report['reports'] if r['language'] == 'engus' and r['file'].startswith('item'))
    assert len(item['changes']) == 5
    assert all(c['id'] == 95000700 and c['after'] == (name if 'Name' in c['fmg'] else info) for c in item['changes'])
    menu = next(r for r in report['reports'] if r['language'] == 'engus' and r['file'].startswith('menu'))
    assert len(menu['changes']) == 4
    assert {(change['id'], change['fmg']) for change in menu['changes']} == {(effect, member) for effect in (95000700, 95000701) for member in ('SpEffectName.fmg', 'SpEffectInfo.fmg')}
    assert all(change['after'] == (name if 'Name' in change['fmg'] else info if change['id'] == 95000700 else 'Edited shield status.') for change in menu['changes'])
    install_stage(work)
    run(work, '--check-descriptions')
    print('PASS: bilingual FMG round trip; seven invalid inputs rejected without package writes; BOM/CRLF/reordering; English edits and inherited status names; unrelated Chinese/native text preserved.')
