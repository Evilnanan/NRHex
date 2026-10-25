# MMV 构建目标

采用同仓库、同主分支维护两种发行目标。海克斯逻辑、数值、中文和英文源继续共用；MMV 特有的资源合并及兼容策略放在这里，不复制一份 Rust 代码或文案长期维护。

加载方式为 **现有 MMV 包＋独立 NRHex-MMV 包**，由 ME3 显式声明覆盖优先级。MMV 安装目录作为只读依赖，不改原文件、原 profile 或原 DLL；适配时保留 MMV 自身玩法。遇到同名资源，合并副本只写入 NRHex-MMV 的暂存／发行目录。ME3 的加载顺序只能决定整文件覆盖，不能替代同名资源内部的合并。

**当前已实现全部 23 份资源的合并、配套 DLL 和独立 ME3 配置构建，已确认测试版启动并完成主更新钩子安装；具体玩法、重投与联机仍待实测。** 默认只输出测试版 `dist/mmv/`（含 Ctrl+F6）。群友版编译移除测试热键，与测试版共用合并资源和玩法代码；暂不在仓库内生成，使用 `-Variant community -CommunityDestination <外部绝对路径>` 才构建，`both` 同样必须指定外部目录。不操作游戏进程。`-Variant dist/community/both` 选择目标；`-BuildOnly` 准备完整包但不替换正在使用的版本；`-AuditOnly` 只检查；`-StageOnly` 仅生成并验证合并资源。

独立版入口为 `dist/启动测试版.ps1`，MMV 入口为 `dist/启动MMV测试版.ps1`。两者分别加载自己的资源与 DLL；MMV 配置位于 `dist/mmv/NRHex-MMV.me3`。不同时启动两版。

## 入口与输入

```powershell
./build-mmv.ps1 -AuditOnly
./build-mmv.ps1 -MmvRoot 'D:/Mods/MMV' -AuditOnly
./build-mmv.ps1 -OriginalSfx 'D:/Original/sfx/sfxbnd_commoneffects_dlc01.ffxbnd.dcx'
```

MMV 路径也可以通过 `NRHEX_MMV` 指定。脚本使用当前 `dist/mod` 作为 NRHex 的共享资源输入，与群友版沿用同样的更新顺序：先更新独立版资源，再生成其他目标。不自动调用完整构建，因为现有 `build.ps1 -NoInstall` 仍会修改 dist 的资源。

工具要求 Python 3.12+（含 pefile 和下述 Lupa 依赖）、PowerShell、.NET 10、Rust、Smithbox/SoulsFormats 及匹配版本的原始游戏文件。沿用 `NRHEX_GAME`、`NRHEX_SMITHBOX` 和 `NRHEX_ME3`。检查报告、编译缓存及生成的条件表放在 `work/compat/mmv/`，发行目录为 `dist/mmv/`；不写入原 MMV、独立版 `dist/mod` 或未指定的群友版目录。

`baseline.lock.json` 锁定 MMV 的所有 mod 资源、dll 目录文件和根目录 ME3 配置；日志、存档不计入。除 `.log` 和 `logs/` 外，明确排除已核实为 nraio 运行日志的 `dll/target/release/0987.txt`，不笼统忽略 `.txt` 配置。锁文件不包含 MMV 游戏资源，也不代表该版本已经兼容。内容变化后检查会停止，不能通过每次自动重录锁来跳过复核。

首次选择 MMV 基线时使用 `-RecordBaseline`。已有锁时拒绝覆盖。升级 MMV 时，先保留旧锁、复核新输入，再显式移走旧锁并重录，检查 Git 差异后重新完成兼容验证。锁不绑定安装盘符。

完整原始公共特效包未收录在仓库原始资产中。完整构建需要 `-OriginalSfx '绝对路径/sfxbnd_commoneffects_dlc01.ffxbnd.dcx'`，也可通过 `NRHEX_ORIGINAL_SFX` 提供。该文件旁须有原版 `sfxbnd_commoneffects.ffxbnd.dcx`，用于核对新特效的依赖。两份原版输入都校验已核实的 SHA-256；不自动猜测历史缓存。

## 合并规则和待完成工作

- **参数、文本**：以 MMV 为基线，应用原版 → NRHex 的行／字段／文本条目差异，并回读验证。同编号不同值会停止，新增或删除表／成员等尚不支持的变化也会停止。保留原有重复行及顺序。
- **公共事件**：已合并十份事件包，三个双方修改的事件按已审查边界插入；不改 MMV 原有条件组，检查相对跳过不会跨越插入点，并回读所有事件和参数绑定。MMV 原生监牢开门流程与原版不同，刷怪笼专用开门事件已改为复用 MMV 的原始指令和参数绑定，保留它的普通开门、钥匙结算、提示及动作。
- **动作**：已生成 150 个函数的 HKS 适配稿。137 个前置补丁从共享字节码自动转换为 Lua；13 个内部修改按 `hks-interior.json` 的已审查锚点插入，双方函数哈希变化时停止。保留 MMV 函数体、额外参数和取消条件；逆向撤销全部源修改可还原 MMV 原稿。Lua 5.1 语法与字节码差分执行测试通过，仍需游戏内验证。它不是重新反编译／替换整个 MMV 脚本。
- **TAE**：通过追加表及局部指针更新保留 MMV 原始字节布局，不完整重写 TAE；回读核对所有原有事件、参数字节和分组。交锋窗口以 MMV 武器攻击事件起点为基准，时长共用 `Parry.GuardWindowMilliseconds`。其他新增事件核对启动时序，大招标记跟随 MMV 原生释放／量表事件。重复动画 ID 按实际条目处理，空动画保留。为 DLL 生成 MMV 专用 `parry_data.rs`；只对实际引用的攻击参数要求同编号相关值一致，不能全表随意取第一行。
- **地图、特效**：按实体／成员合并，检查实体 ID、模型、奖励引用和出生位置。MMV 已占用部分 NRHex 恢复的特效成员，不能默认 NRHex 优先。亮剑当前已停用风暴幻剑：`sword::casting_effect` 仅启用减伤和抗冲击，旧手部视觉只参与清理。包内仍保留共享风暴管束者资源，供原生战技和百般武艺使用；不能据此判断亮剑仍使用该外观。适配时保留 MMV 对共享风暴特效和模型的修改，不重新启用亮剑旧视觉。
- **地图**：四张地图仅添加刷怪笼条目，检查入口位置、名称和实体／组 ID 冲突。撤销新增条目后必须逐字节恢复 MMV 原地图的解压数据。
- **DLL、加载配置**：编译共享代码，两份均启用 `cage-demand,mmv-compat`，测试版另启用 `test-hotkeys`；打包时通过 PE 导入表校验热键轮询是否存在。`build.rs` 经 `NRHEX_PARRY_DATA` 选择 MMV 条件表；未提供时继续使用独立版表。新配置引用 MMV 原目录和原 DLL，保留存档名、在线设置、重定向器及其他 native 项；显式 `load_after = [{ id = "mod", optional = false }]` 让适配包覆盖 MMV 同名资源。ME3 0.11.0 的依赖排序和文件映射源码已核对：后加载的文件覆盖前者。配置用本机 `profile show --file` 校验。测试版已与原 MMV DLL 一同加载并完成初始化；这不代表所有原生钩子的玩法交互都已验证。

构建必须覆盖当前共享资源集合中的每个文件，并通过各格式回读、输入前后哈希核对，才输出 `dist/mmv/NRHex-MMV.me3`、资源和 DLL。报告标注 `built-awaiting-game-test`，不把构建成功写成实测通过。后续构建保留用户 INI；检测到手动修改生成的资源或 profile 时停止，不静默覆盖。构建记录留在 work，不放入发行包。

原 MMV profile 的“Optional”段列出 `NightreignFPSFOV.dll`、`UnlockTheFps.dll` 和 `UltrawideFix.dll`，但没有设置 `optional=true`。这三个文件缺失时，仅在新生成的适配 profile 内标记为可选，防止 ME3 中断后续 DLL／资源加载；原 MMV profile 不变。其他必需 DLL 缺失仍阻止构建。

首次启动定位到 `RestoredLegendaryStones.dll` 已挂接物品显示复制函数 `0x8B7FC0`，使 NRHex 的原始签名扫描拒绝初始化。`mmv_compat.rs` 只在 MMV 构建中认可这一条已审查链：核对该 DLL 文件哈希、内存中的完整 hook 函数体、原函数未覆盖的签名字节、跳转中继目标和原函数 trampoline 目标。NRHex 继续调用已有入口，不卸载或覆盖 MMV 的 detour；其他签名仍严格检查。只读现场覆盖 204 个签名，只有这一处变化；入口 ABI 和转发分支已核对，重启后已确认该兼容链通过检查、测试热键启用及主更新钩子安装；实际发球与重投仍待游戏内验证。

## 同步更新与验证

### 新增内容覆盖

资源合并之外，已逐项审视 42 个海克斯对 MMV 新增敌人、武器、战技和法术的识别，见[完整覆盖报告](../../docs/research/mmv-hex-coverage.md)。初次审计的四个主要缺口中，回响施放已加入 39 个新法祷／96 个入口；大法师、百般武艺和强化死灵术的静态名单仍待适配。亮剑和交锋已有 MMV 专用数据。新增动作通知、奖励入口及联机效果仍需分批验证，不能把合并成功视为所有新内容已适配。

### 回响新增法祷

`compat/mmv/echo-spells.json` 记录每个新增 Magic 的取舍和审查指纹；`tools/build_mmv_echo.py` 读取合并参数，生成 `staged/echo_cast_data.rs`，通过 `NRHEX_ECHO_DATA` 注入同一套回响运行时。普通／蓄力根、六种持续施法配置、混合型法术的原地范围段分别处理；所有新增条目隔离原施法的命中记录，持续回响内部继续共享自己的命中列表。独立版使用原目录，五个用户指定例外保持。

新增支持、暂未支持的动作法术、验证边界见[MMV 回响适配](../../docs/research/mmv-echo.md)。输入 Magic／弹体链变化或出现未审核的新法术时生成器停止；不能只更新审查哈希绕过复核。纯个人效果、公共空辅助根和无名称内部动作不纳入。构建成功不等于完整视觉、碰撞或联机验收。

### 亮剑自动估算

两份手填 CSV 仅保留历史数据，不再控制 DLL。构建从合并后的 MMV regulation 和中文名称导出 Magic、Bullet、AtkParam_Pc，自动生成 `work/compat/mmv/staged/sword_map.rs`，通过 `NRHEX_SWORD_DATA` 编入共享运行时。当前收录 160 个原有、44 个新增法术；加成＝估算原飞行距离（米）对应的百分比，没有可比较飞行距离时为 0%。`docs/research/sword-auto-mmv.csv` 是只供查看的生成结果。

新增法术的自动图遍历不是完整实机验收。融泥块按投掷载体首次落地估算；融泥漩按喷流最远外扩估算，不累计回流路程，也不把高速竖直探地弹的寿命当成400米射程。其他已核对的阶段规则和快速下坠见 `docs/research/sword-auto.md`。原 MMV 文件仍为只读。

日常变更只改共享逻辑、数值和文案；独立版资源更新后重新运行 MMV 检查。MMV 适配代码只处理必要差异，不引入第二套海克斯实现。短期开发分支可用于评审，但不长期维持独立版／MMV 两条内容分支。

当前验证命令：

```powershell
python -m unittest discover -s tools -p test_mmv_build.py
python -m pip install --target .work/mmv-python -r compat/mmv/requirements.txt
python -m unittest discover -s tools -p test_mmv_hks.py
dotnet build tools/HexBuild
./build-mmv.ps1 -AuditOnly
```

后续可玩版本需要验证启动、圆桌、升级领奖、昼夜与 BOSS 生成、刷怪笼、动作海克斯、施法，以及双方使用相同适配包的联机。构建或审计成功不代替这些实测。
