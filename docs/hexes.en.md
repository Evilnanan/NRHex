# 海克斯英文文案

这里是游戏内海克斯英文名称和描述的唯一编辑来源，已保留现有翻译，方便逐条调整。中文对照见 [中文文案](hexes.md)。

直接编辑各项英文标题和下方描述，保留标题前的 `hex` 编号注释，不要更改编号。名称和描述使用纯文本，可以使用英文引号、撇号和换行，不需要代码转义。两种语言通过编号关联，排列顺序不必一致；修改中文不会自动翻译或覆盖英文。

修改后在仓库根目录运行 `./build-texts.ps1`，更新 `dist` 中的中英文文本；也可照常运行完整构建 `./build.ps1`。重启游戏后生效。只检查已构建文本是否一致可运行 `./build-texts.ps1 -Check`。描述中的数值不会修改实际机制。

附加效果的英文说明在 [状态文案](../data/text/status-effects.json) 的 `enInfo` 中编辑；`enName` 是单独的英文名称，缺省时继承这里的能力名称，`enSuffix` 是名称后缀。不要更改 `textId` 和 `ownerId`。这些状态文本也会一起构建。

名称约定：星界躯体、玻璃大炮、终极刷新、亮出你的剑、大力、逃跑计划、吵闹鬼沿用 [ARAM: Mayhem 的英文原名](https://arammayhem.com/augments/)；“别停止咏唱”沿用“别停止引导”的原名 [Don't Change the Channel](https://www.leagueoflegends.com/en-us/news/game-updates/league-of-legends-patch-26-12-notes/)。能力描述仍对应本 Mod 的机制。“人是猫”按指定致敬名称使用 `Humans Are Cats`。

描述约定：参照黑夜君临原生词条，优先使用 `Boosts`、`Restores`、`Reduces`、`Grants` 等简洁陈述句，减少直接称呼玩家的 `you/your`。术语统一为 `attack power`（攻击力）、`damage negation`（减伤率）、`Ultimate Art`（绝招）、`Art gauge`（绝招量表）、`sorceries and incantations`（魔法、祷告）、`critical hits`（处决）、`stance`（架势）、`near-death`（濒死）；圣杯瓶用 `Flask of Crimson Tears`，石剑钥匙用 `Stonesword Key`，资源用 `HP`、`FP`、`stamina`，潜在能力用 `Dormant Power`，海克斯强化符文用 `Mayhem Augments`，赐福用 `Site of Grace`。武器战技用 `skill`，不要混同角色技艺 `Character Skill`。吸血写为命中时回复 HP，避免使用其他游戏的 `lifesteal`；冲击力保留 `impact`，不要改成表示削韧的 stance damage。

以上用语已对照本地原版 `AttachEffectInfo.fmg`、`GoodsName.fmg` 和 `CL_MenuText.fmg`。只缩减过长的英文描述，尽量不超过140个字符；原文已在此范围内的保留，超长时优先精简措辞并保留数值、触发条件和核心机制，必要时才省略次要细节。140字符是编辑目标，不是已确认的引擎上限，最终显示仍需游戏内检查。不套用被致敬游戏的机制；时长使用 `seconds`，冷却可用 `Cooldown: 60 seconds.`；锻体奖励描述为 `a Dormant Power with a passive effect`。

<!-- hex:95000100 -->
## Red Envelope

Grants 88,888 runes and awards each party member a Dormant Power.

<!-- hex:95000200 -->
## Celestial Body

Reduces attack power by 25%. Raises all attributes by 10.

<!-- hex:95000300 -->
## Glass Cannon

Takes 10x damage. Attack power +25%. Ignores enemy defense, resistances and damage negation.

<!-- hex:95000400 -->
## Ultimate Revolution

Fully restores the Art gauge upon activating an Ultimate Art. Cooldown: 60 seconds.

<!-- hex:95000500 -->
## Heavy Drinking

Doubles the number of Flask of Crimson Tears uses, but halves the HP restored. Using the flask boosts attack power by 5% for 15 seconds.

<!-- hex:95000600 -->
## Regeneration

Continuously consumes FP to restore HP while HP is not at maximum. Using a Flask of Crimson Tears at maximum HP restores 50% of maximum FP.

<!-- hex:95000700 -->
## Crystal Shell

Absorbs damage with a crystal shell that shatters and regenerates after 40 seconds.

<!-- hex:95000800 -->
## Humans Are Cats

Allows unlimited climbing. Boosts jump attacks based on fall height. Silences footsteps. Conceals the body for 1 second upon taking damage.

<!-- hex:95000900 -->
## Draw Your Sword

Makes spells melee, converting range into attack power, HP on hit, damage negation and poise. Melee attack power +15%.

<!-- hex:95001100 -->
## Blunt Force

Boosts the impact of melee attacks.

<!-- hex:95001200 -->
## Escape Plan

At low HP, act faster, take 60% less damage and deal 30% less damage for 10s. Cooldown: 70s.

<!-- hex:95001300 -->
## Edgelord's Footwork

Better backstep evasion. Backstep hits/frontal crits restore 25% Art gauge; rolls/damage drain 30%. Charge boosts backstep attacks.

<!-- hex:95001400 -->
## Master of Arms

Reduces FP consumption of weapon skills by 100%. Reforges the armament's skill after each use.

<!-- hex:95001500 -->
## Poltergeist

Marking the player's own location creates lava that erupts after a delay. Repeating this action before the eruption increases its damage.

<!-- hex:95001600 -->
## Spawner

Open evergaols repeatedly without keys to face varied enemies. No Site of Grace appears after clearing.

<!-- hex:95001700 -->
## Balloon

Max HP +50%, damage negation -30%, piercing resistance -100%. Explodes on death or near-death, dealing max-HP damage to nearby foes.

<!-- hex:95001800 -->
## Enhanced [Necromancy]

Stores the soul of a recently defeated powerful enemy. Using your Ultimate Art summons it as a spirit.

<!-- hex:95002000 -->
## Don't Change the Channel

Restores 30 stamina upon fully charging a sorcery or incantation.

<!-- hex:95002100 -->
## Archmage

Reduces FP consumption of a catalyst's secondary spell by 100%. Casting the spell replaces it with a random new spell.

<!-- hex:95002200 -->
## Demon's Gaze

Occasionally creates golden magic circles nearby.

<!-- hex:95002300 -->
## Slingshot

Extends jumping distance after starting a sprint. Stacks up to three times. Each stack boosts jump attack power by 50%.

<!-- hex:95002400 -->
## Clash

Two-handed attacks can guard enemy blows. After a successful guard, guard counters deal double damage and restore HP and stamina once on the first hit.

<!-- hex:95002500 -->
## Night's Blessing

Night's Tide: attack +20%. Below 20% HP, heals for 15 seconds, then drains HP until leaving. Touch a Site of Grace to reset.

<!-- hex:95002600 -->
## Ominous Pact

Causes self-damage equal to 5% of maximum HP upon landing an attack. Cooldown: 10 seconds.

<!-- hex:95002700 -->
## Heartstealer

Releases a mist that charms enemies upon landing a charged attack. Cooldown: 30 seconds.

<!-- hex:95002900 -->
## Essence Reaver

Defeating an enemy raises maximum FP by 1% for the rest of the expedition.

<!-- hex:95003000 -->
## Death's Dance

Healing flasks restore HP gradually. Recoverable damage is taken over 2 seconds. Attacks restore a portion of recently lost HP.

<!-- hex:95003100 -->
## Erosion

Reduces attack power by 30%. Attacks build up the status ailment to which the target is most vulnerable.

<!-- hex:95003200 -->
## Potential Fission

Splits each of the next three Dormant Powers obtained into two of lower quality.

<!-- hex:95003400 -->
## Ambulance

Sprinting, dashing, and running toward a near-death ally consume no stamina. Restores stamina while continuously rescuing allies.

<!-- hex:95003500 -->
## Echo Casting

Increases sorcery and incantation FP consumption by 100% and stamina consumption by 50%. Creates an additional projectile after casting.

<!-- hex:95003600 -->
## Set Meal

Increases the price of armaments sold by merchants. Purchasing an armament also grants a Dormant Power with a passive effect.

<!-- hex:95003700 -->
## Burst of Momentum

After a hit: attack power +30% for 5 seconds, then attack power and damage negation -50% for 5 seconds.

<!-- hex:95003800 -->
## Ultimate Awakening

Fully restores FP upon activating an Ultimate Art and eliminates FP consumption for the next 7 seconds.

<!-- hex:95003900 -->
## Ultimate Unstoppable

Clears status buildup upon activating an Ultimate Art, then greatly increases poise and grants immunity to status buildup for 15 seconds.

<!-- hex:95004000 -->
## Ultimate Challenge

Raises maximum HP by 50% and increases the likelihood of being targeted by enemies for 15 seconds after activating an Ultimate Art.

<!-- hex:95004200 -->
## Slide Tackle

Character Skill cooldown +50%. Launches small enemies; follow-up attacks against kicked enemies deal heavy damage.

<!-- hex:95004300 -->
## Wraithcaller

Wraith tools cost no FP. Enough hits upgrade them to release more wraiths. Grants a Wraith Calling Bell.

<!-- hex:95004400 -->
## Back to Basics

Seals Ultimate Arts. Greatly accelerates Character Skill cooldown recovery and boosts its damage.

<!-- hex:95004500 -->
## Experienced Sniper

Attack power -30%. Damage increases with distance to the target. Greatly extends range.

<!-- hex:95004600 -->
## Metallization

Use Ironjar Aromatic without consuming it. Receive an Ironjar Aromatic.

<!-- hex:95004700 -->
## Mystery Box

Grants two random Mayhem Augments.

<!-- hex:95004800 -->
## Haymaker

The more damage you take during an empowered Character Skill, the more damage your counterattack deals.

<!-- hex:95004900 -->
## Bartender

Switching weapons cycles their enchantment through Magic, Fire, Lightning, and Holy.
