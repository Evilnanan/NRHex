"""Read-only coverage of MMV additions against the shipped runtime catalogues.

Run HexBuild --mmv-content-audit first. Counts are parameter rows, not unique
playable weapons/enemies; catalogue membership is not a gameplay guarantee.
"""
import hashlib
import json
import re
from pathlib import Path
from mmv_hks_fragments import functions
from hks_bytecode import Reader

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'work/compat/mmv/content-audit'


def section(file, name):
    text = (ROOT / file).read_text(encoding='utf8')
    return text.split('pub const ' + name + ':', 1)[1].split('=', 1)[1].split(';', 1)[0]


def numbers(file, name):
    return set(map(int, re.findall(r'-?\d+', section(file, name))))


def pairs(file, name):
    return dict((int(a), int(b)) for a, b in re.findall(r'\(\s*(-?\d+)\s*,\s*(-?\d+)\s*\)', section(file, name)))


def main():
    raw = json.loads((OUT / 'parameters.json').read_text(encoding='utf8'))
    for path, expected in raw['hashes'].items():
        assert hashlib.sha256(Path(path).read_bytes()).hexdigest().lower() == expected.lower(), path
    spellraw = json.loads((ROOT / 'work/compat/mmv/sword-parameters.json').read_text(encoding='utf8'))
    input_hashes = {Path(p).resolve(): h.lower() for p,h in raw['hashes'].items()}
    assert spellraw['regulationSha256'].lower() == input_hashes[(ROOT / 'dist/mmv/mod/regulation.bin').resolve()]
    native = json.loads((ROOT / 'work/sword-range-export/parameters.json').read_text(encoding='utf8'))
    magic = {r['ID']: r['fields'] for r in spellraw['tables']['Magic']}
    oldmagic = {r['ID']: r['fields'] for r in native['tables']['Magic']}
    bullets = {r['ID']: r['fields'] for r in spellraw['tables']['Bullet']}
    attacks = {}
    for row in spellraw['tables']['AtkParam_Pc']:
        attacks.setdefault(row['ID'],[]).append(row['fields'])
    def damage(roots):
        todo=list(roots);seen=set()
        while todo:
            bid=todo.pop()
            if bid in seen or bid not in bullets:continue
            seen.add(bid);b=bullets[bid]
            if any(any(a.get(k,0)>0 for k in ('atkPhys','atkMag','atkFire','atkThun','atkDark')) for a in attacks.get(b['atkId_Bullet'],[])):return True
            todo.extend((b['HitBulletID'],b['intervalCreateBulletId']))
        return False
    names = {int(i): n for i, n in spellraw['names'].items()}
    tables = raw['tables']
    def rows(table, version='installed'):
        return {r['ID']: r for r in tables[table][version]}
    def added(table):
        return sorted(rows(table, 'mmv').keys() - rows(table, 'original').keys())
    def texts(prefix):
        return {r['ID']: r['Text'] for f, entries in raw['text'].items() if f.startswith(prefix) for r in entries}
    weapon_names, art_names = texts('WeaponName'), texts('ArtsName')
    weapons, arts = rows('EquipParamWeapon'), rows('SwordArtsParam')
    defaults = pairs('src/arts_data.rs', 'DEFAULTS')
    bows = numbers('src/arts_data.rs', 'BOW_WEAPONS')
    fire = numbers('src/arts_data.rs', 'FIRE_WEAPONS')
    magic_weapons = numbers('src/arts_data.rs', 'MAGIC_WEAPONS')
    pool = numbers('src/arts_data.rs', 'POOL') | numbers('src/arts_data.rs', 'BOW_POOL')
    known = pairs('src/archmage_data.rs', 'SPELLS')
    random = numbers('src/archmage_data.rs', 'RANDOM_SPELLS')
    echo_path = 'work/compat/mmv/staged/echo_cast_data.rs' if (ROOT / 'work/compat/mmv/staged/echo_cast_data.rs').is_file() else 'src/echo_cast_data.rs'
    echo = {int(mid): set(map(int,re.findall(r'\d+', bullets))) for mid,bullets in
            re.findall(r'\((\d+),&\[([^]]*)\]\)',section(echo_path,'ROOTS'))}
    echo_areas = {int(x) for x in re.findall(r'\((\d+),',section(echo_path,'AREAS'))}
    echoed = set(echo) | echo_areas
    old_echo_roots = {int(bid) for _,value in re.findall(r'\((\d+),&\[([^]]*)\]\)',section('src/echo_cast_data.rs','ROOTS')) for bid in re.findall(r'\d+',value)}
    old_echo_roots.update(int(x) for x in re.findall(r'\(\d+,\s*(\d+),',section('src/echo_cast_data.rs','AREAS')))
    def magic_roots(m):
        return sorted({m[f'refId{i}'] for i in range(1,11) if m[f'refCategory{i}']==1 and m[f'refId{i}']>0})
    sword_spells = {int(x) for x in re.findall(r'\((\d+),',section('work/compat/mmv/staged/sword_map.rs','DEFAULT_BONUSES'))}
    lottery = {r['fields']['magicId'] for r in tables['MagicTableParam']['mmv'] if r['fields']['chanceWeight']>0}
    spell_report=[]
    for mid in sorted(magic.keys()-oldmagic.keys()):
        m=magic[mid]
        roots=magic_roots(m)
        spell_report.append(dict(id=mid,name=names.get(mid,''),kind=m['ezStateBehaviorType'],
            nativeLottery=mid in lottery,enchant=bool(m['isEnchant'] or m['isShieldEnchant']),
            roots=roots,oldEchoRootOverlap=sorted(set(roots)&old_echo_roots),damageChain=damage(roots),archmageKnown=mid in known,archmageRandom=mid in random,
            echoRegistered=mid in echoed,swordRegistered=mid in sword_spells))
    weapon_report=[]
    for wid in added('EquipParamWeapon'):
        row=weapons[wid];w=row['fields']
        weapon_report.append(dict(id=wid,name=weapon_names.get(wid,row['Name']),type=w['wepType'],
            art=w['swordArtsParamId'],defaultRegistered=wid in defaults,
            bow=w['wepType'] in (50,51,53),bowRegistered=wid in bows,
            fire=w['attackBaseFire'],fireRegistered=wid in fire,
            magic=w['attackBaseMagic'],magicRegistered=wid in magic_weapons,
            secondary=w['equippedSpell_R2'],paired=w['isDualBlade'],poise=w['saWeaponDamage']))
    art_report=[dict(id=i,name=art_names.get(arts[i]['fields']['textId'],arts[i]['Name']),
        type=arts[i]['fields']['swordArtsType'],sampleWeapon=arts[i]['fields']['sparringGroundsWeaponId'],
        randomRegistered=i in pool) for i in added('SwordArtsParam')]
    private=pairs('src/spirit_npcs.rs','NPCS');boss=set()
    for n in ['BOSS_NPCS','THREAT_NPCS','CAGE_BOSS_NPCS']:boss|=numbers('src/spirit_bosses.rs',n)
    npcs=rows('NpcParam')
    enemy_report=[dict(id=i,name=npcs[i]['Name'],hp=npcs[i]['fields']['hp'],team=npcs[i]['fields']['teamType'],
        nativeBossReward=npcs[i]['fields']['isSoulGetByBoss'],soulMap=i in private,bossMap=i in boss)
        for i in added('NpcParam')]
    changed=[]
    for wid,w in weapons.items():
        if wid in defaults and w['fields']['swordArtsParamId']!=defaults[wid]:
            changed.append(dict(id=wid,name=weapon_names.get(wid,w['Name']),compiled=defaults[wid],actual=w['fields']['swordArtsParamId']))
    summary=dict(newWeaponRows=len(weapon_report),newArtRows=len(art_report),newNpcRows=len(enemy_report),newMagicRows=len(spell_report),
        newMagicInLottery=sum(s['nativeLottery'] for s in spell_report),newMagicUnknownToArchmage=sum(not s['archmageKnown'] for s in spell_report),
        newMagicNotInEcho=sum(not s['echoRegistered'] for s in spell_report),newMagicNotInSword=sum(not s['swordRegistered'] for s in spell_report),
        newBows=sum(w['bow'] for w in weapon_report),newFireWeapons=sum(w['fire']>0 for w in weapon_report),newMagicWeapons=sum(w['magic']>0 for w in weapon_report),
        changedExistingWeaponDefaults=len(changed),newNpcsMissingSoulMap=sum(not n['soulMap'] for n in enemy_report))
    parry_weapons={int(x) for x in re.findall(r'\(\s*(\d+),',section('work/compat/mmv/staged/parry_data.rs','WEAPONS'))}
    parry_missing=[w['id'] for w in weapon_report if w['poise']>0 and w['id'] not in parry_weapons]
    original_reader=Reader((ROOT/'assets/original/action/script/c0000.hks').read_bytes());original_reader.function()
    original_functions={f['name'] for f in original_reader.functions}
    script=(ROOT/'dist/mmv/mod/action/script/c0000.hks').read_text(encoding='utf-8-sig')
    parsed=functions(script)
    callbacks=[]
    for name in sorted(set(parsed)-original_functions):
        if name.startswith('NRHex'):continue
        span=parsed[name];body=script[span['start']:span['end']]
        callbacks.append(dict(name=name,markers=sorted(set(re.findall(r'\b(?:6999\d+|9500\d+|9900\d+)\b',body))),sha256=hashlib.sha256(body.encode()).hexdigest()))
    summary.update(namedNewMagic=sum(bool(s['name']) for s in spell_report),newNamedDamageSpells=sum(bool(s['name']) and s['damageChain'] for s in spell_report),
                   newBossRewardNpcRows=sum(bool(n['nativeBossReward']) for n in enemy_report),newPositiveWeaponTypes=sum(w['type']>0 for w in weapon_report),
                   newWeaponsMissingParryTable=len(parry_missing),newArtsMissingRandomPool=sum(not a['randomRegistered'] for a in art_report),
                   newMagicWithOldEchoRoot=sum(bool(s['oldEchoRootOverlap']) for s in spell_report))
    report=dict(scope='Static parameter rows and runtime catalogues; not gameplay or availability certification',
        hashes=raw['hashes'],echoCatalogue=echo_path,summary=summary,spells=spell_report,weapons=weapon_report,arts=art_report,enemies=enemy_report,changedWeaponDefaults=changed,
        missingParryWeapons=parry_missing,newHksFunctions=callbacks,
        changedExistingMagicRoots=[mid for mid in sorted(magic.keys()&oldmagic.keys()) if magic_roots(magic[mid])!=magic_roots(oldmagic[mid])],
        catalogueHashes={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in
          [ROOT/'src/archmage_data.rs',ROOT/'src/arts_data.rs',ROOT/echo_path,ROOT/'src/echo_cast_data.rs',ROOT/'src/spirit_npcs.rs',ROOT/'src/spirit_bosses.rs',
           ROOT/'work/compat/mmv/staged/parry_data.rs',ROOT/'work/compat/mmv/staged/sword_map.rs',ROOT/'dist/mmv/mod/action/script/c0000.hks',
           OUT/'parameters.json',ROOT/'work/compat/mmv/sword-parameters.json',ROOT/'work/sword-range-export/parameters.json']})
    (OUT/'coverage.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
    print(json.dumps(summary,ensure_ascii=False,indent=2))
    print('New lottery spell examples:',[(s['id'],s['name']) for s in spell_report if s['nativeLottery']][:10])
    print('New weapon examples:',[(w['id'],w['name'],w['type'],w['art']) for w in weapon_report][:12])
    print('New art examples:',[(a['id'],a['name']) for a in art_report if a['name']][:12])
    print('New boss reward NPC examples:',[(n['id'],n['name']) for n in enemy_report if n['nativeBossReward']][:12])


if __name__=='__main__':main()
