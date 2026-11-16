// node tools/build_cold_juice_vfx.mjs <fxr-32.1.2/dist/fxr.js> <extracted-fxr-dir> <output-dir>
// Inputs: native f000334307/334308.fxr and f000334308.ffxreslist.
import {pathToFileURL} from 'node:url';
import {resolve,join} from 'node:path';
import {mkdir,copyFile,readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const [library,input,output]=process.argv.slice(2);
if(!output)throw Error('Expected library, native input directory, output directory');
const {FXR}=await import(pathToFileURL(resolve(library)).href);
const native=await readFile(join(input,'f000334308.fxr'));
const source=await readFile(join(input,'f000334307.fxr'));
const hash=b=>createHash('sha256').update(b).digest('hex');
if(hash(native)!=='6a40328249923240469d9e0bbb76fc7909efa611f7b9d6d291682a215bebc031'||
   hash(source)!=='359d865a904e95add73382fa546d32437992dccefef1033223c6aa9fe50532f8')
  throw Error('Native FXR baseline changed; relocate nodes before patching');
const visual=await FXR.read(join(input,'f000334308.fxr'));
const start=await FXR.read(join(input,'f000334307.fxr'));
const old=JSON.parse(JSON.stringify(visual)),sound=start.root.nodes[0].configs[0].nodeAudio;
if(sound.type!==75||sound.sound!==523877||visual.root.nodes[0].configs[0].nodeAudio.type!==0)throw Error('Native audio structure changed');
visual.id=950957;visual.root.nodes[0].configs[0].nodeAudio=sound;
const expected=JSON.parse(JSON.stringify(visual));
// Do not serialize the visual tree: library roundtrips rewrite unknown native data.
// The empty audio slot already exists; its three fields fit into section padding.
const data=Buffer.from(native),slot=0x890,from=0xcf0;
const empty=Buffer.alloc(64);empty[3]=1;
if(!native.subarray(slot,slot+64).equals(empty))throw Error('Audio slot is not empty');
const fields=native.readUInt32LE(0x60)+native.readUInt32LE(0x64)*4;
if(fields!==0xd5e4||native.readUInt32LE(0x70)!==fields+12||
   !native.subarray(fields,fields+12).equals(Buffer.alloc(12)))throw Error('Field padding changed');
const descriptor=Buffer.from(source.subarray(from,from+64));
const values=source.subarray(descriptor.readUInt32LE(0x20),descriptor.readUInt32LE(0x20)+12);
if(descriptor.readUInt16LE(0)!==75||descriptor.readUInt32LE(8)!==3||
   values.readUInt32LE(0)!==523877||values.readFloatLE(4)!==1||values.readUInt32LE(8)!==1)
  throw Error('Native sound descriptor changed');
descriptor.writeBigUInt64LE(BigInt(fields),0x20);
descriptor.copy(data,slot);values.copy(data,fields);
data.writeUInt32LE(950957,12);data.writeUInt32LE(native.readUInt32LE(0x64)+3,0x64);
let changes=0;
for(let i=0;i<data.length;i++)if(data[i]!==native[i]){
  if(![[12,16],[0x64,0x68],[slot,slot+64],[fields,fields+12]].some(([a,b])=>i>=a&&i<b))
    throw Error('Unrelated native byte changed');
  changes++;
}
if(changes!==14||hash(data)!=='f7f36eee917b5bf688ddb48a0a71105e421dafbc198f8d329c8f92bb7d2d405a')
  throw Error('Unexpected output bytes');
await mkdir(output,{recursive:true});const path=join(output,'f000950957.fxr');
await writeFile(path,data);
const result=JSON.parse(JSON.stringify(await FXR.read(path)));
if(JSON.stringify(result)!==JSON.stringify(expected))throw Error('FXR roundtrip differs');
result.id=old.id;result.root.nodes[0].configs[0].nodeAudio=null;
if(JSON.stringify(result)!==JSON.stringify(old))throw Error('Unrelated visual structure changed');
await copyFile(join(input,'f000334308.ffxreslist'),join(output,'f000950957.ffxreslist'));
console.log('PASS: 14 native bytes changed; only private ID and native storm sound added; no visual reserialization');
