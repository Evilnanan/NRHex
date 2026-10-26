# MMV-Fixes

独立的 MMV 原包问题修复层，不依赖 Mayhem Augments / NRHex。不修改 MMV 原目录、海克斯参数或 DLL。包含资源包和独立的 `natives/MMVFixes.dll`。当前包含：

- **炎之恶魔 c6220 骨骼越界**：原行为节点 `Fall_Twist_Neck/Head` 引用 124/130，但该骨架只有 124 根骨骼（0～123）。改为实际 `Neck=90 / Head=91`，只改四个 16 位索引字段。该错误与 2026-10-26 19:51 的 `hkbTwistModifier` 崩溃吻合；转储未包含角色堆对象，仍需进游戏复测，不能认定已排除所有崩溃原因。
- **DLC 图标缺失**：补回大图标 21297、小图标 22297，以及 20520～20529。添加两张补充图集，只注册缺失索引；MMV 原有布局、纹理及新增图标保持原样。
- **池沼／锻造村地图标记缺失**：这两种地点的纹理原本完整。MMV 将其放入旧据点槽位后，原生图标定义查找仍按该槽位的旧类型筛选，返回失败，导致主图标和详情图标为零。独立 DLL 仅接管图标缓存构建中的一次查找调用：保留成功的原生结果；失败时，只对实际地图 5000～5006、5010～5011 查验池沼／锻造村定义中的地图列表和图标 77／78，再交回原生代码填充图标。不会修改据点允许类型、地形生成、事件、奖励或参数包。当前 EXE 身份和调用点签名不符时停止安装。隔离执行已复现问题；游戏内验收另行记录。

## 使用与移除

`mod/` 是一个独立 ME3 文件包，加载在 MMV 和其他相关覆盖包之后，包 ID 为 `mmv-fixes`；同名 native 项提前加载 `natives/MMVFixes.dll`。`MMV-Fixes.me3` 可直接运行 MMV + 本修复包，不包含海克斯 DLL。已经使用其他 ME3 配置时添加这两个加载项，不必换掉原配置。

MMV 上游修复后，在实际使用的 `.me3` 配置中移除 `id = "mmv-fixes"` 的 package 和 native 两个项，再删除本目录即可。其他 mod 无须重建。不要直接删文件夹却保留必需的加载项。

仓库内可用以下命令从海克斯 MMV 测试配置移除（只改这一项）：

```powershell
python -X utf8 tools/mmv_fixes.py --disable-profile dist/mmv/NRHex-MMV.me3
```

海克斯 MMV 构建只保留已明确启用的修复项；移除该项后不会自动重新启用。

## 版本核对

`manifest.json` 记录原始 MMV 输入和修复输出的 SHA256。此包覆盖完整资源文件，MMV 更新后不能盲目继续使用旧包，否则可能遮盖上游的新内容。重新构建会核对输入散列，失配时停止，要求重新审查或移除相应修复。

构建配方位于仓库 `compat/mmv-fixes/`，独立构建命令：

```powershell
python -X utf8 tools/mmv_fixes.py --mmv "D:/Users/OwO/Documents/me3-profiles/MMV" --output "D:/Users/OwO/Documents/me3-profiles/MMV-Fixes"
```

图标原始文件需从当前游戏只读提取到 `work/compat/mmv-fixes/original/menu/`，或用 `--original` 指定含 `menu/` 的目录。构建工具使用本机 Smithbox、游戏 Oodle 与 ME3，不随包分发这些工具或 DLL。
