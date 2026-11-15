"""Behavior checks on the actual merged MMV callbacks and shared bytecode edits."""
import json
import os
import unittest
from pathlib import Path
from lupa.lua51 import LuaRuntime
from build_light_footed_hks import patch, NAMES, QUERY
from hks_bytecode import Reader
from mmv_hks_fragments import functions
from mmv_hks_stage import compose, PLAN

ROOT = Path(__file__).resolve().parents[1]

class LightFootedTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.current = (ROOT/'dist/mod/action/script/c0000.hks').read_bytes()
        readers = [Reader((ROOT/'assets/original/action/script/c0000.hks').read_bytes()), Reader(cls.current)]
        for r in readers: r.function()
        mmv = Path(os.environ['NRHEX_MMV'])
        source = (mmv/'mod/action/script/c0000.hks').read_bytes().decode('utf-8-sig')
        cls.merged, _ = compose(source, *readers, json.loads(PLAN.read_text()))
        cls.spans = functions(cls.merged)

    def world(self, enabled, cancel=None):
        lua = LuaRuntime(unpack_returned_tuples=True); g = lua.globals()
        values = dict(UpperDefaultState00=11, LowerDefaultState00=1, MoveSpeedIndex=2,
                      MoveSpeedLevel=1.2, MoveAngle=0, IsLockon=0, IsEnableToggleDashTest=0, ToggleDash=1)
        calls = []
        def record(name, result=0):
            def call(*args):
                calls.append((name, *args))
                return 1 if name == cancel else result
            return call
        def env(code, *args):
            if code == QUERY: return int(enabled and args[0] in (11, 12, 104) and args[1] == 1)
            if code == 'GetStamina': return 100
            if code == 'ActionDuration': return int(args[0] == 'ACTION_ARM_L3')
            return 0
        constants = ['LockonFixedAngleCancel', 'FallPreventionAssist', 'SetIsMagicInUse', 'GetStateChangeType',
                     'SetIsItemAnimationPlaying', 'FALL_TYPE_DEFAULT', 'HAND_RIGHT', 'ESTEP_NONE', 'QUICKTYPE_NORMAL',
                     'ALLBODY', 'UPPER', 'LOWER', 'TURN_TYPE_DEFAULT', 'HAND_RIGHT_BOTH', 'HAND_LEFT_BOTH',
                     'IsPrecisionShoot', 'GetSpEffectID', 'GetHeroID', 'HERO_BALANCE', 'HERO_TANK', 'HERO_SHOOTER',
                     'HERO_SPEED', 'HERO_POWER', 'HERO_NECROMANCER', 'HERO_MAGIC', 'HERO_TECHNICAL', 'HERO_BUFFER',
                     'HERO_ATTACKER', 'GetStamina', 'AddSpEffect', 'GetMoveAnimParamID', 'WEIGHT_OVERWEIGHT',
                     'CONDITION_TYPE_NIGHT', 'SetStaminaRecoveryDisabled', 'ActionDuration', 'ACTION_ARM_ACTION',
                     'ACTION_ARM_L3', 'SwitchMotion']
        for name in constants: g[name] = name
        g.TRUE = 1; g.FALSE = 0; g.ISENABLE_SPRINTSTART = 1; g.ISENABLE_SPRINTSTOP = 1
        g.g_IsMimicry = 0; g.c_Style = 0; g.STAMINA_REDUCE_SPRINTSTART = -30
        g.env = env; g.act = record('act'); g.GetVariable = lambda key: values.get(key, 0)
        g.SetVariable = lambda key, value: values.__setitem__(key, value)
        g.ChangeMoveSpeedIndex = lambda value: values.__setitem__('MoveSpeedIndex', value)
        g.GetMoveSpeed = lambda value: value
        for name in ['SetAIActionState', 'ExecPassiveAction', 'ExecJump', 'ExecHandChange', 'ExecGuardOnCancelTiming',
                     'ExecWeaponChange', 'ExecEvasion', 'ExecItem', 'ExecArtsStanceOnCancelTiming', 'ExecAttack', 'ExecMagic',
                     'MoveStartonCancelTiming', 'ExecQuickTurn', 'IsNodeActive', 'IsDemonSwordMode', 'IsSummonNecromanceMode',
                     'SetSlowFeet', 'ExecEventAllBody', 'AddStamina']:
            g[name] = record(name)
        g.GetHalfBlendInfo = lambda: ('UPPER', 'LOWER_MOVE'); g.LOWER_MOVE = 'LOWER_MOVE'
        for name in ['BanSprint', 'MagicCommonFunction', 'ItemCommonFunction', 'SprintStart_onActivate', 'ExecSprintStart', 'SpeedUpdate']:
            span = self.spans[name]; lua.execute(self.merged[span['start']:span['end']])
        return g, values, calls

    def test_native_sprint_clamp_toggle_and_cancel_paths(self):
        for enabled in [False, True]:
            for speed in [0, 1, 2, 3]:
                g, values, _ = self.world(enabled); values['MoveSpeedIndex'] = speed
                g.BanSprint(); self.assertEqual(values['MoveSpeedIndex'], min(speed, 2))
            for name in ['MagicCommonFunction', 'ItemCommonFunction']:
                for cancel in [None, 'ExecPassiveAction', 'ExecJump', 'ExecHandChange', 'ExecGuardOnCancelTiming',
                               'ExecWeaponChange', 'ExecEvasion', 'ExecArtsStanceOnCancelTiming', 'ExecAttack', 'ExecItem', 'ExecMagic']:
                    g, values, calls = self.world(enabled, cancel)
                    result = g[name]('UPPER', 0, 0)
                    self.assertEqual(result, int(cancel is not None), (name, enabled, cancel))
                    self.assertEqual(values['ToggleDash'], 0)
                    if cancel: self.assertEqual(calls[-1][0], cancel)

    def test_locked_backward_repeated_sprint_does_not_cancel_facing(self):
        for enabled in [False, True]:
            g, values, calls = self.world(enabled)
            values.update(IsLockon=1, MoveAngle=180, MoveSpeedIndex=2)
            for _ in range(10):
                self.assertIsNone(g.ExecSprintStart())  # Native fall-through: no sprint transition.
            self.assertEqual(values['MoveSpeedIndex'], 2)
            self.assertFalse(any(c[0] in ['AddStamina', 'ExecEventAllBody'] or
                                 c[:2] == ('act', 'LockonFixedAngleCancel') for c in calls))
            self.assertEqual(g.ISENABLE_SPRINTSTART, 1)

    def test_sprint_functions_restore_the_pre_feature_bytecode(self):
        import subprocess
        baseline = Reader(subprocess.check_output(['git', 'show', '5d4b3ea:dist/mod/action/script/c0000.hks'], cwd=ROOT))
        baseline.function(); current = Reader(self.current); current.function()
        prior = {f['name']: f for f in baseline.functions}
        for f in current.functions:
            if f['name'] in ['BanSprint', 'ExecSprintStart', 'MagicCommonFunction', 'ItemCommonFunction', 'SpeedUpdate']:
                self.assertEqual(f['code'], prior[f['name']]['code'], f['name'])
                self.assertEqual(f['constants'], prior[f['name']]['constants'], f['name'])
                self.assertNotIn(QUERY, f['constants'])

    def test_patch_reproducible_and_idempotent(self):
        import subprocess
        prior = subprocess.check_output(['git', 'show', '5d4b3ea:dist/mod/action/script/c0000.hks'], cwd=ROOT)
        result, report = patch(prior)
        self.assertEqual(set(report), set(NAMES))
        generated = Reader(result); generated.function()
        current = Reader(self.current); current.function()
        expected = {f["name"]: f for f in generated.functions}
        for f in current.functions:
            if f["name"] in NAMES:
                self.assertEqual(f["code"], expected[f["name"]]["code"], f["name"])
                self.assertEqual(f["constants"], expected[f["name"]]["constants"], f["name"])
        self.assertEqual(patch(self.current), (self.current, {}))
        self.assertEqual(patch(result), (result, {}))

    def test_old_sprint_bypass_is_not_mistaken_for_current_patch(self):
        import subprocess
        legacy = subprocess.check_output(['git', 'show', 'a6daa1a:dist/mod/action/script/c0000.hks'], cwd=ROOT)
        with self.assertRaisesRegex(AssertionError, 'Retired Light-footed sprint bypass'):
            patch(legacy)

if __name__ == '__main__': unittest.main()
