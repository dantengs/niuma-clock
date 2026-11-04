//! 主题、字体和通用小部件。
use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin,
    RichText, Sense, Stroke, TextStyle, Theme, Ui, Vec2,
};

use crate::model::{ColorOverrides, Config};

pub const FAMILY_BOLD: &str = "bold";
pub const FAMILY_NUM: &str = "num";

#[derive(Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub card: Color32,
    pub sunken: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub line: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub green: Color32,
    pub danger: Color32,
    pub danger_soft: Color32,
    pub hero: Color32,
    pub hero_ink: Color32,
    pub hero_muted: Color32,
    pub gold: Color32,
}

impl Palette {
    pub const LIGHT: Palette = Palette {
        dark: false,
        bg: Color32::from_rgb(0xF5, 0xEF, 0xE3),
        card: Color32::from_rgb(0xFF, 0xFC, 0xF6),
        sunken: Color32::from_rgb(0xEC, 0xE4, 0xD4),
        ink: Color32::from_rgb(0x2B, 0x26, 0x20),
        muted: Color32::from_rgb(0x7D, 0x73, 0x65),
        line: Color32::from_rgb(0xE0, 0xD6, 0xC4),
        accent: Color32::from_rgb(0xC9, 0x54, 0x20),
        on_accent: Color32::from_rgb(0xFF, 0xF8, 0xF0),
        green: Color32::from_rgb(0x2F, 0x7D, 0x55),
        danger: Color32::from_rgb(0xB9, 0x38, 0x2E),
        danger_soft: Color32::from_rgb(0xF8, 0xE1, 0xDB),
        hero: Color32::from_rgb(0x24, 0x20, 0x1B),
        hero_ink: Color32::from_rgb(0xF6, 0xEF, 0xE1),
        hero_muted: Color32::from_rgb(0xA8, 0x9E, 0x8E),
        gold: Color32::from_rgb(0xF4, 0xBC, 0x3A),
    };

    pub const DARK: Palette = Palette {
        dark: true,
        bg: Color32::from_rgb(0x18, 0x15, 0x12),
        card: Color32::from_rgb(0x25, 0x21, 0x1C),
        sunken: Color32::from_rgb(0x32, 0x2C, 0x25),
        ink: Color32::from_rgb(0xF2, 0xEB, 0xDD),
        muted: Color32::from_rgb(0xA3, 0x99, 0x89),
        line: Color32::from_rgb(0x3B, 0x34, 0x2C),
        accent: Color32::from_rgb(0xE8, 0x75, 0x35),
        on_accent: Color32::from_rgb(0x1E, 0x12, 0x08),
        green: Color32::from_rgb(0x57, 0xB5, 0x83),
        danger: Color32::from_rgb(0xE8, 0x6A, 0x5C),
        danger_soft: Color32::from_rgb(0x46, 0x25, 0x20),
        hero: Color32::from_rgb(0x0F, 0x0D, 0x0B),
        hero_ink: Color32::from_rgb(0xF6, 0xEF, 0xE1),
        hero_muted: Color32::from_rgb(0xA8, 0x9E, 0x8E),
        gold: Color32::from_rgb(0xF4, 0xBC, 0x3A),
    };

    /// 内置主题叠加调色板。改了底色时，文字和分隔线跟着换成看得清的颜色。
    /// 大部分文字在卡片上，所以文字颜色按卡片底色选；只改了窗口背景时，卡片跟着背景走。
    pub fn with_overrides(mut self, o: &ColorOverrides) -> Palette {
        let rgb = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
        if let Some(c) = o.bg {
            self.bg = rgb(c);
            if o.card.is_none() {
                let lift = if luminance(self.bg) > 140.0 { 0.6 } else { 0.06 };
                self.card = mix(self.bg, Color32::WHITE, lift);
            }
        }
        if let Some(c) = o.card {
            self.card = rgb(c);
        }
        if o.bg.is_some() || o.card.is_some() {
            let base = if luminance(self.card) > 140.0 { Palette::LIGHT } else { Palette::DARK };
            self.ink = base.ink;
            self.muted = base.muted;
            self.sunken = mix(self.card, self.ink, 0.07);
            self.line = mix(self.card, self.ink, 0.13);
            self.danger_soft = mix(self.card, base.danger, 0.15);
        }
        if let Some(c) = o.accent {
            self.accent = rgb(c);
            self.on_accent = self.on_accent_for(self.accent);
        }
        if let Some(c) = o.gold {
            self.gold = rgb(c);
        }
        if let Some(c) = o.hero {
            self.hero = rgb(c);
            self.hero_ink = self.on_accent_for(self.hero);
            self.hero_muted = mix(self.hero, self.hero_ink, 0.6);
        }
        self
    }

    /// 窗口背景和卡片底色一深一浅时，直接写在背景上的标题会看不清。
    pub fn bg_card_clash(&self) -> bool {
        (luminance(self.bg) > 140.0) != (luminance(self.card) > 140.0)
    }

    /// 内置主题在 slot 上的原色，给调色板显示“默认值”用。
    pub fn slot(&self, slot: ColorSlot) -> Color32 {
        match slot {
            ColorSlot::Accent => self.accent,
            ColorSlot::Gold => self.gold,
            ColorSlot::Hero => self.hero,
            ColorSlot::Bg => self.bg,
            ColorSlot::Card => self.card,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorSlot {
    Accent,
    Gold,
    Hero,
    Bg,
    Card,
}

impl ColorSlot {
    pub const ALL: [ColorSlot; 5] = [ColorSlot::Accent, ColorSlot::Gold, ColorSlot::Hero, ColorSlot::Bg, ColorSlot::Card];

    pub fn label(self) -> &'static str {
        match self {
            ColorSlot::Accent => "主色（按钮、选中）",
            ColorSlot::Gold => "金额色",
            ColorSlot::Hero => "赚钱卡片和悬浮条底色",
            ColorSlot::Bg => "窗口背景",
            ColorSlot::Card => "卡片底色",
        }
    }

    pub fn get(self, o: &ColorOverrides) -> Option<[u8; 3]> {
        match self {
            ColorSlot::Accent => o.accent,
            ColorSlot::Gold => o.gold,
            ColorSlot::Hero => o.hero,
            ColorSlot::Bg => o.bg,
            ColorSlot::Card => o.card,
        }
    }

    pub fn set(self, o: &mut ColorOverrides, value: Option<[u8; 3]>) {
        match self {
            ColorSlot::Accent => o.accent = value,
            ColorSlot::Gold => o.gold = value,
            ColorSlot::Hero => o.hero = value,
            ColorSlot::Bg => o.bg = value,
            ColorSlot::Card => o.card = value,
        }
    }
}

/// 一键配色：(名字, 浅色主题, 深色主题)。
pub fn presets() -> [(&'static str, ColorOverrides, ColorOverrides); 6] {
    let set = |accent: [u8; 3], hero: [u8; 3], bg: Option<[u8; 3]>, card: Option<[u8; 3]>| ColorOverrides {
        accent: Some(accent),
        gold: None,
        hero: Some(hero),
        bg,
        card,
    };
    [
        ("牛马橙", ColorOverrides::default(), ColorOverrides::default()),
        (
            "薄荷绿",
            set([0x2F, 0x8F, 0x6B], [0x16, 0x2B, 0x24], Some([0xEC, 0xF4, 0xEE]), Some([0xFA, 0xFD, 0xFB])),
            set([0x4C, 0xC2, 0x95], [0x0A, 0x14, 0x11], Some([0x12, 0x19, 0x16]), Some([0x1B, 0x25, 0x21])),
        ),
        (
            "海盐蓝",
            set([0x2F, 0x6F, 0xC9], [0x16, 0x22, 0x36], Some([0xEB, 0xF1, 0xF8]), Some([0xFA, 0xFC, 0xFF])),
            set([0x5B, 0x9B, 0xF0], [0x09, 0x0F, 0x1A], Some([0x11, 0x16, 0x1F]), Some([0x1A, 0x21, 0x2D])),
        ),
        (
            "樱花粉",
            set([0xC8, 0x4B, 0x7A], [0x35, 0x1A, 0x26], Some([0xF8, 0xEC, 0xF0]), Some([0xFF, 0xFA, 0xFC])),
            set([0xEE, 0x7A, 0xA6], [0x14, 0x09, 0x0E], Some([0x1C, 0x13, 0x17]), Some([0x28, 0x1C, 0x22])),
        ),
        (
            "葡萄紫",
            set([0x7A, 0x4F, 0xC2], [0x23, 0x1A, 0x36], Some([0xF1, 0xEE, 0xF8]), Some([0xFC, 0xFB, 0xFF])),
            set([0xA2, 0x80, 0xEE], [0x0E, 0x0A, 0x18], Some([0x16, 0x13, 0x1F]), Some([0x21, 0x1C, 0x2D])),
        ),
        (
            "极简灰",
            set([0x44, 0x4B, 0x55], [0x1F, 0x22, 0x26], Some([0xF0, 0xF1, 0xF2]), Some([0xFF, 0xFF, 0xFF])),
            set([0xC6, 0xCB, 0xD2], [0x0B, 0x0C, 0x0E], Some([0x16, 0x17, 0x19]), Some([0x21, 0x23, 0x26])),
        ),
    ]
}

fn luminance(c: Color32) -> f32 {
    0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

fn read_font(names: &[&str]) -> Option<Vec<u8>> {
    let dir = std::path::Path::new(r"C:\Windows\Fonts");
    names.iter().find_map(|name| std::fs::read(dir.join(name)).ok())
}

/// egui 默认字体没有中文，这里加载系统字体。全部读取失败时，界面只能显示西文。
pub fn setup_fonts(ctx: &egui::Context) -> bool {
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, key: &str, names: &[&str]| -> bool {
        match read_font(names) {
            Some(bytes) => {
                fonts
                    .font_data
                    .insert(key.to_owned(), Arc::new(FontData::from_owned(bytes)));
                true
            }
            None => false,
        }
    };
    let has_cjk = add(&mut fonts, "cjk", &["msyh.ttc", "msyh.ttf", "simhei.ttf", "simsun.ttc"]);
    let has_cjk_bold = add(&mut fonts, "cjk-bold", &["msyhbd.ttc", "msyhbd.ttf", "simhei.ttf"]);
    let has_mono = add(&mut fonts, "mono", &["consola.ttf"]);
    let has_num = add(&mut fonts, "mono-bold", &["consolab.ttf", "consola.ttf"]);

    let base = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    let mut proportional = base.clone();
    if has_cjk {
        proportional.insert(0, "cjk".to_owned());
    }
    fonts.families.insert(FontFamily::Proportional, proportional);

    let mut bold = base.clone();
    if has_cjk {
        bold.insert(0, "cjk".to_owned());
    }
    if has_cjk_bold {
        bold.insert(0, "cjk-bold".to_owned());
    }
    fonts.families.insert(FontFamily::Name(FAMILY_BOLD.into()), bold.clone());

    let mut num = bold;
    if has_num {
        num.insert(0, "mono-bold".to_owned());
    }
    fonts.families.insert(FontFamily::Name(FAMILY_NUM.into()), num);

    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        if has_cjk {
            mono.insert(0, "cjk".to_owned());
        }
        if has_mono {
            mono.insert(0, "mono".to_owned());
        }
    }
    ctx.set_fonts(fonts);
    has_cjk
}

pub fn apply_theme(ctx: &egui::Context, pal: &Palette) {
    let mut v = if pal.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = pal.bg;
    v.window_fill = pal.card;
    v.window_stroke = Stroke::new(1.0, pal.line);
    v.window_corner_radius = CornerRadius::same(14);
    v.extreme_bg_color = pal.sunken;
    v.faint_bg_color = pal.sunken;
    v.override_text_color = Some(pal.ink);
    v.selection.bg_fill = pal.accent;
    v.selection.stroke = Stroke::new(1.0, pal.on_accent);
    v.hyperlink_color = pal.accent;
    let radius = CornerRadius::same(8);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = radius;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, pal.line);
    v.widgets.inactive.bg_fill = pal.sunken;
    v.widgets.inactive.weak_bg_fill = pal.sunken;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, pal.ink);
    v.widgets.hovered.bg_fill = pal.line;
    v.widgets.hovered.weak_bg_fill = pal.line;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, pal.muted);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, pal.ink);
    v.widgets.active.bg_fill = pal.line;
    v.widgets.active.weak_bg_fill = pal.line;
    v.widgets.active.fg_stroke = Stroke::new(1.0, pal.ink);

    ctx.set_visuals_of(Theme::Light, v.clone());
    ctx.set_visuals_of(Theme::Dark, v);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.interact_size.y = 26.0;
        style.text_styles.insert(TextStyle::Body, FontId::proportional(14.0));
        style.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
        style.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
        style.text_styles.insert(TextStyle::Heading, FontId::new(19.0, FontFamily::Name(FAMILY_BOLD.into())));
        style.text_styles.insert(TextStyle::Monospace, FontId::monospace(13.0));
    });
}

pub fn bold(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into()).size(size).family(FontFamily::Name(FAMILY_BOLD.into()))
}

pub fn num(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into()).size(size).family(FontFamily::Name(FAMILY_NUM.into()))
}

pub fn card<R>(ui: &mut Ui, pal: &Palette, content: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(pal.card)
        .stroke(Stroke::new(1.0, pal.line))
        .corner_radius(14)
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            content(ui)
        })
        .inner
}

pub fn section(ui: &mut Ui, pal: &Palette, title: &str, hint: &str) {
    ui.label(bold(title, 15.0).color(pal.ink));
    if !hint.is_empty() {
        ui.label(RichText::new(hint).size(12.0).color(pal.muted));
    }
    ui.add_space(2.0);
}

/// 实心大按钮。
pub fn solid_button(ui: &mut Ui, text: &str, fill: Color32, ink: Color32, size: Vec2, text_size: f32) -> egui::Response {
    ui.add_sized(
        size,
        egui::Button::new(bold(text, text_size).color(ink))
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(10),
    )
}

/// 填满剩余宽度的主操作按钮。
pub fn wide_button(ui: &mut Ui, text: &str, fill: Color32, ink: Color32, height: f32, text_size: f32) -> egui::Response {
    let width = ui.available_width();
    solid_button(ui, text, fill, ink, Vec2::new(width, height), text_size)
}

pub fn chip(ui: &mut Ui, pal: &Palette, text: &str, selected: bool, on: Color32) -> egui::Response {
    let (fill, ink, stroke) = if selected {
        (on, pal.on_accent_for(on), Stroke::NONE)
    } else {
        (pal.card, pal.ink, Stroke::new(1.0, pal.line))
    };
    ui.add(
        egui::Button::new(RichText::new(text).size(13.0).color(ink))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(13),
    )
}

impl Palette {
    /// 给任意底色选一个看得清的文字色。
    pub fn on_accent_for(&self, fill: Color32) -> Color32 {
        if luminance(fill) > 140.0 {
            Color32::from_rgb(0x1E, 0x12, 0x08)
        } else {
            Color32::from_rgb(0xFF, 0xF8, 0xF0)
        }
    }
}

/// 圆角进度条。
pub fn bar(ui: &mut Ui, frac: f32, height: f32, track: Color32, fill: Color32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter();
    let radius = CornerRadius::same((height / 2.0) as u8);
    painter.rect_filled(rect, radius, track);
    let frac = frac.clamp(0.0, 1.0);
    if frac > 0.0 {
        let mut done = rect;
        done.set_width((rect.width() * frac).max(height));
        painter.rect_filled(done, radius, fill);
    }
}

pub fn clock_text(secs: i64) -> String {
    let secs = secs.max(0);
    format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}

pub fn short_clock(secs: i64) -> String {
    let secs = secs.max(0);
    if secs >= 3600 {
        format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
    } else {
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

pub fn duration_text(secs: f64) -> String {
    let total = secs.max(0.0) as i64 / 60;
    if total >= 60 {
        format!("{} 小时 {} 分", total / 60, total % 60)
    } else {
        format!("{total} 分钟")
    }
}

fn group_thousands(value: f64, decimals: usize) -> String {
    let text = format!("{:.*}", decimals, value.abs());
    let (int, frac) = text.split_once('.').map_or((text.as_str(), ""), |(i, f)| (i, f));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if !frac.is_empty() {
        out.push('.');
        out.push_str(frac);
    }
    if value < 0.0 { format!("-{out}") } else { out }
}

/// 带货币符号的金额。开了隐藏就打码。
pub fn money(cfg: &Config, value: f64) -> String {
    if cfg.hide_money {
        format!("{} ***.**", cfg.salary.currency)
    } else {
        format!("{}{}", cfg.salary.currency, group_thousands(value, 2))
    }
}

/// 拆成整数两位小数 + 后续两位，给按秒跳字时的小尾巴用。
pub fn money_parts(cfg: &Config, value: f64) -> (String, String) {
    if cfg.hide_money {
        return (format!("{} ***.**", cfg.salary.currency), String::new());
    }
    let text = format!("{value:.4}");
    let tail = text[text.len() - 2..].to_owned();
    let head = (value * 100.0).floor() / 100.0;
    (format!("{}{}", cfg.salary.currency, group_thousands(head, 2)), tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_grouping() {
        assert_eq!(group_thousands(1234567.891, 2), "1,234,567.89");
        assert_eq!(group_thousands(999.0, 2), "999.00");
        assert_eq!(group_thousands(1000.0, 2), "1,000.00");
        assert_eq!(group_thousands(-1234.5, 1), "-1,234.5");
    }

    #[test]
    fn money_is_masked_when_hidden() {
        let mut cfg = Config::default();
        assert_eq!(money(&cfg, 12.5), "¥12.50");
        cfg.hide_money = true;
        assert!(!money(&cfg, 12.5).contains("12"));
        assert!(!money_parts(&cfg, 12.5).0.contains("12"));
    }

    #[test]
    fn money_parts_do_not_round_up_head() {
        let cfg = Config::default();
        assert_eq!(money_parts(&cfg, 12.3456), ("¥12.34".to_owned(), "56".to_owned()));
        assert_eq!(money_parts(&cfg, 12.3999).0, "¥12.39");
    }

    #[test]
    fn overrides_keep_text_readable() {
        let none = Palette::LIGHT.with_overrides(&ColorOverrides::default());
        assert!(none == Palette::LIGHT, "不改颜色时和内置主题完全一致");

        let o = ColorOverrides {
            bg: Some([0x10, 0x10, 0x10]),
            accent: Some([0xFF, 0xE0, 0x40]),
            hero: Some([0xFA, 0xFA, 0xFA]),
            ..Default::default()
        };
        let p = Palette::LIGHT.with_overrides(&o);
        assert!(luminance(p.card) < 140.0, "只改背景时卡片跟着变深");
        assert_eq!(p.ink, Palette::DARK.ink, "浅色主题配深色背景时，文字换成浅色");
        assert!(!p.bg_card_clash());
        assert!(luminance(p.on_accent) < 100.0, "亮色按钮上用深色字");
        assert!(luminance(p.hero_ink) < 100.0, "浅色赚钱卡片上用深色字");
        assert!(p.sunken != p.card && p.line != p.card);

        let clash = Palette::LIGHT.with_overrides(&ColorOverrides {
            bg: Some([0x10, 0x10, 0x10]),
            card: Some([0xFF, 0xFF, 0xFF]),
            ..Default::default()
        });
        assert_eq!(clash.ink, Palette::LIGHT.ink, "文字按卡片底色选");
        assert!(clash.bg_card_clash());
    }

    #[test]
    fn presets_cover_both_themes() {
        for (name, light, dark) in presets().iter().skip(1) {
            assert!(light.accent.is_some() && dark.accent.is_some(), "{name}");
        }
        assert_eq!(presets()[0].1, ColorOverrides::default(), "第一个是默认配色");
    }

    #[test]
    fn durations_format() {
        assert_eq!(clock_text(3661), "01:01:01");
        assert_eq!(clock_text(-5), "00:00:00");
        assert_eq!(short_clock(125), "02:05");
        assert_eq!(short_clock(3725), "1:02:05");
        assert_eq!(duration_text(90.0 * 60.0), "1 小时 30 分");
        assert_eq!(duration_text(59.0), "0 分钟");
    }
}
