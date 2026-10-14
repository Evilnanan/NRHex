# 开发与构建

本页保留构建用法。历次增量修改记录见 [历史变更](../CHANGELOG.md)；限定旧输入哈希的增量工具只用于相应历史基线。

DLL和INI位于`dist/mod/natives`；ME3的原生DLL条目也指向`mod/natives/NRHex.dll`。亮剑近身判定固定在`src/sword.rs`的`NEAR_DISTANCE=4.0`，INI不再控制该数值。

亮剑触媒特效可通过 `./build-sword-vfx.ps1` 增量构建：沿当前资源添加左右手视觉与施法手标记，检查后仅替换 DLL、参数包及动作脚本，保留其他资源与配置。完整构建也包含相同改动。该命令不启动游戏。

默认构建启用Cargo的`test-hotkeys`功能，保留自测快捷键。已有最新dist游戏资源后，可运行`./build-no-hotkeys.ps1`，以`--no-default-features`单独编译不含快捷键的DLL，并将当前资源复制到`E:\Projects\me3-manager\nightreign-mods\NRHex`。可用`-Destination`指定其他目录。此脚本复用当前dist资源，不重建参数、地图或文本；修改这些内容时先完成正常构建。独立DLL产物保存在`.work/no-hotkeys-target`，不会覆盖自测DLL。

群友版保留正常升级发球；测试按键检测在编译时移除，添加INI配置不能开启。打包保留已有ME3联机及其他DLL条目，移除废弃的快捷键／射程配置，核对发行DLL不导入`GetAsyncKeyState`，并回读所有复制文件的哈希。已有旧`natives`目录应先迁移到`mod/natives`；本项目和指定管理器目录已迁移。

## 当前环境

| 工具／输入 | 本机位置 |
| --- | --- |
| 工作区 | `D:/Projects/NRMod/NRHex` |
| 游戏 | `D:/Program Files (x86)/Steam/steamapps/common/ELDEN RING NIGHTREIGN/Game` |
| Python 3.14 | `D:/Program Files/Python314/python.exe` |
| Smithbox／SoulsFormats | `D:/Program Files/Smithbox` |
| DSLuaDecompiler | `D:/Program Files/DSLuaDecompiler/DSLuaDecompiler.exe` |
| ME3 | `D:/Program Files/me3/bin/me3.exe` |
| .NET | .NET 10 SDK |
| Rust | Windows x64 MSVC 工具链 |

Python 依赖见 `tools/requirements.txt`，版本按当前已安装且可执行的环境记录。源码仍有本机工具路径；迁到另一台机器时需统一替换这些路径。原始游戏文件不从 Mod 产物反推。

2026-10-09 临时电脑工作区位于 `E:/Projects/NRMod/NRHex`，Python、Smithbox、ME3 与游戏位于 E 盘，.NET 10 SDK 与 Rust 工具链已可用。刷怪笼专用入口 `./build-cages.ps1` 使用当前 Python，并默认从项目所在盘查找 Smithbox、游戏和 ME3；可用 `NRHEX_SMITHBOX`、`NRHEX_GAME`、`NRHEX_ME3` 环境变量覆盖。`HexBuild` 的库路径还可通过 `-p:SmithboxDir=...` 覆盖。启动脚本也支持上述游戏和 ME3 路径。

`build-cages.ps1` 先验证原输入和 EXE 的当前签名集合，再按既有参数包重建刷怪笼事件与地图，核对 NPC 副本、原生标志存储、本地出场及血条控制流，编译 DLL 并校验 ME3 配置。仅四张监牢事件文件允许变化，参数、HKS、地图和用户配置均做哈希范围比较；本次报告为增量检查，不继承旧电脑的运行状态。`-NoInstall` 只跳过 DLL 替换。其他旧探针与完整 `build.ps1` 仍使用上表 D 盘路径，迁移完整构建时需另行适配；本次修复不需要 DSLuaDecompiler。

保留动作参考 `.lua` 使用 LF 换行，并由 `.gitattributes` 固定，避免 Windows 检出换行转换破坏原始参考哈希；没有放宽清单校验。

## 特殊效果栏增量构建

```powershell
./build-effect-sync.ps1
# 只读导出原生及当前效果参数：
dotnet run --project tools/HexBuild -- . --effect-sync-probe
# 在隔离内存中执行当前EXE的匹配谓词，不启动或注入游戏：
python -X utf8 tools/verify_effect_sync_native.py
```

该入口同样支持临时电脑的工具路径。验证原输入、49个签名及153项Rust测试，修改18个效果行的21个字段并回读，执行原生同ID匹配／有效查询检查，编译并安装DLL，核对ME3配置。只有参数包和DLL允许变化，0.17.9刷怪笼资产及用户配置必须保留；不打包。参数增量仅接受已核对的0.17.9输入，或已有研究报告的本次输出；其他参数包需完整构建。首次研究期间已经生成参数时，使用 `-CaptureResearchBaseline` 从保留的改前参数恢复比较基线，普通重建无需此开关。

原生调查工具 `tools/effect_sync_native.py` 只读反汇编当前EXE，`tools/EffectSyncReport.java` 在隔离的Ghidra项目内导出指定函数。研究中间件在 `.work/effect-sync`，当前构建证据为 `dist/effect-sync-validation.json`、`dist/build-validation.json` 和 `dist/runtime-validation.json`。离线接收模型只执行原生匹配谓词，不是实际网络收包测试。

## 完整构建

```powershell
./build.ps1
# 游戏运行时只构建并检查，不覆盖 DLL：
./build.ps1 -NoInstall
```

构建先校验 `assets/original` 和 `data/reference` 的清单，然后导出当前原生战技、生成 Rust 静态表、重建参数／文本／事件、组合原始 HKS 补丁、重建后跃 TAE、执行原生路径检查、编译 DLL、扫描 EXE 的当前签名集合并校验 ME3 配置。

HKS 顺序固定为 `build_walljump.py → build_jiahao.py → build_arts_hks.py`，每次从原始输入开始。不能直接对已包含同一补丁的文件再施加一次。完整构建覆盖的是生成资产，`-NoInstall` 仅跳过最后的 DLL 安装步骤。

```powershell
cargo test --lib
cargo run --release --bin scancheck
& 'D:/Program Files/Python314/python.exe' -X utf8 tools/verify_workspace.py
```

这些检查不启动游戏，不替代实际动作、联机和视觉确认。

游戏已经运行时，可以执行 `tools/check_loaded.py` 只读确认当前 DLL 的 ACTIVE 状态与脚本当前检查的原生 hook。记录当前构建由 `tools/record_build.py` 完成；不会继承旧版本的加载或实测成功声明。

## 输入与可再生成数据

`assets/original/manifest.json` 绑定保留的 HKS、动画、原版文本、地图和事件；`data/reference/manifest.json` 绑定已有研究夹具。它们受版本约束，升级游戏要重新导出、研究并更新清单，不能绕过哈希校验。

`.work` 的子目录由构建自动创建，整个目录可删除。`ArtsProbe` 每次生成 `.work/arts/parameters.json` 与中英文正式名称；`HexBuild` 导出状态参数、法术倍率和二进制测试行；HKS 独立反编译稿放 `.work/hks`。原先读取旧版本构建目录的验证脚本均改为读取这些固定位置或保留的参考夹具。

研究用探针可独立运行：`SwordProbe` 导出法术引用图，`BackstepProbe` 导出攻击／处决／无敌帧，`FlaskProbe` 查看事件控制流，`TaeProbe` 查看墙跳动画，`BehaviorProbe` 导出行为包，`SkillProjectileProbe` 导出战技投射物动画。输出多为 `.work`；更新长期夹具前应核对来源和修改范围。

中文名称与描述在 [docs/hexes.md](hexes.md) 编辑，英文在 [docs/hexes.en.md](hexes.en.md) 编辑。构建直接读取各项标题和描述，以标题前的 `hex` 编号注释关联效果；两种语言可以独立调整顺序，保留编号、使用纯文本即可。构建会分别检查缺项、多余项、空文案、重复编号及重复名称，不回退到旧翻译或自动翻译另一种语言。描述中的数值只是文案，不会修改机制。

附加效果在 `data/text/status-effects.json` 中编辑：`zhInfo`／`enInfo` 为说明，`zhName`／`enName` 可单独指定名称，省略时继承对应语言的能力名称，`zhSuffix`／`enSuffix` 为后缀。保留 `textId` 和 `ownerId`。

修改文案后在仓库根目录运行：

```powershell
./build-texts.ps1            # 构建并更新 dist 的中英文能力／状态文本
./build-texts.ps1 -Check     # 只核对现有 dist 与文案是否一致
./build-texts.ps1 -NoInstall # 只生成到 .work/texts，不更新 dist
```

文案构建回读现有发行包，核对全部两种语言的五份能力 FMG 和两份状态 FMG，验证无关条目和成员未变后安装；不重建 DLL、参数、地图或事件。失败时不会安装未通过检查的产物，安装失败会尝试恢复备份。报告及安装前备份位于 `.work/texts`。重启游戏加载更新；完整 `./build.ps1` 也会读取这两份 Markdown 和状态 JSON。群友版需随后运行 `./build-no-hotkeys.ps1` 重新打包，这个文案入口只更新项目 dist。

底层双语入口为 `HexBuild --stage-translations` 和 `--check-descriptions`；后者检查源与现有包，不生成文本，编辑后尚未构建时不一致是正常的。历史 `--texts-only` 保留中文暂存行为供旧工具使用。`--export-descriptions` 是旧版中文导出探针，不覆盖文案源。

`data/balance/sword.csv` 为 UTF-8（读取也接受 BOM），加成可含小数；`cargo build --release --lib` 通过 `build.rs` 校验当前CSV并将数值静态嵌入DLL，CSV改动会自动触发重编译。DLL运行时使用内置表，修改CSV后必须重新编译、替换DLL并重启游戏。输入缺失、身份重复／不完整或数值非法时构建失败，用户CSV保持原样。当前校验以 `build_sword_ranges.py` 和 `sword_csv.rs` 为准，表中“加成”直接用于增伤和咏唱减伤，吸血百分比使用该数值×2；不能直接将CSV列乘2，否则会连带提高另外两项。

## 再生术数值增量更新

```powershell
python -X utf8 tools/record_regen_balance.py --before
dotnet run --project tools/HexBuild/HexBuild.csproj -- . --regen-balance-only
cargo test --lib
cargo build --release --lib
Copy-Item target/release/NRHex.dll dist/mod/natives/NRHex.dll -Force
python -X utf8 tools/record_regen_balance.py --rust-tests 158
```

修改已安装参数包95000601为每跳回复16HP、消耗4FP，DLL资源门槛同步为4FP；重新解密回读后核对所有其他参数字段和表。包级检查保留事件、文本与用户配置。测试数量应与当次实际结果一致，不打包。

## 当前调试快捷键

亮剑施法保护与历史视觉资产的增量更新使用 `./build-sword-vfx.ps1`，会生成参数、手持与后摇动作标记、检查风暴管束者架势幻化剑资源并运行Rust测试。游戏需退出后才能安装DLL，群友版随后运行 `./build-no-hotkeys.ps1`。当前DLL不再添加亮剑幻剑特效；450955及其模型／纹理保留供原生战技与百般武艺使用，无需Node依赖或新特效副本。资源检查会移除已知的旧罗蕾塔放大副本，并验证其余成员保持原样。

自测版保留 Ctrl+F6～F10，说明见 [发行包用法](../dist/README.md)。群友版编译时移除测试按键入口。亮剑距离固定4米，不属于INI配置。

## 大法师

`HexBuild --archmage-stage`从尚未包含大法师的发行参数包生成新增能力与奖励行，产物及读回报告在`.work/archmage/staged`。`tools/build_archmage_hks.py`从当前动作组合增加34个施法回调通知，默认暂存到`.work/archmage`，`--inplace`用于完整构建。独立检查为`verify_archmage_native.py`与`verify_archmage_decompile.py --decompiler <DSLuaDecompiler.dll>`。`install_archmage.py`校验所有当前输入和暂存哈希，安装前需要退出游戏。中文、英文文案仍在两份海克斯Markdown中维护。

大法师副槽限定的增量构建使用`HexBuild --archmage-secondary`，暂存于`.work/archmage-secondary/staged`，保留其余参数字段。`verify_archmage_secondary_native.py`检查原生选槽、耗蓝与对应调用点；完整构建的拥有行也保持中性耗蓝倍率，由DLL按实际选槽处理。

招架的增量构建使用 `HexBuild --parry-stage`，生成参数、文案、奖励表及动作数据到 `.work/parry/staged`；`--parry-actions` 仅重建动作和削韧查询表。`build.ps1` 在编译DLL前生成并复制 `parry_data.rs` 和动画包。`verify_parry_native.py` 对照当前EXE核对原生状态查询和攻击手选择；增量安装入口为 `tools/install_parry.py`。

招架后续防御更新：`HexBuild --parry-support`校验原生精准防御火花和连续防御时长，增量生成私有标记／视觉参数；配合`--parry-actions`、文案暂存和DLL构建，使用`tools/install_parry.py --guard-update`安装配套资源。

招架防御反击增伤的增量构建：`HexBuild --parry-counter` 从当前 dist 添加原生分类103的双倍增伤参数并回读校验，`install_parry.py --counter-update` 校验哈希、备份并替换参数、DLL及双语文本；完整构建由 `Parry.Support` 生成同一行。

黑夜赐福增量构建使用 `HexBuild --night-blessing`，从尚未包含该能力的发行包生成参数、奖励、双语文本及姿势／赐福资产，产物位于 `.work/night-blessing/staged`。复制生成的 `hex_pool.rs` 后构建 DLL，使用 `tools/install_night_blessing.py` 校验输入、备份并安装。`--night-blessing-actions` 可重复生成姿势与赐福标记，完整 `build.ps1` 已接入。原生共享赐福脚本保留于 `assets/original/script/talk` 并纳入源文件清单。

招架防御通知与连续防御修复：`HexBuild --parry-followup` 生成中性动作标记参数和57个原生防御反应的动画窗口；`--parry-followup-actions` 支持完整构建重复合成。产物及回读检查位于 `.work/parry-fix/staged`。`verify_parry_native.py` 同时核对最终受击通知、原生guard种类和身体接触筛选所依赖的代码。

`verify_parry_feedback_native.py` 在私有内存中执行当前EXE的火花描述符转换和实例选择代码，以受控桩替代分配、渲染和角色分类依赖，验证左右手每次动作开始新建实例、持续帧不重复生成。完整构建已接入；它验证原生分支，不代表游戏中的粒子显示验收。

`verify_parry_turn_native.py` 隔离执行原生朝向设置与读取函数，验证八个方向、角度回读及位置／其他物理字段不变；完整构建同时核对新增设置接口签名。

不祥契约和偷心者通过参数及本地common事件实现。新增时可用 `dotnet run --project tools/HexBuild -- . --contract-heart` 从现有dist生成增量产物；仅接受尚未添加这两项的输入，安装前回读参数、奖励图、事件和中英文文本。当前已安装版本使用 `--contract-heart-check` 导出验证夹具，再运行 `tools/verify_contract_heart_native.py` 和 `tools/verify_contract_heart_events.py`。完整构建已包含同样的生成与检查；两项的Ctrl+F11入口随群友版编译移除。
