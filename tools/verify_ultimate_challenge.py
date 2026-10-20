import argparse
import hashlib
import json
import os
import subprocess
from local_paths import ROOT, GAME

parser = argparse.ArgumentParser()
parser.add_argument("--staged", action="store_true")
args = parser.parse_args()
work = ROOT / "work/ultimate-challenge"
work.mkdir(parents=True, exist_ok=True)
environment = os.environ.copy()
environment.setdefault("NRHEX_NATIVE_EXE", str(GAME / "nightreign.exe"))

def run(command, filename, expected):
    result = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True, errors="replace")
    output = result.stdout + result.stderr
    (work / filename).write_text(output, encoding="utf-8")
    assert result.returncode == 0 and expected in output, output

if args.staged:
    fixture = work / "staged/installed-effects.param"
    audit = json.loads((work / "staged/validation.json").read_text())
    assert audit["audit"] == dict(owner=95004000, buff=95004001, seconds=15, maxHpRate=1.5, targetPriority=9999, historyBit=37)
    assert audit["existingRowsAndUnrelatedTablesUnchanged"] and audit["onlyTwoGuardedCleanupEventsExtended"]
    assert hashlib.sha256((work / "staged/regulation.bin").read_bytes()).hexdigest().upper() == audit["outputSha256"]
else:
    run(["dotnet", "run", "--project", str(ROOT / "tools/HexBuild"), "--", str(ROOT), "--ultimate-challenge-check"], "installed-check.log", "PASS: installed Ultimate Challenge")
    fixture = work / "installed-effects.param"
environment["NRHEX_CHALLENGE_FIXTURES"] = str(fixture)
run(["cargo", "test", "--release", "--lib", "--features", "cage-demand", "native_ultimate_challenge_hp_refresh_expiry_and_death", "--", "--ignored", "--test-threads=1"], "native-test.log", "1 passed; 0 failed;")
report = dict(staged=args.staged, nativeExecutionPassed=True, fixtureSha256=hashlib.sha256(fixture.read_bytes()).hexdigest(),
              cases=["successful/rejected/remote/missing-owner/reset/dead/positive-HP-down notifications",
                     "formal activation pulse 95000404; old action marker 6999615 does not activate",
                     "deferred application becomes visible after three updates and heals exactly once",
                     "full 1000/1000 becomes 1500/1500 after confirmed application",
                     "down-before-visibility and confirmation timeout cannot heal later",
                     "600/1000 to 1100/1500; same-ID refresh heals 500 without stacking",
                     "native independent-current-HP filter excludes automatic proportional heal",
                     "expiry/removal preserves 900 HP and caps 1400 HP to the restored 1000 maximum",
                     "down/death clears both HP and priority effect; positive rescue HP does not reactivate",
                     "revival remains unbuffed until the next ultimate",
                     "successful death-action notifications, native down transition and direct death callbacks",
                     "other maximum-HP sources retained; failed application cannot heal"],
              stubs=["effect allocation and row lookup", "actor baseline stats/authority", "removal skips allocator teardown after native unlink"],
              gameProcessAccessed=False, gameplayVerified=False, multiplayerVerified=False)
(work / "native-validation.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
print("PASS: Ultimate Challenge native HP, refresh, expiry and death paths; no game process accessed")
