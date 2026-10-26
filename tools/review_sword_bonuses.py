"""Read-only review of the user's copied spell balance table."""
from pathlib import Path
import csv,collections,json,math
root=Path(__file__).resolve().parents[1]
source=root/'dist/natives/亮出你的剑射程 - 副本.csv'
original=root/'dist/natives/亮出你的剑射程.csv'
def read(p):
 with p.open(encoding='utf-8-sig',newline='') as f:
  reader=csv.DictReader(f);return reader.fieldnames,list(reader)
header,rows=read(source);_,old=read(original)
old={int(r['MagicID']):r for r in old}
graph=json.loads((root/'.work/combat/sword-ranges.json').read_text(encoding='utf8'))
spells={s['mid']:s for s in graph['spells']}
errors=[];parsed={};shape=[]
for line,r in enumerate(rows,2):
 if None in r or any(v is None for v in r.values()):shape.append(line)
 try:
  mid=int(r['MagicID']);raw=r['估算增伤百分比'].strip();value=float(raw) if raw else None
  if mid in parsed:errors.append(f'Duplicate MagicID {mid}')
  if value is not None and (not math.isfinite(value) or not 0<=value<=200):errors.append(f'Invalid percentage {mid}: {value}')
  parsed[mid]=(r,value)
 except (ValueError,KeyError,TypeError) as e:errors.append(f'Line {line}: {e}')
def details(ids):
 return [{'id':i,'name':parsed[i][0]['名称'],'bonusPercent':parsed[i][1],'mappedAttacks':spells.get(i,{}).get('attacks',[])} for i in sorted(ids)]
eligible={i for i,s in spells.items() if s['eligible']}
mapped={i for i in eligible if spells[i]['attacks']}
named=collections.defaultdict(list)
for i,(r,v) in parsed.items():named[r['名称']].append(i)
changes=[]
for i,(r,v) in parsed.items():
 if i in old:
  diff={k:{'original':old[i].get(k),'copy':r[k]} for k in header if k not in ('估算增伤百分比','估算射程米') and r[k]!=old[i].get(k)}
  if float(r['估算射程米'])!=float(old[i]['估算射程米']):diff['估算射程米']={'original':old[i]['估算射程米'],'copy':r['估算射程米']}
  if diff:changes.append({'id':i,'fields':diff})
report={'source':str(source),'headers':header,'rowCount':len(rows),'uniqueIds':len(parsed),
 'errors':errors,'malformedLines':shape,'unknownIds':sorted(set(parsed)-set(spells)),
 'missingOrdinarySpellIds':sorted(eligible-set(parsed)),
 'missingMappedSpellIds':sorted(mapped-set(parsed)),
 'removedSpecialRows':len(set(old)-eligible-set(parsed)),
 'eligibleSpellCount':len(eligible),'mappedSpellCount':len(mapped),
 'nonBonusChanges':changes,'percentageHistogram':dict(sorted(collections.Counter(v for r,v in parsed.values() if v is not None).items())),
 'filledBonusCount':sum(v is not None for r,v in parsed.values()),'blankBonusCount':sum(v is None for r,v in parsed.values()),
 'damageSpellCount':sum(spells[i]['damaging'] for i in eligible),
 'damageSpellsMissingBonus':details(i for i in eligible if spells[i]['damaging'] and (i not in parsed or parsed[i][1] is None)),
 'blankBonusWithMappedNonDamageAttack':details(i for i,(r,v) in parsed.items() if v is None and i in mapped),
 'maxBonusRows':details(i for i,(r,v) in parsed.items() if v==200),
 'overOld50PercentCapCount':sum(v is not None and v>50 for r,v in parsed.values()),
 'positiveBonusWithoutMappedAttack':details(i for i,(r,v) in parsed.items() if v is not None and v>0 and i not in mapped),
 'zeroBonusRows':details(i for i,(r,v) in parsed.items() if v==0),
 'ordinarySharedAttackIds':graph['sharedSpellAttacks'],
 'duplicateNames':{k:v for k,v in named.items() if len(v)>1},
 'samples':details(i for i in [4000,4007,4021,4080,4100,4380,4381,6460,6940,6941] if i in parsed),
 'runtimeCompatibility':{'currentLoaderRequiresMissingColumn':'手动射程米','currentLoaderIgnoresColumn':'估算增伤百分比','currentMaximumBonus':50,'200PercentBonusMeansFactor':3.0,'currentLoadedFilename':'亮出你的剑射程.csv'}}
out=root/'.work/combat/sword-bonus-review.json';out.write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8')
print(str(out))
