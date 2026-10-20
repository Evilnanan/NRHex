import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import {readFile} from 'node:fs/promises';

const [modulePath, nativePath, outputPath] = process.argv.slice(2);
assert(modulePath && nativePath && outputPath, 'Usage: node verify_heart_ready_fog.mjs fxr.js native.fxr output.fxr (@cccode/fxr 32.1.2)');
const {FXR} = await import(pathToFileURL(resolve(modulePath)));
const original = JSON.parse(JSON.stringify(await FXR.read(resolve(nativePath))));
const modified = JSON.parse(JSON.stringify(await FXR.read(resolve(outputPath))));
assert.equal(original.id, 4090);
assert.equal(modified.id, 950941);
const movement = effect => effect.root.nodes[0].nodes[0].nodes[0].nodes[0].configs[0].particleMovement;
assert.equal(movement(original).type, 105);
assert.equal(movement(original).followRotation, false);
assert.deepEqual(movement(original).followFactor.components[0].map(frame => frame.value), [1, 0]);
const expected = structuredClone(original);
expected.id = 950941;
for (const keyframe of movement(expected).followFactor.components[0]) {
    keyframe.value = 1;
    keyframe.t1 = 0;
    keyframe.t2 = 0;
}
const config = expected.root.nodes[0].nodes[0].nodes[0].nodes[0].configs[0];
for (const axis of ['sizeX', 'sizeY', 'sizeZ']) config.emitterShape[axis] *= .5;
for (const field of ['speed', 'scaleX', 'scaleY']) config.particleModifier[field].value *= .5;
assert.deepEqual(modified, expected);
const originalBytes = await readFile(nativePath);
const modifiedBytes = await readFile(outputPath);
assert.equal(originalBytes.length, modifiedBytes.length);
const permitted = new Set([12, 19448, 19452, 19456, 19476, 19488, 19500, 20348, 20368, 20372].flatMap(offset => [offset, offset + 1, offset + 2, offset + 3]));
for (let offset = 0; offset < originalBytes.length; offset++) {
    if (!permitted.has(offset)) assert.equal(modifiedBytes[offset], originalBytes[offset]);
}
console.log('PASS: parsed FXR differs only in private ID, constant translation-follow curve and half-size spatial fields; all other bytes/semantics preserved');
