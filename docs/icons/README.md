# 图标资料

打开 [海克斯图标图鉴](海克斯图标图鉴.html) 可按编号／名称查看原版大、小图标与参数引用，页面已嵌入缩略图，不依赖被删除的 DDS／PNG 解包目录。

- [用途表 CSV](图标用途表.csv)
- [常用大图标](常用大图标.jpg)、[常用小图标](常用小图标.jpg)
- [原版潜在能力大图标](潜在能力原版大图标.jpg)
- [原版小图标 1](原版小图标（1）.jpg)、[原版小图标 2](原版小图标（2）.jpg)

图鉴的“当前海克斯”区是当时生成的快照，现用图标以 `HexBuild` 为准，海克斯名称与描述见 [海克斯文案](../hexes.md)。品质由参数控制，图标不决定金色品质。

大图标由 `PermanentBuffParam.iconId` 控制；词条小图标查 `AttachEffectParam.statusIconId`／`SpEffectParam.iconId`；血条下 HUD 还受效果种类和显示标志影响。要隐藏有限冷却图标直接把图标字段设为 -1，不能只关闭永久效果显示位。

若需重新裁图，`IconProbe` 与 `build_icon_catalog.py` 已保留，临时输入／输出放 `.work/icons`。原始 atlas 的 layout、TPF／DDS 需要按本机游戏资源重新选择性提取，不保留约 800 MiB 的整套历史纹理副本。
