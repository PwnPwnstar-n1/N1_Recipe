# N1_Recipe v0.2.2

N1_Recipe 的首个推荐公开使用的稳定版本。

## Highlights

- Recipe 导出现在支持按分类或精确勾选配方
- 快捷复制窗口完整支持 Command Group 分步复制和 Copy All
- 修复部分 Windows 设备上快捷窗口首次关闭后全局快捷键失效的问题
- 加固快捷窗口生命周期和本地诊断能力

## Windows

支持 Windows x64。

下载：

`N1_Recipe_0.2.2_x64-setup.exe`

## Upgrade

可以从现有 N1_Recipe / SnipDeck 版本直接覆盖安装。

现有本地 Recipe、分类和数据库数据保持兼容。

## Security / Execution Model

N1_Recipe 当前采用 local-first 模式。

Recipe 中的命令只负责生成和复制，不会自动执行 shell、PowerShell 或 CMD 命令。

## Verification

SHA-256 校验信息见：

`N1_Recipe_0.2.2_SHA256.txt`
