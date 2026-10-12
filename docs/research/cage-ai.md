# 刷怪笼索敌和追击范围：0.17.24

用户反馈 0.17.23 暂未再遇到开启无 BOSS 的问题；这只是暂未复现的反馈，仍需持续观察。同时报告部分 BOSS，尤其塔楼 BOSS，索敌／追击过远。

本机当前原版 regulation（10350000）和 NpcThinkParam 定义确认：视觉、搜索视觉、听觉和嗅觉距离以米计；maxBackhomeDist 为无条件返程距离，backhomeDist 为战斗中返程距离，backhomeBattleDist 为停止返程并继续战斗的目标距离。这些返程字段传给 COMMON_SetBattleActLogic。

当前复制角色沿用了源地图的主 AI，而塔楼地图还带有备用野外 AI 引用。例如熔铁恶魔、舞娘、小葛的主 AI 嗅觉为 999 米，返程阈值和目标遗忘时间为 9999；恶兆王嗅觉 99 米、返程阈值 9999；石像鬼、大土龙、牛牛嗅觉 200 米、强制返程 200 米。对应备用 AI 的嗅觉为 0，返程 50／80 米，视觉记忆 8 秒、目标记忆 2 秒。原生监牢多数为 30～70 米开始返程、40～90 米强制返程，也存在古龙等较大的视觉范围。

为 37 只刷怪笼 BOSS 增加专用 Think 行 95300000～95300036，四张监牢模板的 148 个复制角色改用这些行。原版 Think 行及原版地图角色不变。每行从原主 AI 复制，七个塔楼原型的范围参考原地图原始字节中已核对的备用 AI；只改下表字段，logicId、battleGoalID、动作、角度和其他字段保留主 AI 原值。每个字段取原值、原生备用值、上限中的最小值，不扩大原有距离。

| 字段 | 上限 |
| --- | --- |
| eye_dist / battleEye_updateDist | 30 米 |
| searchEye_dist | 20 米 |
| ear_dist | 30 米；当前原值 0.1 不变 |
| nose_dist | 0；清除超远全向感知 |
| BattleStartDist | 30 米；原值 0 保留 |
| backhomeDist | 40 米 |
| maxBackhomeDist | 60 米 |
| backhomeBattleDist | 20 米 |
| SightTargetForgetTime | 8 秒 |
| MemoryTargetForgetTime | 5 秒；塔楼备用值 2 秒则使用 2 秒 |

以原生返程 AI 控制追击，不增加瞬移、强制回血或逃离判负事件。已开始的动作及个别自定义 logic 脚本仍可能影响实际返程时机，参数距离不等于瞬时物理边界。

`HexBuild --cage-ai-probe` 导出原值与同原型变体；`--cage-ai-only` 在 `.work/cage-ai/staged` 独立生成参数包与四张地图。重新解密／回读检查原有 AI 行、所有其他参数表与 binder 成员保留，新增 37 行正确；地图撤销 37 个 Think 引用修改后，序列化内容与修改前一致。整包生成和 cage-only 后续生成也使用相同独立 AI。

实际对照在 `.work/cage-ai/native.json`、`.work/cage-ai/patch.json`；交付范围记录在 dist/cage-ai-validation.json。构建和文件验证不代替游戏追击、返程或联机验收。

已安装 0.17.24 并按项目 ME3 配置重启。当前进程 PID 22472，确认 DLL ACTIVE=1、四个原生 Mod 和十五处 hook 正常加载；参数包覆盖路径已出现在当前启动记录中。用户配置、原有平衡和事件哈希保留。尚未收到本版索敌／返程的实际游戏反馈。
