//! 《牛马上下班》安装程序。内嵌压缩后的 NiuMa.exe，只为当前用户安装，不需要管理员权限。
//!
//! - 安装：`NiuMa-Setup.exe [--silent]`
//! - 卸载：`Uninstall.exe [--uninstall] [--silent] [--purge]`。安装目录里的 Uninstall.exe 就是本程序的副本，
//!   它先把自己复制到临时目录再从那里运行，这样才能删掉安装目录。`--purge` 连设置和打卡记录一起删。
#![windows_subsystem = "windows"]

mod win;

use std::fs;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use win::{Dialog, IDCANCEL, Icon};

const VERSION: &str = env!("NIUMA_VERSION");
static PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.bin"));

const DIR_NAME: &str = "NiuMaClock";
const EXE_NAME: &str = "NiuMa.exe";
const UNINSTALLER: &str = "Uninstall.exe";
const LINK_NAME: &str = "牛马上下班.lnk";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\NiuMaClock";
// 下面三项与主程序 platform.rs 保持一致。
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "NiuMaClock";
const TOAST_KEY: &str = r"Software\Classes\AppUserModelId\NiuMa.Clock";

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const ID_OK: i32 = 100;
const ID_LAUNCH: i32 = 101;
const ID_RETRY: i32 = 102;

/// 用户点了取消。与真正的错误区分开，取消时不弹错误框。
const CANCELLED: &str = "";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|arg| arg == name);
    let silent = flag("--silent");
    let purge = flag("--purge");
    let me = std::env::current_exe().unwrap_or_default();
    let named_uninstaller = me
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(UNINSTALLER));

    let result = if let Some(i) = args.iter().position(|arg| arg == "--uninstall-from") {
        match args.get(i + 1) {
            Some(dir) => uninstall(Path::new(dir), silent, purge),
            None => Err("--uninstall-from 后面缺少目录".to_owned()),
        }
    } else if flag("--uninstall") || named_uninstaller {
        relaunch_from_temp(&me, silent, purge)
    } else {
        install(silent)
    };
    if let Err(message) = result {
        if !silent && message != CANCELLED {
            win::error(&message);
        }
        std::process::exit(1);
    }
}

fn install_dir() -> Result<PathBuf, String> {
    win::known_folder(&win::FOLDERID_UserProgramFiles)
        .map(|programs| programs.join(DIR_NAME))
        .ok_or_else(|| "找不到用户程序目录（%LOCALAPPDATA%\\Programs）".to_owned())
}

fn shortcut_paths() -> (Option<PathBuf>, Option<PathBuf>) {
    let start = win::known_folder(&win::FOLDERID_Programs).map(|dir| dir.join(LINK_NAME));
    let desktop = win::known_folder(&win::FOLDERID_Desktop).map(|dir| dir.join(LINK_NAME));
    (start, desktop)
}

fn has_desktop_link(link: &Option<PathBuf>) -> bool {
    link.as_ref().is_some_and(|path| path.exists())
}

fn data_dir() -> Option<PathBuf> {
    win::known_folder(&win::FOLDERID_RoamingAppData).map(|dir| dir.join(DIR_NAME))
}

/// 执行文件操作；失败时先自动重试约 3 秒（等刚退出的进程放开文件），
/// 仍失败就让用户选择重试或取消。静默模式下直接报错。
fn retry<T>(silent: bool, hint: &str, mut op: impl FnMut() -> io::Result<T>) -> Result<T, String> {
    let mut attempts = 0;
    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(_) if attempts < 15 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) if silent => return Err(format!("{hint}（{e}）")),
            Err(e) => {
                let content = format!("{hint}\n\n系统提示：{e}");
                let (button, _) = Dialog {
                    instruction: "文件被占用",
                    content: &content,
                    buttons: &[(ID_RETRY, "重试"), (IDCANCEL, "取消")],
                    check: None,
                    icon: Icon::Warning,
                }
                .show();
                if button != ID_RETRY {
                    return Err(CANCELLED.to_owned());
                }
            }
        }
    }
}

// ---------- 安装 ----------

fn install(silent: bool) -> Result<(), String> {
    let dir = install_dir()?;
    let exe = dir.join(EXE_NAME);
    let (start_link, desktop_link) = shortcut_paths();
    let upgrading = win::reg_get_str(UNINSTALL_KEY, "DisplayVersion");

    let mut want_desktop = true;
    if !silent {
        let (instruction, default_desktop) = match &upgrading {
            Some(old) if old == VERSION => (format!("重新安装牛马上下班 v{VERSION}"), has_desktop_link(&desktop_link)),
            Some(old) => (format!("把牛马上下班从 v{old} 升级到 v{VERSION}"), has_desktop_link(&desktop_link)),
            None => (format!("安装牛马上下班 v{VERSION}"), true),
        };
        // 完整路径太长会被对话框截成省略号，这里用环境变量写法。
        let content = format!(
            "安装位置：%LOCALAPPDATA%\\Programs\\{DIR_NAME}\n\n只为当前用户安装，不需要管理员权限。\n设置和打卡记录保存在 %APPDATA%\\{DIR_NAME}，升级和卸载都不会动它们。"
        );
        let (button, checked) = Dialog {
            instruction: &instruction,
            content: &content,
            buttons: &[(ID_OK, if upgrading.is_some() { "升级" } else { "安装" }), (IDCANCEL, "取消")],
            check: Some(("创建桌面快捷方式", default_desktop)),
            icon: Icon::App,
        }
        .show();
        if button != ID_OK {
            return Err(CANCELLED.to_owned());
        }
        want_desktop = checked;
    }

    let program = miniz_oxide::inflate::decompress_to_vec(PAYLOAD).map_err(|e| format!("安装包已损坏：{e:?}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建 {}：{e}", dir.display()))?;
    // 先写临时文件再改名：程序正在运行时改名会失败，旧文件保持完好。
    let staged = dir.join("NiuMa.exe.new");
    fs::write(&staged, &program).map_err(|e| format!("无法写入 {}：{e}", staged.display()))?;
    let replaced = retry(
        silent,
        "牛马上下班正在运行。请右键任务栏托盘里的牛头图标，选“退出”，然后点重试。",
        || fs::rename(&staged, &exe),
    );
    if replaced.is_err() {
        let _ = fs::remove_file(&staged);
    }
    replaced?;
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let uninstaller = dir.join(UNINSTALLER);
    if me != uninstaller {
        fs::copy(&me, &uninstaller).map_err(|e| format!("无法写入卸载程序：{e}"))?;
    }

    win::com_init();
    let description = "打工人的身体健康提醒器 + 实时赚钱表";
    if let Some(link) = &start_link {
        win::create_shortcut(link, &exe, description)?;
    }
    if let Some(link) = &desktop_link {
        if want_desktop {
            win::create_shortcut(link, &exe, description)?;
        } else {
            let _ = fs::remove_file(link);
        }
    }
    register_uninstall(&dir, program.len());
    // 之前在便携版里开了开机自启的，改为启动安装后的程序。
    if let Some(command) = win::reg_get_str(RUN_KEY, RUN_VALUE)
        && !command.to_lowercase().contains(&exe.display().to_string().to_lowercase())
    {
        win::reg_set_str(RUN_KEY, RUN_VALUE, &format!("\"{}\" --minimized", exe.display()));
    }

    if !silent {
        let place = if want_desktop && desktop_link.is_some() { "开始菜单和桌面" } else { "开始菜单" };
        let content = format!(
            "{place}里都能找到“牛马上下班”了。\n关掉主窗口后它会留在任务栏托盘里继续提醒。\n要卸载，在系统设置的“已安装的应用”里找“牛马上下班”。"
        );
        let (button, _) = Dialog {
            instruction: if upgrading.is_some() { "升级完成" } else { "安装完成" },
            content: &content,
            buttons: &[(ID_LAUNCH, "立即启动"), (IDCANCEL, "完成")],
            check: None,
            icon: Icon::App,
        }
        .show();
        if button == ID_LAUNCH {
            Command::new(&exe).current_dir(&dir).spawn().map_err(|e| format!("启动失败：{e}"))?;
        }
    }
    Ok(())
}

fn register_uninstall(dir: &Path, program_bytes: usize) {
    let exe = dir.join(EXE_NAME);
    let uninstaller = dir.join(UNINSTALLER);
    let key = UNINSTALL_KEY;
    win::reg_set_str(key, "DisplayName", win::APP_NAME);
    win::reg_set_str(key, "DisplayVersion", VERSION);
    win::reg_set_str(key, "Publisher", "牛马上下班");
    win::reg_set_str(key, "DisplayIcon", &format!("{},0", exe.display()));
    win::reg_set_str(key, "InstallLocation", &dir.display().to_string());
    win::reg_set_str(key, "UninstallString", &format!("\"{}\" --uninstall", uninstaller.display()));
    win::reg_set_str(key, "QuietUninstallString", &format!("\"{}\" --uninstall --silent", uninstaller.display()));
    win::reg_set_dword(key, "NoModify", 1);
    win::reg_set_dword(key, "NoRepair", 1);
    let total = program_bytes + PAYLOAD.len();
    win::reg_set_dword(key, "EstimatedSize", (total / 1024) as u32);
}

// ---------- 卸载 ----------

/// 从安装目录运行时，复制到临时目录再启动，自己退出，让安装目录可以被删掉。
fn relaunch_from_temp(me: &Path, silent: bool, purge: bool) -> Result<(), String> {
    let dir = me.parent().ok_or("找不到安装目录")?;
    let temp = std::env::temp_dir().join(format!("NiuMa-uninstall-{}.exe", std::process::id()));
    fs::copy(me, &temp).map_err(|e| format!("无法准备卸载程序：{e}"))?;
    let mut cmd = Command::new(&temp);
    cmd.arg("--uninstall-from").arg(dir);
    if silent {
        cmd.arg("--silent");
    }
    if purge {
        cmd.arg("--purge");
    }
    cmd.spawn().map_err(|e| format!("无法启动卸载程序：{e}"))?;
    Ok(())
}

fn uninstall(dir: &Path, silent: bool, mut purge: bool) -> Result<(), String> {
    // 只删我们自己装的目录，防止参数写错删掉别的东西。
    let ours = dir.file_name().is_some_and(|name| name == DIR_NAME)
        && (dir.join(EXE_NAME).exists() || dir.join(UNINSTALLER).exists());
    if !ours {
        delete_self_later();
        return Err(format!("{} 不是牛马上下班的安装目录", dir.display()));
    }
    let data = data_dir();

    if !silent {
        let content = format!(
            "将删除程序文件、开始菜单和桌面快捷方式，并取消开机自启。\n\n设置和打卡记录在 %APPDATA%\\{DIR_NAME}，默认保留，重新安装后会自动接上。"
        );
        let (button, checked) = Dialog {
            instruction: "卸载牛马上下班？",
            content: &content,
            buttons: &[(ID_OK, "卸载"), (IDCANCEL, "取消")],
            check: Some(("同时删除我的设置和打卡记录", false)),
            icon: Icon::Warning,
        }
        .show();
        if button != ID_OK {
            delete_self_later();
            return Err(CANCELLED.to_owned());
        }
        purge = checked;
    }

    let removed = retry(
        silent,
        "牛马上下班正在运行。请右键任务栏托盘里的牛头图标，选“退出”，然后点重试。",
        || fs::remove_file(dir.join(EXE_NAME)).or_else(ignore_missing),
    )
    // 卸载程序副本可能还没完全退出，retry 会等它。
    .and_then(|_| retry(silent, "无法删除安装目录。", || fs::remove_dir_all(dir).or_else(ignore_missing)));
    if let Err(message) = removed {
        delete_self_later();
        return Err(message);
    }

    let (start_link, desktop_link) = shortcut_paths();
    for link in [start_link, desktop_link].into_iter().flatten() {
        let _ = fs::remove_file(link);
    }
    // 开机自启只在指向本安装目录时才删，免得误删便携版的设置。
    if let Some(command) = win::reg_get_str(RUN_KEY, RUN_VALUE)
        && command.to_lowercase().contains(&dir.display().to_string().to_lowercase())
    {
        win::reg_delete_value(RUN_KEY, RUN_VALUE);
    }
    win::reg_delete_tree(TOAST_KEY);
    win::reg_delete_tree(UNINSTALL_KEY);
    if purge && let Some(data) = &data {
        let _ = fs::remove_dir_all(data);
    }

    if !silent {
        let content = if purge {
            "程序和你的设置、打卡记录都已删除。".to_owned()
        } else {
            format!("设置和打卡记录保留在 %APPDATA%\\{DIR_NAME}，不需要的话可以手动删除。")
        };
        Dialog {
            instruction: "已卸载，下班快乐",
            content: &content,
            buttons: &[(IDCANCEL, "完成")],
            check: None,
            icon: Icon::App,
        }
        .show();
    }
    delete_self_later();
    Ok(())
}

fn ignore_missing(e: io::Error) -> io::Result<()> {
    if e.kind() == io::ErrorKind::NotFound { Ok(()) } else { Err(e) }
}

/// 临时目录里的卸载程序副本没法删自己，交给一个隐藏的 cmd 等两秒后删。
fn delete_self_later() {
    let Ok(me) = std::env::current_exe() else { return };
    if !me.starts_with(std::env::temp_dir()) {
        return;
    }
    let _ = Command::new("cmd")
        .raw_arg(format!("/d /c ping -n 3 127.0.0.1 >nul & del /f /q \"{}\"", me.display()))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}
