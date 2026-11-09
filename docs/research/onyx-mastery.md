# 黑王（原“精通【黑王大剑】”）（0.17.299）

能力95005300，无角色限制，复用战技类图标。抽取历史追加到第49位；惊喜盲盒沿用通用发放流程。获得时在本人脚边生成一把原生8010000黑王大剑，按实际创建结果记录95005301收据；未拾取的赠送武器支持原有复活奖励迁移。回庭院／新远征清理拥有、收据和临时动作标记。

0.17.294起，私人掉落95553000的类别由2（基础武器）改为6（原生生成武器），同一编号8010000指向EquipParamCustomWeapon，再生成targetWeaponId=8010000。保留战技1041、weaponLevel=0、非诅咒、随机法术配置和全部原生词条槽。普通模式仅槽1引用501200200，槽2／3为空；沿用目标版本的权重及条件抽取一个词条。深夜槽4～6的原生配置保持不变。原版和MMV分别核对当前生成表及词条池，只修改私有掉落类别，不修改原生词条池。旧局已经发放的武器不额外重发或补抽词条。

## 耗精与战技优先级

弹体类费用沿用原生384AA1付款点；近战碰撞费用另从434783的原生扣精调用拦截，读取本次命中的已解析武器和行为判定，不能将弹体付款点当作全部动作的统一入口。两条路径检查801系列，普通动作费用降低25%，普通／蓄力／跳跃重攻击保留50%减耗，实际费用向上取整。保留原生非战斗与免耗标志。冲刺攻击由a40的30200、30210、32200、32210、34200映射到120、125、320、325、860、865行为判定，分别生成原版／MMV行为表，不将BehaviorParam.category当作冲刺分类。另一手的非黑王武器和法术不受益。

耗精、引导资格和陨石行为解析直接使用PlayerIns虚表+250（66CDE0→3818C0→381220），从实际来源／选中槽获取武器ID；不再借用百般武艺的实例解析及原生战技目录。原生静态句柄可解析出有效8010000但实例指针为空；强化ID也可能没有单独的战技目录行，两者都不应阻断精通。HKS的c_SwordArtsHand为0左／1右，分别映射原生source -2／-1，与调酒师RELIC_WEAPONCHANGE_ENCHANT的编号不同。

精通拥有者的黑王大剑固定使用1041战技，优先于百般武艺的临时替换，不改库存或丢弃数据。拥有状态变化会要求重新广播装备快照，发送有效战技1041；丢给没有精通的人时仍是普通武器。

## 动作与伤害

保留a841/40000原生起手、剑身攻击和推力波。引导资格读取本次起手保存的IsEnoughArtPointsL2（0为满足条件），不再依赖会在动画结束前过期的6955301短标记；缺蓝起手仍不可进入引导。0.17.295不再套用法杖陨石动画：从目标原生a840/40000的第81帧（2.7秒插剑／重力波时刻）生成3.2秒保持段，并采样其余2.3秒作为拔剑收招。两段为独立HKX，TAE不再使用ImportsHKX。新增事件不读法术槽，也不调用TAE64／115魔法释放、魔法特效事件。

起手更新的动画结束查询改走990056；拥有者、来源武器、原生付款标记与插剑标记均满足时提前返回结束，仍由原有HKS检查长按、取消和后续状态。其他动作继续原生339。40003在0.4、1.0、1.6、2.2、2.8秒各发射一次武器行为3998；3.1秒的6955300短标记让HKS转入战技收招。仍检查长按、精力和原生取消／受击逻辑。只对实际来源黑王大剑映射到95005390行为。正常黑王大剑及百般武艺在其他武器上随到的同款战技不能进入新循环。

陨石使用3600810→3600811→3600812的原生出射定位链，复制为95005350～95005352。最后一个发射器生命周期0.05秒，取消周期发射，改为终末HitBulletID=95005353；子弹launchConditionType=0，数量1。每次只派生一颗95005353；命中衍生95005354保留原生共享命中表与0.2秒去重。实际陨石／命中特效复用636000、636001，传送门复用636003；相关模型和贴图已存在于原生公共特效包。源c3600特效包纳入只读资源清单，构建仅向公共DLC包追加所需条目。

当前保留原生起手27点专注，每颗陨石基础消耗10点，五颗完整施放合计77点。990054只替换引导阶段的足量判断；不足一颗费用时结束引导，其他武器／战技仍走原生344。实际费用交给6CE5A0：使用私有10点费用视图，按战技耗蓝倍率（5061B0）聚合并向上取整，保留减耗／免耗效果。3975D0武器弹体提交路径仅对95005390、实际发射描述符、本人精通黑王大剑收费；成功生成后调用原生4261C0扣蓝，失败不扣，后续子弹／爆炸不重复付费。不修改共享战技参数，不按时间补扣。每颗基础精力20，当前精通折扣后15。伤害行95005360从原生黑王大剑波303400807复制，当前基础100物理＋100魔力、10削韧、7号冲击力，沿用玩家武器战技来源与补正；不是固定200点最终伤害。起手冲击波仍为原生0物理＋200魔力、20削韧，不随陨石调整。

## MMV与验证范围

原版使用HKS前缀及循环专注查询替换，保持原生分支与取消流程。MMV由同一前缀翻译为Lua，并用整函数哈希保护的查询替换与原MMV脚本合并。MMV动作合并单独接受a841新增40003／40004，恢复控制事件取自本目标a841起手的后半段；原有a841事件和动画头逐项校验，序列化只允许末尾零填充差异。其余动画成员保持原输入。运行时沿用MMV原有各目标数据表，并单独编译冲刺行为表。

资源回读检查覆盖私有参数与原生行不变、五次发射、导入动画、缺蓝边界、抽取与生命周期。自动测试覆盖脚本分支和寄存器、MMV Lua与字节码差异；原生隔离测试覆盖实际扣精、两种免耗标志、左右来源、其他武器、法术、起手标记和收招条件。

用户已在MMV实测旧版：刚领取、未强化的武器能获得，但战斗中耗精没有明显变化，持续按住战技也没有陨石。旧测试为武器读取提供了实例和战技覆盖值，未覆盖真实静态武器路径。0.17.294隔离验证改为执行原生来源／槽位读取，实例记录置空，覆盖左右各三槽组合、强化ID、另一手空手、非法来源、陨石资格及实际扣精；能力查询和外部奖励环境仍使用夹具。当前没有用户现场句柄记录，故这些修正不能证明旧版实测问题只有这一原因。

修复版尚未游戏内或联机实测。重点验收：新TAE导入动作能否被原生战技选择器正常播放；推力波接入举剑引导时的姿势；陨石生成高度、锁定与室内碰撞；提前松手、受击、精力耗尽、第五颗后的收招；新奖励随机词条及与百般武艺共存时的远端表现。静态与隔离检查不代表这些实机项目已通过。

## 本机进一步诊断（2026-11-08，未完成验收）

用户已实测确认新增近战付款点使耗精正常，长按战技仍无陨石且收招后可能僵直。记录捕获art=241进入phase=1，但未见收尾标记。用户期间另用法杖施放过陨石，未绑定来源的env日志不用于证明强化战技成功。当前诊断仅记录战技回调与3998专属武器发射判定。

只读检查确认原版与MMV行为图均无a841_040003/040004的动画名称和selector子节点；TAE ImportsHKX本身不会为这些动作补齐行为图绑定。build_onyx_behavior.py向六个原生SwordArtsLoop选择器各追加对应子节点，并向动画列表登记两条导入动作；保留原有节点、TYPE/THSH布局和MMV行为内容。二进制追加只改原有数据中的七处数组引用，新增项／重定位表已回读核对。导入的a415_045012实际长度约3.133秒，末次2.8秒发射和3.1秒结束标记均在动画范围内。当前补丁已部署到MMV测试版，需继续验收动作播放、实际武器陨石及正常收招；未更新群友版。

### 0.17.295 插剑动作与一次性陨石

用户确认上一版已进入引导，有天空生成特效，但法杖姿势不适合持剑且没有飞出的陨石。当前从目标a9x动作包解码原生黑王大剑动作，生成保持／收招两个独立自包含HKX；72条骨骼轨道、全部帧与时长经序列化回读校验，收招根运动从插剑时刻重新归零。保留a9x所有既有成员，只追加1841040003和1841040004；MMV有自己的a9x时以其为输入，否则使用清单登记的原生包。工具需要本机Smithbox与DSAS（NRHEX_SMITHBOX／NRHEX_DSAS可覆盖路径）。

行为图中的两段私有动作均使用非循环播放，避免低帧率越过末尾后回绕、漏掉收尾标记。启动与取消条件仍由原生动作流程控制。生成器由0.05秒寿命配1秒周期的配置改为单次终末派生，并取消陨石继承的条件门控；原生NPC行和伤害数值不变。

验证：2项HKS控制流测试、4项MMV适配测试（172函数、6912内部片段场景及32圣杯瓶场景）、1868项原生付款检查通过，新增资格用例覆盖拥有、双手来源、付款及插剑标记组合。临时Onyx日志已移除。MMV测试版0.17.295已更新并重启，四处钩子和HKS／TAE／行为图／a9x动作包加载已确认；群友版未改。新动作观感和陨石实际命中仍待用户实测，不以静态检查或成功加载代替。

2026-11-08: The startup callback 990055 collided with static_charge::EVASION_QUERY and returned 0 before reaching Onyx. Moved only the Onyx startup query to 990056 in Rust, standalone HKS and the MMV adapter. Added dispatcher-ID uniqueness and legacy-script migration tests. Live verification remains pending.

### Planted-motion polish and lock aiming (2026-11-08)

User confirmed the channel and meteors now work after the callback-ID fix, but reported a frozen channel, an exit twitch and missed lock targets. Read-only sampling captured ten root/meteor chains: all ten retained the same kind-1 target handle through 95005350..95005353. The target was not lost. The copied NPC meteor had lockShootLimitAng=0, hormingOffsetRange=3 and targetYOffsetRange=1. Only private 95005353 now permits 180-degree initial lock aiming and zero aim offsets; speed, ongoing homing, damage, count and native NPC rows are unchanged. This still requires live hit verification.

Motion inspection found a roughly 29-degree one-frame impact recoil immediately after native frame81. The old held pose deferred this recoil until recovery. The new hold plays native frames81..99 over the first0.6s, then smoothly samples the settled91..99 range with an eased envelope, returning exactly to99 before the3.1s finish marker. Recovery starts at99 and follows the remaining1.7s. Root motion is rebased at each segment's own entry, and recovery TAE controls are shifted from3.3s. Decoded rotations are normalized before interpolation. Serialization readback verifies every transform, the finish-marker/recovery seam on all72 tracks, a bounded first recovery step, and all pre-existing a9x entries unchanged. Motion appearance and interruption transitions still require user testing. No extra DLL diagnostics were added.

### Native recovery controls, slow hold, visuals and flight budget

Follow-up user feedback requested no free turning while planted, less frequent body movement, native-looking recovery, a 20m meteor flight budget, and the Meteorite casting/grass wind visuals. The hold now reuses the native TAE0 flag7 (Disable Turning) for its full duration. The pose curve performs only one small out-and-back excursion (1.5 native frames across2.4s), with a reversal-count guard. Recovery retains the original3.3..5.0s motion and now also shifts the original action/early-cancel flags, idle-blend curves, footsteps and sound events into that tail; payloads and timing are compared after serialization. This restores native recovery controls previously omitted by the limited16/225/712 allowlist.

The private meteor flies at15m/s with life=20/15s and no lifetime randomness. This budgets approximately20m of projectile travel from its portal, not20m horizontal distance from the player; terminal collision behavior remains native. The channel copies Meteorite a415/45012's body effect530125 at dummy900/slot2, and explicitly resolves Magic4700 effectSfxId530124 to body slot3 rather than reading the player's active spell slot. Both resources exist in the native common effect binder. Their grass-wind appearance and cleanup on interruption still require live validation; no enemy-pushing behavior is added.

### 0.17.296 按颗扣蓝与降伤

用户已认可上一版动作观感。本轮保留其动作、锁定、20米飞行预算及身上特效，只改引导扣蓝和私有陨石伤害。原版及当前MMV的Magic4700陨石为起手30、后续10；4701艾斯提陨石为60、后续12。mp_charge是原生续发费用，不能不核对TAE触发节奏就直接写成每秒或每颗费用。黑王大剑的5点是用户明确选择的按颗收费。

原生FP为stats+150（最大值+154），4261C0经427E50执行原生免耗与上下限；不能误读相邻的+14C。参数回读按行顺序核对AtkParam_Pc，保留MMV中合法的重复行ID，仅允许95005360的atkPhys/atkMag变更。

验证：35组原生费用／余量组合通过，并覆盖五颗合计25、失败不扣、免耗标志、失效倍率节点、其他行为／远端来源／死亡与失去武器或能力。原生调用签名、3项精通单元测试和4项HKS测试通过。MMV测试版0.17.296已部署DLL及regulation并重启，现场确认五处相关钩子、DLL哈希及新参数包加载；未更新群友版。实际战技扣蓝与伤害仍待用户游戏内验收。

### 0.17.297 姿态恢复与分类减耗

用户要求恢复此前轻微姿态活动。HoldSource恢复99−8×sin²(πu)×sin²(2πu)，u=(t−0.6)/2.4，保留首次0.6秒原生冲击过渡及1.7秒原生收招。生成后与上次smooth资源逐帧比较：引导6984、收招3744个变换完全一致。保留当前TAE的禁止转向、特效、自然取消和收招控制，不回退陨石50/50伤害、20米预算及每颗5点FP。

原生行为图的AttackRight/BothHeavy Start/End选择器对应305xx/325xx，跳跃Hard对应31230..31270/33230..33270；从各目标a40的TAE1提取重击判定100/105/110/115/116/160及300/305/310/315/316/360，再映射到BehaviorParam_PC生成HEAVY_BEHAVIORS。不将普通双手轻击200或格挡反击180/380误归为重击；冲刺判定始终优先免费。原版和MMV各自生成数据。普通消耗ceil(0.75×cost)、重击ceil(0.5×cost)，均用无溢出的整数算式；原生普通、蓄力、跳跃重击的两条扣精路径共同验证。

验证：两版各4项精通单元测试通过；2108项原生扣精检查通过，涵盖新普通／重击折扣、原生免耗、跑攻与来源隔离。恢复动作与此前smooth版的全部引导／收招变换逐项一致；现有a9x成员保持原样。MMV测试版0.17.297已更新DLL及a9x并重启，确认五处钩子、新DLL哈希及a9x加载；其他资源和群友版未更新。当前动作观感与分类减耗待用户验收。

### 0.17.298 百般武艺不再免除强化黑王费用

百般武艺95001400使用artsConsumptionRate=0，会通过5061B0免除全部战技费用。强化黑王大剑改为只在实际来源为801系列、持有精通及百般武艺的本机费用路径中忽略此效果：48F9B0为原生按手的费用资格检查，48FA90为按手预约费用，两者接收module/button/hand；module+8是actor，原生3CEFE0将hand=0/1映射为source=-2/-1。包装调用用线程内作用域记录当前角色的效果管理器，退出后恢复旧作用域，不依赖容易过期的HKS动作状态或仅凭战技1041判断武器。

5061B0的过滤分支保持原生节点资格、f32相乘顺序及负数／NaN排除，唯一省略的节点ID为95001400；其他减耗或免耗效果照常聚合。6CE5A0继续负责原生费用读取和向上取整，起手仍由原生预约与TAE付款流程扣费。私有每颗5点的报价在同步局部作用域中使用同一例外。没有修改共享参数、运行中的效果链或其他武器费用；另一手即便同为1041战技，只要不是精通黑王来源仍享受百般武艺免耗。

验证：55组原生费用／余量用例通过，执行原生48F9B0、48FA90、6CE5A0和4261C0，覆盖27+5×5总额、缺蓝、左右手互换、另一手相同战技ID、丢失精通、停用、其他减耗／免耗效果及无共享节点改写。MMV测试版0.17.298仅更新DLL。首次启动在安装钩子日志之前于ntdll+1A12D写入0x24崩溃，栈内可见NVIDIA驱动模块地址；原DLL配置不变重试正常启动，确认主更新、弹体、扣精及新增3处费用钩子已加载。尚不能单凭此推断完整崩溃根因；费用实际游戏表现待用户测试。群友版及资源未更新。

### 0.17.299 数值与文案

陨石基础伤害调整为100物理＋100魔力，每颗基础专注消耗10；原生起手27，五颗合计77，仍按原生战技减耗／免耗及向上取整规则计算，并保留百般武艺的既有费用例外。仅调整私有伤害行95005360的两个攻击力字段及运行时单颗费用，不改变引导动作、耗精折扣、陨石数量或发射节奏。中英文奖励及状态说明同步更新，中文为“降低黑王大剑的攻击耗精。战技可持续引导召唤陨石。获得黑王大剑”。

### 2026-11-09 射程调整

强化陨石飞行预算由20米延长至30米。速度保持15米/秒，私有弹体95005353的life由20/15秒改为2秒；只调整飞行寿命，不改伤害、耗蓝和碰撞后的原生效果。这里仍是从空中生成点起算的飞行距离。
