# NRHex

《艾尔登法环 黑夜君临》海克斯 Mod，当前代码版 **0.17.148**。复用原生金色潜在能力，在 2／7／10／13 级独立发放三选一奖励；未领取奖励排队，复活后移动到本人脚下。同次远征中，展示过的海克斯暂不重复，候选轮完后重新循环。海克斯三选一的每栏可使用原生“放置”按键刷新一次。

自测包位于 `dist`，保留 Ctrl+F6～F11；群友版通过 `build-no-hotkeys.ps1` 构建，编译时移除测试快捷键。两版的 DLL／INI 均位于 `mod/natives`。

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

运行 `dist/启动测试版.ps1` 启动。开发构建运行 `./build.ps1`；只验证和构建、不替换正在运行的 DLL 时使用 `./build.ps1 -NoInstall`。

用户维护的 `data/balance/sword.csv`、INI、ME3 以及辅助 Mod 配置不会由资料清理覆盖。研究结论、静态检查、加载检查和实际游戏反馈分别记录，验证状态见实现说明。

可用 `./clean.ps1 -CachesOnly -Apply` 清理编译缓存；不带 `-Apply` 仅预览。脚本不会遍历目录联接或符号链接的目标。详情见 [维护记录](docs/maintenance.md)。
