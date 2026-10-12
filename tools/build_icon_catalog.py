"""Read-only native texture catalog: crop atlas entries and attach actual param/text uses."""
from pathlib import Path
from collections import defaultdict
import base64,csv,html,io,json,re,xml.etree.ElementTree as ET
from PIL import Image,ImageDraw,ImageFont
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'.work/icons'
PARAMS=json.loads((OUT/'native-params.json').read_text(encoding='utf8'))
TEXTS=json.loads((OUT/'native-texts.json').read_text(encoding='utf8'))
def texts(prefix):
 d={}
 for n,v in TEXTS.items():
  if n.startswith(prefix):d.update({int(k):v for k,v in v.items()})
 return d
NAMES={n:texts(n+'Name') for n in ['PermanentBuff','AttachEffect','Goods','Accessory','Magic','Arts']}
ENGLISH={}
for n in PARAMS:
 p=Path('D:/Program Files/Smithbox/Assets/PARAM/NR/Param Row Names/English')/(n+'.json')
 if p.exists():ENGLISH[n]={v['ID']:' / '.join(x for x in v['Entries'] if x) for v in json.loads(p.read_text(encoding='utf8'))['Entries']}
def clean(s):return re.sub(r'\s+',' ',s).strip()
ATLAS={'item':{},'status':{}}
for folder in sorted((OUT/'layouts').iterdir()):
 for p in folder.glob('*.layout'):
  kind='item' if 'ItemIcon' in p.name else 'status' if 'StatusIcon' in p.name else None
  if not kind:continue
  im=Image.open(OUT/'textures'/folder.name/(p.stem+'.dds')).convert('RGBA')
  for node in ET.parse(p).getroot():
   a=node.attrib;id=int(re.search(r'_(\d+)\.png$',a['name'])[1]);x,y,w,h=(int(a[k]) for k in ['x','y','width','height'])
   assert 0<=x<x+w<=im.width and 0<=y<y+h<=im.height
   icon=im.crop((x,y,x+w,y+h));target=OUT/'png'/kind/f'{id}.png';target.parent.mkdir(exist_ok=True,parents=True);icon.save(target)
   ATLAS[kind][id]={'id':id,'kind':kind,'file':str(target.relative_to(OUT)).replace('\\','/'),'source':folder.name+'/'+p.name,'width':w,'height':h}
USES={k:defaultdict(list) for k in ATLAS}
PRIMARY={k:defaultdict(list) for k in ATLAS}
def add(kind,id,name,param,row,field):
 if id not in ATLAS[kind]:return
 USES[kind][id].append({'name':clean(name),'param':param,'row':row,'field':field})
SP={r['ID']:r['fields'] for r in PARAMS['SpEffectParam']}
for r in PARAMS['PermanentBuffParam']:
 f=r['fields'];tid=SP.get(f['spEffectId'],{}).get('spEffectTextId_1',-1)
 name=NAMES['PermanentBuff'].get(tid,NAMES['PermanentBuff'].get(r['ID'],ENGLISH['PermanentBuffParam'].get(r['ID'],'')))
 add('item',f['iconId'],name,'PermanentBuffParam',r['ID'],'iconId')
 PRIMARY['item'][f['iconId']].append(clean(name))
for param,textname in [('EquipParamGoods','Goods'),('EquipParamAccessory','Accessory'),('Magic','Magic'),('SwordArtsParam','Arts')]:
 for r in PARAMS[param]:
  f=r['fields'];id=f.get('iconId',-1)
  # Native dormant-power goods use the 20xxx category code; PermanentBuff supplies the 21xxx item icon.
  if param=='EquipParamGoods' and f['goodsType']==11:continue
  name=NAMES[textname].get(r['ID'],ENGLISH[param].get(r['ID'],''))
  add('item',id,name,param,r['ID'],'iconId')
for r in PARAMS['AttachEffectParam']:
 f=r['fields'];tid=f.get('attachTextId',r['ID']);name=NAMES['AttachEffect'].get(tid,ENGLISH['AttachEffectParam'].get(r['ID'],''))
 add('status',f['statusIconId'],name,'AttachEffectParam',r['ID'],'statusIconId')
 if name:PRIMARY['status'][f['statusIconId']].append(clean(name))
for r in PARAMS['SpEffectParam']:
 f=r['fields'];tid=f.get('spEffectTextId_1',-1)
 name=NAMES['PermanentBuff'].get(tid,NAMES['AttachEffect'].get(tid,ENGLISH['SpEffectParam'].get(r['ID'],'')))
 for field in ['iconId','secondaryIconId','tertiaryIconId','quaternaryIconId']:
  add('status',f.get(field,-1),name,'SpEffectParam',r['ID'],field)
  if tid in NAMES['PermanentBuff'] and f.get(field,-1) in ATLAS['status']:PRIMARY['status'][f[field]].append(clean(name))
LABELS={
 21206:'攻击／攻击类特殊能力',21207:'魔法与祷告',21208:'防御／强韧／抗性',21222:'闪避／移动／特殊触发',21267:'卢恩收益',21291:'生命／回血／圣杯瓶恢复',21292:'专注值／FP',21293:'精力',21297:'大空洞的恩惠',
 22206:'永久能力：攻击',22207:'永久能力：魔法与祷告',22208:'永久能力：防御与抗性',22222:'永久能力：闪避与特殊触发',22267:'永久能力：卢恩',22291:'永久能力：生命与回血',22292:'永久能力：FP',22293:'永久能力：精力',22297:'永久能力：大空洞的恩惠',
 20206:'攻击类词条',20207:'魔法／祷告类词条',20208:'防御／抗性类词条',20222:'闪避／特殊能力类词条',20223:'死亡保护',20267:'卢恩／收益类词条',20291:'生命／回血类词条',20292:'FP类词条',20293:'精力类词条',20295:'战技类词条',20296:'技艺类词条',20297:'绝招类词条',
 20006:'临时攻击力增益',20008:'临时防御增益',20115:'绝招量表减少／累积受限',20422:'消除脚步声',20408:'防御增益（含低血减伤）',
}
ITEM_NATIVE={r['fields']['iconId'] for r in PARAMS['PermanentBuffParam']}
CATALOG=[]
for kind,icons in ATLAS.items():
 for id,info in sorted(icons.items()):
  uses=USES[kind][id]
  labels=list(dict.fromkeys(x for x in PRIMARY[kind][id] if x)) or list(dict.fromkeys(x['name'] for x in uses if x['name']))
  # Only curated labels relevant to the matching bank; 202xx is not a large item ID.
  label=LABELS.get(id) if (kind=='status' or id>=21000) else None
  info.update(title=label or (labels[0] if labels else '原版纹理；未找到已读取参数中的命名用途'),meanings=labels,uses=uses,nativeDormant=(kind=='item' and id in ITEM_NATIVE),referenced=bool(uses))
  CATALOG.append(info)
(OUT/'catalog.json').write_text(json.dumps(CATALOG,ensure_ascii=False,indent=2),encoding='utf8')
with (OUT/'图标用途表.csv').open('w',encoding='utf-8-sig',newline='') as f:
 w=csv.writer(f);w.writerow(['图标库','图标ID','概括','原版用途','原生潜在能力使用','引用行数'])
 for c in CATALOG:w.writerow(['大图标' if c['kind']=='item' else '小图标',c['id'],c['title'],'；'.join(c['meanings']),c['nativeDormant'],len(c['uses'])])
font=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',18)
small=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',14)
def sheet(name,entries,columns=6,size=112):
 cw,ch=200,190;rows=(len(entries)+columns-1)//columns
 im=Image.new('RGB',(columns*cw,70+rows*ch),(24,28,35));d=ImageDraw.Draw(im);d.text((20,18),name,font=font,fill='#eadbc0')
 for j,c in enumerate(entries):
  x=(j%columns)*cw;y=70+(j//columns)*ch
  icon=Image.open(OUT/c['file']).convert('RGBA');icon.thumbnail((size,size));im.paste(icon,(x+(cw-icon.width)//2,y),icon)
  d.text((x+12,y+size+5),str(c['id']),font=font,fill='white')
  title=c['title'];parts=[title[k:k+12] for k in range(0,min(len(title),24),12)]
  for k,line in enumerate(parts):d.text((x+12,y+size+32+k*20),line,font=small,fill='#c9cbd0')
 path=OUT/(name+'.jpg');im.save(path,quality=92);return path
native=[c for c in CATALOG if c['nativeDormant']]
sheet('潜在能力原版大图标',native)
statuses=[c for c in CATALOG if c['kind']=='status']
sheet('原版小图标（1）',statuses[:84],size=80)
sheet('原版小图标（2）',statuses[84:],size=80)
common_ids=[21206,21207,21208,21222,21267,21291,21292,21293,21297]
sheet('常用大图标', [c for id in common_ids for c in CATALOG if c['kind']=='item' and c['id']==id],columns=3)
common_status=[22206,22207,22208,22222,22267,22291,22292,22293,22297,20295,20296,20297,20422,20408,20115]
sheet('常用小图标',[c for id in common_status for c in CATALOG if c['kind']=='status' and c['id']==id],columns=5,size=80)
# All sprite previews and searchable usage text are embedded for a portable HTML catalog.
embedded=[]
for c in CATALOG:
 data={k:v for k,v in c.items() if k!='uses'}
 data['refs']=[f"{u['param']} {u['row']} {u['field']}: {u['name']}" for u in c['uses']]
 preview=Image.open(OUT/c['file']).convert('RGBA');preview.thumbnail((112,112));buffer=io.BytesIO();preview.save(buffer,format='WEBP',quality=90,method=4)
 data['image']='data:image/webp;base64,'+base64.b64encode(buffer.getvalue()).decode('ascii')
 embedded.append(data)
page='''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Nightreign 原版图标图鉴</title>
<style>body{font:15px/1.55 system-ui,"Microsoft YaHei";background:#171b22;color:#e6e8ec;margin:24px}h1{font-size:26px}p{max-width:1000px;color:#c0c5ce}nav{position:sticky;top:0;padding:14px 0;background:#171b22;z-index:2;display:flex;gap:10px;flex-wrap:wrap}button,input,select{font:inherit;padding:8px 12px;background:#28313e;color:white;border:1px solid #536273;border-radius:6px}button.active{background:#5e4929;border-color:#c5a46b}#grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:14px}article{background:#232a35;border:1px solid #394454;border-radius:10px;padding:16px;overflow:hidden}img{width:120px;height:120px;object-fit:contain;display:block;margin:auto}article.status img{width:80px;height:80px;margin:20px auto}h2{font-size:17px;margin:10px 0}ul{padding-left:20px;color:#cbd2dd}details{font-size:12px;color:#aab7c8;overflow-wrap:anywhere}summary{cursor:pointer}small{color:#b8a581}#count{margin:12px 0}</style>
<h1>Nightreign 原版图标图鉴</h1><p>从当前游戏纹理、参数和中文文本提取。用途表示原版哪些词条共用该图标，图标本身不会赋予能力。大图标与小图标属于两个独立的库。优先显示原版潜在能力使用的大图标；其余物品、魔法和护符图标可作为候选素材，移用于海克斯后仍需游戏内确认。没有命名引用的纹理已单独标记。</p>
<p>缺图原因已核对：当前海克斯把大图标 21206／21291 也写到了小图标字段。小图标库没有这两个编号；原版永久能力对应使用 22206／22291。人是猫使用的 20422 存在于小图标库。此图鉴没有修改 Mod。</p>
<nav><button class="active" data-tab="native">潜在能力大图标</button><button data-tab="status">全部小图标</button><button data-tab="item">全部大图标素材</button><input id="query" type="search" placeholder="搜索编号、卢恩、回血、隐身…"><select id="usage"><option value="all">全部用途</option><option value="named">仅有参数引用</option></select></nav><div id="count"></div><main id="grid"></main>
<script>const data=__DATA__;let tab='native';const q=document.querySelector('#query'),u=document.querySelector('#usage'),grid=document.querySelector('#grid');function render(){const term=q.value.trim().toLowerCase();const rows=data.filter(x=>(tab==='native'?x.nativeDormant:x.kind===tab)&&(u.value==='all'||x.referenced)&&(!term||[x.id,x.title,...x.meanings,...x.refs].join(' ').toLowerCase().includes(term)));document.querySelector('#count').textContent=`显示 ${rows.length} 个图标。点击「原版引用」可以查看具体词条和参数行。`;grid.replaceChildren();for(const x of rows){const a=document.createElement('article');a.className=x.kind;const img=new Image();img.loading='lazy';img.src=x.image;img.alt=x.title;a.append(img);const h=document.createElement('h2');h.textContent=`${x.id} · ${x.title}`;a.append(h);const s=document.createElement('small');s.textContent=x.kind==='status'?'小图标 / Status':'大图标 / Item';a.append(s);const ul=document.createElement('ul');for(const v of x.meanings.slice(0,5)){const li=document.createElement('li');li.textContent=v;ul.append(li)}a.append(ul);const d=document.createElement('details'),su=document.createElement('summary');su.textContent=`原版引用 ${x.refs.length} 行 · ${x.meanings.length} 种用途`;d.append(su);const p=document.createElement('p');p.textContent=x.meanings.join('；')||'未找到已读取参数中的命名用途。';d.append(p);const refs=document.createElement('div');for(const r of x.refs){const line=document.createElement('div');line.textContent=r;refs.append(line)}d.append(refs);a.append(d);grid.append(a)}}document.querySelectorAll('[data-tab]').forEach(b=>b.onclick=()=>{tab=b.dataset.tab;document.querySelectorAll('[data-tab]').forEach(v=>v.classList.toggle('active',v===b));render()});q.oninput=render;u.onchange=render;render();</script></html>'''
page=page.replace('__DATA__',json.dumps(embedded,ensure_ascii=False).replace('</','<\\/'))
(OUT/'海克斯图标图鉴.html').write_text(page,encoding='utf8')
CURRENT=json.loads((OUT/'current-hexes.json').read_text(encoding='utf8'))
missing={n:[{'row':r['ID'],'field':field,'id':r['fields'][field]} for r in CURRENT[n] for field in ['iconId' if n!='AttachEffectParam' else 'statusIconId'] if r['fields'][field]>0 and r['fields'][field] not in ATLAS['status']] for n in ['SpEffectParam','AttachEffectParam']}
(OUT/'validation.json').write_text(json.dumps({'nativeDormantLargeCount':len(native),'allItemSprites':len(ATLAS['item']),'statusSprites':len(ATLAS['status']),'allCropsInBounds':True,'metadataSource':'current original regulation 10350000, original zhocn FMGs, original high-resolution layout+TPF','missingCurrentStatusIds':missing,'modModified':False},ensure_ascii=False,indent=2),encoding='utf8')
print(f'PASS: {len(native)} native dormant icons, {len(ATLAS["item"])} item sprites, {len(ATLAS["status"])} status sprites; searchable catalog and usage CSV written; no mod files changed.')
