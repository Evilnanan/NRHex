"""Coarse parameter-based comparison, deliberately not an engine simulation."""
import math

SMOOTH_ARC_REACH = 4.


def smooth_arc_gravity(row, reach=SMOOTH_ARC_REACH, height=1.5):
    """Calibrate neutral launch pitch only; player aiming is retained at runtime."""
    speed=row['initVellocity']; angle=math.radians(row['shootAngleXZ'])
    vx=speed*math.cos(angle); vy=speed*math.sin(angle)
    if vx<=0 or reach<=0:raise ValueError('Invalid smooth arc calibration')
    duration=reach/vx
    gravity=2*(height+vy*duration)/(duration*duration)
    if not math.isfinite(gravity) or gravity<=0:raise ValueError('Invalid smooth arc gravity')
    return gravity


def smooth_arc_reach(row, pitch=0., height=1.5):
    gravity=smooth_arc_gravity(row)
    angle=math.radians(row['shootAngleXZ']+pitch)
    speed=row['initVellocity'];vx=speed*math.cos(angle);vy=speed*math.sin(angle)
    duration=(vy+math.sqrt(vy*vy+2*gravity*height))/gravity
    return max(0.,vx)*min(row['life'],duration)


def shadow_child_flight(row, cap):
    """Reviewed MMV 210668009 instance: live range cache is zero, not row dist.

    347EF0 skips velocity update on its first frame; 34F130 skips first motion.
    34FF00 switches on travelled > instance+224. At 60 Hz this gives one
    in-range acceleration step, then out-range acceleration. No hand bonus.
    The zero cache was observed live; its initialization cause is not generalized.
    """
    assert row['FollowType'] == 0 and not row['isSyncBulletCulcDumypolyPos']
    assert row['gravityInRange'] == row['gravityOutRange'] == row['shootAngleXZ'] == 0
    assert 0 < row['life'] <= 30 and row['accelTime'] == 0
    dt = 1 / 60
    speed = row['initVellocity']
    distance = 0.
    remaining = row['life'] - dt  # native first update does not move
    while remaining > 1e-9:
        step = min(dt, remaining)
        accel = row['accelOutRange'] if distance > 0 else row['accelInRange']
        speed = max(row['minVellocity'], min(row['maxVellocity'], speed + accel * step))
        distance += speed * step
        remaining -= step
    return dict(before=distance, after=min(distance, cap), cut=distance > cap,
                notes=['幽影之地末段按实测实例分段阈值0、60Hz更新估算；首帧不移动，起步一次范围内加速后使用范围外加速度；未使用本行dist=999作为阈值；不含生成偏移与追踪绕行'])


def flight(row, *, terrain_probe=False, ground_contact=False, planar=False, cap=6., lifetime=None, dt=1/240, horizontal_cap=False):
    """Flat terrain; neutral horizontal aim plus the row's additional elevation.

    Forward acceleration and downward gravity are integrated separately. This is
    an explicit approximation; native steering/velocity normalization is omitted.
    """
    notes = set()
    duration = row['life'] if lifetime is None else lifetime
    if duration < 0 or duration > 30:
        duration = 10.
        notes.add('不定/超长寿命按10秒观察窗')
    v = max(0., row['initVellocity'])
    angle = math.radians(row['shootAngleXZ'])
    # Downward terrain locators are not lobbed projectiles. Their horizontal
    # progress ends when they touch the plane and spawn the next component.
    height = .3 if terrain_probe else 1.5
    gravity = bool(row['gravityInRange'] or row['gravityOutRange'])
    if terrain_probe:
        notes.add('贴地探测链；每跳探地高度暂取0.3米')
    elif gravity and not planar:
        notes.add('游戏参数分段抛射粗估；平地发射高度1.5米')
    elif planar:
        notes.add('水平定位/铺地模型；竖向只负责探地')
    else:
        notes.add('无追踪直线近似')
    if ground_contact and not gravity:
        notes.add('无重力下射按平地交点截断，不用完整寿命穿入地下')
    if row['accelTime']:
        notes.add('按accelTime延迟加速')
    if row['isInheritSpeedToChild']:
        notes.add('继承初速暂以本行初速替代')
    if row['lifeRandomRange']:
        notes.add('采用标称寿命，忽略随机项')
    if row['homingAngle'] or row['homingAngleX'] > 0:
        notes.add('忽略追踪转弯；默认水平瞄准')
    x = y = falling = t = before = after = 0.
    cut = False
    while t < duration - 1e-9:
        step = min(dt, duration - t)
        radius = math.hypot(x, y)
        outside = radius >= row['dist']
        accel = row['accelOutRange'] if outside else row['accelInRange']
        g = row['gravityOutRange'] if outside else row['gravityInRange']
        if planar:
            g = 0.
        active_dt = max(0., t + step - max(t, row['accelTime']))
        new_v = v + accel * active_dt
        if accel > 0:
            new_v = min(new_v, max(v, row['maxVellocity']))
        elif accel < 0:
            new_v = max(new_v, min(v, max(0., row['minVellocity'])))
        speed = (v + new_v) / 2
        dx = speed * math.cos(angle) * step
        dy = 0. if planar else speed * math.sin(angle) * step - falling * step - .5 * g * step * step
        fraction = 1.
        if (gravity or terrain_probe or ground_contact) and not planar and y + dy < -height and dy < 0:
            fraction = max(0., (-height - y) / dy)
        nx, ny = x + dx * fraction, y + dy * fraction
        distance = abs(nx) if planar or horizontal_cap else math.hypot(nx, ny)
        # Airborne/ground-probe results compare horizontal reach, while the
        # actual hook still uses 3D displacement except its explicit whitelist.
        reach = abs(nx) if (gravity or planar or terrain_probe or ground_contact) else distance
        before = max(before, reach)
        if not cut:
            if distance > cap:
                # Interpolate the crossing, rather than count a whole extra step.
                old_distance = abs(x) if planar or horizontal_cap else math.hypot(x, y)
                f = max(0., min(1., (cap-old_distance)/(distance-old_distance))) if distance != old_distance else 0.
                cx, cy = x + (nx-x)*f, y + (ny-y)*f
                after = max(after, abs(cx) if (gravity or planar or terrain_probe or ground_contact) else math.hypot(cx, cy))
                cut = True
            else:
                after = max(after, reach)
        x, y = nx, ny
        v = new_v
        falling += g * step
        t += step
        if fraction < 1:
            break
    return dict(before=before, after=after, cut=cut, notes=sorted(notes))


def meteor_landings(roots, seen, bullets, limit, launch_height=1.5):
    """Reviewed meteor chains: constant-speed positioning and downward flight.

    Angles are taken relative to neutral horizontal aim, not accumulated between
    stages. Initial animation/dummy-poly height remains a documented assumption.
    """
    def trace(limited):
        endpoints = []
        def walk(bid, x, height, path):
            if bid not in seen or bid in path:
                return
            r = bullets[bid]
            assert r['gravityInRange'] == r['gravityOutRange'] == 0, 'Meteor gravity changed'
            speed = r['initVellocity']
            for a in (r['accelInRange'], r['accelOutRange']):
                assert a == 0 or (a > 0 and speed == r['maxVellocity']), 'Meteor acceleration changed'
            assert r['life'] > 0 and not r['isInheritSpeedToChild'], 'Meteor stage changed'
            angle = math.radians(r['shootAngleXZ'])
            vx, vy = speed * math.cos(angle), speed * math.sin(angle)
            duration = r['life']
            ground_time = height / -vy if vy < -1e-9 else math.inf
            cutoff_time = limit / speed if limited and speed > 0 else math.inf
            time = min(duration, ground_time, cutoff_time)
            nx, nh = x + vx*time, max(0., height + vy*time)
            next_path = path + [bid]
            if ground_time <= min(duration, cutoff_time):
                endpoints.append(dict(distance=nx, height=0., outcome='首次撞地', path=next_path))
                return  # Do not add post-impact explosions/bounces to landing distance.
            if cutoff_time < min(duration, ground_time):
                endpoints.append(dict(distance=nx, height=nh, outcome='撞地前空中退场', path=next_path))
                return
            children = {r['HitBulletID'], r['intervalCreateBulletId']} & seen
            if not children:
                return  # Visual-only/nonlanding branches are not meteor endpoints.
            for child in sorted(children):
                walk(child, nx, nh, next_path)
        for root in sorted(roots):
            walk(root, 0., launch_height, [])
        return endpoints
    native, shortened = trace(False), trace(True)
    assert native and shortened, 'Missing meteor landing paths'
    before = max(e['distance'] for e in native)
    after = max(e['distance'] for e in shortened)
    outcomes = '、'.join(sorted({e['outcome'] for e in shortened}))
    method = ('平地首次撞地水平距离；沿向上定位→空中生成→斜向下落链计算；'
              '下落段无重力且匀速，不按标准抛物线；初始生成高度暂取1.5米、水平瞄准；'
              '累计定位段高度和水平偏移，不含碰撞半径、随机方向及真实动画绑定偏移；'
              f'限制后终点：{outcomes}；若空中退场，限制后数字为退场点水平距离，并无对应落地点')
    return before, after, method, dict(nativeEndpoints=native, limitedEndpoints=shortened,
                                     assumedLaunchHeight=launch_height)


def compare(mid, seen, moving, bullets, continuations, ground, crystal, limit, crystal_limit, roots, quick_drop=()):
    if mid in (4700, 4701):
        return meteor_landings(roots, seen, bullets, limit)
    samples = {}
    notes = set()
    for bid in sorted(moving):
        # Explosive Ghostflame alternates downward locators and ground emitters.
        # This reviewed classification is deliberately not inferred from g alone.
        r=bullets[bid];child=bullets.get(r['HitBulletID'])
        # MMV ground placement probes: near-vertical high speed, no own attack,
        # followed by a stationary ground collision. Lifetime is a search budget.
        mmv_probe = mid>8000 and r['shootAngleXZ']<=-80 and r['initVellocity']>=100 and r['atkId_Bullet']==1 and child and child['initVellocity']==0
        probe = (mid == 5010 and bid in {10501001, 10501003, 10501005, 10501007, 10501009}) or bool(mmv_probe)
        sample = flight(bullets[bid], terrain_probe=probe, ground_contact=mid>8000 and r['shootAngleXZ']<0, planar=bid in ground,
                        cap=crystal_limit if bid in crystal else limit, horizontal_cap=bid in quick_drop)
        if mid == 2006680 and bid == 210668009:
            sample = shadow_child_flight(r, limit)
        if bid in quick_drop:
            sample['after']=smooth_arc_reach(r)
            sample['cut']=False
            notes.add('特殊弹体从发射起平滑抛射；默认瞄准、发射高度1.5米按约4米落地校准；不是硬上限，高抛或低处地形允许超出6米；不含爆炸半径')
        samples[bid] = sample
        notes.update(sample['notes'])
    if mid == 2006210:
        # The thrown mass ends at the first ground hit. The little ground AOE
        # and its placement probes do not add travel to the delivery projectile.
        carriers=[b for b in roots if b in samples]
        before=max(samples[b]['before'] for b in carriers)
        after=max(samples[b]['after'] for b in carriers)
        notes.add('融泥块仅按投掷载体首次落地；不将落地AOE及其探地弹计入飞行距离')
    elif mid == 2006200:
        # Vortex's visible delivery is a stepped, zero-gravity downward arc.
        # Its later 200m/s vertical ground probes are not a 400m flight.
        x=0.;height=1.5;after=0.
        for bid in range(210620001,210620008):
            r=bullets[bid];v=r['initVellocity'];angle=math.radians(r['shootAngleXZ'])
            assert r['gravityInRange']==r['gravityOutRange']==r['accelInRange']==r['accelOutRange']==0
            vx,vy=v*math.cos(angle),v*math.sin(angle)
            duration=min(r['life'],height/-vy if vy<0 else math.inf)
            x+=vx*duration;height+=vy*duration
            after+=vx*min(duration,limit/v) if v else 0.
            if height<=1e-6:break
        before=x
        notes.add('融泥漩按喷流最远外扩至平地交点；初始高度1.5米；落地返回与沿途AOE不累加射程；竖直探地不算400米飞行')
    elif mid==6908 and 12407000 in roots:
        before=sum(samples[b]['before'] for b in range(12407000,12408400,100) if b in samples)
        after=min(before,limit)
        notes.add('黑蛇按连续贴地链水平推进粗估；所有接续共用首段原点；探地起伏不计长度')
    elif mid == 5010:
        # Only the reviewed main strip; passive side flames add no travel.
        chain = [10501001, 10501003, 10501005, 10501007, 10501009]
        before = sum(samples[b]['before'] for b in chain if b in samples)
        after = 0.
        for b in chain:
            if b not in samples:
                continue
            after += samples[b]['after']
            if samples[b]['cut']:
                break  # forced retirement does not produce the next ground hit
        notes.add('迸发灵火按连续铺地总推进；各探地弹独立计距，不误作一条空中抛物线')
    else:
        edges = {}
        for a, b in continuations:
            if a in seen and b in seen:
                edges.setdefault(a, []).append(b)
        def length(bid, visited):
            if bid in visited:
                notes.add('循环接续只展开一轮')
                return 0.
            own = samples.get(bid, {}).get('before', 0.)
            return own + max((length(c, visited | {bid}) for c in edges.get(bid, [])), default=0.)
        children = {b for values in edges.values() for b in values}
        starts = (seen - children) | (moving - set(edges) - children)
        values = []
        for bid in starts:
            distance = length(bid, set())
            if bid in edges:
                values.append((distance, min(distance, limit)))
                notes.add('同弹接续按各段同向展开相加；整条接续共用6米')
            elif bid in samples:
                values.append((samples[bid]['before'], samples[bid]['after']))
        before = max((v[0] for v in values), default=0.)
        after = max((v[1] for v in values), default=0.)
        notes.add('最长独立飞行/接续段；真正分裂的新弹另计，不累加整次施法距离')
    notes.add('粗估：不含命中半径、爆炸/残留范围和真实地形；并非原生引擎仿真')
    return before, after, '；'.join(sorted(notes)), samples
