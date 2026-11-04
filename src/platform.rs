//! Windows 系统能力：单实例、锁屏检测、音效、自启动、系统通知、资源写入。
use std::path::Path;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, RECT, WAIT_OBJECT_0,
};
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
use windows_sys::Win32::System::LibraryLoader::{
    BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW,
};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows_sys::Win32::System::StationsAndDesktops::{CloseDesktop, OpenInputDesktop};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, EVENT_MODIFY_STATE, OpenEventW, SetEvent, WaitForSingleObject,
};
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, MB_ICONERROR, MB_OK, MessageBoxW, RT_GROUP_ICON, RT_ICON, SM_CXSCREEN, SM_CYSCREEN,
    SPI_GETWORKAREA, SystemParametersInfoW,
};

const APP_ID: &str = "NiuMa.Clock";
const APP_NAME: &str = "牛马上下班";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "NiuMaClock";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn error_box(text: &str) {
    let body = wide(text);
    let title = wide(APP_NAME);
    unsafe { MessageBoxW(std::ptr::null_mut(), body.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR) };
}

/// 抢单实例锁。已有实例在运行时通知它显示主窗口，并返回 None。
/// 锁按数据目录区分，指定了别的数据目录就能另开一个实例。
pub fn acquire_single_instance(data_dir: &Path) -> Option<ShowSignal> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data_dir.to_string_lossy().to_lowercase().hash(&mut hasher);
    let tag = format!("{:016x}", hasher.finish());
    let show_event = format!("Local\\NiuMaClock.Show.{tag}");
    let name = wide(&format!("Local\\NiuMaClock.Single.{tag}"));
    unsafe {
        let mutex = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        if mutex.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
            let event_name = wide(&show_event);
            let event = OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr());
            if !event.is_null() {
                SetEvent(event);
                CloseHandle(event);
            }
            return None;
        }
        // 互斥量故意不关闭，进程退出时系统回收。
        let event_name = wide(&show_event);
        let event = CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr());
        Some(ShowSignal(event))
    }
}

/// 其他实例发来的“显示主窗口”信号。
pub struct ShowSignal(HANDLE);

// 事件句柄可以跨线程使用。
unsafe impl Send for ShowSignal {}

impl ShowSignal {
    pub fn poll(&self) -> bool {
        !self.0.is_null() && unsafe { WaitForSingleObject(self.0, 0) } == WAIT_OBJECT_0
    }
}

/// 锁屏时当前输入桌面是 Winlogon，普通进程打不开。
pub fn session_locked() -> bool {
    const DESKTOP_SWITCHDESKTOP: u32 = 0x0100;
    unsafe {
        let desktop = OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP);
        if desktop.is_null() {
            return true;
        }
        CloseDesktop(desktop);
        false
    }
}

/// 全屏程序或演示模式。设置环境变量 NIUMA_NO_QUIET 可关掉这项检测，方便测试。
pub fn quiet_mode() -> bool {
    static DISABLED: OnceLock<bool> = OnceLock::new();
    if *DISABLED.get_or_init(|| std::env::var_os("NIUMA_NO_QUIET").is_some()) {
        return false;
    }
    let mut state = 0;
    let ok = unsafe { SHQueryUserNotificationState(&mut state) } >= 0;
    ok && matches!(
        state,
        QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE
    )
}

/// 主显示器工作区（不含任务栏），单位是逻辑像素。
pub fn work_area_points() -> [f32; 4] {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
    };
    unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, (&raw mut rect).cast(), 0) };
    let scale = unsafe { GetDpiForSystem() }.max(96) as f32 / 96.0;
    // 任务栏自动隐藏时工作区就是整屏，弹窗贴底会被弹出的任务栏挡住，留出它的高度。
    let screen_bottom = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let reserve = if rect.bottom >= screen_bottom { 48.0 } else { 0.0 };
    [
        rect.left as f32 / scale,
        rect.top as f32 / scale,
        rect.right as f32 / scale,
        rect.bottom as f32 / scale - reserve,
    ]
}

/// 主显示器整屏范围，单位是逻辑像素。
pub fn screen_points() -> [f32; 4] {
    let scale = unsafe { GetDpiForSystem() }.max(96) as f32 / 96.0;
    let (w, h) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    [0.0, 0.0, w as f32 / scale, h as f32 / scale]
}

// ---------- 音效：现场合成 WAV，不带资源文件 ----------

fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// 依次响起的钟声：(开始秒, 频率, 衰减速度, 音量)。
fn bells(notes: &[(f32, f32, f32, f32)], seconds: f32) -> Vec<u8> {
    const RATE: u32 = 22050;
    let count = (seconds * RATE as f32) as usize;
    let tau = std::f32::consts::TAU;
    let samples: Vec<i16> = (0..count)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let mut mix = 0.0;
            for &(start, freq, decay, volume) in notes {
                if t >= start {
                    let local = t - start;
                    let env = (-decay * local).exp() * (local * 400.0).min(1.0);
                    // 加一点不和谐泛音，听起来更像金属。
                    mix += volume
                        * env
                        * ((tau * freq * local).sin() + 0.3 * (tau * freq * 2.76 * local).sin());
                }
            }
            (mix.clamp(-1.0, 1.0) * 26000.0) as i16
        })
        .collect();
    wav(&samples, RATE)
}

fn play(bytes: &'static [u8]) {
    unsafe {
        PlaySoundW(
            bytes.as_ptr().cast(),
            std::ptr::null_mut(),
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

pub fn play_chime() {
    static SOUND: OnceLock<Vec<u8>> = OnceLock::new();
    play(SOUND.get_or_init(|| bells(&[(0.0, 880.0, 6.0, 0.45), (0.16, 1318.5, 5.0, 0.45)], 0.9)));
}

pub fn play_coins() {
    static SOUND: OnceLock<Vec<u8>> = OnceLock::new();
    play(SOUND.get_or_init(|| {
        bells(
            &[
                (0.00, 1568.0, 8.0, 0.3),
                (0.09, 1760.0, 8.0, 0.3),
                (0.18, 2093.0, 8.0, 0.3),
                (0.27, 2349.0, 8.0, 0.3),
                (0.40, 2637.0, 4.0, 0.4),
                (0.40, 1318.5, 4.0, 0.3),
            ],
            1.6,
        )
    }));
}

// ---------- 弹出窗口外观 ----------

/// 清掉系统给本线程里这些无边框窗口加的装饰，每帧调用即可（没变化时什么也不做）：
/// - Windows 11 的系统圆角和描边：悬浮条自己画圆角，系统再圆一次就会在四角露出一圈浅色底；
/// - 窗口最上面一行像素：winit 的无边框窗口保留了标题栏样式，系统会在这一行画一条灰线，
///   透明窗口上很显眼。用窗口区域把这一行裁掉。
pub fn clean_popup_frames(titles: &[&str]) {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmSetWindowAttribute,
    };
    use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, SetWindowRgn};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumThreadWindows, GetClientRect, GetWindowTextW};

    struct Search {
        titles: Vec<Vec<u16>>,
        found: Vec<HWND>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, param: LPARAM) -> i32 {
        let search = unsafe { &mut *(param as *mut Search) };
        let mut buf = [0u16; 128];
        let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
        if search.titles.iter().any(|t| buf[..len] == t[..]) {
            search.found.push(hwnd);
        }
        1
    }
    // 每个窗口上次裁剪时的尺寸。窗口句柄存成整数，跨帧比较用。
    static CLIPPED: Mutex<Option<HashMap<isize, (i32, i32)>>> = Mutex::new(None);

    let mut search = Search {
        titles: titles.iter().map(|t| t.encode_utf16().collect()).collect(),
        found: Vec::new(),
    };
    unsafe { EnumThreadWindows(GetCurrentThreadId(), Some(visit), &mut search as *mut Search as LPARAM) };
    let mut clipped = CLIPPED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let clipped = clipped.get_or_insert_with(HashMap::new);
    // DWMWA_COLOR_NONE：不画边框。
    const COLOR_NONE: u32 = 0xFFFF_FFFE;
    for &hwnd in &search.found {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if unsafe { GetClientRect(hwnd, &mut rect) } == 0 || rect.right <= 0 || rect.bottom <= 1 {
            continue;
        }
        let size = (rect.right, rect.bottom);
        let known = clipped.insert(hwnd as isize, size);
        if known == Some(size) {
            continue;
        }
        unsafe {
            if known.is_none() {
                let corner = DWMWCP_DONOTROUND;
                DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, (&corner as *const i32).cast(), 4);
                DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR as u32, (&COLOR_NONE as *const u32).cast(), 4);
            }
            // 区域交给系统管理，不用自己释放。
            SetWindowRgn(hwnd, CreateRectRgn(0, 1, size.0, size.1), 1);
        }
    }
}

// ---------- 注册表（只写 HKCU） ----------

fn reg_set(key: &str, name: &str, value: &str) -> bool {
    let key = wide(key);
    let name = wide(name);
    let value = wide(value);
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            REG_SZ,
            value.as_ptr().cast(),
            (value.len() * 2) as u32,
        ) == 0
    }
}

pub fn autostart_enabled() -> bool {
    let key = wide(RUN_KEY);
    let name = wide(RUN_VALUE);
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) == 0
    }
}

pub fn set_autostart(enabled: bool) -> bool {
    if enabled {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        reg_set(RUN_KEY, RUN_VALUE, &format!("\"{}\" --minimized", exe.display()))
    } else {
        let key = wide(RUN_KEY);
        let name = wide(RUN_VALUE);
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) == 0 }
    }
}

/// 未打包的程序要先登记显示名，系统通知才不会显示成一串 ID。
pub fn register_toast_identity(icon_path: &Path) {
    let key = format!(r"Software\Classes\AppUserModelId\{APP_ID}");
    reg_set(&key, "DisplayName", APP_NAME);
    reg_set(&key, "IconUri", &icon_path.display().to_string());
}

pub fn toast(title: &str, body: &str) {
    use tauri_winrt_notification::Toast;
    let mut toast = Toast::new(APP_ID).title(title);
    if !body.is_empty() {
        toast = toast.text1(body);
    }
    let _ = toast.sound(None).show();
}

pub fn reveal_in_explorer(path: &Path) {
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
}

pub fn open_folder(path: &Path) {
    let _ = std::process::Command::new("explorer").arg(path).spawn();
}

// ---------- 打包：把图标写进 exe 资源 ----------

/// images 是 (边长, DIB 数据)，与 ICO 目录一一对应。
pub fn stamp_icon(exe: &Path, images: &[(u32, Vec<u8>)]) -> Result<(), String> {
    let path = wide(&exe.display().to_string());
    const LANG: u16 = 0x0409;
    // MAKEINTRESOURCE(1)：资源 ID 借指针参数传递。
    const GROUP_ID: usize = 1;
    unsafe {
        let handle = BeginUpdateResourceW(path.as_ptr(), 0);
        if handle.is_null() {
            return Err(format!("无法打开 {}", exe.display()));
        }
        let mut group = Vec::new();
        group.extend_from_slice(&0u16.to_le_bytes());
        group.extend_from_slice(&1u16.to_le_bytes());
        group.extend_from_slice(&(images.len() as u16).to_le_bytes());
        for (index, (size, bytes)) in images.iter().enumerate() {
            let id = (index + 1) as u16;
            let ok = UpdateResourceW(
                handle,
                RT_ICON,
                id as usize as *const u16,
                LANG,
                bytes.as_ptr().cast(),
                bytes.len() as u32,
            );
            if ok == 0 {
                EndUpdateResourceW(handle, 1);
                return Err("写入图标资源失败".to_owned());
            }
            let dim = if *size >= 256 { 0 } else { *size as u8 };
            group.extend_from_slice(&[dim, dim, 0, 0]);
            group.extend_from_slice(&1u16.to_le_bytes());
            group.extend_from_slice(&32u16.to_le_bytes());
            group.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            group.extend_from_slice(&id.to_le_bytes());
        }
        let ok = UpdateResourceW(
            handle,
            RT_GROUP_ICON,
            GROUP_ID as *const u16,
            LANG,
            group.as_ptr().cast(),
            group.len() as u32,
        );
        if ok == 0 || EndUpdateResourceW(handle, 0) == 0 {
            return Err("提交图标资源失败".to_owned());
        }
    }
    Ok(())
}
