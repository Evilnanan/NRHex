"""Resolve native health-bar event bindings into original NPC parameter IDs."""
from pathlib import Path
import argparse,json,struct,hashlib
root=Path(__file__).resolve().parents[1];work=root/'.work/cage-groups'
load=lambda p:json.loads(p.read_text(encoding='utf8'))
args=argparse.ArgumentParser();args.add_argument('--inplace',action='store_true');options=args.parse_args()
for evidence in load(work/'manifest.json'):
 name=evidence['name'];kind=evidence['kind'];relative=('event/'+name+'.emevd.dcx') if kind=='event' else ('map/mapstudio/'+name+'.msb.dcx')
 candidates=[root/'assets/original'/relative,work/'extracted'/relative,work/'field-extracted'/relative]
 source=next((p for p in candidates if p.is_file()),None)
 assert source and hashlib.sha256(source.read_bytes()).hexdigest()==evidence['sha256'].lower(),('Native boss catalogue input changed',relative)
events={p.stem:{e['ID']:e for e in load(p)} for p in (work/'events').glob('*.json')}
common={**events.get('common',{}),**events.get('common_func',{})}
bosses=set();proof=[];visits=0
for p in sorted((work/'maps').glob('*.json')):
 name=p.stem;local={**common,**events.get(name,{}),**events.get(name+'_dlc01',{})}
 wanted=set();seen=set()
 def walk(event,args=b''):
  global visits
  key=(event['ID'],args)
  if key in seen:return
  seen.add(key);visits+=1
  for index,instruction in enumerate(event['instructions']):
   data=bytearray.fromhex(instruction['hex'])
   for binding in event['Parameters']:
    if binding['InstructionIndex']!=index:continue
    start=binding['SourceStartByte'];n=binding['ByteCount'];target=binding['TargetStartByte']
    if start+n<=len(args):data[target:target+n]=args[start:start+n]
   bank,op=instruction['Bank'],instruction['ID']
   if bank==2003 and op in [11,102] and len(data)>=12:
    entity=struct.unpack_from('<I',data,4 if op==11 else 8)[0]
    if entity>20000:wanted.add(entity)
   elif bank==2000 and op in [0,6] and len(data)>=8:
    target=struct.unpack_from('<I',data,4)[0]
    if target in local:walk(local[target],bytes(data[8:]))
 if 0 in local:walk(local[0])
 # Independent fixed-entity HUD events also carry useful source evidence even
 # when their native initialization is deferred by linked DLC events.
 for e in events.get(name,{}).values():
  if not e['Parameters']:walk(e)
 for enemy in load(p)['enemies']:
  if enemy['entity'] in wanted or any(g in wanted for g in enemy['groups'] if g):
   if enemy['NpcParamId']>0:
    bosses.add(enemy['NpcParamId']);proof.append({'map':name,'entity':enemy['entity'],'npc':enemy['NpcParamId']})
# Native gaol/tower templates and their private clones retain the same role.
for b in load(root/'data/reference/world/cage-bosses.json'):bosses.add(b['npc'])
groups=load(root/'data/reference/world/cage-encounters.json')['groups']
combat=[m for g in groups for m in g['members'] if m['combat']]
cage_bosses=sorted(set(range(95300000,95300037))|{95300000+m['index'] for m in combat})
for m in combat:bosses.add(m['npc'])
noncombat={m['npc'] for g in groups for m in g['members'] if not m['combat']}-{m['npc'] for m in combat}
bosses-=noncombat
out=root/'.work/new-hexes-01733/staged';out.mkdir(parents=True,exist_ok=True)
threats=load(root/'data/reference/world/spirit-threat-npcs.json')
assert threats['sourceRegulationSha256'].lower()=='876a3ca279a4561d0c69f81fe5e510c75c59ab9a8a201694f8cf4a42c91e0268'
assert threats['ids']==sorted(set(threats['ids'])) and 42700030 in threats['ids']
(out/'spirit_bosses.rs').write_text('// Generated from native boss HUD event bindings.\npub const BOSS_NPCS: &[i32] = &['+','.join(map(str,sorted(bosses)))+'];\npub const CAGE_BOSS_NPCS: &[i32] = &['+','.join(map(str,cage_bosses))+'];\n',encoding='utf8')
with (out/'spirit_bosses.rs').open('a',encoding='utf-8') as f:f.write('pub const THREAT_NPCS: &[i32] = &['+','.join(map(str,threats['ids']))+'];\n')
if options.inplace:(root/'src/spirit_bosses.rs').write_bytes((out/'spirit_bosses.rs').read_bytes())
(out/'boss-catalog-validation.json').write_text(json.dumps({'bossNpcCount':len(bosses),'eventInstancesResolved':visits,'bindings':proof,'source':'original boss HUD/link events and native map enemy parts','runtimeThreatTagFallback':True,'gameplayVerified':False},indent=2))
assert len(bosses)>40
print('PASS: native health-bar bindings resolved;',len(bosses),'boss NPC variants,',visits,'event instances')
