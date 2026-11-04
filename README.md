# 牛马上下班

> 牛马 · 上下班 ／ 牛 · 马上下班。打工人的身体健康提醒器 + 实时赚钱表。

一个 Windows 桌面小工具，Rust 编写。不联网，数据只存在本机。提供安装包，也可以直接运行单个 exe。

> **English summary.** NiuMa Clock is a small offline Windows tray app for office workers: clock in/out, recurring health reminders (stand up, drink water, take medicine), a live "money earned today" meter based on your monthly salary, an expense ledger, a floating earnings bar and an end-of-day celebration screen. The UI is in Chinese. Download `NiuMa-Setup.exe` from [Releases](https://github.com/dantengs/niuma-clock/releases); it installs per user without admin rights. MIT licensed.

## 功能

| 功能 | 说明 |
|---|---|
| 上下班打卡 | 主界面大按钮或托盘菜单打卡。一天可以打多段，结算时合并计算 |
| 循环事项 | 记事内容加倒计时，到点弹出置顶卡片，可选“搞定了 / 稍后 / 跳过”。下班打卡时清空所有运行中的循环；存为“常用事项”的下次上班自动开始 |
| 身心状态 | 身体分健康、亚健康、不健康，心情有六档。设为不健康后按吃药方案提醒（固定时间或固定间隔），下班也照常提醒，恢复健康后停止 |
| 赚钱表 | 填月薪后实时跳字，频率可选每秒、每分钟、每小时。午休可设为不计薪，加班按倍率计算，也可选“白嫖”模式 |
| 修改本月已赚 | 工资不是从装软件那天算起的，在“今日”页“本月已赚”上点“修改”，填上这个月到现在实际赚了多少。程序记下差额，之后照常累加，下个月自动从零开始 |
| 记账 | “记账”页记录每笔花费：金额、分类、备注、时间（默认现在）。按月看花费、已赚、结余和分类占比，会换算“这笔相当于上几小时班”。可删除和撤销，可导出 CSV |
| 调色板 | 设置里有 6 套一键配色，也可以单独改主色、金额色、赚钱卡片底色、窗口背景和卡片底色。浅色和深色主题各存一套 |
| 下班特效 | 下班打卡时全屏撒金币，展示今日收入、工时、完成提醒次数、牛马血条和一句语录。点击或按 Esc 关闭 |
| 悬浮赚钱条 | 桌面右下角的小条，显示今日收入和下班倒计时。按住拖到任意位置，位置会记住；单击打开主界面；设置里可以放回右下角 |
| 老板键 | `Ctrl + Alt + B` 隐藏所有金额和悬浮条 |
| 防打扰 | 锁屏或休眠时暂停循环倒计时；全屏或演示时推迟提醒、隐藏悬浮条 |
| 忘打卡补救 | 跨天没打下班卡会询问，可按下班时间补卡或自定义时间 |
| 打卡记录 | 月度出勤、工时、收入和每日工时柱状图；记录可修改、删除，可导出 CSV（Excel 可直接打开） |
| 其他 | 牛马血条、发薪日倒计时、牛马语录、到点提醒下班、托盘常驻、开机自启、浅色和深色主题 |

## 安装

从 [Releases](https://github.com/dantengs/niuma-clock/releases) 下载 `NiuMa-Setup.exe`（约 3.5 MB），双击后点“安装”即可。下载后可以用同目录的 `.sha256` 文件核对：`Get-FileHash NiuMa-Setup.exe -Algorithm SHA256`。

安装包没有代码签名，首次运行时 Windows SmartScreen 可能提示“未知发布者”，点“更多信息”→“仍要运行”。

- 只为当前用户安装到 `%LOCALAPPDATA%\Programs\NiuMaClock`，不需要管理员权限。
- 会在开始菜单创建“牛马上下班”，可选创建桌面快捷方式。
- 卸载：在系统设置的“已安装的应用”里找“牛马上下班”。设置和打卡记录默认保留，卸载时可勾选一并删除。
- 升级：直接运行新版安装包。如果程序正在运行，先右键托盘牛头图标选“退出”。
- 之前用便携版开了开机自启的，安装后自启会改为启动安装版。
- 命令行：`NiuMa-Setup.exe --silent` 静默安装；`Uninstall.exe --uninstall --silent [--purge]` 静默卸载，`--purge` 连数据一起删。

不想安装的话，也可以自己构建后直接运行 `dist\NiuMa.exe`（便携版）。两种方式共用同一份数据。

## 使用

启动后到“设置”填写月薪和上下班时间。

- 关闭窗口后程序留在托盘，提醒照常运行。要退出，右键托盘图标选“退出”。
- 重复启动只会唤出已打开的窗口。
- 命令行参数：`--minimized` 启动后只留在托盘（开机自启用的就是它）；`--preview-effect` 启动时预览下班特效。

数据位置：`%APPDATA%\NiuMaClock\`，包含 `config.json`（设置）和 `data.json`（打卡记录与状态）。文件损坏时程序会把它改名备份，不会覆盖。设置环境变量 `NIUMA_DATA_DIR` 可以改用别的目录，比如测试时。测试时还可以设置 `NIUMA_NO_QUIET`，关掉“全屏时隐藏悬浮条”的检测。

## 构建

需要 Rust 工具链（`stable-x86_64-pc-windows-gnu`，再加 `rustup target add x86_64-pc-windows-gnullvm`）和一个 llvm-mingw 版的 clang。开发机没有 MSVC，所以用 `x86_64-pc-windows-gnullvm` 目标：mingw clang 负责组织链接参数，Rust 自带的 rust-lld 负责链接。默认用 GHC 9.10.3 自带的 clang（`C:\ghcup\ghc\9.10.3\mingw\bin`），别的位置用环境变量 `NIUMA_CLANG_BIN` 指定。

第一次运行 `scripts\env.ps1` 时，它会按 `.cargo\config.example.toml` 生成本机的 `.cargo\config.toml`（不提交）。如果装了 MSVC，可以删掉生成的文件，改用 msvc 目标。

```powershell
. .\scripts\env.ps1      # 临时把 cargo 和 clang 加入 PATH，并生成 .cargo\config.toml
cargo test               # 68 个单元测试
.\scripts\build.ps1      # 生成 dist\NiuMa.exe 和安装包 dist\NiuMa-Setup.exe，并写入图标
```

安装包是 `installer\` 下的独立小程序：构建时把 `dist\NiuMa.exe` 压缩后嵌进去，界面用系统自带的任务对话框，清单资源由 GHC 自带的 `llvm-windres` 编译。没有用 Inno Setup 或 NSIS，构建时不需要额外安装任何工具。

## 代码结构

| 文件 | 职责 |
|---|---|
| `engine.rs` | 中央状态机：打卡、循环、吃药、提醒确认和心跳。时间由调用方传入，便于测试 |
| `salary.rs` | 工资计算：午休扣除、跨夜班、加班倍率、跳字取整、发薪日 |
| `model.rs` | 持久化数据结构，字段都有默认值，旧文件可直接读入 |
| `store.rs` | JSON 原子写入 |
| `app.rs` | 后台心跳线程、托盘、热键、窗口显隐 |
| `pages.rs` | 主窗口六个页面（含记账）和调色板 |
| `windows.rs` | 提醒卡片、悬浮条、下班特效 |
| `platform.rs` | Windows 接口：单实例、锁屏检测、合成音效、注册表、系统通知 |
| `icon.rs` | 用代码画的牛头图标 |
| `installer/src/main.rs` | 安装包：安装、升级、卸载流程 |
| `installer/src/win.rs` | 安装包用的 Windows 接口：任务对话框、快捷方式、注册表 |

开发过程、验证记录和决策见 [开发台账.md](开发台账.md)。

## 许可证与贡献者

- 本项目代码以 [MIT 许可证](LICENSE) 发布。
- 依赖库的许可证见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)，原文保存在 `third-party-licenses/`。
- 贡献者和 AI 协作说明见 [CONTRIBUTORS.md](CONTRIBUTORS.md)；安全和隐私说明见 [SECURITY.md](SECURITY.md)。
