# 开发与构建

仓库只保留三个构建入口。新增能力、参数、事件、动作和特效统一使用完整构建；仅改 Rust 代码时使用 DLL 模式。原先按海克斯划分的九个专项 PowerShell 入口及其九个不再被引用的安装／差异记录脚本已删除，有效验证已接入统一流程。底层仍被使用的生成器和研究工具保留；旧入口可从 Git 历史恢复，历史用法见 [CHANGELOG.md](../CHANGELOG.md)。

| 入口 | 用途 |
| --- | --- |
| `build.ps1` | 默认完整构建；`-DllOnly` 仅验证、编译和更新 DLL |
| `build-texts.ps1` | 构建或检查中英文文案 |
| `build-no-hotkeys.ps1` | 编译无测试热键的 DLL，搭配当前 dist 资源打包群友版 |

## 完整构建

```powershell
./build.ps1
./build.ps1 -NoInstall
```

完整流程校验原始输入与夹具，生成参数、文本、事件、地图、动作和特效资源，运行原生与脚本检查，再验证、编译并安装 DLL，检查 ME3 配置和记录结果。

`-NoInstall` **只跳过最后的 DLL 替换**，资源和生成代码仍会更新。完整构建需要匹配版本的游戏资源与特效包；ME3 配置必须同时加载 `mod/natives/NRHex.dll` 和 `mod` 资源包。DLL 和 INI 位于 `dist/mod/natives`。

动作构建从保留的原始输入开始，再按主脚本顺序组合补丁。不要把单项 HKS 补丁重复施加到已打补丁的文件。主脚本检查咏唱通知后加入救护车的方向查询和起步费用补丁；两种构建模式均运行救护车的原生隔离与HKS费用验证。

## 仅更新 DLL

```powershell
./build.ps1 -DllOnly
./build.ps1 -DllOnly -NoInstall
```

此模式复用现有资源，执行常规 Rust 测试，以及死亡之舞、侵蚀、潜能分裂、奖励模板、救护车、回响施放、武器倾向和加餐的编译后原生隔离验证，再编译 DLL 并检查当前游戏 EXE 签名。它不需要重新导出参数、重建动作或读取完整特效包；加餐检查会回读当前参数包，也不调用 ME3。

默认只替换 `dist/mod/natives/NRHex.dll`。加 `-NoInstall` 后不修改 dist；构建记录保存在 `work/build/dll-validation.json`，不覆盖完整资源构建的验证报告。适用于互斥、抽取限制等仅修改运行时代码的任务；新增参数或资源仍须完整构建。

## 文案更新

中文名称与描述在 [hexes.md](hexes.md) 编辑，英文在 [hexes.en.md](hexes.en.md) 编辑，附加效果文本在 `data/text/status-effects.json` 编辑。构建以稳定的海克斯编号关联内容；描述里的数字不会自动改变机制。

```powershell
./build-texts.ps1
./build-texts.ps1 -Check
./build-texts.ps1 -NoInstall
```

默认仅更新 dist 的四份中英文消息容器。`-Check` 只检查当前包是否匹配文案源；`-NoInstall` 只生成并验证暂存结果。安装前备份，回读失败会恢复；不重建 DLL、参数、地图或动作。完整构建也读取相同文案源。

## 群友版打包

```powershell
./build-no-hotkeys.ps1 -Destination 'D:/Mods/NRHex'
```

先保证 dist 资源是最新版本，再运行打包。脚本以 `--no-default-features` 单独编译无测试快捷键 DLL，搭配现有 dist 资源复制到目标目录，保留目标已有的 ME3 和用户设置，并检查必要的 Arxan 设置与 DLL 导入。群友版不能通过 INI 重新开启测试热键。

未指定 `-Destination` 时沿用脚本中的本机默认目录。独立 DLL 产物放在 `.work/no-hotkeys-target`，不会覆盖 dist 自测 DLL；打包记录只留在项目 `.work/no-hotkeys-package`。

## 工具和环境

构建使用 PowerShell、Python、.NET 10、Rust Windows 工具链、Smithbox/SoulsFormats、ME3，以及需要动作反编译时的 DSLuaDecompiler。Python 依赖见 `tools/requirements.txt`。原始 EXE 与 regulation 必须匹配项目验证的版本。

主构建入口支持：

| 环境变量 | 用途 |
| --- | --- |
| `NRHEX_GAME` | 包含 `nightreign.exe` 和原始 `regulation.bin` 的游戏目录 |
| `NRHEX_SMITHBOX` | Smithbox 库与参数定义目录；由底层 .NET 工具读取 |
| `NRHEX_ME3` | ME3 可执行文件路径 |
| `NRHEX_CARGO_TARGET` | 可选 Rust 编译目标，例如 `x86_64-pc-windows-gnu`；DLL 从对应目标目录安装 |
| `NRHEX_NATIVE_EXE` | Windows 测试进程及签名检查程序所见的 EXE 路径，供 Wine 等跨目标环境使用 |

未设置路径时，主脚本沿用项目所在盘的默认安装位置。其他底层历史工具仍可能包含本机路径；主脚本的环境变量支持不代表完整构建已在所有平台验证。Linux 跨目标 DLL 模式需要可用的 Windows linker 和 runner；完整资源构建还依赖对应的原始资产与压缩工具。

## 验证与保留输入

完整构建保留已有原生、动作和资源检查，并补入此前专项入口独有的刷怪笼组合、效果同步验证。效果同步在完整构建中检查当前生成参数，不要求存在旧版本迁移的差异报告。

两种主构建模式均执行常规 Rust 测试及四项编译后原生隔离验证。隔离执行只操作私有映像，不访问实际游戏进程；通过构建与签名检查不等于游戏、动作或联机验收。

`./tools/verify_build_entrypoints.ps1` 用外部工具夹具检查真实主脚本的流程：完整构建、DLL模式、禁止安装、失败中止，以及跨目标产物和EXE路径。它不运行实际资源生成器或修改发行包。

`assets/original/manifest.json` 和 `data/reference/manifest.json` 约束保留的输入与研究夹具。升级游戏时应重新导出、研究和更新清单，不能绕过失配检查。`.work`、`work` 与 `target` 用于生成数据和缓存，不作为原始游戏数据来源。

用户维护的 CSV、INI 和其他 Mod 配置应保留。`data/balance/sword.csv` 在 DLL 编译时静态嵌入，修改后需要重新编译 DLL；其中的加成值不能为改变回收倍率而统一翻倍。亮剑近身范围当前固定为5米，不由 INI 控制。

构建后重启游戏加载 DLL、动作或文本。运行状态只读检查使用 `tools/check_loaded.py`，不继承旧版本的加载或实测结论。
