"""Stage, verify, then install the final cage pass into a specified mod overlay."""
import argparse
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def install(mod, stage):
    mod, stage = Path(mod).resolve(), Path(stage).resolve()
    assert mod != stage
    subprocess.run(['dotnet', 'run', '--project', str(ROOT / 'tools/HexBuild'), '--', str(ROOT),
                    '--cage-escalation-stage', str(mod), str(stage)], cwd=ROOT, check=True)
    subprocess.run(['python', '-X', 'utf8', str(ROOT / 'tools/verify_cage_escalation.py'),
                    str(stage / 'validation.json')], cwd=ROOT, check=True)
    files = {'regulation.bin': stage / 'regulation.bin'}
    files.update({f'event/{p.name}': p for p in (stage / 'event').glob('*.emevd.dcx')})
    files.update({f'msg/{p.relative_to(stage / "texts").as_posix()}': p
                  for p in (stage / 'texts').rglob('*.msgbnd.dcx')})
    for relative, source in files.items():
        target = mod / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
    return files


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--mod', type=Path, default=ROOT / 'dist/mod')
    parser.add_argument('--stage', type=Path, default=ROOT / 'work/cage-escalation/build')
    args = parser.parse_args()
    install(args.mod, args.stage)
