import argparse
import hashlib
import json
import os
import subprocess
from local_paths import ROOT, GAME

parser = argparse.ArgumentParser()
parser.add_argument("--installed", action="store_true")
args = parser.parse_args()
stage = ROOT / "work/ultimate-timing/staged"
actions = json.loads((stage / "actions-validation.json").read_text())
parameters = json.loads((stage / "parameter-validation.json").read_text())
assert actions["pulse"] == parameters["addedEffect"] == 95000404
assert actions["entryOnlyEvent"] == 302
assert actions["originalEventsUnchanged"] and actions["drainAndFollowupsExcluded"]
assert parameters["existingRowsUnchanged"]
expected = {599010, 600000, 600005, 600100, 600200, 605000, 610000, 615000,
            620000, 620005, 630000, 635000, 640000, 650000, 660000, 670000,
            680000, 690000, 695000}
assert len(actions["starts"]) == 19
assert {entry["animation"] for entry in actions["starts"]} == expected
for name, installed, audit in [
    ("c0000.anibnd.dcx", "chr/c0000.anibnd.dcx", actions),
    ("regulation.bin", "regulation.bin", parameters),
]:
    assert hashlib.sha256((stage / name).read_bytes()).hexdigest().upper() == audit["outputSha256"]
    wanted = audit["outputSha256" if args.installed else "inputSha256"]
    assert hashlib.sha256((ROOT / "dist/mod" / installed).read_bytes()).hexdigest().upper() == wanted
environment = os.environ.copy()
environment.setdefault("NRHEX_NATIVE_EXE", str(GAME / "nightreign.exe"))
result = subprocess.run(
    ["cargo", "test", "--release", "--features", "cage-demand", "--lib",
     "native_ultimate_timing_entry_only_dispatch", "--", "--ignored", "--test-threads=1"],
    cwd=ROOT, env=environment, capture_output=True, text=True, errors="replace")
output = result.stdout + result.stderr
(stage.parent / "native-test.log").write_text(output, encoding="utf-8")
assert result.returncode == 0 and "1 passed; 0 failed;" in output, output
report = dict(installed=args.installed, nativeEntryAndConsumptionPassed=True,
              starts=actions["starts"], actionsSha256=actions["outputSha256"],
              regulationSha256=parameters["outputSha256"],
              originalEventsUnchanged=True, existingRowsUnchanged=True,
              gameProcessAccessed=False, gameplayVerified=False, multiplayerVerified=False)
(stage.parent / "native-validation.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
print("PASS: 19 formal-activation anchors, staged resource integrity, native entry-only and gauge consumption")
