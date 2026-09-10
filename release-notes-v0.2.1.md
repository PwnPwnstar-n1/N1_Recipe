# N1_Recipe v0.2.1

N1_Recipe 的首个稳定公开版本。

## Highlights

- 更灵活的 Recipe 导出：支持按分类或精确勾选配方
- 快捷复制窗口支持 Command Group 分步复制和 Copy All
- 修复部分 Windows 设备第一次打开快捷窗口后全局快捷键失效的问题
- 加固快捷窗口生命周期，并加入不记录配方内容的本地诊断日志

## Download

Windows x64 用户下载：

`N1_Recipe_0.2.1_x64-setup.exe`

## Upgrade

可直接覆盖安装旧版。bundle identifier 与数据库文件名保持不变，现有 Recipe 与本地数据会保留。

## Verification

SHA-256 校验值见：

`N1_Recipe_0.2.1_SHA256.txt`

## Notes

N1_Recipe 当前仍采用 local-first 模式。Recipe 中的命令只负责生成与复制，不会自动执行。
