//! 主窗口之外的三个窗口：提醒卡片、悬浮赚钱条、下班特效。
//!
//! 三个窗口每帧都提交，不需要时只是隐藏。原因：eframe 在主窗口隐藏时
//! 不在事件循环里，那时新建窗口会失败并直接 panic（托盘模式下提醒一到点就崩）。
//! 所以窗口必须在启动首帧就建好，之后只切换可见性。
//!
//! 副作用：首帧时还不知道屏幕缩放比例，窗口按 100% 建出来，在 150% 缩放的屏幕上
//! 只有应有大小的三分之二，内容被裁掉、圆角露底。所以显示时要按实际缩放把尺寸和位置再设一次，
//! 见 `keep_geometry`。
use std::time::Instant;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Frame, Key, Margin, Pos2, Rect,
    RichText, Sense, Shape, Stroke, TextureHandle, Vec2, ViewportBuilder, ViewportCommand, ViewportId,
};

use crate::engine::{AckAction, Alert, AlertKind, DaySummary, TodayStats, hp_rank};
use crate::model::Config;
use crate::platform;
use crate::widgets::{self, FAMILY_BOLD, FAMILY_NUM, Palette, bold};

const ALERT_TITLE: &str = "牛马提醒";
const FLOATING_TITLE: &str = "牛马赚钱条";
const CELEBRATION_TITLE: &str = "下班啦";
/// 三个弹出窗口的标题，用来找到窗口去掉系统圆角。
pub const POPUP_TITLES: [&str; 3] = [ALERT_TITLE, FLOATING_TITLE, CELEBRATION_TITLE];

fn popup_builder(title: &str, size: Vec2, pos: Pos2, visible: bool) -> ViewportBuilder {
    ViewportBuilder::default()
        .with_visible(visible)
        .with_title(title)
        .with_inner_size(size)
        .with_position(pos)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_taskbar(false)
        .with_resizable(false)
        .with_active(false)
}

/// 让当前子窗口保持 size 大小、左上角在 pos（都是逻辑点）。已经对上时返回 true。
fn keep_geometry(ctx: &egui::Context, size: Vec2, pos: Option<Pos2>) -> bool {
    let (inner, outer) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect));
    let mut ok = true;
    if let Some(inner) = inner
        && (inner.size() - size).length() > 1.0
    {
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
        ok = false;
    }
    if let (Some(pos), Some(outer)) = (pos, outer)
        && (outer.min - pos).length() > 1.0
    {
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
        ok = false;
    }
    ok && inner.is_some() && outer.is_some()
}

// ---------- 提醒卡片 ----------

pub enum AlertAction {
    Ack(AlertKind, AckAction),
    OpenHealth,
}

const CARD_WIDTH: f32 = 348.0;
const CARD_ROW: f32 = 100.0;
const CARD_MAX_ROWS: usize = 4;

pub fn show_alerts(
    ctx: &egui::Context,
    alerts: &[Alert],
    cfg: &Config,
    pal: &Palette,
    floating: bool,
    visible: bool,
) -> Vec<AlertAction> {
    let visible = visible && !alerts.is_empty();
    let rows = alerts.len().clamp(1, CARD_MAX_ROWS);
    let more = alerts.len().saturating_sub(rows);
    let height = 20.0 + CARD_ROW * rows as f32 + if more > 0 { 24.0 } else { 0.0 };
    let [_, _, right, bottom] = platform::work_area_points();
    let lift = if floating && cfg.floating_pos.is_none() { 70.0 } else { 14.0 };
    let pos = Pos2::new(right - CARD_WIDTH - 14.0, bottom - height - lift);
    let builder = popup_builder(ALERT_TITLE, Vec2::new(CARD_WIDTH, height), pos, visible);

    ctx.show_viewport_immediate(ViewportId::from_hash_of("niuma-alerts"), builder, |ui, _| {
        let mut actions = Vec::new();
        if !visible {
            return actions;
        }
        keep_geometry(ui.ctx(), Vec2::new(CARD_WIDTH, height), Some(pos));
        Frame::new()
            .fill(pal.card)
            .stroke(Stroke::new(1.5, pal.accent))
            .corner_radius(14)
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(ui.available_height());
                for alert in alerts.iter().take(rows) {
                    alert_row(ui, alert, cfg, pal, &mut actions);
                }
                if more > 0 {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(format!("还有 {more} 条，处理完会接着显示")).size(12.0).color(pal.muted));
                    });
                }
            });
        actions
    })
}

fn alert_row(ui: &mut egui::Ui, alert: &Alert, cfg: &Config, pal: &Palette, out: &mut Vec<AlertAction>) {
    let snooze = format!("稍后 {} 分钟", cfg.snooze_minutes.max(1));
    ui.allocate_ui(Vec2::new(ui.available_width(), CARD_ROW - 8.0), |ui| {
        ui.set_min_height(CARD_ROW - 8.0);
        ui.label(bold(alert.title.as_str(), 16.0).color(pal.ink));
        if alert.body.is_empty() {
            ui.label(RichText::new("到时间了，照顾一下自己。").size(13.0).color(pal.muted));
        } else {
            ui.add(egui::Label::new(RichText::new(alert.body.as_str()).size(13.0).color(pal.muted)).truncate());
        }
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let size = Vec2::new(0.0, 28.0);
            let primary = |ui: &mut egui::Ui, text: &str| widgets::solid_button(ui, text, pal.accent, pal.on_accent, Vec2::new(92.0, size.y), 13.0).clicked();
            let plain = |ui: &mut egui::Ui, text: &str| ui.add(egui::Button::new(text).min_size(size)).clicked();
            match alert.kind {
                AlertKind::Loop(_) => {
                    if primary(ui, "搞定了") {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Done));
                    }
                    if plain(ui, &snooze) {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Snooze));
                    }
                    if plain(ui, "跳过") {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Skip));
                    }
                }
                AlertKind::Med(_) => {
                    if primary(ui, "吃过了") {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Done));
                    }
                    if plain(ui, &snooze) {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Snooze));
                    }
                    if plain(ui, "这次不吃") {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Skip));
                    }
                }
                AlertKind::WorkEnd => {
                    if primary(ui, "下班打卡") {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Done));
                    }
                    if plain(ui, "再卷半小时") {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Snooze));
                    }
                    if plain(ui, "知道了") {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Skip));
                    }
                }
                AlertKind::Sick => {
                    if primary(ui, "知道了") {
                        out.push(AlertAction::Ack(alert.kind, AckAction::Skip));
                    }
                    if cfg.meds.is_empty() && plain(ui, "设置吃药提醒") {
                        out.push(AlertAction::OpenHealth);
                    }
                }
            }
        });
    });
}

// ---------- 悬浮赚钱条 ----------

const BAR_SIZE: Vec2 = Vec2::new(286.0, 50.0);

/// 悬浮条该在的位置：用户拖过就用拖到的位置，否则放右下角。屏幕变小后跑出去的拉回来。
fn floating_target(cfg: &Config) -> Pos2 {
    let [left, top, right, bottom] = platform::work_area_points();
    let [x, y] = cfg
        .floating_pos
        .unwrap_or([right - BAR_SIZE.x - 14.0, bottom - BAR_SIZE.y - 12.0]);
    Pos2::new(
        x.clamp(left, (right - BAR_SIZE.x).max(left)),
        y.clamp(top, (bottom - BAR_SIZE.y).max(top)),
    )
}

#[derive(Default)]
pub struct FloatingResult {
    pub open_main: bool,
    pub moved_to: Option<[f32; 2]>,
    /// 尺寸和位置已经按实际缩放对好。
    pub placed: bool,
}

/// `placed` 为 false 时先把窗口摆到位；之后窗口位置以用户拖动为准。
#[allow(clippy::too_many_arguments)] // 都是一次性的显示参数，打包成结构体反而更绕
pub fn show_floating(
    ctx: &egui::Context,
    stats: &TodayStats,
    cfg: &Config,
    pal: &Palette,
    logo: &TextureHandle,
    pending: usize,
    visible: bool,
    placed: bool,
) -> FloatingResult {
    let target = floating_target(cfg);
    let builder = popup_builder(FLOATING_TITLE, BAR_SIZE, target, visible);
    ctx.show_viewport_immediate(ViewportId::from_hash_of("niuma-floating"), builder, |ui, _| {
        let mut result = FloatingResult::default();
        if !visible {
            return result;
        }
        if !placed {
            result.placed = keep_geometry(ui.ctx(), BAR_SIZE, Some(target));
            ui.ctx().request_repaint();
        } else {
            keep_geometry(ui.ctx(), BAR_SIZE, None);
            if let Some(outer) = ui.ctx().input(|i| i.viewport().outer_rect) {
                let now = [outer.min.x, outer.min.y];
                let saved = cfg.floating_pos.map_or(target, |[x, y]| Pos2::new(x, y));
                if (outer.min - saved).length() > 1.0 && outer.min.x > -10000.0 {
                    result.moved_to = Some(now);
                }
            }
        }

        let rect = ui.max_rect();
        let resp = ui.interact(rect, ui.scope_id().with("drag"), Sense::click_and_drag());
        // 按住移动超过 3 点就交给系统拖动（此时鼠标还按着，系统才接得住）；没移动就松开算单击。
        let press_id = egui::Id::unique("floating-press");
        let (pressed, down, pointer) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.latest_pos()));
        if pressed && resp.hovered() {
            ui.ctx().data_mut(|d| d.insert_temp(press_id, pointer));
        }
        let origin = ui.ctx().data(|d| d.get_temp::<Option<Pos2>>(press_id)).flatten();
        if !down {
            ui.ctx().data_mut(|d| d.remove::<Option<Pos2>>(press_id));
        } else if let (Some(origin), Some(pointer)) = (origin, pointer)
            && (pointer - origin).length() > 3.0
        {
            ui.ctx().data_mut(|d| d.remove::<Option<Pos2>>(press_id));
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if resp.clicked() {
            result.open_main = true;
        }
        resp.on_hover_text("单击打开主界面，按住可以拖动");

        let painter = ui.painter();
        painter.rect(
            rect.shrink(0.5),
            CornerRadius::same(14),
            pal.hero,
            Stroke::new(1.0, Color32::from_white_alpha(38)),
            egui::StrokeKind::Inside,
        );
        let logo_rect = Rect::from_center_size(rect.left_center() + Vec2::new(26.0, -1.0), Vec2::splat(32.0));
        painter.image(
            logo.id(),
            logo_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        painter.text(
            rect.left_center() + Vec2::new(50.0, -1.0),
            Align2::LEFT_CENTER,
            widgets::money(cfg, stats.earned_display),
            FontId::new(21.0, FontFamily::Name(FAMILY_NUM.into())),
            pal.gold,
        );
        let (top, bottom) = status_lines(stats, pending);
        let right = rect.right_center();
        painter.text(
            right + Vec2::new(-12.0, -9.0),
            Align2::RIGHT_CENTER,
            top,
            FontId::proportional(11.0),
            pal.hero_muted,
        );
        painter.text(
            right + Vec2::new(-12.0, 8.0),
            Align2::RIGHT_CENTER,
            bottom,
            FontId::new(14.0, FontFamily::Name(FAMILY_NUM.into())),
            if pending > 0 { Color32::from_rgb(0xF2, 0x8B, 0x6A) } else { pal.hero_ink },
        );
        if stats.on_duty {
            let track = Rect::from_min_size(rect.left_bottom() + Vec2::new(14.0, -6.0), Vec2::new(rect.width() - 28.0, 2.5));
            painter.rect_filled(track, 1, Color32::from_white_alpha(30));
            let mut done = track;
            done.set_width(track.width() * stats.progress);
            painter.rect_filled(done, 1, pal.gold);
        }
        result
    })
}

fn status_lines(stats: &TodayStats, pending: usize) -> (String, String) {
    if pending > 0 {
        return ("有提醒待处理".to_owned(), format!("{pending} 条"));
    }
    if !stats.on_duty {
        return ("今日收入".to_owned(), "未在上班".to_owned());
    }
    if let Some(until) = stats.lunch_until {
        return ("午休中".to_owned(), format!("{} 结束", until.format("%H:%M")));
    }
    match stats.to_end_secs {
        Some(left) if left > 0 => ("距下班".to_owned(), widgets::clock_text(left)),
        Some(left) => ("已超时".to_owned(), widgets::clock_text(-left)),
        None => ("已工作".to_owned(), widgets::clock_text(stats.worked_secs as i64)),
    }
}

// ---------- 下班特效 ----------

struct Coin {
    x: f32,
    delay: f32,
    speed: f32,
    size: f32,
    spin: f32,
    phase: f32,
    sway: f32,
}

struct Confetti {
    x: f32,
    delay: f32,
    speed: f32,
    size: f32,
    spin: f32,
    color: Color32,
}

pub struct Celebration {
    started: Instant,
    seconds: f32,
    summary: DaySummary,
    quote: &'static str,
    coins: Vec<Coin>,
    confetti: Vec<Confetti>,
}

/// 特效用的小随机数，不值得为此引入依赖。
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % 100_000) as f32 / 100_000.0
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next()
    }
}

impl Celebration {
    pub fn new(summary: DaySummary, quote: &'static str, seconds: f32) -> Self {
        let seed = (summary.income * 100.0) as u32 ^ 0x9E37_79B9;
        let mut rng = Rng(seed.max(1));
        let coins = (0..110)
            .map(|_| Coin {
                x: rng.next(),
                delay: rng.range(0.0, seconds * 0.55),
                speed: rng.range(0.22, 0.48),
                size: rng.range(14.0, 34.0),
                spin: rng.range(3.0, 9.0),
                phase: rng.range(0.0, std::f32::consts::TAU),
                sway: rng.range(4.0, 26.0),
            })
            .collect();
        let palette = [
            Color32::from_rgb(0xF4, 0xBC, 0x3A),
            Color32::from_rgb(0xE8, 0x75, 0x35),
            Color32::from_rgb(0x57, 0xB5, 0x83),
            Color32::from_rgb(0xF6, 0xEF, 0xE1),
            Color32::from_rgb(0xE8, 0x6A, 0x5C),
        ];
        let confetti = (0..90)
            .map(|i| Confetti {
                x: rng.next(),
                delay: rng.range(0.0, seconds * 0.4),
                speed: rng.range(0.25, 0.55),
                size: rng.range(5.0, 11.0),
                spin: rng.range(2.0, 7.0),
                color: palette[i % palette.len()],
            })
            .collect();
        Self {
            started: Instant::now(),
            seconds: seconds.clamp(2.0, 30.0),
            summary,
            quote,
            coins,
            confetti,
        }
    }

    pub fn finished(&self) -> bool {
        self.started.elapsed().as_secs_f32() >= self.seconds
    }
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

/// 返回 true 表示用户点掉了。
pub fn show_celebration(
    ctx: &egui::Context,
    fx: Option<&Celebration>,
    cfg: &Config,
    pal: &Palette,
    logo: &TextureHandle,
) -> bool {
    let [left, top, right, bottom] = platform::screen_points();
    // 高度少 1 像素：窗口恰好等于屏幕时，系统可能把它当独占全屏，透明会失效。
    let size = Vec2::new(right - left, bottom - top - 1.0);
    let builder = popup_builder(CELEBRATION_TITLE, size, Pos2::new(left, top), fx.is_some());

    ctx.show_viewport_immediate(ViewportId::from_hash_of("niuma-celebration"), builder, |ui, _| {
        let Some(fx) = fx else {
            return false;
        };
        keep_geometry(ui.ctx(), size, Some(Pos2::new(left, top)));
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        let t = fx.started.elapsed().as_secs_f32();
        if t < 0.3 {
            // 窗口是预先建好的隐藏窗口，显示时要主动抢焦点，Esc 才能关掉。
            ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
        }
        let rect = ui.max_rect();
        let resp = ui.interact(rect, ui.scope_id().with("close"), Sense::click());
        let closed = resp.clicked() || ui.ctx().input(|i| i.key_pressed(Key::Escape));

        // 整体淡入淡出。
        // 不做全屏半透明压暗：实测透明窗口里半透明像素会被直接当成透明，
        // 所以文字都放在不透明的卡片上，任何桌面背景下都看得清。
        let fade = (t / 0.35).min(1.0) * ((fx.seconds - t) / 0.7).clamp(0.0, 1.0);
        let painter = ui.painter();

        let h = rect.height();
        for c in &fx.confetti {
            let local = t - c.delay;
            if local <= 0.0 {
                continue;
            }
            let y = -20.0 + local * c.speed * h;
            if y > h + 20.0 {
                continue;
            }
            let x = rect.left() + c.x * rect.width() + (local * c.spin).sin() * 18.0;
            let flip = (local * c.spin).cos().abs().max(0.25);
            let piece = Rect::from_center_size(Pos2::new(x, rect.top() + y), Vec2::new(c.size, c.size * 0.55 * flip));
            painter.rect_filled(piece, 1, with_alpha(c.color, fade));
        }

        for coin in &fx.coins {
            let local = t - coin.delay;
            if local <= 0.0 {
                continue;
            }
            let y = -coin.size + local * coin.speed * h + 0.05 * h * local * local;
            if y > h + coin.size {
                continue;
            }
            let x = rect.left() + coin.x * rect.width() + (local * 1.7 + coin.phase).sin() * coin.sway;
            let center = Pos2::new(x, rect.top() + y);
            // 用椭圆的横向压缩假装硬币在翻转。
            let turn = (local * coin.spin + coin.phase).cos().abs().max(0.12);
            let radius = Vec2::new(coin.size * turn, coin.size);
            painter.add(Shape::ellipse_filled(center, radius, with_alpha(Color32::from_rgb(0xC9, 0x8A, 0x12), fade)));
            painter.add(Shape::ellipse_filled(center, radius * 0.82, with_alpha(pal.gold, fade)));
            if turn > 0.55 {
                painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    "¥",
                    FontId::new(coin.size * 1.05, FontFamily::Name(FAMILY_NUM.into())),
                    with_alpha(Color32::from_rgb(0x8A, 0x5A, 0x08), fade),
                );
            }
        }

        // 中央结算面板。
        let center = rect.center();
        let rise = (1.0 - ease_out(t / 0.6)) * 40.0;
        let shown = fx.summary.income * ease_out(t / 1.8) as f64;
        let amount = if cfg.hide_money {
            format!("{} ***.**", cfg.salary.currency)
        } else {
            widgets::money(cfg, shown)
        };
        let shadow = with_alpha(Color32::BLACK, 0.55 * fade);
        let panel = Rect::from_center_size(center + Vec2::new(0.0, 14.0 + rise), Vec2::new(840.0, 380.0));
        painter.rect(
            panel,
            CornerRadius::same(28),
            with_alpha(pal.hero, fade),
            Stroke::new(2.0, with_alpha(pal.gold, fade)),
            egui::StrokeKind::Inside,
        );
        let title_font = FontId::new(38.0, FontFamily::Name(FAMILY_BOLD.into()));
        let amount_font = FontId::new(104.0, FontFamily::Name(FAMILY_NUM.into()));
        let title_pos = center + Vec2::new(0.0, -128.0 + rise);
        painter.text(title_pos + Vec2::splat(2.0), Align2::CENTER_CENTER, "下班！今日窝囊费到账", title_font.clone(), shadow);
        painter.text(title_pos, Align2::CENTER_CENTER, "下班！今日窝囊费到账", title_font, with_alpha(pal.hero_ink, fade));
        let amount_pos = center + Vec2::new(0.0, -32.0 + rise);
        painter.text(amount_pos + Vec2::splat(4.0), Align2::CENTER_CENTER, &amount, amount_font.clone(), shadow);
        painter.text(amount_pos, Align2::CENTER_CENTER, &amount, amount_font, with_alpha(pal.gold, fade));

        let details = detail_line(&fx.summary);
        let detail_font = FontId::new(19.0, FontFamily::Proportional);
        let detail_pos = center + Vec2::new(0.0, 56.0 + rise);
        painter.text(detail_pos + Vec2::splat(1.0), Align2::CENTER_CENTER, &details, detail_font.clone(), shadow);
        painter.text(detail_pos, Align2::CENTER_CENTER, &details, detail_font, with_alpha(pal.hero_ink, fade));

        let rank = format!("牛马血条 {} · {}", fx.summary.hp, hp_rank(fx.summary.hp));
        let rank_font = FontId::new(21.0, FontFamily::Name(FAMILY_BOLD.into()));
        painter.text(center + Vec2::new(0.0, 92.0 + rise), Align2::CENTER_CENTER, rank, rank_font, with_alpha(pal.gold, fade));
        painter.text(
            center + Vec2::new(0.0, 130.0 + rise),
            Align2::CENTER_CENTER,
            fx.quote,
            FontId::new(18.0, FontFamily::Proportional),
            with_alpha(pal.hero_ink, 0.9 * fade),
        );

        // 牛马撒腿跑路。
        let run = ((t - 0.4) / (fx.seconds - 0.8)).clamp(0.0, 1.0);
        let cow_x = rect.left() - 90.0 + run * (rect.width() + 180.0);
        let hop = (t * 11.0).sin().abs() * 26.0;
        let cow_center = Pos2::new(cow_x, rect.bottom() - 90.0 - hop);
        let tag = Rect::from_center_size(cow_center + Vec2::new(0.0, -68.0), Vec2::new(112.0, 34.0));
        painter.rect_filled(tag, CornerRadius::same(17), with_alpha(pal.hero, fade));
        painter.image(
            logo.id(),
            Rect::from_center_size(cow_center, Vec2::splat(92.0)),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            with_alpha(Color32::WHITE, fade),
        );
        painter.text(
            cow_center + Vec2::new(0.0, -68.0),
            Align2::CENTER_CENTER,
            "溜了溜了",
            FontId::new(20.0, FontFamily::Name(FAMILY_BOLD.into())),
            with_alpha(pal.hero_ink, fade),
        );

        painter.text(
            panel.center_bottom() + Vec2::new(0.0, -22.0),
            Align2::CENTER_CENTER,
            "点击任意处或按 Esc 关闭",
            FontId::new(13.0, FontFamily::Proportional),
            with_alpha(pal.hero_muted, fade),
        );
        closed
    })
}

fn detail_line(s: &DaySummary) -> String {
    let mut parts = vec![
        format!("今日上班 {}", widgets::duration_text(s.worked_secs)),
        format!("完成提醒 {} 次", s.done),
    ];
    if s.skipped > 0 {
        parts.push(format!("跳过 {} 次", s.skipped));
    }
    if s.overtime_secs > 0.0 {
        let label = if s.free_labor { "白嫖" } else { "加班" };
        parts.push(format!("{label} {}", widgets::duration_text(s.overtime_secs)));
    }
    parts.join("  ·  ")
}
