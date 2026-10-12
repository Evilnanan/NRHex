# 数据定位、提取与工具调用

## 本机线索

2026-10-03 使用过的路径；其他机器或未来版本先核对，不要求这些位置必须存在：

| 项目 | 路径 |
| --- | --- |
| 游戏 | `D:\Program Files (x86)\Steam\steamapps\common\ELDEN RING NIGHTREIGN\Game` |
| ME3 | `D:\Program Files\me3`，执行程序 `bin/me3.exe` |
| Smithbox | `D:\Program Files\Smithbox` |
| WitchyBND | `D:\Program Files\WitchyBND` |
| Ghidra | `D:\Program Files\ghidra` |
| DSLuaDecompiler | `D:\Program Files\DSLuaDecompiler` |
| ME3 配置 | `%LOCALAPPDATA%\garyttierney\me3\config\profiles` |

EXE 使用 `(Get-Item -LiteralPath <exe>).VersionInfo`；参数包解开后的 `_witchy-bnd4.xml` 有内部 `version`。当时 EXE 为 1.3.3.0、regulation 为 10350000、ME3 为 0.11.0、WitchyBND 为 3.0.1.0。

## 从名称追到数据

Smithbox 的本机资料比重新下载整份项目方便：

- `Assets/PARAM/NR/Param Row Names/English/<Param>.json`：行名称，结构通常为 ID 和 Entries。
- `Assets/PARAM/NR/Defs/<Param>.xml`：字段定义、位域与适用版本。
- `Assets/PARAM/NR/Param Meta/<Param>.xml`：默认值、引用等解释。
- `Assets/Aliases/NR/EventFlags.json`：事件标志线索。
- `Assets/File Dictionaries/NR-File-Dictionary.json`：Archive、Path、Validated 等资源索引。

用中英文名称、同义词搜索行名称，再查看本机实际行。PARAM 的定义可能包含 `FirstVersion` / `RemovedVersion`，未应用内部版本过滤的手写偏移计算容易出错。位域也不能逐字段简单累加字节。Smithbox、SoulsFormats 或 Witchy 的版本化读取优先于猜偏移。

PARAM 文件名与定义名不总是一一对应：`ItemLotParam_enemy/map` 使用 `ItemLotParam` 定义，部分带 PC/NPC 后缀的参数也共享基础定义。按当前库和实际行尺寸选择定义，不靠去后缀猜全部表。适用范围为 `FirstVersion <= 当前内部版本 < RemovedVersion`；若库不能直接处理版本条件，可先过滤 XML，再生成本次研究用定义，保留原定义。

有些表允许多个相同 ID 的条目，例如抽取表或 ItemTable 的候选行。读取引用前确认该表的基数，用分组/多值索引，而非将所有 PARAM 强行转成 ID 唯一字典。新建行检查碰撞，同时审查 `disableParam_*` 等模板标志；不要把编辑器展示出来等同于游戏正在使用。

## regulation 与 WitchyBND

先复制 regulation 到研究目录，然后：

```powershell
& '<WitchyBND>/WitchyBND.exe' -p -u -c '<research>/regulation.bin'
```

- `-p` 禁止交互提示和结束等待，适合脚本；`-u` 只解包；`-c` 递归序列化 binder 内的 PARAM。
- 本次 Windows 非 TTY 调用没有可见输出；相同程序以 TTY 调用可以看见日志。先查退出码、实际产物及必要时切换 TTY，避免将空输出误判为未执行。
- 程序可能在第一次运行时解开其自带 `Assets/Paramdex.zip`。其中 NR 定义可用，但并不保证覆盖所有当前参数。
- 本次 `Bullet.param` 出现 Row size paramdef mismatch，SpEffect 和 PermanentBuff 读取成功。仅研究不相关文件时可记录局部失败并继续；准备重打整个 regulation 时必须解决相关定义问题或保留未改文件，不能将错误忽略成全包有效。
- XML 行常只写非默认值；读取字段时要结合 `<fields>` 中的默认值。未出现的属性不等于值为零。
- 如果只需要事件 mod，不要因为已经解开 regulation 就重新打包和发布它。

生成参数包时，只替换实际编辑的 PARAM 成员，保留未编辑二进制；加密前后核对 binder 版本、成员 ID/名称/flags/顺序与压缩设置。重新解密输出，以 row/cell 比较原生行和允许变动集合，新行逐个检查引用与值；容器哈希或“成功重新打开”不足以证明范围正确。

处理中文 FMG/CSV/JSON 时显式指定 UTF-8，控制台编码单独处理，读取端兼容 BOM 与否。面向表格软件导出的 CSV 可使用 UTF-8 BOM，但以实际运行时读取器为准；构建后的文件不能在不同默认编码下再次写坏。用户配置的列名、ID、缺失/重复、非有限数值和百分比量纲需验证；估算表与运行时人工表分开，构建保留人工值。

## 选择性提取 BHD/BDT

原始 BHD 可能加密；本次游戏的 `_DSAS_CACHE/data*.bhd` 已是 `BHD5` 明文头，可与原 `data*.bdt` 配合读取。不能假定所有机器都有此缓存。缺少或失配时使用合适的提取工具/解密流程，避免把加密原始 BHD 当明文传给解析器。

缓存 `_CACHE_INFO.json` 有 BhdName、LastModifiedTime 和 First0x40；对照当前原始 BHD 的时间与前 64 字节检查新鲜度。仅凭缓存存在不够。本技能工具检测到已知失配会停止；无对应缓存元数据时明确报为未核验。

已验证的 Nightreign 路径哈希计算：将字典中的带前导 `/` 路径转小写、反斜线换 `/`，以无符号 64 位溢出计算 `hash = hash * 0x85 + character`。不要套用旧游戏 32 位路径哈希，也不要丢掉前导 `/`。

本机 Smithbox 提供 `Andre.SoulsFormats.dll`，命名空间仍是 `SoulsFormats`：

```csharp
BHD5.Read(bytes, BHD5.Game.EldenRing);
header.ReadFile(bdtStream);       // 利用 header 的 AESKey/ranges 解出文件
EMEVD.Read(path);                // 识别并解压 DCX
```

这是该库版本的 API 事实；其 BHD5 枚举没有 Nightreign，不代表游戏是 ER，也不保证未来 API 相同。反射检查本机方法签名，比照搬另一分支的样例可靠。

Oodle 相关要点：

- 本次 Smithbox 库从**进程当前工作目录**探测 `oo2core_9_win64.dll`，不是只从构建产物目录探测。最初只复制到 bin 仍报 NoOodleFound。
- 附带工具在子进程内将工作目录指向游戏目录，并指定 native library resolver，直接使用本机 Oodle，不复制/分发游戏 DLL。
- 根据库需要解析 Smithbox 的托管依赖；不同库版本可能使用 ZstdNet、DotNext 等。不要在尚有现成库时重写 AES、DCX、BND 和 EMEVD 格式。

## 附带工具

全局技能中的 scripts/GameData/GameData.csproj 和 Program.cs 是通用 C# 源码；不含游戏文件，不修改游戏输入。需要 .NET 10 SDK，以及兼容的本机 Smithbox 库。先在任务研究目录构建，避免把产物写回技能目录：

```powershell
dotnet build '<skill>/scripts/GameData/GameData.csproj' '-p:SmithboxDir=<Smithbox>' --artifacts-path '<research>/tool-build'
# 使用上一步实际输出的 GameData.dll：
dotnet '<GameData.dll>' extract '<Smithbox>' '<Game>' '<research>/extracted' '^/event/(common|common_func)\.emevd\.dcx$|^/action/script/c0000\.hks$'
dotnet '<GameData.dll>' inspect '<Smithbox>' '<Game>' '<research>/extracted/event/common.emevd.dcx' '8970060,8970061'
dotnet '<GameData.dll>' compare '<Smithbox>' '<Game>' '<original.emevd.dcx>' '<modified.emevd.dcx>' '1188'
```

- extract 按正则选择字典条目，对每个涉及 archive 单独读取；不是固定只查 data0。输出目标必须位于游戏目录之外，默认不覆盖已有提取文件。
- inspect 的 ID 匹配按每个可能的字节起始位置查 32 位小端值，仅用于候选发现，可能误报。它输出完整指令原始字节、四字节调试视图和 Parameters；语义须用 EMEDF 解码。
- compare 重新读取双方，校验改变的事件集合恰好等于指定集合，并检查元数据。这验证修改范围，不证明目标逻辑正确或游戏测试成功。

HKS 可使用：

```powershell
& '<DSLuaDecompiler>/DSLuaDecompiler.exe' '<c0000.hks>' -o '<research>/c0000.lua'
```

本次 c0000.hks 以 `1B 4C 75 61` 开头，是编译过的 Lua/HavokScript；不能将原文件作为 UTF-8 源码搜索。反编译输出可沿效果 ID 查实际动作分支。

需要局部函数时，先核对本机 CLI 支持的 `--included-function-ids` 等选项，减少整份脚本噪声。修改路线见 [动作脚本与原生运行时](hks-and-native.md)：反编译输出用于调查/独立回读，不能假定重新编译的整份脚本与原版等价。

图标调查需同时提取纹理 atlas 的 layout 与 TPF/DDS，按坐标裁切并与实际 PARAM/FMG 引用交叉索引。地图机制调查可能需要 MSB、对应地图 EMEVD、世界地图参数和 HKS；只提取涉及的分块/复合地图，不因一张图标相同就假定实体机制相同。各库对 `MSB_NR` 等类型的命名/属性先通过反射核对。
