# 行为分析、ME3 与 Rust DLL

## EMEVD 的常见误读

控制流调查要包含：

- 目标事件在哪里初始化，是否仅在抽中某事件/地图/模式时运行。
- 开头的 EndIf、Goto、条件组、等待及后续 Restart，是否依赖获取途径。
- 角色实体参数、同步设置与游戏阶段标志的含义。不要从 ER 直接移植玩家 ID；本次 NR 原事件使用 20000，在修改中保留该实体。
- 一次触发是立刻执行，还是提前准备一个效果，由 HKS/TAE/native 后续消耗。
- 同一个效果对应 icon/text、永久 buff 展示、等待状态、过渡状态或真实触发，不能将其混为一项。

当前指令语义查 [Nightreign EMEDF](https://soulsmods.github.io/emedf/nr-emedf.html)，版本更新时重新核对。已在案例中使用的 opcode：

| Bank[ID] | 名称 | 关键点 |
| --- | --- | --- |
| 3[0] | IfEventFlag | 条件组、ON/OFF/CHANGE、flag 类型和 flag ID |
| 3[42] | IfEventFlagAndRandomCondition | 条件组、uint flag、float 概率；案例为 0.3 |
| 4[5] | IfCharacterHasSpEffect | 末尾目标数是 float |
| 1004[0] | SkipIfCharacterHasSpEffect | 末尾目标数是 int；与 4[5] 不能混用编码 |
| 2004[8] | SetSpEffect | uint entity、int effect ID |
| 2004[21] | ClearSpEffect | 原生清除效果；实体身份与网络上下文须核对 |
| 2004[86] / [87] | AddEstusCharge / RefillEstus | 容量增加与补满不同，复用原生存储/限幅 |
| 2003[124] | DisplayPermanentBuffPopup | 名称不能当显示保证：本版本传record-only标志，不入提示队列；自定义能力需复用原生拾取的本地展示分支，见[双重惊喜](double-dip.md) |

Bank/ID 之外的参数以四字节数字输出可能把 bool/byte、padding 和 float 错读成整数。直接用 `object[]` 构造时每个 boxed 类型影响打包；优先复用已确认原指令字节，新增指令按签名显式给类型并确认对齐。

改写有 Parameters 的事件时，检查 SourceStartByte、TargetStartByte、ByteCount 和 InstructionIndex 等映射。增删指令、改变参数长度后旧映射可能指向错误位置。保留 Layer、RestBehavior、linked files 和 StringData；删除随机分支也要重新计算受影响的跳过行数。

主条件组的等待和 Restart 可能让代码每次远征/每次地图加载/每次效果消耗重跑。授予持续 buff 前检查是否已经持有，免得同一循环反复叠加或重复显示提示。

新事件的初始化放在实际可达的位置；构造器内部可能已有提前结束分支，末尾追加并不保证执行。地图本地事件沿本地图的命名范围/实例上下文建立，不把 common 的自定义范围直接搬到每张地图。未被初始化的同名原事件只作线索，不能随意启动它并带出其他行为。

个体奖励/资格事件核对是否禁用网络同步以及玩家实体所指对象。条件组正/负值、地图参数里的 byte/sbyte、相对跳过计数和 Restart 等待都需按实际签名解码。新增循环应有有效条件等待或适当间隔，避免无等待重启。

跨阶段逻辑可以用原生效果做请求/确认握手：API 调用、事件条件成立和效果查询更新可能跨帧。状态机记录请求已发，不在同帧查询失败后重复发送，更不能提前写永久完成记录。需要固定等若干帧时解释原生提交依据，不把任意延迟当功能成功证明。

## HKS 与原生层

HKS 是动作/状态控制的另一层。搜效果 ID、状态函数、动画事件、act/env 调用；反编译结果应作为当前版本的行为证据，不将未命名函数或 act/env 编号随意解释成稳定 API。

如果原功能已有完整动作、恢复和过渡链，复用激活效果通常比直接改 HP 更贴合游戏状态机。但要检查直接死亡、坠落、强制退场和特殊状态的例外；某一正常伤害触发成功不能证明所有死亡都能避免。

Rust DLL 适用场景包括纯数据/事件无法表达的行为、需要 UI/热配置、用户明确指定纯 DLL 等。可交付 `crate-type=["cdylib"]` 并由 ME3 natives 加载。

针对原生路线，需建立本版本 PE 地址映射、函数调用者与签名证据，核对 x64 ABI、调用线程、对象生命周期和 hook trampoline 的指令重定位。初始化工作应离开 loader lock；运行游戏动作/API 的工作按已验证的游戏线程入口安排。签名必须唯一且符合预期指令，失配拒绝该 hook。具体 HKS 补丁、原生调用及隔离验证方法见 [动作脚本与原生运行时](hks-and-native.md)。

已验证某个效果、伤害或生成入口，不自动证明已经定位资源加载 hook 或能修改内存中 EMEVD。相邻 Rust 项目的地址、签名或 hook engine 成功记录只能作线索，新 hook 仍需自己的调用点和运行证据。

## ME3 集成

文件覆盖配置示例：

```toml
profileVersion = "v1"
start_online = false

[[supports]]
game = "nightreign"

[[packages]]
id = "my-mod"
source = "mod"
```

`mod` 中保持相对游戏资源路径，如 `event/common.emevd.dcx`。DLL 则根据当前 ME3 schema 添加 `[[natives]] path="mod/natives/MyMod.dll"`；不要凭文章截图混用 singular/plural 版本配置。

本机 CLI 的只读验证命令：

```powershell
& '<ME3>/bin/me3.exe' profile show --file '<dist>/MyMod.me3'
```

启动命令（执行会实际启动游戏，按任务需求使用）：

```powershell
& '<ME3>/bin/me3.exe' launch -g nightreign -p '<dist>/MyMod.me3' -e '<Game>/nightreign.exe'
```

本案例使用独立离线配置。继承用户已有授权和配置，不自行添加其他 Mod、切换保存路径或修改默认配置。多个文件包相同路径是整文件冲突；同一 regulation 的改动要按 param/row/cell 合并，同一 EMEVD 按 event/instruction/parameter 合并。两个修改涉及同一事件时还需要重新分析控制流。

DLL hook 冲突也要检查：两个 Mod 修改同一入口、覆盖的原始字节或共享资源时，不凭 natives 排序假定兼容。用户已安装的辅助 Mod、种子和快捷键配置在本任务之外保留；临时测试配置可独立生成。更新已加载 DLL 通常需要退出游戏后替换，再按已有授权启动，避免把磁盘新文件当正在运行的新构建。

Windows 用 `Start-Process` 后台启动 ME3 时加 `-WindowStyle Hidden`；含空格的 profile/EXE 路径需要传递给子进程的实际引号，不只是 PowerShell 中字符串边界。校验解析后的 profile、实际进程模块路径/哈希、DLL ACTIVE 状态和资源覆盖日志，不以 launcher 返回或某个旧日志代表当前构建加载。

## 证据与测试

事件文件变化范围至少比较：事件集合/顺序、ID、RestBehavior、Name、Bank/ID/ArgData/Layer、Parameters、字符串与 linked offsets、格式/压缩类型。记录源和输出哈希以确定对应版本，不用哈希代替语义比较。

对预期修改的事件，还需核对目标控制流。例如 buff 授予与重新准备有没有可达性、原随机/预约限制是否仍在、是否出现无等待的重启循环。附带 compare 工具只检查结构差异范围，不做这些语义推断。

游戏测试由任务目标决定，报告已知覆盖。用户“成功了”的反馈可以记为用户确认功能成功；不要把它改写成自己观察了连续触发、所有角色、Seamless 或其他未报告场景。

## 分层验证与排错

| 证据 | 能支持的结论 | 不能替代 |
| --- | --- | --- |
| 当前资产与原生代码的只读分析 | 目标字段、分支、消费者与候选 ABI | 游戏里已实际触发 |
| 纯状态机/真实包装与 trampoline 测试 | 去重、计时、ABI 转发和寄存器/栈保留 | 敌方反应或网络权威 |
| 隔离执行原生机器码 | 明确夹具内的谓词、倍率、槽位和返回路径 | 实际物品创建、动作和联机 |
| 输出回读与范围比较 | 引用、序列化、原始内容保留、限定变更 | 时序正确、事件可达 |
| 进程/模块/覆盖加载检查 | 指定构建已加载、入口安装 | Buff 数值或玩法已成功 |
| 用户或实际游戏验收 | 本次明确报告的角色/模式/动作/结果 | 未报告的场景覆盖 |

只增加能解决当前疑问的验证，不重复扩大全量测试。常见排错顺序：未知奖励先检查 Goods 封装/类别/文本；缺图先查 atlas；有效果而数值不变查适用过滤与倍率缓存；有资格记录但无奖励查是否真正调用创建；本机正常、队友异常查缓存重放、网络副本及显示状态。根据证据区分“没触发”“触发后未提交”和“已提交但显示错误”，不要先猜游戏清掉了结果。

保留原版、确认成功的包、失败反馈和本次产物之间的对应关系，记录源/输出/DLL 哈希和明确的待测项。旧生成器、历史诊断报告、过期配置快照不能自动充当新版本验收；后续修复后的事实应覆盖旧推断，同时保留失败材料作为调查依据。
