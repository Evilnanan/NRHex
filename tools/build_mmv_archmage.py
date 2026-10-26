"""Apply the shared Archmage catalogue rules to the exact target regulation.

Classification includes zero-weight original slots. Only positive lottery
entries, excluding weapon/shield enchantments, may be randomly selected.
"""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def generate(raw):
    magic = {}
    for row in raw['tables']['Magic']:
        mid, fields = row['ID'], row['fields']
        if mid in magic and magic[mid] != fields:
            raise ValueError(f'Ambiguous Magic {mid}')
        magic[mid] = fields
    known = {mid: m['ezStateBehaviorType'] for mid, m in magic.items()
             if m['ezStateBehaviorType'] in (0, 1)}
    positive = {r['fields']['magicId'] for r in raw['tables']['MagicTableParam']
                if r['fields']['chanceWeight'] > 0}
    missing = positive - magic.keys()
    # Native tables also contain obsolete references; match the shared builder
    # by excluding absent rows, and expose them in the validation report.
    enchanted = {mid for mid, m in magic.items() if m['isEnchant'] or m['isShieldEnchant']}
    random = sorted(positive & known.keys() - enchanted)
    if not all(any(known[i] == kind for i in random) for kind in (0, 1)):
        raise ValueError('Empty sorcery/incantation pool')
    source = '// Generated from target Magic and MagicTableParam; do not edit.\n'
    source += 'pub const SPELLS:&[(i32,u8)]=&[\n'
    source += ''.join(f' ({mid},{kind}),\n' for mid, kind in sorted(known.items())) + '];\n'
    source += 'pub const RANDOM_SPELLS:&[i32]=&[' + ','.join(map(str, random)) + '];\n'
    return source, dict(known=len(known), random=len(random),
                        sorceries=sum(known[i] == 0 for i in random),
                        incantations=sum(known[i] == 1 for i in random),
                        classified=sorted(known), randomSpells=random,
                        excludedEnchantments=sorted(positive & enchanted),
                        missingMagic=sorted(missing),
                        excludedSpecial=sorted(positive - known.keys()), gameplayVerified=False)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--overlay', type=Path, default=ROOT / 'work/compat/mmv/staged')
    args = ap.parse_args()
    raw = json.loads((ROOT / 'work/compat/mmv/archmage-parameters.json').read_text(encoding='utf8'))
    path = args.overlay.resolve() / 'mod/regulation.bin'
    if Path(raw['regulationPath']).resolve() != path or hashlib.sha256(path.read_bytes()).hexdigest() != raw['regulationSha256'].lower():
        raise ValueError('Stale Archmage export; export the selected overlay first')
    source, report = generate(raw)
    out = ROOT / 'work/compat/mmv/staged'
    out.mkdir(parents=True, exist_ok=True)
    (out / 'archmage_data.rs').write_text(source, encoding='utf8')
    report.update(regulationSha256=raw['regulationSha256'].lower(),
                  outputSha256=hashlib.sha256(source.encode()).hexdigest())
    (out / 'archmage-validation.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
    print(f"Archmage: {report['known']} classified; {report['sorceries']} sorceries / {report['incantations']} incantations")


if __name__ == '__main__':
    main()
