"""Freeze verified native encounter identities, rewards and riding dependencies."""
from pathlib import Path
import hashlib,json,math,shutil
root=Path(__file__).resolve().parents[1]
work=root/'.work/cage-groups'
read=lambda p:json.loads(p.read_text(encoding='utf8'))
research=read(work/'encounters.json')['encounters']
metadata={m['map'].removesuffix('.msb.dcx'):m for m in read(work/'metadata.json')}
manifest={(r['kind'],r['name']):r for r in read(work/'manifest.json')}
selected=[('m46_50_00_00',46505840,'法姆兽人组合'),('m46_50_00_00',46505850,'混种组合'),
 ('m46_60_00_00',46605820,'三结晶人'),('m46_60_00_00',46605830,'石肤双王'),
 ('m46_60_00_00',46605840,'失乡骑士组合'),('m46_60_00_00',46605850,'诺克斯组合'),
 ('m46_80_00_00',46805800,'神皮双人'),('m48_50_00_00',48505800,'龙装树守卫与骑士'),
 ('m48_60_00_00',48605800,'黄金树守卫与骑士'),('m49_28_00_00',49285800,'黑夜骑兵双人'),
 ('m49_29_00_00',49295220,'亚人女王与剑圣'),('m60_43_38_10',1038105800,'双山妖'),
 ('m60_42_38_10',1028105820,'白金之子骑射手与狼')]
catalog=[];actor_index=37
for choice,(map_name,target,name) in enumerate(selected,37):
 source=next(p for p in [root/'assets/original/map/mapstudio'/f'{map_name}.msb.dcx',
  work/'extracted/map/mapstudio'/f'{map_name}.msb.dcx',work/'field-extracted/map/mapstudio'/f'{map_name}.msb.dcx'] if p.exists())
 assert hashlib.sha256(source.read_bytes()).hexdigest()==manifest['map',map_name]['sha256']
 dest=root/'assets/original/map/mapstudio'/source.name
 if not dest.exists():shutil.copyfile(source,dest)
 e=next(e for e in research if e['map']==map_name and e['target']==target)
 original=read(work/'maps'/f'{map_name}.json')['enemies']
 members=[dict(m,combat=True) for m in e['members']]
 rides=[json.loads(x['json']) for x in metadata[map_name]['events'] if x['type']=='Riding' and json.loads(x['json'])['RiderPartName'] in {m['Name'] for m in members}]
 for ride in rides:
  if ride['MountPartName'] not in {m['Name'] for m in members}:
   members.append(dict(next(m for m in original if m['Name']==ride['MountPartName']),combat=False))
 combat=[m for m in members if m['combat']]
 mounts={r['MountPartName']:r['RiderPartName'] for r in rides}
 places={}
 # Mounted pairs occupy one position. Spread independent bodies on a flat ring.
 independent=[m for m in combat if m['Name'] not in mounts]
 radius=6 if len(independent)>4 else 5
 for i,m in enumerate(independent):places[m['Name']]=[round(radius*math.sin(2*math.pi*i/len(independent)),4),0,round(radius*math.cos(2*math.pi*i/len(independent)),4)]
 for mount,rider in mounts.items():places[mount]=places[rider]
 frozen=[]
 for m in members:
  binding=m.get('rewardBinding') or {}
  frozen.append(dict(index=actor_index,sourceName=m['Name'],entity=m['entity'],npc=m['NpcParamId'],think=m['NpcThinkParamId'],
   alternateThink=binding.get('alternateThink',0),model=m['ModelName'],combat=m['combat'],offset=places[m['Name']]))
  actor_index+=1
 reward=next(json.loads(x['json']) for x in metadata[map_name]['events'] if x['type']=='TeamFight' and json.loads(x['json'])['TargetEntityGroupID']==target)
 bars=[dict(member=-1 if b['entity']==target else next(i for i,m in enumerate(members) if m['entity']==b['entity']),name=b['name']) for b in e['hudBars']]
 catalog.append(dict(choice=choice,map=map_name,target=target,name=name,sourceSha256=manifest['map',map_name]['sha256'],sourceEventSha256=manifest['event',map_name]['sha256'],sourceCommonEventSha256=manifest['event','common_func']['sha256'],members=frozen,bars=bars,
  riding=[dict(rider=next(i for i,m in enumerate(members) if m['Name']==r['RiderPartName']),mount=next(i for i,m in enumerate(members) if m['Name']==r['MountPartName']),sourceName=r['Name']) for r in rides],
  reward=reward,sourceHud=e['hud'],sourceLifecycle=e.get('lifecycle',[]),birthAnimation=20025 if map_name.startswith('m46_') else None))
assert len(catalog)==13 and actor_index==84
out=root/'data/reference/world/cage-encounters.json'
out.write_text(json.dumps(dict(encounterCount=50,actorCount=actor_index,groups=catalog),ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(f'13 native groups, {actor_index-37} added actors including mounts; {out}')
