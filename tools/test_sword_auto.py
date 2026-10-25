import unittest
from unittest.mock import patch
from build_sword_auto import generate
from sword_range_estimate import flight, compare, shadow_child_flight, smooth_arc_gravity, smooth_arc_reach


def bullet(**changes):
    row=dict(life=2.,initVellocity=10.,shootAngleXZ=0.,gravityInRange=0.,gravityOutRange=0.,
             accelInRange=0.,accelOutRange=0.,accelTime=0.,minVellocity=0.,maxVellocity=100.,
             dist=0.,lifeRandomRange=0.,homingAngle=0.,homingAngleX=0.,isInheritSpeedToChild=0,
             FollowType=0,isSyncBulletCulcDumypolyPos=0,HitBulletID=-1,intervalCreateBulletId=-1,atkId_Bullet=40000)
    row.update(changes);return row


class AutomaticSword(unittest.TestCase):
    def test_moon_expiry_preserves_zero_speed_blast_with_residual_acceleration(self):
        for mid,parent in [(4360,10436000),(4361,10436100)]:
            magic={'ezStateBehaviorType':0}
            for i in range(1,11):
                magic[f'refCategory{i}']=1
                magic[f'refId{i}']=parent if i==1 else -1
            blast=bullet(initVellocity=0.,minVellocity=0.,maxVellocity=0.,accelInRange=.1,
                         FollowType=0,launchConditionType=0)
            raw={'regulationSha256':'fixture','names':{str(mid):'Moon'},'tables':{
                'Magic':[{'ID':mid,'fields':magic}],
                'Bullet':[{'ID':parent,'fields':bullet(HitBulletID=parent+1)},
                          {'ID':parent+1,'fields':blast}],
                'AtkParam_Pc':[{'ID':40000,'fields':{'atkMag':100}}]}}
            _,report=generate(raw)
            self.assertEqual(report['spells'][0]['nativeRangeEnds'],{parent:parent+1})
            # A changed, genuinely moving child must require another review.
            blast['maxVellocity']=10.
            with self.assertRaises(AssertionError):generate(raw)

    def test_smooth_arc_calibrates_neutral_pitch_not_a_hard_radius(self):
        for speed,angle in [(30.,25.),(12.,0.),(15.,5.),(15.,0.),(23.,-6.)]:
            row=bullet(life=3.,initVellocity=speed,shootAngleXZ=angle)
            self.assertGreater(smooth_arc_gravity(row),0.)
            self.assertAlmostEqual(smooth_arc_reach(row),4.,places=5)
            self.assertGreater(smooth_arc_reach(row,height=4.),4.)
        row=bullet(life=3.,initVellocity=15.)
        self.assertGreater(smooth_arc_reach(row,pitch=45.),6.)

    def test_shadow_live_range_cache_switch(self):
        row = bullet(initVellocity=15., accelInRange=120., accelOutRange=0.,
                     minVellocity=15., maxVellocity=45., dist=999.)
        self.assertAlmostEqual(flight(row)['before'], 86.25, places=2)
        sample = shadow_child_flight(row, 6.)
        self.assertAlmostEqual(sample['before'], 33.7166667, places=4)
        self.assertEqual(sample['after'], 6.)
        # The estimate responds to parameters rather than a hand-filled distance.
        row['life'] = 1.
        self.assertAlmostEqual(shadow_child_flight(row,6.)['before'], 16.7166667, places=4)

    def test_zero_lifetime_is_not_an_infinite_controller(self):
        self.assertEqual(flight(bullet(life=0.))['before'],0.)

    def test_downward_ground_probe_is_not_four_hundred_metres(self):
        row=bullet(life=2.,initVellocity=200.,shootAngleXZ=-89.)
        value=flight(row,terrain_probe=True)
        self.assertLess(value['before'],.01)
        self.assertAlmostEqual(value['before'],value['after'])

    def test_delayed_acceleration_and_game_gravity(self):
        r=bullet(life=1.,accelTime=.5,accelInRange=10.,accelOutRange=10.)
        self.assertAlmostEqual(flight(r)['before'],11.25,places=2)
        r=bullet(gravityInRange=10.,gravityOutRange=10.)
        self.assertAlmostEqual(flight(r)['before'],10*(.3**.5),places=2)
        self.assertLess(flight(r,cap=3.)['after'],3.)

    def test_generation_uses_parameters_and_zero_for_nonflight(self):
        def magic(bid):
            r={'ezStateBehaviorType':0}
            for i in range(1,11):r[f'refCategory{i}']=1;r[f'refId{i}']=bid if i==1 else -1
            return r
        raw={'regulationSha256':'fixture','names':{'4000':'Flying','4440':'Direct'},'tables':{
            'Magic':[{'ID':4000,'fields':magic(10400000)},{'ID':4440,'fields':magic(-1)}],
            'Bullet':[{'ID':10400000,'fields':bullet()}],
            'AtkParam_Pc':[{'ID':40000,'fields':{'atkMag':100}}]}}
        with patch('build_sword_auto.rules',return_value=(set(),set(),set(),set(),set())):
            code,report=generate(raw)
        values={s['MagicID']:s['percentage'] for s in report['spells']}
        self.assertEqual(values,{4000:20.,4440:0.})
        self.assertIn('(40000,&[4000])',code)
        self.assertFalse(report['manualBalanceInputs'])

    def test_putrescence_ignores_post_landing_search_budget(self):
        rows={210621000:bullet(life=3.,initVellocity=15.,shootAngleXZ=25.,gravityInRange=10.,gravityOutRange=10.),
              210621006:bullet(initVellocity=200.,shootAngleXZ=-89.,atkId_Bullet=1,HitBulletID=210621007),
              210621007:bullet(initVellocity=0.)}
        before,_,method,_=compare(2006210,set(rows),{210621000,210621006},rows,set(),set(),set(),6.,3.,{210621000})
        self.assertLess(before,40.)
        self.assertIn('首次落地',method)
        vortex={i:bullet(life=.2,initVellocity=15.,shootAngleXZ=-30.) for i in range(210620001,210620008)}
        vortex.update({210620012:rows[210621006],210621007:rows[210621007]})
        before,_,method,_=compare(2006200,set(vortex),set(vortex)-{210621007},vortex,set(),set(),set(),6.,3.,{210620001})
        self.assertLess(before,10.)
        self.assertIn('返回',method)


if __name__=='__main__':unittest.main()
