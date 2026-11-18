"""Capture the selected weapon hand before native skill events are dispatched."""
from pathlib import Path
from hks_bytecode import Reader
from build_marking_hks import patch_queries

ROOT = Path(__file__).resolve().parents[1]
QUERY = 990063
NAMES = {'GetSwordArtsRequestNew', 'SetSwordArtsPointInfo', 'SwordArts_Activate',
         'SwordArtsLeft_Activate'}


def patch(source):
    reader = Reader(source)
    reader.function()
    targets = [f for f in reader.functions if f['name'] in NAMES]
    if len(targets) != len(NAMES):
        raise ValueError('Missing or duplicated native skill callback')
    def code(f):
        return QUERY + 1 if f['name'] == 'GetSwordArtsRequestNew' else QUERY

    installed = [code(f) in f['constants'] for f in targets]
    if all(installed):
        return source
    if any(installed):
        raise ValueError('Partial Arts cast patch')

    def hand(f, reg, const, glob):
        return glob(reg + 2, 'HAND_LEFT' if f['name'] == 'SwordArtsLeft_Activate' else 'c_SwordArtsHand')

    for query in (QUERY, QUERY + 1):
        source, _ = patch_queries(source, query, [f['id'] for f in targets if code(f) == query], hand)
    return source


if __name__ == '__main__':
    path = ROOT / 'dist/mod/action/script/c0000.hks'
    source = path.read_bytes()
    result = patch(source)
    if source != result:
        path.write_bytes(result)
