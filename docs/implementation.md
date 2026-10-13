# 当前实现入口

本页列出当前源码与配置入口。版本变更集中在 [历史变更](../CHANGELOG.md)，原生依据保留在各主题研究文档。

## 运行时

`src/lib.rs` 负责版本检查、初始化、所有原生 hook、线程局部调用范围和游戏线程更新；玩法计算尽量拆在纯状态机中。

| 功能 | 主要源码／工具 |
| --- | --- |
| 奖励池、排除已拥有项 | `hex_pool.rs`、`reward_choices.rs`、`HexBuild` |
| 等级发放与排队 | `level_rewards.rs`、`personal_reward.rs`、`verify_level_rewards.py` |
| 复活搬回原球 | `reward_recovery.rs`、`verify_reward_recovery.py` |
| 终极刷新、豪饮叠层、再生术 | `runtime.rs`、`flasks.rs` |
| 豪饮HUD图标与原生叠层计数 | `HexBuild/Program.cs`、`record_drinking_hud.py`、[原生HUD研究](research/drinking-hud.md) |
| 结晶外壳、受击隐身 | `crystal.rs`、`cat.rs` |
| 队友特殊效果列表／图标维护 | `status_sync.rs`、`lib.rs`、`HexBuild/Program.cs`、`verify_effect_sync_native.py`、`build-effect-sync.ps1` |
| 详细状态页的特殊效果／附加效果分栏 | `status_display.rs`、`lib.rs`、[归栏证据](research/status-routing-01732.md) |
| 无限墙跳、下落增伤 | `wall_input.rs`、`cat_jump.rs`、`build_walljump.py` |
| 亮出你的剑 | `sword.rs`、`sword_map.rs`、`build_sword_ranges.py` |
| 亮剑角色抽取限制 | `reward_choices.rs::role_exclusions`、`lib.rs::local_hero`／`unavailable_hexes`、`record_sword_balance.py` |
| 大力 | `strength.rs`、`verify_strength_native.py` |
| 逃跑计划 | `escape.rs`、`verify_escape_native.py` |
| 嘉豪身法 | `jiahao.rs`、`build_jiahao.py`、`JiahaoTae` |
| 吵闹鬼 | `noisy.rs`、`verify_noisy.py`、`HexBuild`的私有熔岩链 |
| 刷怪笼 | `HexBuild/Cages.cs`、`verify_cages.py`、`cage_local_events.py`、`build-cages.ps1`、`data/reference/world/cage-bosses.json` |
| 百般武艺 | `arts.rs`、`arts_data.rs`、`build_arts_data.py`、`build_arts_hks.py` |
| 气球、强化【死灵术】、别停止咏唱 | `balloon.rs`／`balloon_runtime.rs`／`ability_damage.rs`（只提供基础攻击值，余下由原生结算）、`spirit.rs`／`spirit_runtime.rs`、`chant.rs` |
| 大法师：法术槽随机、丢拾与装备同步 | `archmage.rs`、`archmage_runtime.rs`、`archmage_cost.rs`、`archmage_data.rs`、`HexBuild/Archmage.cs`、`build_archmage_hks.py` |
| PE／特征／挂钩 | `pe.rs`、`pattern.rs`、`hook.rs`、`win.rs` |

再生术蓝血转换比为1:4：血量不满时，每0.25秒消耗4FP、回复16HP。数值由 `HexBuild/Program.cs` 的95000601参数行生成，运行时仅管理触发间隔与资源条件；满血喝瓶回复50%FP沿用原逻辑。增量回读证据见 `dist/regen-balance-validation.json`。

玻璃大炮、红包、星界躯体等静态数值主要由 `tools/HexBuild/Program.cs` 的参数生成实现。混沌使者已删除，当前对局无需旧效果兼容。

## 修改入口

- 参数／原生效果复制：`HexBuild/Program.cs`；中文名称与描述：`docs/hexes.md`；英文名称与描述：`docs/hexes.en.md`。`HexBuild/HexCatalog.cs` 按相同编号读取两种语言，`HexTexts.cs` 写入并检查文本包；仅更新文案使用 `build-texts.ps1`。
- 奖励等级与一局内去重：`level_rewards.rs` 及构建中的生命周期事件，二者一起核对。
- 测试快捷键：`lib.rs` 中受 `test-hotkeys` 编译特性控制的入口；按键配置为 `dist/mod/natives/NRHex.ini`。Ctrl+F6～F9功能见发行说明，群友版编译时移除该入口。
- 亮剑近身距离：`sword.rs::NEAR_DISTANCE`，固定4米；旧INI字段只兼容忽略。
- 亮剑施法状态：`build_sword_vfx_hks.py` 通知实际施法手，`build_sword_recovery_hks.py` 覆盖原生MagicFire*Cancel收招回调，DLL在后摇沿用刚施放法术的减伤值。`sword.rs::casting_effect` 仅启用减伤和抗打断；风暴管束者幻剑不再添加，`lib.rs::update_sword` 仍清理旧左右手视觉状态。共享风暴管束者资源保留供原生战技与百般武艺使用。
- 人工法术加成：`data/balance/sword.csv`；`build.rs` 每次编译按当前CSV校验并生成 `OUT_DIR/sword_bonuses.rs`，DLL只使用这份静态数值，修改后需重编译。`sword_map.rs` 提供法术身份及攻击映射，不直接手改。
- 战技随机池：从 `ArtsProbe` 原版数据生成 `arts_data.rs`，不要手改静态 ID 清单。

`dist/*.json` 保留当前功能检查结果，各脚本里的历史“version”可能指该检查首次引入的版本，而不是当前完整包。代码版本由 Cargo 管理。运行时加载状态与用户玩法验证分别判断，不用旧日志证明新 DLL 已加载。

## 已知的验证边界

用户已确认精简风暴管束者特效包的测试成功。两份构建的快捷键差异、4米固定距离及目录布局经过构建和相关检查；未将这些检查视为新版本实机或联机验收。其他玩法与联机状态以对应研究记录和用户反馈为准。

恶魔的注视：`src/devils_gaze.rs`负责周期和三角分布，`src/devils_gaze_runtime.rs`负责地面／障碍检测、环境来源和原生弹丸发射；`src/devils_gaze_hit.rs`在私有法阵接触角色时补回场景来源分支缺失的原生攻击免疫筛选，统一阻止免疫时的伤害、受击和发狂；`src/devils_gaze_audio.rs`负责原生Libra声音包的一次性引用；`tools/HexBuild/DevilsGaze.cs`负责原生慢版法阵、深度伤害、精简资源包及参数回读。
