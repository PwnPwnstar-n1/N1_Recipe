# N1_Recipe v0.2.3

## 简体中文

### 稳定性改进

- 重构全局快捷键管理机制，明确区分注册中、已注册、冲突、失败和恢复状态。
- 主快捷键与备用快捷键独立注册；替换快捷键失败时保留原有可用配置。
- 对缺失注册增加 2/5/15 秒的有限退避恢复，不反复重注册正常快捷键。
- 改善 Launcher 显示、聚焦与输入时序，并分别记录输入到达、窗口显示和耗时。
- 增加快捷键诊断、测试 Launcher 和手动重新注册入口。

### 新增支持

- 支持 Mouse XButton1 / XButton2，可选 Ctrl、Alt、Shift、Win 修饰键。
- 默认保留原始鼠标事件，并可显式选择只消费精确匹配的侧键组合。
- 增加鼠标侧键去抖，修复一次输入触发两次 Launcher 切换的问题。
- 扩展键盘支持至 F1–F24、方向/导航键、Insert、Pause 和 Print Screen 等经过后端验证的按键。
- 增加快捷键录入器与主键、备用键、鼠标绑定的独立状态显示。

### 兼容性

- 保持 App Identifier `com.snipdeck.app` 和数据库 `snipdeck.db` 不变。
- 保留已有 Recipe、分类、Command Group、用户配置以及旧版 `launcher.shortcut` 设置。
- 支持从 v0.2.2 覆盖升级至 v0.2.3。

### 已知限制

- Windows 不允许普通应用抢占其他应用或系统已经注册的全局快捷键。
- 原生热键 API 不保证能够识别具体的冲突占用进程。
- 部分游戏鼠标驱动可能预先把侧键映射为键盘输入，应用可能无法收到原始 XButton 事件。
- UAC 安全桌面、RDP、睡眠/唤醒、锁屏和所有多屏/DPI 组合尚未完成全面验证。

## English

### Stability improvements

- Reworked global-shortcut state management with explicit registering, registered, conflict, failure, and recovery states.
- Registered primary and backup shortcuts independently, while preserving the working binding and saved settings when a replacement fails.
- Added bounded 2/5/15-second recovery for missing registrations without repeatedly churning healthy shortcuts.
- Improved Launcher display/focus timing and separated input receipt, display result, and latency diagnostics.
- Added shortcut diagnostics, Test Launcher, and manual re-registration controls.

### New support

- Added Mouse XButton1/XButton2 bindings with optional Ctrl, Alt, Shift, and Win modifiers.
- Preserves original mouse input by default, with an explicit option to consume only the exact matched binding.
- Added matched XButton debouncing to prevent a single press from toggling Launcher twice.
- Expanded validated keyboard support through F24, navigation keys, Insert, Pause, and Print Screen.
- Added shortcut recording and independent status display for primary, backup, and mouse bindings.

### Compatibility

- Keeps the `com.snipdeck.app` identifier and `snipdeck.db` database unchanged.
- Preserves existing Recipes, categories, Command Groups, user settings, and the legacy `launcher.shortcut` field.
- Supports an in-place upgrade from v0.2.2 to v0.2.3.

### Known limitations

- Windows does not allow a normal application to override a shortcut already registered by another application or the system.
- The native hotkey API does not reliably identify the process owning a conflicting shortcut.
- Some gaming-mouse software remaps side buttons to keyboard input before applications receive the original XButton event.
- UAC secure desktop, RDP, sleep/resume, lock-screen, and every multi-display/DPI combination are not comprehensively verified.
