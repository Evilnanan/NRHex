# Mayhem Augments

Mod 正式名称为 **Mayhem Augments**；`NRHex` 仅作为源码、DLL、INI 和日志的内部名称。

《艾尔登法环 黑夜君临》海克斯 Mod，当前代码版 **0.17.263**。复用原生金色潜在能力，在 2／7／10／13 级独立发放三选一奖励；未领取奖励排队，复活后移动到本人脚下。同次远征中，展示过的海克斯暂不重复，候选轮完后重新循环。海克斯三选一的每栏可使用原生“放置”按键刷新一次。

自测包位于 `dist`，保留 Ctrl+F6；群友版通过 `build-no-hotkeys.ps1` 构建，编译时移除测试快捷键。两版的 DLL／INI 均位于 `natives`。

纯净发行版使用 `./build-release.ps1`，输出 `releases/Mayhem Augments-<版本>/` 和同名 ZIP。解压后通过 ME3 加载 `Mayhem Augments.me3`；仅包含本 Mod，不带 MMV、辅助 Mod 或测试快捷键。需要自行安装 ME3；发行配置不指定在线或 Arxan 选项。

- [使用方式与测试快捷键](dist/README.md)
- [全部已实现海克斯与中文文案](docs/hexes.md)
- [英文名称与翻译](docs/hexes.en.md)
- [研究资料索引](docs/README.md)
- [开发环境与构建](docs/build.md)
- [源码与数据入口](docs/implementation.md)
- [维护约定](docs/maintenance.md)
- [历史变更](CHANGELOG.md)

## 目录

| 目录 | 内容 |
| --- | --- |
| `src/` | Rust DLL、状态机、原生 hook，以及构建生成的静态表 |
| `tools/` | 当前构建器、可复用探针和检查脚本 |
| `assets/original/` | 保留的原始 HKS、动画、消息、事件和地图输入；`manifest.json` 记录哈希 |
| `data/reference/` | 按主题整理的原生参数、事件与验证夹具；不包含旧版 Mod 包 |
| `docs/` | 当前实现说明、长期研究结论和图标图鉴 |
| `dist/` | 当前可运行包、INI／ME3 配置及检查报告 |
| `.work/` | 可重新生成的参数导出、反编译稿和构建检查中间文件 |
| `target/` | Cargo 构建缓存，可删除 |

运行 `dist/启动测试版.ps1` 启动。完整构建使用 `./build.ps1`；仅改运行时代码时使用 `./build.ps1 -DllOnly`，加 `-NoInstall` 可只验证编译、不替换 DLL。文案更新使用 `./build-texts.ps1`，群友版打包使用 `./build-no-hotkeys.ps1`。完整构建的 `-NoInstall` 仍会更新资源，详情见 [构建说明](docs/build.md)。

退出卡住或Steam一直显示“正在停止”时，双击 `dist/强制结束游戏.cmd`；清理脚本仅保留在dist，不随群友版打包。它强制结束黑夜君临，并清理可确认归属的ME3启动器残留，不关闭Steam、不重启游戏、不改存档；强退可能结束当前出征。可先运行 `powershell -NoProfile -ExecutionPolicy Bypass -File dist/Stop-Nightreign.ps1 -WhatIf` 仅预览。权限不足时再右键CMD以管理员身份运行；无法确认归属的其他ME3进程不会被结束。

用户维护的 INI、ME3 以及辅助 Mod 配置不会由资料清理覆盖。研究结论、静态检查、加载检查和实际游戏反馈分别记录，验证状态见实现说明。

可用 `./clean.ps1 -CachesOnly -Apply` 清理编译缓存；不带 `-Apply` 仅预览。脚本不会遍历目录联接或符号链接的目标。详情见 [维护记录](docs/maintenance.md)。
