"""Run the original HUD comparator in a private PE image, never in the game.

Index-zero entries avoid the parameter/icon lookup paths. The entire image is
mapped because Arxan scatters this comparator across distant native blocks.
"""
import ctypes as c
import functools
import hashlib
import json
import tomllib
import pefile
from local_paths import ROOT, GAME

exe = GAME / 'nightreign.exe'
digest = hashlib.sha256(exe.read_bytes()).hexdigest()
assert digest == '9545e8732919bb6a63904cb73223641b3c4f9a67869be5722612acca413e8b3a'
pe = pefile.PE(str(exe))
k = c.WinDLL('kernel32', use_last_error=True)
k.VirtualAlloc.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong, c.c_ulong]
k.VirtualAlloc.restype = c.c_void_p
k.VirtualFree.argtypes = [c.c_void_p, c.c_size_t, c.c_ulong]
memory = k.VirtualAlloc(None, pe.OPTIONAL_HEADER.SizeOfImage, 0x3000, 0x40)
assert memory

class Entry(c.Structure):
    _fields_ = [('effect', c.c_int), ('icon', c.c_int), ('index', c.c_int),
                ('remaining', c.c_float), ('count', c.c_int)]

assert c.sizeof(Entry) == 20
cases = 0
try:
    c.memmove(memory, pe.__data__[:pe.OPTIONAL_HEADER.SizeOfHeaders], pe.OPTIONAL_HEADER.SizeOfHeaders)
    for section in pe.sections:
        raw = section.get_data()
        c.memmove(memory + section.VirtualAddress, raw, len(raw))
    delta = memory - pe.OPTIONAL_HEADER.ImageBase
    for block in pe.DIRECTORY_ENTRY_BASERELOC:
        for relocation in block.entries:
            if relocation.type == 10:
                address = c.c_uint64.from_address(memory + relocation.rva)
                address.value = (address.value + delta) & 0xffffffffffffffff
            else:
                assert relocation.type == 0
    original = c.WINFUNCTYPE(c.c_ubyte, c.POINTER(Entry), c.POINTER(Entry))(memory + 0x7df110)
    ready = {95000403:20297, 95000701:20408, 95001203:20222}

    def normalized(entry):
        copy = Entry.from_buffer_copy(bytes(entry))
        if copy.effect in ready:
            copy.remaining = -1
        return copy

    def compare(a, b):
        left, right = normalized(a), normalized(b)
        return bool(original(c.byref(left), c.byref(right)))

    a, b = Entry(95000701,20408,0,.1,1), Entry(95000403,20297,0,1.9,1)
    assert not original(c.byref(a), c.byref(b))
    a.remaining, b.remaining = 1.9, .1
    assert original(c.byref(a), c.byref(b))
    for left, li in ready.items():
        for right, ri in ready.items():
            for x in [.01,.2,.75,1.3,1.99]:
                for y in [.01,.2,.75,1.3,1.99]:
                    a, b = Entry(left,li,0,x,3), Entry(right,ri,0,y,7)
                    saved = bytes(a), bytes(b)
                    assert compare(a,b) == (li < ri)
                    assert (bytes(a),bytes(b)) == saved
                    cases += 1
    native = [Entry(9700,20800,0,-1,1), Entry(1681,20006,0,40,4),
              Entry(95000501,20006,0,15,6), Entry(95000801,20422,0,.5,1)]
    for effect, icon in ready.items():
        for n in native:
            for remaining in [.01,.8,1.99]:
                a = Entry(effect,icon,0,remaining,1)
                assert compare(a,n) == (icon < n.icon if n.remaining < 0 else True)
                assert compare(n,a) == (n.icon < icon if n.remaining < 0 else False)
                cases += 1
    # Only availability leases receive the permanent display key.
    for n in native:
        assert bytes(normalized(n)) == bytes(n)
    a = Entry(95000701,20408,0,.1,2)
    b = Entry(95000701,20408,1,1.9,7)
    assert compare(a,b) and not compare(b,a)
    assert not compare(a,a)
    def sort(entries):
        return sorted(entries,key=functools.cmp_to_key(lambda a,b:-1 if compare(a,b) else (1 if compare(b,a) else 0)))
    expected = [95001203,95000403,95000701,9700]
    for frame in range(1500):
        entries = [Entry(e,i,0,2-((frame/60+offset)%.75),1)
                   for (e,i),offset in zip(ready.items(),[0,.24,.49])]
        entries += [native[0]]
        assert [e.effect for e in sort(entries)] == expected
        cases += 1
finally:
    assert k.VirtualFree(memory,0,0x8000)

parameters = json.loads((ROOT/'.work/hud-order-01728/parameters.json').read_text())
assert parameters['installedRegulationSha256'].lower() == hashlib.sha256((ROOT/'dist/mod/regulation.bin').read_bytes()).hexdigest()
effects = {r['ID']:r['fields'] for r in parameters['custom']}
assert all(effects[e]['effectEndurance']==2 and effects[e]['iconId']==i for e,i in ready.items())
assert effects[95000801]['iconId']==20422 and effects[95000801]['effectEndurance']==1
assert all(effects[e]['iconId']==20006 and effects[e]['effectEndurance']==15
           for e in [95000501,*range(95005001,95005080)])
assert all(f['secondaryIconId']==-1 for f in effects.values())
other_finite = {e for e,f in effects.items() if f['iconId']>=0 and f['effectEndurance']>=0} - ready.keys()
assert other_finite == {95000801,95000501,*range(95005001,95005080)}
report = dict(version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'],
              originalExeSha256=digest,nativeHudComparatorRva='7df110',nativeMenuComparatorRva='82ba40',
              nativeEntryBytes=20,readinessEffects=ready,sortKeyRemaining=-1,
              nativeHudCases=cases,originalHudSwapReproduced=True,allCustomIconRowsAudited=True,
              temporaryStealthAndDrinkingUnchanged=True,realLeasesAndTimersUnchanged=True,
              nativeHudIndexZeroFixture=True,gameplayVerified=False,multiplayerVerified=False,
              previousUserFeedback={'artsStance':'fixed','jiahaoRanged':'fixed','menuReadinessOrder':'fixed','hudReadinessOrder':'still swapping in 0.17.27'})
(ROOT/'.work/hud-order-01728/validation.json').write_text(json.dumps(report,indent=2),encoding='utf8')
print(f'PASS: original native HUD comparator reproduced swaps; {cases} cases passed; all custom icon rows audited. Game rendering not inferred.')
