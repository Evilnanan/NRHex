# 资料索引

这里按问题组织结论，不再按每次试作版本保存一份重复说明。[海克斯文案](hexes.md) 列出全部已实现海克斯，并作为游戏内中文名称和描述的唯一编辑来源；使用方法和机制说明见 [测试包说明](../dist/README.md)。实际数值由源码及用户配置决定；通用资料中的原生参考 ID 不等于本 Mod 的最终设计。

| 要查的问题 | 资料 |
| --- | --- |
| 已实现哪些海克斯、统一修改中文文案 | [海克斯文案](hexes.md) |
| 修改英文名称与翻译 | [英文文案](hexes.en.md) |
| 如何使用、测试快捷键与机制细节 | [测试包说明](../dist/README.md) |
| 如何构建、工具在哪里、哪些输入必须保留 | [构建](build.md) |
| 各海克斯由哪些文件实现 | [实现入口](implementation.md) |
| 历次版本变更与当时的验证记录 | [历史变更](../CHANGELOG.md) |
| PARAM、FMG、解包、Oodle、EMEDF | [数据与工具](research/data-and-tools.md) |
| 增伤、承伤、分类、吸血、图标、护盾联机视觉 | [效果与战斗结算](research/effects-and-combat.md) |
| 侵蚀异常选择、免疫和多段攻击补正 | [侵蚀与原生异常](research/erosion.md) |
| 连续受击时旧回收额度如何衰减 | [再次受击与回收槽](research/regain-rehit.md) |
| 队友特殊效果栏重复、复活后缺失及原生同步 | [效果栏同步](research/effect-sync.md) |
| 豪饮攻击力图标、原生HUD合并和层数 | [豪饮HUD](research/drinking-hud.md) |
| 刷怪笼连续开启卡住、队友血条与深夜血量调查 | [刷怪笼生命周期](research/cage-lifecycle.md) |
| 刷怪笼红化概率、原生红化奖励与塔楼武器球 | [红化与塔楼奖励](research/cage-mutation.md) |
| 奖励全排列容量、运行时模板和压缩历史 | [运行时奖励与扩容](research/reward-scaling.md) |
| 潜在能力链、私人奖励、升级排队、圣杯瓶 | [奖励与生命周期](research/rewards-and-lifecycle.md) |
| HKS 字节码、输入、跳跃高度、原生 ABI | [动作脚本与运行时](research/hks-and-native.md) |
| 战技随机、子弹行为、武器丢拾身份 | [战技与武器](research/skills-and-weapons.md) |
| Libra慢版法阵、环境来源与深度伤害 | [恶魔的注视](research/devils-gaze.md) |
| 当前 EXE 的地址、模块与关键字段 | [版本锚点](research/native-anchors.md) |
| 事件控制流、合并和验证边界 | [行为与验证](research/behavior-and-validation.md) |
| 大／小图标及原版用途 | [图标资料](icons/README.md) |
| 什么删掉了、今后资料如何归档 | [维护约定](maintenance.md) |

通用研究五篇由本仓库已有调查及 Nightreign 开发技能参考整合而来。技能仍在 `C:/Users/OwO/.codex/skills/nightreign-mod-dev/`，本次只整理项目，不修改全局技能。参数夹具和原始输入各有哈希清单；关键机器码检查仍在 `tools/verify_*.py` 中，可以重新执行。

- [王室幽魂](research/wraith.md)：咒魂工具命中成长、局内保存、免蓝及动作结束后替换。
