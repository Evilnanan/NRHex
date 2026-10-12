# 数值编辑来源

`sword.csv` 维护“亮出你的剑”的法术加成。加成允许0～100及小数，留空按0处理；保留全部MagicID，避免重复或缺失。增伤、咏唱减伤使用表中原值，吸血使用原值×1.5。

`cargo build --release --lib` 自动读取并校验当前表格，将数值嵌入DLL。修改后重新编译，将 `target/release/NRHex.dll` 更新到 `dist/mod/natives/NRHex.dll`，重启游戏后生效。构建保留表格原内容，dist成品包无需附带CSV。
