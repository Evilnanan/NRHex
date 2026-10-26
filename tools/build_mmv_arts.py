"""Generate MMV weapon coverage and explicitly reviewed portable skill entries."""
import argparse
import hashlib
import json
import re
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / 'compat/mmv/arts.json'


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def table(raw, name):
    result = {}
    for r in raw['tables'][name]:
        if r['ID'] in result and result[r['ID']] != r['fields']:
            raise ValueError(f'Ambiguous {name} {r["ID"]}')
        result[r['ID']] = r['fields']
    return result


def array(source, name):
    return re.search(r'pub const '+name+r':[^=]+ = \[(.*?)\];', source, re.S)[1]


def numbers(source, name):
    return list(map(int, re.findall(r'-?\d+', array(source, name))))


def judges(raw, art):
    return sorted({struct.unpack_from('<i', bytes.fromhex(e['data']), 8)[0]
                   for a in raw['animations'].get(str(art['swordArtsType']+600), [])
                   for e in a['events'] if e['Type'] in (1, 2)})


def common(judge):
    return (judge // 1000) * 100000000 + judge % 1000


def evidence(raw, mid):
    arts = table(raw, 'SwordArtsParam'); behaviors = table(raw, 'BehaviorParam_PC')
    a = arts[mid]
    return digest(dict(art=a, animations=raw['animations'].get(str(a['swordArtsType']+600), []),
                       commonBehaviors={j: behaviors.get(common(j)) for j in judges(raw, a)}))


def generate(raw, profile, native):
    if profile.get('schemaVersion') != 1:
        raise ValueError('Unsupported Arts review schema')
    weapons = table(raw, 'EquipParamWeapon'); arts = table(raw, 'SwordArtsParam')
    behaviors = table(raw, 'BehaviorParam_PC')
    reviewed = profile['skills']
    if sorted(arts.keys()-set(profile['nativeArtIds'])) != [r['id'] for r in reviewed]:
        raise ValueError('New/removed MMV skills require review')
    ordinary = set(numbers(native, 'POOL')); bows = numbers(native, 'BOW_POOL')
    variations = dict(tuple(map(int, x)) for x in re.findall(r'\((-?\d+),(-?\d+)\)', array(native, 'SKILL_VARIATIONS')))
    # Original skills retain their established canonical source weapon; the
    # target may have changed that weapon's behavior variation.
    for mid in variations:
        source = arts[mid]['sparringGroundsWeaponId']
        if source not in weapons: raise ValueError(f'Missing original skill sample weapon: {mid}')
        variations[mid] = weapons[source]['behaviorVariationId']
    added=[]; mapping=[]
    for record in reviewed:
        mid=record['id']; art=arts[mid]
        if evidence(raw,mid)!=record['evidenceSha256']:
            raise ValueError(f'Reviewed art/TAE/common behavior changed: {mid}')
        if not record['enabled']:
            if not record['reason']: raise ValueError(f'Missing exclusion reason: {mid}')
            continue
        if art['swordArtsType']==255 or (art['isGrayOutRightHand'] and art['isGrayOutBothHand']) or not raw['names'].get(str(art['textId'])):
            raise ValueError(f'Unusable new skill: {mid}')
        js=judges(raw,art)
        if not js or any(j<1000 or common(j) not in behaviors for j in js):
            raise ValueError(f'New skill requires weapon-specific behavior review: {mid}')
        mapping.extend((mid,j,common(j)) for j in js)
        added.append(mid); ordinary.add(mid)
    output='// Generated MMV Arts catalogue; shared runtime and native inventory remain unchanged.\n'
    arrays=dict(POOL=sorted(ordinary), BOW_POOL=bows,
                BOW_WEAPONS=sorted(i for i,w in weapons.items() if w['wepType'] in (50,51,53)),
                FIRE_WEAPONS=sorted(i for i,w in weapons.items() if w['attackBaseFire']>0),
                MAGIC_WEAPONS=sorted(i for i,w in weapons.items() if w['attackBaseMagic']>0),
                BEHAVIORS=sorted(behaviors))
    for name, values in arrays.items():
        output+=f'pub const {name}: [i32; {len(values)}] = ['+','.join(map(str,values))+'];\n'
    for name, values in [('DEFAULTS', sorted((i,w['swordArtsParamId']) for i,w in weapons.items())),('SKILL_VARIATIONS',sorted(variations.items()))]:
        output+=f'pub const {name}: [(i32,i32); {len(values)}] = [\n'+''.join(f' ({i},{v}),\n' for i,v in values)+'];\n'
    output+='pub const MMV_SKILLS:&[i32]=&['+','.join(map(str,added))+'];\n'
    output+='pub const MMV_JUDGES:&[i32]=&['+','.join(map(str,sorted({j for _,j,_ in mapping})))+'];\n'
    output+='pub const MMV_BEHAVIORS:&[(i32,i32,i32)]=&[\n'+''.join(f' ({i},{j},{b}),\n' for i,j,b in sorted(mapping))+'];\n'
    old_weapons=set(numbers(native,'DEFAULTS')[::2])
    report=dict(weapons=len(weapons),addedWeapons=len(weapons.keys()-old_weapons),
                addedBowWeapons=sorted(set(arrays['BOW_WEAPONS'])-set(numbers(native,'BOW_WEAPONS'))),
                addedFireWeapons=sorted(set(arrays['FIRE_WEAPONS'])-set(numbers(native,'FIRE_WEAPONS'))),
                addedMagicWeapons=sorted(set(arrays['MAGIC_WEAPONS'])-set(numbers(native,'MAGIC_WEAPONS'))),
                addedSkills=added,ordinaryPool=len(ordinary),bowPool=len(bows),explicitBehaviorMappings=len(mapping),
                exclusions=[r for r in reviewed if not r['enabled']],gameplayVerified=False)
    return output,report


def main():
    ap=argparse.ArgumentParser();ap.add_argument('--overlay',type=Path,default=ROOT/'work/compat/mmv/staged');args=ap.parse_args()
    raw=json.loads((ROOT/'work/compat/mmv/arts-parameters.json').read_text(encoding='utf8'))
    if Path(raw['overlay']).resolve()!=args.overlay.resolve():raise ValueError('Wrong Arts overlay export')
    for p,h in raw['hashes'].items():
        if hashlib.sha256((args.overlay/p).read_bytes()).hexdigest()!=h.lower():raise ValueError(f'Stale Arts export: {p}')
    source,report=generate(raw,json.loads(PROFILE.read_text(encoding='utf8')),(ROOT/'src/arts_data.rs').read_text(encoding='utf8'))
    out=ROOT/'work/compat/mmv/staged';out.mkdir(exist_ok=True,parents=True)
    (out/'arts_data.rs').write_text(source,encoding='utf8')
    report.update(hashes=raw['hashes'],outputSha256=hashlib.sha256(source.encode()).hexdigest())
    (out/'arts-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
    print(f"Arts: {report['addedWeapons']} added weapons, {len(report['addedSkills'])} reviewed skills, {report['explicitBehaviorMappings']} behavior mappings")


if __name__=='__main__':main()
