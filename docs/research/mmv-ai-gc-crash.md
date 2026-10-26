# MMV 运行时崩溃：AI Lua 垃圾回收

2026-10-26，本地约 21:38，进程 32244。用户反馈当时在移动。

## 已确认的直接故障

- WER 1000/1001 与 minidump 一致：nightreign.exe 1.3.3.0，RVA `0x214e298`，`0xc0000005`。
- 故障指令是 `mov dword ptr [0], 0xdeadba`，为引擎 panic 主动触发的访问异常，不能解释成普通空指针业务错误。
- 进入 panic 的调用点 `0x219999d`，断言信息为 `given memory block seems to be improper or freed already.`；文件名常量 `DLRegularHeap.cpp`，行号 710。
- 原生释放路径把用户指针减去 0x10 读取块头。占用位为零时触发该断言；本次块头 next/tag 值低位为 2。待释放用户指针 `0x7ff3975ec8a0`。
- 调用者 `0x22cbab0` 先读对象 +0x18 的数组与 +0x50 的数量，按 4 字节元素释放；其后依次释放 +0x20 指针数组、+0x10 常量数组等。布局和顺序对应 Lua 函数 prototype 的代码及相关数组。参考 [Lua 5.1 lfunc.c](https://www.lua.org/source/5.1/lfunc.c.html) 与 [lobject.h](https://www.lua.org/source/5.1/lobject.h.html)，外部代码仅用于布局交叉核对。
- 更直接的游戏内类型证据：执行函数 `0x3292d0` 的虚表 RVA `0x2c06c90`，RTTI 名为 `.?AVCSAiLuaCollectGabageFragmaent@CS@@`。这是 **AI Lua 垃圾回收任务**，不能仅凭“Lua”归因到玩家 c0000.hks。

已通过返回位置及调用指令逐层交叉核对：panic → RegularHeap free → 内存分配器释放 → Lua prototype 释放 → GC sweep → AI Lua GC 工作任务。受保护原生代码的静态 unwind 信息不完整，初始工具产出的 `frames.json` 中 leaf 回退进入数据地址，**不可当作有效完整调用栈**；结论依据上述实际调用指令、返回地址、结构及 RTTI。

## 能排除与不能排除的范围

本轮新增刷怪笼资源于崩溃之后才部署，崩溃进程使用的是之前的 DLL；不能把这次故障归因于本轮新增的 59 组。转储中 NRHex 和 MMVFixes 均加载。

该直接故障与先前 c6220 骨骼数组相关崩溃点不同，但不能因此证明两者完全没有上游关联。也没有足够证据认定是 MMV 原包、原生引擎、强化死灵术或其他 DLL 的某一方造成。

日志最后记有大法师法术切换和海克斯重投；用户确认当时在移动。AI 回收异步执行，时间靠近不等于因果。最后一次记录的灵魂退场约在 21:36:19，约两分钟前；这同样不能证明或排除先前对象生命周期问题。

此 minidump 有线程栈和部分内存，但待释放块、相关 prototype、Lua state 的堆页面均未收录。无法恢复所属脚本名、首次释放者或最早覆盖位置。只有一个线程栈匹配该 AI GC 链；这不排除更早的竞态。

## 后续定位边界

若继续修复，需要保留 AI 脚本加载／prototype 身份、代码指针的分配及首次释放记录，或取得包含相应堆页的完整转储，才能区分重复释放、块头被覆盖、脚本生命周期／并发问题。不能通过跳过 panic、跳过 free 或禁用整个 GC 作为已验证修复。

本轮只做记录分析，未增加诊断钩子、未改 Lua GC、未修改转储注册表，也未启动游戏。证据保存在 `work/compat/mmv/crash-20261026-213850/`：原 dmp、NRHex 日志、WER 记录、`findings.json` 和线程候选；后续分析可以直接复用。
