"""Append named MMV-only spells for manual balance review; never assign bonuses.

First run HexBuild --mmv-spells <MMV root>. This does not enable runtime support.
"""
import csv
import io
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIELDS = ['MagicID', '名称', '类别', '加成']


def main():
    source = ROOT / 'work/compat/mmv/spells.json'
    catalog = json.loads(source.read_text(encoding='utf8'))
    shared_path = ROOT / 'data/balance/sword.csv'
    shared_before = shared_path.read_bytes()
    shared = {int(r['MagicID']) for r in csv.DictReader(io.StringIO(shared_before.decode('utf-8-sig')))}
    original = {r['ID'] for r in catalog['originalMagic']}
    names = {}
    for member, entries in catalog['names'].items():
        if not member.startswith('MagicName'):
            continue
        for entry in entries:
            text = entry['Text']
            if text and text.strip() and text != '%null%':
                mid = entry['ID']
                if mid in names and names[mid] != text:
                    raise ValueError(f'Conflicting spell name: {mid}')
                names[mid] = text
    rows = {}
    excluded = []
    weighted = {r['fields']['magicId'] for r in catalog['magicTables'] if r['fields']['chanceWeight'] > 0}
    for r in catalog['magic']:
        mid = r['ID']
        if mid in shared or mid in original:
            continue
        if mid in rows:
            raise ValueError(f'Duplicate MMV Magic row: {mid}')
        if mid not in names:
            excluded.append({'MagicID': mid, 'parameterName': r['Name'], 'reason': 'No player-facing MagicName; internal row requires mapping review'})
            continue
        kind = r['fields']['ezStateBehaviorType']
        if kind not in (0, 1):
            raise ValueError(f'Unreviewed spell category {kind}: {mid}')
        rows[mid] = {'MagicID': str(mid), '名称': names[mid], '类别': ['魔法', '祷告'][kind], '加成': ''}

    output = ROOT / 'data/balance/sword-mmv.csv'
    previous = output.read_bytes() if output.exists() else b''
    existing = {}
    if previous:
        reader = csv.DictReader(io.StringIO(previous.decode('utf-8-sig')))
        if reader.fieldnames != FIELDS:
            raise ValueError('Unexpected MMV CSV columns')
        for row in reader:
            mid = int(row['MagicID'])
            if mid in existing or mid not in rows or None in row or any(v is None for v in row.values()):
                raise ValueError(f'Unknown, duplicate or malformed MMV CSV row: {mid}')
            if any(row[k] != rows[mid][k] for k in FIELDS[:-1]):
                raise ValueError(f'MMV spell metadata changed; review before updating: {mid}')
            existing[mid] = row
    added = [rows[mid] for mid in sorted(rows.keys() - existing.keys())]
    text = io.StringIO(newline='')
    writer = csv.DictWriter(text, FIELDS, lineterminator='\r\n')
    if not previous:
        writer.writeheader()
    writer.writerows(added)
    if added:
        if previous and not previous.endswith((b'\n', b'\r')):
            previous += b'\r\n'
        output.write_bytes(previous + text.getvalue().encode('utf8') if previous else text.getvalue().encode('utf-8-sig'))
    actual = list(csv.DictReader(output.open(encoding='utf-8-sig', newline='')))
    assert {int(r['MagicID']) for r in actual} == rows.keys()
    assert len(actual) == len(rows)
    assert all(next(r for r in actual if r['MagicID'] == str(mid)) == row for mid, row in existing.items())
    assert shared_path.read_bytes() == shared_before
    report = dict(regulationSha256=catalog['regulationSha256'], textSha256=catalog['textSha256'],
                  output=output.relative_to(ROOT).as_posix(), spellCount=len(rows), addedCount=len(added),
                  withoutPositiveMagicTableWeight=[mid for mid in sorted(rows) if mid not in weighted],
                  excludedInternalRows=excluded, runtimeIntegrated=False)
    (source.parent / 'sword-catalog-report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n', encoding='utf8')
    print(f'MMV Sword CSV: {len(rows)} spells, {len(added)} appended; existing values preserved.')


if __name__ == '__main__':
    main()
