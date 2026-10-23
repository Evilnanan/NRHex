# 海克斯英文文案

这里是游戏内海克斯英文名称和描述的唯一编辑来源，已保留现有翻译，方便逐条调整。中文对照见 [中文文案](hexes.md)。

直接编辑各项英文标题和下方描述，保留标题前的 `hex` 编号注释，不要更改编号。名称和描述使用纯文本，可以使用英文引号、撇号和换行，不需要代码转义。两种语言通过编号关联，排列顺序不必一致；修改中文不会自动翻译或覆盖英文。

修改后在仓库根目录运行 `./build-texts.ps1`，更新 `dist` 中的中英文文本；也可照常运行完整构建 `./build.ps1`。重启游戏后生效。只检查已构建文本是否一致可运行 `./build-texts.ps1 -Check`。描述中的数值不会修改实际机制。

附加效果的英文说明在 [状态文案](../data/text/status-effects.json) 的 `enInfo` 中编辑；`enName` 是单独的英文名称，缺省时继承这里的能力名称，`enSuffix` 是名称后缀。不要更改 `textId` 和 `ownerId`。这些状态文本也会一起构建。

名称约定：星界躯体、玻璃大炮、终极刷新、亮出你的剑、大力、逃跑计划、吵闹鬼沿用 [ARAM: Mayhem 的英文原名](https://arammayhem.com/augments/)；“别停止咏唱”沿用“别停止引导”的原名 [Don't Change the Channel](https://www.leagueoflegends.com/en-us/news/game-updates/league-of-legends-patch-26-12-notes/)。能力描述仍对应本 Mod 的机制。“人是猫”按指定致敬名称使用 `Humans Are Cats`。

描述约定：参照黑夜君临原生词条，优先使用 `Boosts`、`Restores`、`Reduces`、`Grants` 等简洁陈述句，减少直接称呼玩家的 `you/your`。术语统一为 `attack power`（攻击力）、`damage negation`（减伤率）、`Ultimate Art`（绝招）、`Art gauge`（绝招量表）、`sorceries and incantations`（魔法、祷告）、`critical hits`（处决）、`stance`（架势）、`near-death`（濒死）；圣杯瓶用 `Flask of Crimson Tears`，石剑钥匙用 `Stonesword Key`，资源用 `HP`、`FP`、`stamina`，潜在能力用 `Dormant Power`，赐福用 `Site of Grace`。武器战技用 `skill`，不要混同角色技艺 `Character Skill`。吸血写为命中时回复 HP，避免使用其他游戏的 `lifesteal`；冲击力保留 `impact`，不要改成表示削韧的 stance damage。

以上用语已对照本地原版 `AttachEffectInfo.fmg`、`GoodsName.fmg` 和 `CL_MenuText.fmg`。适配原生措辞时仍须写清触发条件、作用对象、持续时间和现有数值，不套用被致敬游戏的机制；时长统一写 `seconds`，冷却统一用 `Cooldown: 60 seconds.` 形式；锻体奖励描述为 `a Dormant Power with a passive effect`。

<!-- hex:95000100 -->
## Red Envelope

Grants 88,888 runes and awards each party member a Dormant Power.

<!-- hex:95000200 -->
## Celestial Body

Reduces attack power by 25%. Raises all attributes by 10.

<!-- hex:95000300 -->
## Glass Cannon

Increases damage taken tenfold. Boosts attack power by 20%. Damage dealt ignores enemy defense, resistances, and damage negation, while retaining bonuses from negative resistances.

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

Allows unlimited climbing. Boosts jump attack damage based on fall height. Silences footsteps. Conceals the body for 1 second upon taking damage.

<!-- hex:95000900 -->
## Draw Your Sword

Treats sorceries and incantations as melee attacks. Converts their original range into attack power, HP restoration on hit, damage negation, and poise. Boosts melee attack power by 15%.

<!-- hex:95001100 -->
## Blunt Force

Boosts the impact of melee attacks.

<!-- hex:95001200 -->
## Escape Plan

Increases action speed and boosts damage negation by 60% for 10 seconds when HP is low. Cooldown: 60 seconds.

<!-- hex:95001300 -->
## Edgelord's Footwork

Improves backstep evasion. Landing a backstep attack or frontal critical hit restores 25% of the Art gauge. Rolling or taking damage depletes 30% of the Art gauge. Boosts backstep attack damage based on the Art gauge's current charge.

<!-- hex:95001400 -->
## Master of Arms

Reduces FP consumption of weapon skills by 100%. Reforges the armament's skill after each use.

<!-- hex:95001500 -->
## Poltergeist

Marking the player's own location creates lava that erupts after a brief delay. Repeating this action before the eruption increases its damage.

<!-- hex:95001600 -->
## Spawner

Allows evergaols to be opened without a Stonesword Key and increases enemy variety. No Site of Grace appears after clearing an evergaol. Each evergaol can be opened up to five times.

<!-- hex:95001700 -->
## Balloon

Raises maximum HP by 50%. Reduces damage negation by 30%. Reduces resistance to piercing attacks by 100%. Triggers an explosion upon death or entering near-death, dealing damage equal to maximum HP to nearby enemies.

<!-- hex:95001800 -->
## Enhanced [Necromancy]

Stores the soul of a recently defeated powerful enemy, prioritizing souls with higher maximum HP. Activating [Immortal March] summons its spirit to fight alongside the summoner for a short time.

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

Allows attacks made while two-handing an armament to guard against enemy attacks. Guard counters after successfully guarding consume no stamina and deal 100% more damage.

<!-- hex:95002500 -->
## Night's Blessing

Boosts attack power by 20% in the Night's Tide. When HP is below 20% in the Night's Tide, gradually restores HP for 15 seconds. A backlash then gradually depletes HP until leaving the Night's Tide. Touching a Site of Grace resets this effect.

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

The Flask of Crimson Tears restores HP gradually. Recoverable damage is taken gradually over 2 seconds. Attacks restore a portion of recently lost HP.

<!-- hex:95003100 -->
## Erosion

Reduces attack power by 20%. Attacks build up the status ailment to which the target is most vulnerable.

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

For 10 seconds after landing an attack, boosts attack power by 30% for the first 5 seconds, then reduces attack power and damage negation by 50% for the next 5 seconds.

<!-- hex:95003800 -->
## Ultimate Awakening

Eliminates FP consumption for 15 seconds after activating an Ultimate Art.

<!-- hex:95003900 -->
## Ultimate Unstoppable

Greatly increases poise and grants immunity to status buildup for 15 seconds after activating an Ultimate Art.

<!-- hex:95004000 -->
## Ultimate Challenge

Raises maximum HP by 50% and increases the likelihood of being targeted by enemies for 15 seconds after activating an Ultimate Art.

<!-- hex:95004200 -->
## Enhanced [Marking]

Allows sliding kicks to launch small enemies. Follow-up attacks deal heavy damage to enemies hit by the slide.

<!-- hex:95004300 -->
## Wraithcaller

Eliminates FP consumption for wraith-calling items. Landing enough hits upgrades the item to one that releases more wraiths. Grants a Wraith Calling Bell.

<!-- hex:95004400 -->
## Back to Basics

Seals Ultimate Arts. Greatly accelerates Character Skill cooldown recovery and boosts its damage.

<!-- hex:95004500 -->
## Experienced Sniper

Reduces attack power by 30%. Boosts damage as the distance to the target increases. Greatly reduces arrow damage falloff with distance.
