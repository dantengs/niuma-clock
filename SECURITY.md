# Security and private data / 安全与隐私

## English

NiuMa Clock works fully offline. It makes no network requests; all settings, clock-in records, salary figures and expenses stay in `%APPDATA%\NiuMaClock\` on your own computer.

- Do not attach your real `config.json` or `data.json` to a public issue. They contain your salary, working hours and spending. Reproduce problems with made-up data (set `NIUMA_DATA_DIR` to an empty folder) or redact the values first.
- Screenshots of the main window or the floating bar show your earnings. Press `Ctrl + Alt + B` (boss key) to mask amounts before capturing.
- The installer is not code-signed. Verify a download against the `.sha256` file published with each release, and only download from this repository's Releases page.
- The installer writes only to the current user's profile: `%LOCALAPPDATA%\Programs\NiuMaClock`, Start menu and desktop shortcuts, and `HKCU` keys for uninstall registration and autostart. It never needs administrator rights. Uninstall keeps your data unless you tick the delete option.

To report a vulnerability, open an issue without personal data, or use GitHub's private vulnerability reporting for this repository if it is enabled.

## 简体中文

牛马上下班完全离线运行，不发任何网络请求。设置、打卡记录、工资和记账数据都只保存在本机的 `%APPDATA%\NiuMaClock\`。

- 不要把真实的 `config.json`、`data.json` 贴到公开 issue 里，里面有你的工资、工时和花费。复现问题请用虚构数据（把 `NIUMA_DATA_DIR` 指向一个空目录），或先把数值抹掉。
- 主界面和悬浮条截图会露出收入。截图前可以按 `Ctrl + Alt + B`（老板键）隐藏金额。
- 安装包没有代码签名。请只从本仓库的 Releases 页面下载，并用同时发布的 `.sha256` 文件核对。
- 安装包只写当前用户的目录：`%LOCALAPPDATA%\Programs\NiuMaClock`、开始菜单和桌面快捷方式，以及 `HKCU` 下的卸载登记和开机自启，不需要管理员权限。卸载默认保留数据，勾选后才会删除。

报告安全问题：开 issue 时不要附个人数据；如果本仓库开启了 GitHub 私密漏洞报告，也可以用那个渠道。
