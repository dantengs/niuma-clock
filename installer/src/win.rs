//! 安装程序用到的 Windows 接口：对话框、已知文件夹、快捷方式、注册表（只写 HKCU）。
use std::ffi::c_void;
use std::path::{Path, PathBuf};

use windows_sys::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_DWORD, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegDeleteTreeW,
    RegGetValueW, RegSetKeyValueW,
};
use windows_sys::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TD_ERROR_ICON, TD_WARNING_ICON, TDF_ALLOW_DIALOG_CANCELLATION,
    TDF_USE_HICON_MAIN, TDF_VERIFICATION_FLAG_CHECKED, TaskDialogIndirect,
};
use windows_sys::Win32::UI::Shell::SHGetKnownFolderPath;
use windows_sys::Win32::UI::WindowsAndMessaging::LoadIconW;
use windows_sys::core::{GUID, HRESULT, PCWSTR, PWSTR};

pub use windows_sys::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Programs, FOLDERID_RoamingAppData, FOLDERID_UserProgramFiles,
};

pub const APP_NAME: &str = "牛马上下班";
pub const IDCANCEL: i32 = 2;

pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_path(path: &Path) -> Vec<u16> {
    wide(&path.display().to_string())
}

// ---------- 对话框 ----------

#[derive(Clone, Copy)]
pub enum Icon {
    App,
    Warning,
    Error,
}

pub struct Dialog<'a> {
    pub instruction: &'a str,
    pub content: &'a str,
    /// (按钮 ID, 文字)，第一个是默认按钮。
    pub buttons: &'a [(i32, &'a str)],
    /// 底部复选框的文字和初始状态。
    pub check: Option<(&'a str, bool)>,
    pub icon: Icon,
}

impl Dialog<'_> {
    /// 返回 (按下的按钮 ID, 复选框是否勾选)。关闭窗口算 IDCANCEL。
    pub fn show(&self) -> (i32, bool) {
        let title = wide(APP_NAME);
        let instruction = wide(self.instruction);
        let content = wide(self.content);
        let texts: Vec<Vec<u16>> = self.buttons.iter().map(|(_, text)| wide(text)).collect();
        let buttons: Vec<TASKDIALOG_BUTTON> = self
            .buttons
            .iter()
            .zip(&texts)
            .map(|((id, _), text)| TASKDIALOG_BUTTON { nButtonID: *id, pszButtonText: text.as_ptr() })
            .collect();
        let check_text = self.check.map(|(text, _)| wide(text));

        let mut cfg = TASKDIALOGCONFIG {
            cbSize: size_of::<TASKDIALOGCONFIG>() as u32,
            dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
            pszWindowTitle: title.as_ptr(),
            pszMainInstruction: instruction.as_ptr(),
            pszContent: content.as_ptr(),
            cButtons: buttons.len() as u32,
            pButtons: buttons.as_ptr(),
            nDefaultButton: self.buttons.first().map_or(0, |(id, _)| *id),
            ..Default::default()
        };
        if let Some(text) = &check_text {
            cfg.pszVerificationText = text.as_ptr();
            if self.check.is_some_and(|(_, on)| on) {
                cfg.dwFlags |= TDF_VERIFICATION_FLAG_CHECKED;
            }
        }
        match self.icon {
            Icon::App => {
                // 图标组 1 由 NiuMa.exe --stamp-icon 写入；没写入时不显示图标。
                let icon = unsafe { LoadIconW(GetModuleHandleW(std::ptr::null()), 1 as PCWSTR) };
                if !icon.is_null() {
                    cfg.dwFlags |= TDF_USE_HICON_MAIN;
                    cfg.Anonymous1.hMainIcon = icon;
                }
            }
            Icon::Warning => cfg.Anonymous1.pszMainIcon = TD_WARNING_ICON,
            Icon::Error => cfg.Anonymous1.pszMainIcon = TD_ERROR_ICON,
        }

        let mut button = IDCANCEL;
        let mut checked = 0;
        let hr = unsafe { TaskDialogIndirect(&cfg, &mut button, std::ptr::null_mut(), &mut checked) };
        if hr < 0 {
            return (IDCANCEL, false);
        }
        (button, checked != 0)
    }
}

pub fn error(text: &str) {
    Dialog {
        instruction: "出错了",
        content: text,
        buttons: &[(IDCANCEL, "关闭")],
        check: None,
        icon: Icon::Error,
    }
    .show();
}

// ---------- 已知文件夹 ----------

pub fn known_folder(id: &GUID) -> Option<PathBuf> {
    let mut raw: PWSTR = std::ptr::null_mut();
    unsafe {
        let hr = SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut raw);
        if hr < 0 || raw.is_null() {
            return None;
        }
        let len = (0..).take_while(|&i| *raw.add(i) != 0).count();
        let path = String::from_utf16_lossy(std::slice::from_raw_parts(raw, len));
        CoTaskMemFree(raw.cast());
        Some(PathBuf::from(path))
    }
}

// ---------- 快捷方式（手写 IShellLinkW / IPersistFile 的虚表） ----------

const CLSID_SHELL_LINK: GUID = GUID::from_u128(0x00021401_0000_0000_c000_000000000046);
const IID_SHELL_LINK_W: GUID = GUID::from_u128(0x000214f9_0000_0000_c000_000000000046);
const IID_PERSIST_FILE: GUID = GUID::from_u128(0x0000010b_0000_0000_c000_000000000046);

type Raw = *mut c_void;
type SetText = unsafe extern "system" fn(Raw, PCWSTR) -> HRESULT;

#[repr(C)]
struct UnknownVtbl {
    query_interface: unsafe extern "system" fn(Raw, *const GUID, *mut Raw) -> HRESULT,
    add_ref: usize,
    release: unsafe extern "system" fn(Raw) -> u32,
}

/// 方法顺序与 shobjidl_core.h 一致，用不到的只占位。
#[repr(C)]
struct ShellLinkVtbl {
    base: UnknownVtbl,
    get_path: usize,
    get_id_list: usize,
    set_id_list: usize,
    get_description: usize,
    set_description: SetText,
    get_working_directory: usize,
    set_working_directory: SetText,
    get_arguments: usize,
    set_arguments: usize,
    get_hotkey: usize,
    set_hotkey: usize,
    get_show_cmd: usize,
    set_show_cmd: usize,
    get_icon_location: usize,
    set_icon_location: unsafe extern "system" fn(Raw, PCWSTR, i32) -> HRESULT,
    set_relative_path: usize,
    resolve: usize,
    set_path: SetText,
}

#[repr(C)]
struct PersistFileVtbl {
    base: UnknownVtbl,
    get_class_id: usize,
    is_dirty: usize,
    load: usize,
    save: unsafe extern "system" fn(Raw, PCWSTR, i32) -> HRESULT,
    save_completed: usize,
    get_cur_file: usize,
}

unsafe fn vtbl<T>(obj: Raw) -> &'static T {
    unsafe { &**(obj as *mut *const T) }
}

pub fn com_init() {
    unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
}

pub fn create_shortcut(lnk: &Path, target: &Path, description: &str) -> Result<(), String> {
    let fail = |step: &str, hr: HRESULT| format!("创建快捷方式 {} 失败（{step}，0x{hr:08X}）", lnk.display());
    let target_w = wide_path(target);
    let dir_w = wide_path(target.parent().unwrap_or(target));
    let desc_w = wide(description);
    let lnk_w = wide_path(lnk);
    unsafe {
        let mut link: Raw = std::ptr::null_mut();
        let hr = CoCreateInstance(&CLSID_SHELL_LINK, std::ptr::null_mut(), CLSCTX_INPROC_SERVER, &IID_SHELL_LINK_W, &mut link);
        if hr < 0 {
            return Err(fail("CoCreateInstance", hr));
        }
        let v = vtbl::<ShellLinkVtbl>(link);
        (v.set_path)(link, target_w.as_ptr());
        (v.set_working_directory)(link, dir_w.as_ptr());
        (v.set_description)(link, desc_w.as_ptr());
        (v.set_icon_location)(link, target_w.as_ptr(), 0);

        let mut file: Raw = std::ptr::null_mut();
        let hr = (v.base.query_interface)(link, &IID_PERSIST_FILE, &mut file);
        let result = if hr < 0 {
            Err(fail("IPersistFile", hr))
        } else {
            let pf = vtbl::<PersistFileVtbl>(file);
            let hr = (pf.save)(file, lnk_w.as_ptr(), 1);
            (pf.base.release)(file);
            if hr < 0 { Err(fail("Save", hr)) } else { Ok(()) }
        };
        (v.base.release)(link);
        result
    }
}

// ---------- 注册表（HKCU） ----------

fn reg_set(key: &str, name: &str, kind: u32, data: &[u8]) -> bool {
    let key = wide(key);
    let name = wide(name);
    unsafe {
        RegSetKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), kind, data.as_ptr().cast(), data.len() as u32) == 0
    }
}

pub fn reg_set_str(key: &str, name: &str, value: &str) -> bool {
    let bytes: Vec<u8> = wide(value).iter().flat_map(|unit| unit.to_le_bytes()).collect();
    reg_set(key, name, REG_SZ, &bytes)
}

pub fn reg_set_dword(key: &str, name: &str, value: u32) -> bool {
    reg_set(key, name, REG_DWORD, &value.to_le_bytes())
}

pub fn reg_get_str(key: &str, name: &str) -> Option<String> {
    let key = wide(key);
    let name = wide(name);
    let mut buf = vec![0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if rc != 0 {
        return None;
    }
    let len = buf.iter().position(|&unit| unit == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

pub fn reg_delete_value(key: &str, name: &str) {
    let key = wide(key);
    let name = wide(name);
    unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) };
}

pub fn reg_delete_tree(key: &str) {
    let key = wide(key);
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, key.as_ptr()) };
}
