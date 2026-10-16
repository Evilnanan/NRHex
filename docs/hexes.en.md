# 海克斯英文文案

这里是游戏内海克斯英文名称和描述的唯一编辑来源，已保留现有翻译，方便逐条调整。中文对照见 [中文文案](hexes.md)。

直接编辑各项英文标题和下方描述，保留标题前的 `hex` 编号注释，不要更改编号。名称和描述使用纯文本，可以使用英文引号、撇号和换行，不需要代码转义。两种语言通过编号关联，排列顺序不必一致；修改中文不会自动翻译或覆盖英文。

修改后在仓库根目录运行 `./build-texts.ps1`，更新 `dist` 中的中英文文本；也可照常运行完整构建 `./build.ps1`。重启游戏后生效。只检查已构建文本是否一致可运行 `./build-texts.ps1 -Check`。描述中的数值不会修改实际机制。

附加效果的英文说明在 [状态文案](../data/text/status-effects.json) 的 `enInfo` 中编辑；`enName` 是单独的英文名称，缺省时继承这里的能力名称，`enSuffix` 是名称后缀。不要更改 `textId` 和 `ownerId`。这些状态文本也会一起构建。

名称约定：星界躯体、玻璃大炮、终极刷新、亮出你的剑、大力、逃跑计划、吵闹鬼沿用 [ARAM: Mayhem 的英文原名](https://arammayhem.com/augments/)；“别停止咏唱”沿用“别停止引导”的原名 [Don't Change the Channel](https://www.leagueoflegends.com/en-us/news/game-updates/league-of-legends-patch-26-12-notes/)。能力描述仍对应本 Mod 的机制。“人是猫”按指定致敬名称使用 `Humans Are Cats`。

描述约定：参照黑夜君临原生词条，优先使用 `Boosts`、`Restores`、`Reduces`、`Grants` 等简洁陈述句，减少直接称呼玩家的 `you/your`。术语统一为 `attack power`（攻击力）、`damage negation`（减伤率）、`Ultimate Art`（绝招）、`Art gauge`（绝招量表）、`sorceries and incantations`（魔法、祷告）、`critical hits`（处决）、`stance`（架势）、`near-death`（濒死）；圣杯瓶用 `Flask of Crimson Tears`，石剑钥匙用 `Stonesword Key`，资源用 `HP`、`FP`、`stamina`。武器战技用 `skill`，不要混同角色技艺 `Character Skill`。吸血写为命中时回复 HP，避免使用其他游戏的 `lifesteal`；冲击力保留 `impact`，不要改成表示削韧的 stance damage。

以上用语已对照本地原版 `AttachEffectInfo.fmg`、`GoodsName.fmg` 和 `CL_MenuText.fmg`。适配原生措辞时仍须写清触发条件、作用对象、持续时间和现有数值，不套用被致敬游戏的机制；时长统一写 `seconds`。

<!-- hex:95000100 -->
## Red Envelope

Grants 88,888 runes and a latent power reward for each party member.

<!-- hex:95000200 -->
## Celestial Body

Reduces attack power by 25%. Raises all attributes by 10.

<!-- hex:95000300 -->
## Glass Cannon

Increases damage taken tenfold. Boosts attack power by 20%. Damage dealt ignores enemy resistances, defenses, and damage negation.

<!-- hex:95000400 -->
## Ultimate Revolution

Fully restores the Art gauge upon using an Ultimate Art. (45-second cooldown.)

<!-- hex:95000500 -->
## Heavy Drinking

Doubles the number of Flask of Crimson Tears uses, but halves HP restoration. Using a flask boosts attack power by 5% for 15 seconds.

<!-- hex:95000600 -->
## Regeneration

Continuously consumes FP to restore HP while HP is not at maximum. Using a Flask of Crimson Tears at maximum HP restores 50% of maximum FP.

<!-- hex:95000700 -->
## Crystal Shell

A crystal shell absorbs an incoming attack and shatters. The shell regenerates after 40 seconds.

<!-- hex:95000800 -->
## Humans Are Cats

Allows unlimited climbing. Boosts jump attack damage based on fall height. Silences footsteps. Conceals the body for 1 second upon taking damage.

<!-- hex:95000900 -->
## Draw Your Sword

Sorceries and incantations also count as melee attacks, but can only hit nearby enemies. Spell range determines increased attack power and damage negation while casting. Attacks recover some recently lost HP for a short time after taking damage. Spell recovery scales with damage dealt and spell range. Makes spell casting harder to interrupt. Boosts melee attack power by 15%.

<!-- hex:95001100 -->
## Blunt Force

Boosts the impact of successful attacks.

<!-- hex:95001200 -->
## Escape Plan

Increases action speed and boosts damage negation by 60% for 10 seconds when HP is low. (60-second cooldown.)

<!-- hex:95001300 -->
## Edgelord's Footwork

Improves backstep evasion. Successful backstep attacks against enemies whose stance is not broken, and critical hits from the front, each fill 25% of the Art gauge. Dodge rolls and damage taken each deplete 30% of the Art gauge. Boosts backstep attack damage by up to 200%, based on the current Art gauge.

<!-- hex:95001400 -->
## Master of Arms

Reduces skill FP consumption by 100%. Using a skill reforges the skill of that armament.

<!-- hex:95001500 -->
## Poltergeist

Marking one's own position generates roiling lava. Further marks of one's own position before eruption increase its damage.

<!-- hex:95001600 -->
## Spawner

Allows evergaols to be opened without a Stonesword Key, releasing a wider variety of enemies. Defeating the prisoners does not create a Site of Grace, but allows the same evergaol to be opened again. Each evergaol can be opened up to five times.

<!-- hex:95001700 -->
## Balloon

Raises maximum HP by 50%. Reduces damage negation by 30%. Reduces resistance to piercing attacks by 100%. Triggers an explosion upon death or entering near-death, dealing damage equal to maximum HP to nearby enemies.

<!-- hex:95001800 -->
## Enhanced [Necromancy]

Stores the soul of a powerful enemy defeated by the party, replacing it only with a soul of higher maximum HP. Using [Immortal March] consumes the stored soul to summon a spirit that fights alongside the summoner. The spirit loses 1/60 of its maximum HP each second and deals 25% of its original damage.

<!-- hex:95002000 -->
## Don't Change the Channel

Restores 30 stamina per second while charging sorceries or incantations.

<!-- hex:95002100 -->
## Archmage

Reduces FP consumption of a catalyst’s secondary spell by 100%. After casting, replaces it with a random new spell.

<!-- hex:95002200 -->
## Demon's Gaze

Occasionally creates golden magic circles nearby.

<!-- hex:95002300 -->
## Slingshot

Extends jumping distance after starting a sprint. Stacks up to three times. Each stack increases jump attack power by 50%.

<!-- hex:95002400 -->
## Clash

Attacks grant a guarding effect while two-handing a weapon. After successfully guarding with this effect, guard counters consume no stamina and deal 100% more damage.

<!-- hex:95002500 -->
## Night’s Blessing

Raises attack power by 20% in the Night’s Tide. When HP falls below 20% in the Night’s Tide, gradually restores HP for 15 seconds, then gradually depletes HP until leaving the Night’s Tide. Resets upon touching a site of grace.

<!-- hex:95002600 -->
## Ominous Pact

Landing attacks causes damage to self equal to 5% of maximum HP. Triggers at most once every 10 seconds.

<!-- hex:95002700 -->
## Heartstealer

Grants two Bewitching Branches. Using a Bewitching Branch grants another after 60 seconds. Up to two branches used during this interval are returned.

<!-- hex:95002900 -->
## Essence Reaver

Defeating enemies permanently raises maximum FP by 1%.
