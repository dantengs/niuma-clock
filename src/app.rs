//! 程序外壳：后台心跳线程、托盘、热键、主窗口显隐和页面调度。
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{Datelike, Local, NaiveDateTime};
use eframe::egui::{self, Color32, Frame, Margin, RichText, Sense, Stroke, ViewportCommand};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::engine::{AckAction, AlertKind, DaySummary, Engine, TickEnv};
use crate::model::*;
use crate::pages::{ExpenseForm, LoopForm, MedForm, MonthEdit, SessionEdit};
use crate::platform::{self, ShowSignal};
use crate::widgets::{self, Palette};
use crate::windows::{self, Celebration};
use crate::{icon, quotes, store};

pub fn local_now() -> NaiveDateTime {
    Local::now().naive_local()
}

pub fn lock(engine: &Mutex<Engine>) -> MutexGuard<'_, Engine> {
    engine.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Today,
    Loops,
    Health,
    Records,
    Ledger,
    Settings,
}

impl Page {
    const ALL: [Page; 6] = [Page::Today, Page::Loops, Page::Health, Page::Records, Page::Ledger, Page::Settings];

    fn label(self) -> &'static str {
        match self {
            Page::Today => "今日",
            Page::Loops => "循环事项",
            Page::Health => "身心状态",
            Page::Records => "打卡记录",
            Page::Ledger => "记账",
            Page::Settings => "设置",
        }
    }
}

enum UiCmd {
    Show,
    ToggleClock,
    ToggleFloating,
    ToggleHideMoney,
    Quit,
}

/// 后台线程和界面线程共享的标记。
#[derive(Default)]
pub struct Shared {
    show_requested: AtomicBool,
    /// 全屏或演示中，隐藏所有悬浮窗口。
    pub quiet: AtomicBool,
    pub save_error: Mutex<Option<String>>,
}

const MENU_SHOW: &str = "show";
const MENU_CLOCK: &str = "clock";
const MENU_FLOATING: &str = "floating";
const MENU_HIDE: &str = "hide-money";
const MENU_QUIT: &str = "quit";

const BOSS_KEY_LABEL: &str = "Ctrl + Alt + B";
const MIN_FRAME: Duration = Duration::from_millis(16);

struct Tray {
    clock: MenuItem,
    floating: CheckMenuItem,
    hide: CheckMenuItem,
    last_tip: Instant,
    icon: TrayIcon,
}

pub struct App {
    last_frame: Instant,
    /// eframe 首帧画完会无条件显示根窗口，托盘启动时要再藏回去。
    startup_hides: u8,
    children_visible: bool,
    pub engine: Arc<Mutex<Engine>>,
    pub shared: Arc<Shared>,
    pub dir: PathBuf,
    pub page: Page,
    pub hidden: bool,
    pub palette: Palette,
    applied_palette: Option<Palette>,
    pub logo: egui::TextureHandle,
    pub effect: Option<Celebration>,
    pub has_cjk_font: bool,
    tray: Option<Tray>,
    hotkey: Option<GlobalHotKeyManager>,
    boss_hotkey: HotKey,
    pub boss_key_error: Option<String>,
    rx: Receiver<UiCmd>,
    last_forgot_check: Instant,
    /// 悬浮条已经按正确的缩放放到位。之前用户拖到哪就在哪，不再强行摆回去。
    pub floating_placed: bool,
    pub autostart: bool,
    pub notice: Option<(String, Instant)>,
    pub loop_form: LoopForm,
    pub med_form: MedForm,
    pub session_edit: Option<SessionEdit>,
    pub pending_delete: Option<usize>,
    pub record_month: (i32, u32),
    pub ledger_month: (i32, u32),
    pub expense_form: ExpenseForm,
    pub expense_undo: Option<Expense>,
    pub month_edit: Option<MonthEdit>,
    /// 调色板里正在选色的那一项，以及它的十六进制输入框内容。
    pub palette_open: Option<widgets::ColorSlot>,
    pub palette_hex: String,
    pub forgot_custom: String,
    pub forgot_error: String,
}

fn spawn_worker(
    engine: Arc<Mutex<Engine>>,
    shared: Arc<Shared>,
    ctx: egui::Context,
    dir: PathBuf,
    show_signal: ShowSignal,
) {
    thread::spawn(move || {
        let mut last_second = 0;
        loop {
            thread::sleep(Duration::from_millis(250));
            let env = TickEnv {
                away: platform::session_locked(),
                quiet: platform::quiet_mode(),
            };
            shared.quiet.store(env.quiet, Ordering::Relaxed);
            let now = local_now();
            let (notices, cfg, data) = {
                let mut e = lock(&engine);
                let notices = e.tick(now, env);
                let cfg = std::mem::take(&mut e.cfg_dirty).then(|| e.cfg.clone());
                let data = std::mem::take(&mut e.data_dirty).then(|| e.data.clone());
                (notices, cfg, data)
            };
            let mut failure = None;
            if let Some(cfg) = cfg
                && let Err(err) = store::save_config(&dir, &cfg)
            {
                failure = Some(err.to_string());
            }
            if let Some(data) = data
                && let Err(err) = store::save_data(&dir, &data)
            {
                failure = Some(err.to_string());
            }
            if let Some(reason) = failure {
                *shared.save_error.lock().unwrap_or_else(PoisonError::into_inner) = Some(reason);
            }
            if notices.iter().any(|n| n.sound) {
                platform::play_chime();
            }
            for notice in notices.iter().filter(|n| n.toast) {
                platform::toast(&notice.title, &notice.body);
            }
            if show_signal.poll() {
                shared.show_requested.store(true, Ordering::Relaxed);
                ctx.request_repaint();
            }
            // 每秒唤醒一次界面。窗口隐藏时也靠这个继续处理托盘和弹窗。
            let second = now.and_utc().timestamp();
            if !notices.is_empty() || second != last_second {
                last_second = second;
                ctx.request_repaint();
            }
        }
    });
}

fn build_tray(tx: Sender<UiCmd>, ctx: egui::Context, cfg: &Config, on_duty: bool) -> Option<Tray> {
    let clock = MenuItem::with_id(MENU_CLOCK, clock_menu_text(on_duty), true, None);
    let floating = CheckMenuItem::with_id(MENU_FLOATING, "显示悬浮赚钱条", true, cfg.floating_bar, None);
    let hide = CheckMenuItem::with_id(
        MENU_HIDE,
        format!("老板键：隐藏金额\t{BOSS_KEY_LABEL}"),
        true,
        cfg.hide_money,
        None,
    );
    let menu = Menu::new();
    menu.append_items(&[
        &MenuItem::with_id(MENU_SHOW, "打开主界面", true, None),
        &clock,
        &PredefinedMenuItem::separator(),
        &floating,
        &hide,
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(MENU_QUIT, "退出", true, None),
    ])
    .ok()?;

    let image = tray_icon::Icon::from_rgba(icon::rgba(32), 32, 32).ok()?;
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("牛马上下班")
        .with_icon(image)
        .build()
        .ok()?;

    let menu_tx = tx.clone();
    let menu_ctx = ctx.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let cmd = match event.id.as_ref() {
            MENU_SHOW => UiCmd::Show,
            MENU_CLOCK => UiCmd::ToggleClock,
            MENU_FLOATING => UiCmd::ToggleFloating,
            MENU_HIDE => UiCmd::ToggleHideMoney,
            MENU_QUIT => UiCmd::Quit,
            _ => return,
        };
        let _ = menu_tx.send(cmd);
        menu_ctx.request_repaint();
    }));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            let _ = tx.send(UiCmd::Show);
            ctx.request_repaint();
        }
    }));

    Some(Tray {
        clock,
        floating,
        hide,
        last_tip: Instant::now() - Duration::from_secs(60),
        icon: tray_icon,
    })
}

fn clock_menu_text(on_duty: bool) -> &'static str {
    if on_duty { "下班打卡" } else { "上班打卡" }
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        engine: Arc<Mutex<Engine>>,
        dir: PathBuf,
        start_hidden: bool,
        preview_effect: bool,
        show_signal: ShowSignal,
    ) -> Self {
        let ctx = cc.egui_ctx.clone();
        let has_cjk_font = widgets::setup_fonts(&ctx);
        let shared = Arc::new(Shared::default());

        let (cfg, on_duty) = {
            let e = lock(&engine);
            (e.cfg.clone(), e.on_duty())
        };

        let (tx, rx) = channel();
        let tray = build_tray(tx.clone(), ctx.clone(), &cfg, on_duty);

        let boss_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyB);
        let hotkey = GlobalHotKeyManager::new().ok();
        let mut boss_key_error = None;
        if cfg.boss_key {
            boss_key_error = register_hotkey(hotkey.as_ref(), boss_hotkey);
        }
        let hotkey_ctx = ctx.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state() == HotKeyState::Pressed {
                let _ = tx.send(UiCmd::ToggleHideMoney);
                hotkey_ctx.request_repaint();
            }
        }));

        let icon_path = dir.join("icon.ico");
        if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&icon_path, icon::ico_file()).is_ok() {
            platform::register_toast_identity(&icon_path);
        }

        let logo = ctx.load_texture(
            "logo",
            egui::ColorImage::from_rgba_unmultiplied([128, 128], &icon::rgba(128)),
            egui::TextureOptions::LINEAR,
        );

        spawn_worker(engine.clone(), shared.clone(), ctx.clone(), dir.clone(), show_signal);

        let today = local_now().date();
        let mut app = Self {
            last_frame: Instant::now(),
            startup_hides: if start_hidden { 5 } else { 0 },
            children_visible: false,
            engine,
            shared,
            dir,
            page: Page::Today,
            hidden: start_hidden,
            palette: Palette::LIGHT,
            applied_palette: None,
            logo,
            effect: None,
            has_cjk_font,
            tray,
            hotkey,
            boss_hotkey,
            boss_key_error,
            rx,
            last_forgot_check: Instant::now() - Duration::from_secs(60),
            floating_placed: false,
            autostart: platform::autostart_enabled(),
            notice: None,
            loop_form: LoopForm::default(),
            med_form: MedForm::default(),
            session_edit: None,
            pending_delete: None,
            record_month: (today.year(), today.month()),
            ledger_month: (today.year(), today.month()),
            expense_form: ExpenseForm::default(),
            expense_undo: None,
            month_edit: None,
            palette_open: None,
            palette_hex: String::new(),
            forgot_custom: String::new(),
            forgot_error: String::new(),
        };
        if preview_effect {
            let engine = app.engine.clone();
            app.preview_effect(&lock(&engine));
        }
        app
    }

    pub fn set_notice(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), Instant::now()));
    }

    pub fn show_main(&mut self, ctx: &egui::Context) {
        self.hidden = false;
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn hide_main(&mut self, ctx: &egui::Context) {
        self.hidden = true;
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
    }

    /// 上班或下班。下班时结算并放特效。
    pub fn toggle_clock(&mut self, e: &mut Engine, now: NaiveDateTime) {
        if e.on_duty() {
            if let Some(summary) = e.clock_out(now) {
                self.celebrate(summary, e);
            }
        } else if e.clock_in(now) {
            if e.cfg.sound {
                platform::play_chime();
            }
            let started = e.data.loops.len();
            self.set_notice(format!("打卡成功，{started} 个循环事项开始计时"));
        }
    }

    /// 用今天的实时数据预览下班特效，不打卡。
    pub fn preview_effect(&mut self, e: &Engine) {
        let now = local_now();
        let stats = e.stats(now);
        let income = if stats.earned_exact > 0.0 { stats.earned_exact } else { 666.66 };
        let summary = DaySummary {
            date: now.date(),
            income,
            worked_secs: stats.worked_secs.max(9.0 * 3600.0),
            overtime_secs: stats.overtime_secs,
            done: stats.done,
            skipped: stats.skipped,
            hp: stats.hp,
            free_labor: false,
        };
        self.celebrate(summary, e);
    }

    pub fn celebrate(&mut self, summary: DaySummary, e: &Engine) {
        let money = widgets::money(&e.cfg, summary.income);
        if e.cfg.sound {
            platform::play_coins();
        }
        if e.cfg.effect_enabled {
            let quote = quotes::off_work(summary.date);
            self.effect = Some(Celebration::new(summary, quote, e.cfg.effect_seconds));
        } else {
            self.set_notice(format!("下班！今日收入 {money}"));
        }
    }

    fn sync_boss_key(&mut self, enabled: bool) {
        self.boss_key_error = None;
        if enabled {
            self.boss_key_error = register_hotkey(self.hotkey.as_ref(), self.boss_hotkey);
        } else if let Some(manager) = &self.hotkey {
            let _ = manager.unregister(self.boss_hotkey);
        }
    }

    pub fn set_boss_key(&mut self, e: &mut Engine, enabled: bool) {
        e.cfg.boss_key = enabled;
        e.cfg_dirty = true;
        self.sync_boss_key(enabled);
    }

    fn quit(&mut self) -> ! {
        {
            let e = lock(&self.engine);
            let _ = store::save_config(&self.dir, &e.cfg);
            let _ = store::save_data(&self.dir, &e.data);
        }
        // 先移除托盘图标，否则任务栏会残留一个幽灵图标。
        self.tray = None;
        std::process::exit(0);
    }

    fn handle_commands(&mut self, ctx: &egui::Context) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                UiCmd::Show => self.show_main(ctx),
                UiCmd::Quit => self.quit(),
                UiCmd::ToggleClock => {
                    let engine = self.engine.clone();
                    let mut e = lock(&engine);
                    self.toggle_clock(&mut e, local_now());
                }
                UiCmd::ToggleFloating => {
                    let mut e = lock(&self.engine);
                    e.cfg.floating_bar = !e.cfg.floating_bar;
                    e.cfg_dirty = true;
                }
                UiCmd::ToggleHideMoney => {
                    let mut e = lock(&self.engine);
                    e.cfg.hide_money = !e.cfg.hide_money;
                    e.cfg_dirty = true;
                }
            }
        }
    }

    fn update_tray(&mut self) {
        let Some(tray) = self.tray.as_mut() else {
            return;
        };
        let e = lock(&self.engine);
        tray.floating.set_checked(e.cfg.floating_bar);
        tray.hide.set_checked(e.cfg.hide_money);
        tray.clock.set_text(clock_menu_text(e.on_duty()));
        if tray.last_tip.elapsed() < Duration::from_secs(5) {
            return;
        }
        tray.last_tip = Instant::now();
        let now = local_now();
        let stats = e.stats(now);
        let money = widgets::money(&e.cfg, stats.earned_exact);
        let tip = match stats.to_end_secs {
            Some(left) if stats.on_duty && left > 0 => {
                format!("牛马上下班\n今日 {money} · 距下班 {}", widgets::clock_text(left))
            }
            _ if stats.on_duty => format!("牛马上下班\n今日 {money} · 已超时"),
            _ => format!("牛马上下班\n今日 {money} · 未在上班"),
        };
        let _ = tray.icon.set_tooltip(Some(tip));
    }

    fn resolve_palette(&mut self, ctx: &egui::Context, cfg: &Config) {
        let dark = match cfg.theme {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => ctx.input(|i| i.raw.system_theme) == Some(egui::Theme::Dark),
        };
        self.palette = if dark {
            Palette::DARK.with_overrides(&cfg.colors_dark)
        } else {
            Palette::LIGHT.with_overrides(&cfg.colors_light)
        };
        if self.applied_palette != Some(self.palette) {
            widgets::apply_theme(ctx, &self.palette);
            self.applied_palette = Some(self.palette);
        }
    }

    fn nav(&mut self, ui: &mut egui::Ui, e: &Engine) {
        let pal = self.palette;
        ui.add_space(14.0);
        ui.vertical_centered(|ui| {
            ui.add(egui::Image::new((self.logo.id(), egui::vec2(46.0, 46.0))));
        });
        ui.add_space(12.0);
        let alert_count = e.alerts().len();
        for page in Page::ALL {
            let selected = self.page == page;
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 42.0), Sense::click());
            let hover = resp.hovered();
            let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
            let painter = ui.painter();
            if selected {
                painter.rect_filled(rect.shrink2(egui::vec2(8.0, 2.0)), 10, pal.accent);
            } else if hover {
                painter.rect_filled(rect.shrink2(egui::vec2(8.0, 2.0)), 10, pal.sunken);
            }
            let color = if selected { pal.on_accent } else { pal.ink };
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                page.label(),
                egui::FontId::new(13.0, egui::FontFamily::Name(widgets::FAMILY_BOLD.into())),
                color,
            );
            if page == Page::Loops && alert_count > 0 {
                painter.circle_filled(rect.right_top() + egui::vec2(-16.0, 9.0), 4.0, pal.danger);
            }
            if resp.clicked() {
                self.page = page;
            }
        }
    }

    fn main_window(&mut self, ui: &mut egui::Ui, e: &mut Engine, now: NaiveDateTime) {
        let pal = self.palette;
        let stats = e.stats(now);
        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(92.0)
            .frame(Frame::new().fill(pal.card).stroke(Stroke::new(1.0, pal.line)))
            .show(ui, |ui| self.nav(ui, e));

        egui::CentralPanel::default()
            .frame(Frame::new().fill(pal.bg).inner_margin(Margin::same(14)))
            .show(ui, |ui| {
                if let Some(error) = self.shared.save_error.lock().unwrap_or_else(PoisonError::into_inner).clone() {
                    self.banner(ui, &format!("数据保存失败：{error}"), pal.danger);
                }
                if !self.has_cjk_font {
                    self.banner(ui, "No Chinese system font found (C:\\Windows\\Fonts).", pal.danger);
                }
                egui::ScrollArea::vertical()
                    .id_salt(self.page.label())
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.page {
                        Page::Today => self.page_today(ui, e, now, &stats),
                        Page::Loops => self.page_loops(ui, e, now),
                        Page::Health => self.page_health(ui, e, now),
                        Page::Records => self.page_records(ui, e, now),
                        Page::Ledger => self.page_ledger(ui, e, now),
                        Page::Settings => self.page_settings(ui, e),
                    });
            });

        self.show_notice(ui);
        self.modals(ui, e, now);
    }

    fn banner(&self, ui: &mut egui::Ui, text: &str, color: Color32) {
        Frame::new()
            .fill(self.palette.danger_soft)
            .corner_radius(10)
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(text).color(color));
            });
        ui.add_space(8.0);
    }

    fn show_notice(&mut self, ui: &mut egui::Ui) {
        let Some((text, at)) = self.notice.clone() else {
            return;
        };
        if at.elapsed() > Duration::from_secs(3) {
            self.notice = None;
            return;
        }
        let pal = self.palette;
        egui::Area::new(egui::Id::unique("notice"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(46.0, -18.0))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                Frame::new()
                    .fill(pal.hero)
                    .corner_radius(10)
                    .inner_margin(Margin::symmetric(14, 9))
                    .show(ui, |ui| {
                        ui.label(RichText::new(text).color(pal.hero_ink));
                    });
            });
    }

    /// 处理弹窗卡片的按钮结果。
    fn apply_alert_actions(&mut self, ctx: &egui::Context, e: &mut Engine, now: NaiveDateTime, actions: Vec<windows::AlertAction>) {
        for action in actions {
            match action {
                windows::AlertAction::Ack(kind, ack) => {
                    if kind == AlertKind::WorkEnd && ack == AckAction::Done {
                        e.ack(kind, ack, now);
                        self.toggle_clock(e, now);
                    } else {
                        e.ack(kind, ack, now);
                    }
                }
                windows::AlertAction::OpenHealth => {
                    e.ack(AlertKind::Sick, AckAction::Skip, now);
                    self.page = Page::Health;
                    self.show_main(ctx);
                }
            }
        }
    }

    fn child_windows(&mut self, ctx: &egui::Context, e: &mut Engine, now: NaiveDateTime) {
        let pal = self.palette;
        let quiet = self.shared.quiet.load(Ordering::Relaxed);

        // 三个窗口每帧都要提交，见 windows.rs 开头的说明。
        let alerts = e.alerts();
        let alerts_on = !quiet && !alerts.is_empty();
        let actions = windows::show_alerts(ctx, &alerts, &e.cfg, &pal, e.cfg.floating_bar, alerts_on);
        self.apply_alert_actions(ctx, e, now, actions);

        let floating_on = e.cfg.floating_bar && !e.cfg.hide_money && !quiet;
        let stats = e.stats(now);
        let pending = e.alerts().len();
        let result = windows::show_floating(ctx, &stats, &e.cfg, &pal, &self.logo, pending, floating_on, self.floating_placed);
        self.floating_placed |= result.placed;
        if result.open_main {
            self.show_main(ctx);
        }
        if let Some(pos) = result.moved_to {
            e.cfg.floating_pos = Some(pos);
            e.cfg_dirty = true;
        }

        let done = windows::show_celebration(ctx, self.effect.as_ref(), &e.cfg, &pal, &self.logo);
        if done || self.effect.as_ref().is_some_and(Celebration::finished) {
            self.effect = None;
        }
        self.children_visible = alerts_on || floating_on || self.effect.is_some();

        // 窗口在上面首次提交时才建出来；尺寸变了要重新裁剪，所以每帧检查。
        platform::clean_popup_frames(&windows::POPUP_TITLES);
    }

    /// 主窗口隐藏时是否还要跑界面帧。子窗口刚变成不需要时也要再跑一帧，把它藏起来。
    fn wants_children(&self) -> bool {
        let e = lock(&self.engine);
        self.children_visible || self.effect.is_some() || e.cfg.floating_bar || !e.alerts().is_empty()
    }
}

fn register_hotkey(manager: Option<&GlobalHotKeyManager>, hotkey: HotKey) -> Option<String> {
    let manager = manager?;
    manager
        .register(hotkey)
        .err()
        .map(|_| format!("{BOSS_KEY_LABEL} 已被其他程序占用，请用托盘菜单切换"))
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_commands(ctx);
        self.update_tray();

        if self.startup_hides > 0 {
            self.startup_hides -= 1;
            if self.hidden {
                ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                ctx.request_repaint_after(Duration::from_millis(50));
            }
        }

        if self.shared.show_requested.swap(false, Ordering::Relaxed) {
            self.show_main(ctx);
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.hide_main(ctx);
            let mut e = lock(&self.engine);
            if !e.cfg.tray_hint_shown {
                e.cfg.tray_hint_shown = true;
                e.cfg_dirty = true;
                platform::toast("牛马上下班还在托盘里", "提醒和赚钱表会继续运行。要退出请右键托盘图标。");
            }
        }

        // 疑似忘打卡要立刻让用户看到，哪怕窗口藏在托盘里。
        if self.hidden && self.last_forgot_check.elapsed() > Duration::from_secs(5) {
            self.last_forgot_check = Instant::now();
            let forgotten = lock(&self.engine).forgotten(local_now()).is_some();
            if forgotten {
                self.show_main(ctx);
            }
        }

        if self.hidden && self.wants_children() {
            ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 垂直同步关掉了（多个窗口会互相等待），这里自己把帧率限在约 60，
        // 否则动画和鼠标移动会让界面空转，吃掉大半个核。
        let since = self.last_frame.elapsed();
        if since < MIN_FRAME {
            thread::sleep(MIN_FRAME - since);
        }
        self.last_frame = Instant::now();
        let ctx = ui.ctx().clone();
        let engine = self.engine.clone();
        let mut guard = lock(&engine);
        let e = &mut *guard;
        let now = local_now();

        let cfg = e.cfg.clone();
        self.resolve_palette(&ctx, &cfg);
        if !self.hidden {
            self.main_window(ui, e, now);
        }
        self.child_windows(&ctx, e, now);

        let delay = if self.hidden { 1000 } else { 250 };
        ctx.request_repaint_after(Duration::from_millis(delay));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // 悬浮窗要透明。主窗口有不透明面板铺满，不受影响。
        [0.0, 0.0, 0.0, 0.0]
    }
}
