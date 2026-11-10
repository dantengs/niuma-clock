//! 主窗口的五个页面。
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use eframe::egui::{self, Align, Frame, Layout, Margin, RichText, Sense, Stroke, Ui, Vec2};

use crate::app::{App, Page, local_now};
use crate::engine::{Engine, TodayStats, hp_rank};
use crate::model::*;
use crate::widgets::{self, ColorSlot, Palette, bar, bold, card, chip, num, section, wide_button};
use crate::{platform, quotes, salary};

pub struct LoopForm {
    pub title: String,
    pub note: String,
    pub minutes: u32,
    pub save_template: bool,
    pub auto_start: bool,
    pub editing: Option<u64>,
    pub error: String,
}

impl Default for LoopForm {
    fn default() -> Self {
        Self {
            title: String::new(),
            note: String::new(),
            minutes: 45,
            save_template: false,
            auto_start: true,
            editing: None,
            error: String::new(),
        }
    }
}

pub struct MedForm {
    pub name: String,
    pub dose: String,
    pub note: String,
    pub by_times: bool,
    pub minutes: u32,
    pub times: String,
    pub error: String,
}

impl Default for MedForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            dose: String::new(),
            note: String::new(),
            by_times: true,
            minutes: 240,
            times: "08:00, 12:30, 20:00".to_owned(),
            error: String::new(),
        }
    }
}

/// 修改“本月已赚”的弹窗。
pub struct MonthEdit {
    pub amount: f64,
}

pub struct ExpenseForm {
    pub amount: String,
    pub category: String,
    pub note: String,
    /// 空表示现在。
    pub when: String,
    pub error: String,
}

impl Default for ExpenseForm {
    fn default() -> Self {
        Self {
            amount: String::new(),
            category: EXPENSE_CATEGORIES[0].to_owned(),
            note: String::new(),
            when: String::new(),
            error: String::new(),
        }
    }
}

pub struct SessionEdit {
    pub index: usize,
    pub start: String,
    pub end: String,
    pub error: String,
}

const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M";

fn health_color(pal: &Palette, h: Health) -> egui::Color32 {
    match h {
        Health::Healthy => pal.green,
        Health::SubHealthy => pal.gold,
        Health::Unwell => pal.danger,
    }
}

fn health_chips(ui: &mut Ui, e: &mut Engine, pal: &Palette, now: NaiveDateTime) {
    ui.horizontal_wrapped(|ui| {
        for h in Health::ALL {
            let selected = e.data.health == h;
            if chip(ui, pal, h.label(), selected, health_color(pal, h)).clicked() {
                e.set_health(h, now);
            }
        }
    });
}

fn mood_chips(ui: &mut Ui, e: &mut Engine, pal: &Palette, now: NaiveDateTime) {
    ui.horizontal_wrapped(|ui| {
        for m in Mood::ALL {
            let selected = e.data.mood == m;
            if chip(ui, pal, m.label(), selected, pal.accent).clicked() {
                e.set_mood(m, now);
            }
        }
    });
}

/// 调色板里的常用颜色：前面是主色和金额色的候选，后面是深色和浅色底色。
const SWATCHES: [[u8; 3]; 26] = [
    [0xC9, 0x54, 0x20], [0xE8, 0x75, 0x35], [0xD9, 0x46, 0x2F], [0xE8, 0xA3, 0x3A],
    [0xF4, 0xBC, 0x3A], [0xF2, 0xD0, 0x4B], [0x2F, 0x7D, 0x55], [0x57, 0xB5, 0x83],
    [0x2F, 0x8F, 0x6B], [0x7C, 0xB3, 0x42], [0x2F, 0x6F, 0xC9], [0x5B, 0x9B, 0xF0],
    [0x1E, 0x88, 0xE5], [0x39, 0x49, 0xAB], [0x7A, 0x4F, 0xC2], [0xA2, 0x80, 0xEE],
    [0xC8, 0x4B, 0x7A], [0xEE, 0x7A, 0xA6], [0x24, 0x20, 0x1B], [0x16, 0x22, 0x36],
    [0x1F, 0x22, 0x26], [0x0F, 0x0D, 0x0B], [0xF5, 0xEF, 0xE3], [0xEB, 0xF1, 0xF8],
    [0xF0, 0xF1, 0xF2], [0xFF, 0xFF, 0xFF],
];

fn hex(c: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let text = text.trim().trim_start_matches('#');
    if text.len() != 6 || !text.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&text[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// 可点击的色块。选中时外圈描主色。
fn swatch(ui: &mut Ui, pal: &Palette, color: [u8; 3], size: Vec2, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let fill = egui::Color32::from_rgb(color[0], color[1], color[2]);
    let stroke = if selected {
        Stroke::new(2.0, pal.ink)
    } else {
        Stroke::new(1.0, if resp.hovered() { pal.muted } else { pal.line })
    };
    ui.painter().rect(rect, 6, fill, stroke, egui::StrokeKind::Outside);
    resp
}

fn signed_money(cfg: &Config, value: f64) -> String {
    let text = widgets::money(cfg, value.abs());
    if value < 0.0 { format!("-{text}") } else { format!("+{text}") }
}

/// 这笔钱要上多久的班才能赚回来。没设月薪时返回 None。
fn work_equivalent(cfg: &Config, amount: f64) -> Option<String> {
    let rate = salary::rate_per_sec(&cfg.salary);
    if rate <= 0.0 || amount <= 0.0 {
        return None;
    }
    let minutes = (amount / rate / 60.0).round() as i64;
    Some(match minutes {
        0 => "不到 1 分钟的班".to_owned(),
        1..=59 => format!("{minutes} 分钟的班"),
        _ if minutes % 60 == 0 => format!("{} 小时的班", minutes / 60),
        _ => format!("{} 小时 {} 分钟的班", minutes / 60, minutes % 60),
    })
}

/// 填的金额大约是几天的工资，帮用户核对有没有填错。
fn salary_days_hint(cfg: &Config, amount: f64) -> Option<String> {
    let daily = salary::rate_per_sec(&cfg.salary) * salary::standard_daily_secs(&cfg.salary);
    (daily > 0.0 && amount > 0.0).then(|| format!("约等于 {:.1} 天的日薪。", amount / daily))
}

fn weekday(date: NaiveDate) -> &'static str {
    ["周一", "周二", "周三", "周四", "周五", "周六", "周日"][date.weekday().num_days_from_monday() as usize]
}

impl App {
    // ---------- 今日 ----------

    pub fn page_today(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime, stats: &TodayStats) {
        let pal = self.palette;
        self.hero(ui, e, now, stats);
        ui.add_space(4.0);

        ui.columns(3, |cols| {
            card(&mut cols[0], &pal, |ui| {
                ui.label(RichText::new("牛马血条").size(12.0).color(pal.muted));
                ui.label(num(stats.hp.to_string(), 24.0).color(hp_color(&pal, stats.hp)));
                bar(ui, stats.hp as f32 / 100.0, 5.0, pal.sunken, hp_color(&pal, stats.hp));
                ui.label(RichText::new(hp_rank(stats.hp)).size(12.0).color(pal.muted));
            });
            card(&mut cols[1], &pal, |ui| {
                ui.label(RichText::new("距发薪日").size(12.0).color(pal.muted));
                let days = salary::days_to_payday(now.date(), e.cfg.salary.payday);
                if days == 0 {
                    ui.label(bold("今天！", 22.0).color(pal.accent));
                } else {
                    ui.horizontal(|ui| {
                        ui.label(num(days.to_string(), 24.0).color(pal.ink));
                        ui.label(RichText::new("天").size(12.0).color(pal.muted));
                    });
                }
                ui.label(RichText::new(format!("每月 {} 号", e.cfg.salary.payday)).size(12.0).color(pal.muted));
            });
            card(&mut cols[2], &pal, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("本月已赚").size(12.0).color(pal.muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let link = ui.link(RichText::new("修改").size(12.0).color(pal.accent));
                        if link.on_hover_text("工资不是从装软件那天开始算的？在这里补上之前已经赚到的").clicked() {
                            self.month_edit = Some(MonthEdit {
                                amount: (stats.month_earned * 100.0).round() / 100.0,
                            });
                        }
                    });
                });
                ui.add(egui::Label::new(num(widgets::money(&e.cfg, stats.month_earned), 19.0).color(pal.ink)).truncate());
                ui.add_space(3.0);
                let hint = if stats.month_adjust.abs() >= 0.005 && !e.cfg.hide_money {
                    format!("含手动补记 {}", signed_money(&e.cfg, stats.month_adjust))
                } else {
                    format!("今日完成提醒 {} 次", stats.done)
                };
                ui.label(RichText::new(hint).size(12.0).color(pal.muted));
            });
        });
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            section(ui, &pal, "今天状态如何", "");
            ui.horizontal(|ui| {
                ui.label(RichText::new("身体").size(13.0).color(pal.muted));
                health_chips(ui, e, &pal, now);
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("心情").size(13.0).color(pal.muted));
                mood_chips(ui, e, &pal, now);
            });
            if e.data.health == Health::Unwell {
                ui.add_space(2.0);
                let label = if e.cfg.meds.is_empty() {
                    "还没有吃药提醒，点这里设置"
                } else {
                    "查看吃药提醒"
                };
                if ui.link(RichText::new(label).color(pal.danger)).clicked() {
                    self.page = Page::Health;
                }
            }
        });
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            ui.horizontal(|ui| {
                section(ui, &pal, "运行中的循环", "");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("管理").clicked() {
                        self.page = Page::Loops;
                    }
                });
            });
            if e.data.loops.is_empty() {
                let text = if stats.on_duty {
                    "当前没有循环事项，去添加一个吧。"
                } else {
                    "上班打卡后，常用事项会自动开始计时。"
                };
                ui.label(RichText::new(text).size(13.0).color(pal.muted));
            }
            for l in &e.data.loops {
                loop_progress(ui, l, now, &pal);
            }
        });
        ui.add_space(6.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(quotes::daily(now.date(), e.data.mood)).size(13.0).italics().color(pal.muted));
        });
        ui.add_space(6.0);
    }

    fn hero(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime, stats: &TodayStats) {
        let pal = self.palette;
        Frame::new()
            .fill(pal.hero)
            .corner_radius(16)
            .inner_margin(Margin::same(18))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("今日已赚 · 窝囊费实时到账").size(13.0).color(pal.hero_muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = if e.cfg.hide_money { "显示金额" } else { "隐藏金额" };
                        let button = egui::Button::new(RichText::new(label).size(12.0).color(pal.hero_ink))
                            .fill(egui::Color32::from_white_alpha(22))
                            .stroke(Stroke::NONE)
                            .corner_radius(8);
                        if ui.add(button).clicked() {
                            e.cfg.hide_money = !e.cfg.hide_money;
                            e.cfg_dirty = true;
                        }
                        if stats.on_duty || stats.lunch_until.is_some() {
                            let label = if stats.lunch_until.is_some() { "结束午休" } else { "开始午休" };
                            let button = egui::Button::new(RichText::new(label).size(12.0).color(pal.hero_ink))
                                .fill(egui::Color32::from_white_alpha(22))
                                .stroke(Stroke::NONE)
                                .corner_radius(8);
                            let hint = "午休期间不弹任何提醒，循环倒计时暂停；吃药提醒等午休结束再响";
                            if ui.add(button).on_hover_text(hint).clicked() {
                                if stats.lunch_until.is_some() {
                                    e.end_lunch(now);
                                } else {
                                    e.start_lunch(now);
                                }
                            }
                        }
                    });
                });

                if e.cfg.salary.monthly <= 0.0 {
                    ui.add_space(6.0);
                    ui.label(bold("先告诉我你月薪多少", 26.0).color(pal.gold));
                    ui.label(RichText::new("数据只存在这台电脑上，不联网、不上传。").size(13.0).color(pal.hero_muted));
                    ui.add_space(4.0);
                    if widgets::solid_button(ui, "去设置月薪", pal.gold, egui::Color32::from_rgb(0x2B, 0x20, 0x08), Vec2::new(120.0, 30.0), 14.0).clicked() {
                        self.page = Page::Settings;
                    }
                } else {
                    let s = &e.cfg.salary;
                    // 不能用 with_layout(Align::Max)：它会占满剩余高度，把下面的内容挤出卡片。
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if s.tick_unit == TickUnit::Second {
                            let (head, tail) = widgets::money_parts(&e.cfg, stats.earned_display);
                            ui.label(num(head, 46.0).color(pal.gold));
                            // 小尾巴对齐到大数字的底部。
                            ui.vertical(|ui| {
                                ui.add_space(24.0);
                                ui.label(num(tail, 22.0).color(pal.hero_muted));
                            });
                        } else {
                            ui.label(num(widgets::money(&e.cfg, stats.earned_display), 46.0).color(pal.gold));
                        }
                    });
                    let rate = if e.cfg.hide_money {
                        format!("{} +***", s.tick_unit.label())
                    } else {
                        format!("{} +{}{:.4}", s.tick_unit.label(), s.currency, salary::tick_amount(s))
                    };
                    ui.label(RichText::new(rate).size(13.0).color(pal.hero_muted));
                }

                ui.add_space(8.0);
                bar(ui, stats.progress, 8.0, egui::Color32::from_white_alpha(28), pal.gold);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("今日进度 {:.0}%", stats.progress * 100.0)).size(12.0).color(pal.hero_muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(countdown_text(stats, &e.cfg).color(pal.hero_ink));
                    });
                });
                ui.add_space(4.0);
                let (text, fill, ink) = if stats.on_duty {
                    ("下班打卡", pal.accent, pal.on_accent)
                } else {
                    ("上班打卡", Palette::LIGHT.green, egui::Color32::WHITE)
                };
                if wide_button(ui, text, fill, ink, 50.0, 20.0).clicked() {
                    self.toggle_clock(e, now);
                }
                let sub = match stats.session_start {
                    _ if stats.lunch_until.is_some() => format!(
                        "午休中，{} 结束 · 期间不弹提醒，循环倒计时已暂停",
                        stats.lunch_until.unwrap_or(now).format("%H:%M")
                    ),
                    Some(start) => format!("{} 上班 · 已工作 {}", start.format("%H:%M"), widgets::duration_text(stats.worked_secs)),
                    None if stats.worked_secs > 0.0 => format!("今天已工作 {}，可以再次打卡继续", widgets::duration_text(stats.worked_secs)),
                    None => "今天还没打卡".to_owned(),
                };
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(sub).size(12.0).color(pal.hero_muted));
                });
            });
    }

    // ---------- 循环事项 ----------

    pub fn page_loops(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime) {
        let pal = self.palette;
        ui.heading("循环事项");
        ui.label(RichText::new("记事本 + 倒计时闹钟。到点弹卡片提醒，确认后进入下一轮。下班打卡时，运行中的循环全部清除；常用事项下次上班自动恢复。").size(13.0).color(pal.muted));
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            section(ui, &pal, "运行中", "");
            if e.data.loops.is_empty() {
                let text = if e.on_duty() { "现在没有运行中的循环。" } else { "还没上班，循环不计时。" };
                ui.label(RichText::new(text).size(13.0).color(pal.muted));
            }
            let mut pause = None;
            let mut restart = None;
            let mut remove = None;
            for l in &e.data.loops {
                loop_progress(ui, l, now, &pal);
                if !l.note.is_empty() {
                    ui.label(RichText::new(l.note.as_str()).size(12.0).color(pal.muted));
                }
                ui.horizontal(|ui| {
                    let label = if l.paused_left.is_some() { "继续" } else { "暂停" };
                    if ui.small_button(label).clicked() {
                        pause = Some(l.id);
                    }
                    if ui.small_button("重新计时").clicked() {
                        restart = Some(l.id);
                    }
                    if ui.small_button("移除").clicked() {
                        remove = Some(l.id);
                    }
                    let kind = if l.template_id.is_some() { "常用" } else { "今日临时" };
                    ui.label(RichText::new(format!("{kind} · 每 {} 分钟 · 完成 {} 次 · 跳过 {} 次", l.interval_min, l.done, l.skipped)).size(12.0).color(pal.muted));
                });
                ui.add_space(4.0);
            }
            if let Some(id) = pause {
                e.toggle_pause(id, now);
            }
            if let Some(id) = restart {
                e.restart_loop(id, now);
            }
            if let Some(id) = remove {
                e.remove_loop(id);
            }
        });
        ui.add_space(4.0);

        self.loop_form_card(ui, e, now);
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            section(ui, &pal, "常用事项", "设为自动的，每天上班打卡时自动开始。");
            if e.cfg.reminders.is_empty() {
                ui.label(RichText::new("还没有常用事项。").size(13.0).color(pal.muted));
            }
            let mut start = None;
            let mut edit = None;
            let mut delete = None;
            let mut toggled = None;
            for tpl in &e.cfg.reminders {
                let running = e.data.loops.iter().any(|l| l.template_id == Some(tpl.id));
                ui.horizontal(|ui| {
                    ui.label(bold(tpl.title.as_str(), 14.0).color(pal.ink));
                    ui.label(RichText::new(format!("每 {} 分钟", tpl.interval_min)).size(12.0).color(pal.muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("删除").clicked() {
                            delete = Some(tpl.id);
                        }
                        if ui.small_button("编辑").clicked() {
                            edit = Some(tpl.clone());
                        }
                        if !running && e.on_duty() && ui.small_button("今天开始").clicked() {
                            start = Some(tpl.id);
                        }
                        let mut auto = tpl.auto_start;
                        if ui.checkbox(&mut auto, "自动").changed() {
                            toggled = Some((tpl.clone(), auto));
                        }
                    });
                });
            }
            if let Some(id) = start {
                e.start_template(id, now);
            }
            if let Some(tpl) = edit {
                self.loop_form = LoopForm {
                    title: tpl.title,
                    note: tpl.note,
                    minutes: tpl.interval_min,
                    save_template: true,
                    auto_start: tpl.auto_start,
                    editing: Some(tpl.id),
                    error: String::new(),
                };
            }
            if let Some(id) = delete {
                e.remove_template(id);
            }
            if let Some((mut tpl, auto)) = toggled {
                tpl.auto_start = auto;
                e.update_template(tpl, now);
            }
        });
    }

    fn loop_form_card(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime) {
        let pal = self.palette;
        let editing = self.loop_form.editing;
        card(ui, &pal, |ui| {
            section(ui, &pal, if editing.is_some() { "编辑常用事项" } else { "添加事项" }, "");
            ui.add(egui::TextEdit::singleline(&mut self.loop_form.title).hint_text("事项，比如：喝水、拉伸、给客户回电").desired_width(f32::INFINITY));
            ui.add(egui::TextEdit::multiline(&mut self.loop_form.note).hint_text("备注（可选），到点时显示在提醒卡片上").desired_rows(2).desired_width(f32::INFINITY));
            ui.horizontal_wrapped(|ui| {
                ui.label("每隔");
                ui.add(egui::DragValue::new(&mut self.loop_form.minutes).range(1..=1440).suffix(" 分钟"));
                for m in [20, 30, 45, 60, 90] {
                    if chip(ui, &pal, &m.to_string(), self.loop_form.minutes == m, pal.accent).clicked() {
                        self.loop_form.minutes = m;
                    }
                }
            });
            if editing.is_none() {
                ui.checkbox(&mut self.loop_form.save_template, "存为常用事项，下班后保留");
            }
            if self.loop_form.save_template || editing.is_some() {
                ui.checkbox(&mut self.loop_form.auto_start, "每天上班打卡时自动开始");
            }
            if !self.loop_form.error.is_empty() {
                ui.label(RichText::new(self.loop_form.error.as_str()).size(12.0).color(pal.danger));
            }
            ui.horizontal(|ui| {
                let label = if editing.is_some() { "保存修改" } else { "添加" };
                if widgets::solid_button(ui, label, pal.accent, pal.on_accent, Vec2::new(96.0, 32.0), 14.0).clicked() {
                    self.submit_loop_form(e, now);
                }
                if editing.is_some() && ui.button("取消").clicked() {
                    self.loop_form = LoopForm::default();
                }
            });
        });
    }

    fn submit_loop_form(&mut self, e: &mut Engine, now: NaiveDateTime) {
        let form = &mut self.loop_form;
        let title = form.title.trim().to_owned();
        if title.is_empty() {
            form.error = "先写一下要提醒什么事。".to_owned();
            return;
        }
        let note = form.note.trim().to_owned();
        let minutes = form.minutes.max(1);
        if let Some(id) = form.editing {
            e.update_template(
                ReminderTemplate {
                    id,
                    title,
                    note,
                    interval_min: minutes,
                    auto_start: form.auto_start,
                },
                now,
            );
        } else if form.save_template {
            let id = e.add_template(title, note, minutes, form.auto_start);
            if e.on_duty() {
                e.start_template(id, now);
            }
        } else if !e.add_temp_loop(title, note, minutes, now) {
            form.error = "还没上班。临时事项要上班后才能开始，或者勾选存为常用事项。".to_owned();
            return;
        }
        self.loop_form = LoopForm::default();
        self.set_notice("已保存");
    }

    // ---------- 身心状态 ----------

    pub fn page_health(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime) {
        let pal = self.palette;
        ui.heading("身心状态");
        ui.add_space(4.0);
        card(ui, &pal, |ui| {
            section(ui, &pal, "身体状况", "设为不健康后，会按下面的方案提醒吃药。上下班都有效，恢复健康后自动停止。");
            health_chips(ui, e, &pal, now);
            ui.add_space(6.0);
            section(ui, &pal, "心情", "");
            mood_chips(ui, e, &pal, now);
            if e.data.mood == Mood::Broken {
                ui.add_space(4.0);
                ui.label(RichText::new("辛苦了。如果这种感觉持续很久，找信任的人或专业人士聊聊会有帮助。").size(13.0).color(pal.danger));
            }
        });
        ui.add_space(4.0);

        if e.data.health == Health::Unwell {
            Frame::new()
                .fill(pal.danger_soft)
                .corner_radius(14)
                .inner_margin(Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(bold("正在提醒吃药", 15.0).color(pal.danger));
                    if e.data.meds.is_empty() {
                        ui.label(RichText::new("还没有吃药方案，在下面添加一个。").size(13.0).color(pal.ink));
                    }
                    for med in &e.data.meds {
                        let Some(plan) = e.cfg.meds.iter().find(|p| p.id == med.plan_id) else {
                            continue;
                        };
                        let when = if med.pending_since.is_some() {
                            "到点了，等你确认".to_owned()
                        } else if med.next_fire.date() == now.date() {
                            format!("下一次 {}", med.next_fire.format("%H:%M"))
                        } else {
                            format!("下一次 {}", med.next_fire.format("%m-%d %H:%M"))
                        };
                        ui.label(RichText::new(format!("{}：{when}", plan.name)).size(13.0).color(pal.ink));
                    }
                });
            ui.add_space(4.0);
        }

        self.med_form_card(ui, e, now);
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            section(ui, &pal, "最近的状态变化", "");
            if e.data.status_log.is_empty() {
                ui.label(RichText::new("还没有记录。").size(13.0).color(pal.muted));
            }
            for entry in e.data.status_log.iter().rev().take(8) {
                let text = match entry.change {
                    StatusChange::Health(h) => format!("身体：{}", h.label()),
                    StatusChange::Mood(m) => format!("心情：{}", m.label()),
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(entry.at.format("%m-%d %H:%M").to_string()).size(12.0).color(pal.muted));
                    ui.label(RichText::new(text).size(13.0));
                });
            }
        });
    }

    fn med_form_card(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime) {
        let pal = self.palette;
        card(ui, &pal, |ui| {
            section(ui, &pal, "吃药方案", "这里只负责提醒，用药请遵医嘱。");
            let mut remove = None;
            for plan in &e.cfg.meds {
                ui.horizontal(|ui| {
                    ui.label(bold(plan.name.as_str(), 14.0).color(pal.ink));
                    let schedule = match &plan.schedule {
                        MedSchedule::Interval { minutes } => format!("每 {minutes} 分钟"),
                        MedSchedule::Times { times } => times.iter().map(|t| t.format("%H:%M").to_string()).collect::<Vec<_>>().join("、"),
                    };
                    let detail = if plan.dose.is_empty() { schedule } else { format!("{} · {schedule}", plan.dose) };
                    ui.label(RichText::new(detail).size(12.0).color(pal.muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("删除").clicked() {
                            remove = Some(plan.id);
                        }
                    });
                });
            }
            if let Some(id) = remove {
                e.remove_med(id);
            }
            ui.separator();
            let form = &mut self.med_form;
            ui.add(egui::TextEdit::singleline(&mut form.name).hint_text("药名").desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut form.dose).hint_text("用量，如 1 粒").desired_width(120.0));
                ui.add(egui::TextEdit::singleline(&mut form.note).hint_text("备注，如 饭后").desired_width(f32::INFINITY));
            });
            ui.horizontal(|ui| {
                ui.radio_value(&mut form.by_times, true, "固定时间");
                ui.radio_value(&mut form.by_times, false, "固定间隔");
            });
            if form.by_times {
                ui.add(egui::TextEdit::singleline(&mut form.times).hint_text("08:00, 12:30, 20:00").desired_width(f32::INFINITY));
            } else {
                ui.horizontal(|ui| {
                    ui.label("每隔");
                    ui.add(egui::DragValue::new(&mut form.minutes).range(1..=2880).suffix(" 分钟"));
                });
            }
            if !form.error.is_empty() {
                ui.label(RichText::new(form.error.as_str()).size(12.0).color(pal.danger));
            }
            if widgets::solid_button(ui, "添加方案", pal.accent, pal.on_accent, Vec2::new(96.0, 32.0), 14.0).clicked() {
                let name = form.name.trim().to_owned();
                let schedule = if form.by_times {
                    salary::parse_times(&form.times).map(|times| MedSchedule::Times { times })
                } else {
                    Some(MedSchedule::Interval { minutes: form.minutes.max(1) })
                };
                if name.is_empty() {
                    form.error = "先写一下药名。".to_owned();
                } else if let Some(schedule) = schedule {
                    e.add_med(name, form.dose.trim().to_owned(), form.note.trim().to_owned(), schedule, now);
                    *form = MedForm::default();
                } else {
                    form.error = "时间格式不对，例子：08:00, 12:30, 20:00".to_owned();
                }
            }
        });
    }

    // ---------- 打卡记录 ----------

    pub fn page_records(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime) {
        let pal = self.palette;
        let (year, month) = self.record_month;
        ui.horizontal(|ui| {
            ui.heading("打卡记录");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("导出 CSV").clicked() {
                    self.export_csv("牛马上下班记录", e.to_csv());
                }
            });
        });
        ui.horizontal(|ui| {
            if ui.button("<").clicked() {
                self.record_month = if month == 1 { (year - 1, 12) } else { (year, month - 1) };
            }
            ui.label(bold(format!("{year} 年 {month} 月"), 15.0).color(pal.ink));
            let is_current = (year, month) >= (now.year(), now.month());
            if ui.add_enabled(!is_current, egui::Button::new(">")).clicked() {
                self.record_month = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
            }
        });
        let (year, month) = self.record_month;
        let summary = e.month_summary(year, month);

        ui.columns(3, |cols| {
            card(&mut cols[0], &pal, |ui| {
                ui.label(RichText::new("出勤").size(12.0).color(pal.muted));
                ui.label(num(format!("{} 天", summary.days), 20.0).color(pal.ink));
            });
            card(&mut cols[1], &pal, |ui| {
                ui.label(RichText::new("总工时").size(12.0).color(pal.muted));
                ui.label(num(format!("{:.1} h", summary.worked_secs / 3600.0), 20.0).color(pal.ink));
            });
            card(&mut cols[2], &pal, |ui| {
                ui.label(RichText::new("收入").size(12.0).color(pal.muted));
                ui.add(egui::Label::new(num(widgets::money(&e.cfg, summary.income), 20.0).color(pal.ink)).truncate());
                if summary.adjust.abs() >= 0.005 && !e.cfg.hide_money {
                    ui.label(RichText::new(format!("含手动补记 {}", signed_money(&e.cfg, summary.adjust))).size(12.0).color(pal.muted));
                }
            });
        });
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            let overtime = summary.overtime_secs / 3600.0;
            section(ui, &pal, "每日工时", &format!("其中加班 {overtime:.1} 小时，橙色部分。"));
            month_chart(ui, e, year, month, &pal);
        });
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            section(ui, &pal, "明细", "可以补卡或修正。修改后按当前工资设置重新计算。");
            let mut rows: Vec<usize> = e
                .data
                .sessions
                .iter()
                .enumerate()
                .filter(|(_, s)| s.start.year() == year && s.start.month() == month)
                .map(|(i, _)| i)
                .collect();
            rows.reverse();
            if rows.is_empty() {
                ui.label(RichText::new("这个月还没有打卡记录。").size(13.0).color(pal.muted));
            }
            for index in rows {
                let s = &e.data.sessions[index];
                let date = s.start.date();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} {}", date.format("%m-%d"), weekday(date))).size(13.0).color(pal.ink));
                    match s.end {
                        Some(end) => {
                            let end_text = if end.date() == date { end.format("%H:%M").to_string() } else { end.format("%m-%d %H:%M").to_string() };
                            ui.label(RichText::new(format!("{} → {end_text}", s.start.format("%H:%M"))).size(13.0).color(pal.muted));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("删除").clicked() {
                                    self.pending_delete = Some(index);
                                }
                                if ui.small_button("修改").clicked() {
                                    self.session_edit = Some(SessionEdit {
                                        index,
                                        start: s.start.format(DATETIME_FORMAT).to_string(),
                                        end: end.format(DATETIME_FORMAT).to_string(),
                                        error: String::new(),
                                    });
                                }
                                ui.label(num(widgets::money(&e.cfg, s.income), 13.0).color(pal.accent));
                            });
                        }
                        None => {
                            ui.label(RichText::new(format!("{} 上班中", s.start.format("%H:%M"))).size(13.0).color(pal.green));
                        }
                    }
                });
            }
        });
    }

    fn export_csv(&mut self, prefix: &str, csv: String) {
        let dir = std::env::var_os("USERPROFILE")
            .map(|home| std::path::PathBuf::from(home).join("Documents"))
            .filter(|p| p.is_dir())
            .unwrap_or_else(|| self.dir.clone());
        let name = format!("{prefix}_{}.csv", local_now().format("%Y%m%d_%H%M%S"));
        let path = dir.join(name);
        // 加 BOM，Excel 才能认出 UTF-8 中文。
        let text = format!("\u{feff}{csv}");
        match std::fs::write(&path, text) {
            Ok(()) => {
                platform::reveal_in_explorer(&path);
                self.set_notice(format!("已导出到 {}", path.display()));
            }
            Err(err) => self.set_notice(format!("导出失败：{err}")),
        }
    }

    // ---------- 记账 ----------

    pub fn page_ledger(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime) {
        let pal = self.palette;
        ui.horizontal(|ui| {
            ui.heading("记账");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("导出账单 CSV").clicked() {
                    self.export_csv("牛马账单", e.expenses_csv());
                }
            });
        });
        ui.label(RichText::new("记一下钱花哪了，看看窝囊费还剩多少。").size(13.0).color(pal.muted));
        ui.add_space(4.0);

        self.expense_form_card(ui, e, now);
        ui.add_space(4.0);

        let (year, month) = self.ledger_month;
        ui.horizontal(|ui| {
            if ui.button("<").clicked() {
                self.ledger_month = if month == 1 { (year - 1, 12) } else { (year, month - 1) };
            }
            ui.label(bold(format!("{year} 年 {month} 月"), 15.0).color(pal.ink));
            let is_current = (year, month) >= (now.year(), now.month());
            if ui.add_enabled(!is_current, egui::Button::new(">")).clicked() {
                self.ledger_month = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
            }
        });
        let (year, month) = self.ledger_month;
        let spent = e.month_expenses(year, month);
        let earned = if (year, month) == (now.year(), now.month()) {
            e.stats(now).month_earned
        } else {
            e.month_summary(year, month).income
        };

        ui.columns(3, |cols| {
            card(&mut cols[0], &pal, |ui| {
                ui.label(RichText::new("本月花费").size(12.0).color(pal.muted));
                ui.add(egui::Label::new(num(widgets::money(&e.cfg, spent.total), 20.0).color(pal.ink)).truncate());
                ui.label(RichText::new(format!("共 {} 笔", spent.count)).size(12.0).color(pal.muted));
            });
            card(&mut cols[1], &pal, |ui| {
                ui.label(RichText::new("本月已赚").size(12.0).color(pal.muted));
                ui.add(egui::Label::new(num(widgets::money(&e.cfg, earned), 20.0).color(pal.ink)).truncate());
                ui.label(RichText::new("来自赚钱表").size(12.0).color(pal.muted));
            });
            card(&mut cols[2], &pal, |ui| {
                let left = earned - spent.total;
                ui.label(RichText::new("结余").size(12.0).color(pal.muted));
                let color = if left < 0.0 { pal.danger } else { pal.green };
                ui.add(egui::Label::new(num(widgets::money(&e.cfg, left), 20.0).color(color)).truncate());
                let text = if left < 0.0 { "花得比赚得快" } else { "还算守得住" };
                ui.label(RichText::new(text).size(12.0).color(pal.muted));
            });
        });
        ui.add_space(4.0);

        if !spent.by_category.is_empty() {
            card(ui, &pal, |ui| {
                section(ui, &pal, "钱花哪了", "");
                let top = spent.by_category.first().map_or(1.0, |(_, v)| *v).max(0.01);
                for (name, value) in &spent.by_category {
                    ui.horizontal(|ui| {
                        ui.add_sized(Vec2::new(40.0, 18.0), egui::Label::new(RichText::new(name.as_str()).size(13.0).color(pal.ink)));
                        let share = if spent.total > 0.0 { value / spent.total * 100.0 } else { 0.0 };
                        let tail = format!("{}  {share:.0}%", widgets::money(&e.cfg, *value));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add_sized(Vec2::new(128.0, 18.0), egui::Label::new(num(tail, 13.0).color(pal.muted)));
                            ui.add_space(4.0);
                            bar(ui, (*value / top) as f32, 8.0, pal.sunken, pal.accent);
                        });
                    });
                }
            });
            ui.add_space(4.0);
        }

        card(ui, &pal, |ui| {
            ui.horizontal(|ui| {
                section(ui, &pal, "明细", "");
                if let Some(undo) = self.expense_undo.clone() {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("撤销").clicked() {
                            e.restore_expense(undo.clone());
                            self.expense_undo = None;
                        }
                        ui.label(RichText::new(format!("已删除：{} {}", undo.category, widgets::money(&e.cfg, undo.amount))).size(12.0).color(pal.muted));
                    });
                }
            });
            let rows: Vec<&Expense> = e
                .data
                .expenses
                .iter()
                .rev()
                .filter(|x| x.at.year() == year && x.at.month() == month)
                .collect();
            if rows.is_empty() {
                ui.label(RichText::new("这个月还没有记账。").size(13.0).color(pal.muted));
            }
            let mut delete = None;
            let mut last_day = None;
            for x in rows {
                let day = x.at.date();
                if last_day != Some(day) {
                    last_day = Some(day);
                    let total: f64 = e.data.expenses.iter().filter(|y| y.at.date() == day).map(|y| y.amount).sum();
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(bold(format!("{} {}", day.format("%m-%d"), weekday(day)), 13.0).color(pal.ink));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(format!("当天 {}", widgets::money(&e.cfg, total))).size(12.0).color(pal.muted));
                        });
                    });
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(x.at.format("%H:%M").to_string()).size(12.0).color(pal.muted));
                    Frame::new()
                        .fill(pal.sunken)
                        .corner_radius(8)
                        .inner_margin(Margin::symmetric(6, 1))
                        .show(ui, |ui| ui.label(RichText::new(x.category.as_str()).size(12.0).color(pal.ink)));
                    if !x.note.is_empty() {
                        ui.add(egui::Label::new(RichText::new(x.note.as_str()).size(13.0).color(pal.ink)).truncate());
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("删除").clicked() {
                            delete = Some(x.id);
                        }
                        let amount = ui.label(num(widgets::money(&e.cfg, x.amount), 13.0).color(pal.accent));
                        if let Some(equiv) = work_equivalent(&e.cfg, x.amount).filter(|_| !e.cfg.hide_money) {
                            amount.on_hover_text(format!("相当于 {equiv}"));
                        }
                    });
                });
            }
            if let Some(id) = delete {
                self.expense_undo = e.delete_expense(id);
            }
        });
    }

    fn expense_form_card(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime) {
        let pal = self.palette;
        let mut added = None;
        card(ui, &pal, |ui| {
            section(ui, &pal, "记一笔", "");
            let form = &mut self.expense_form;
            let mut submit = false;
            ui.horizontal(|ui| {
                ui.label(&e.cfg.salary.currency);
                let amount = ui.add(egui::TextEdit::singleline(&mut form.amount).hint_text("金额").desired_width(90.0));
                submit |= amount.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let note = ui.add(egui::TextEdit::singleline(&mut form.note).hint_text("备注（可选），比如：午饭、打车").desired_width(f32::INFINITY));
                submit |= note.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
            ui.horizontal_wrapped(|ui| {
                for name in EXPENSE_CATEGORIES {
                    if chip(ui, &pal, name, form.category == name, pal.accent).clicked() {
                        form.category = name.to_owned();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("时间").size(13.0).color(pal.muted));
                ui.add(egui::TextEdit::singleline(&mut form.when).hint_text("不填就是现在").desired_width(150.0));
                ui.label(RichText::new("格式 2026-11-04 12:30").size(12.0).color(pal.muted));
            });
            let parsed = form.amount.trim().parse::<f64>().ok().filter(|v| *v > 0.0 && v.is_finite());
            // 这一行一直占着，免得出现提示时把按钮挤下去。
            let hint = match parsed.and_then(|v| work_equivalent(&e.cfg, v)).filter(|_| !e.cfg.hide_money) {
                Some(equiv) => format!("这笔相当于 {equiv}"),
                None => "填好金额后按回车也能记下来".to_owned(),
            };
            ui.label(RichText::new(hint).size(12.0).color(pal.muted));
            if !form.error.is_empty() {
                ui.label(RichText::new(form.error.as_str()).size(12.0).color(pal.danger));
            }
            submit |= widgets::solid_button(ui, "记下来", pal.accent, pal.on_accent, Vec2::new(96.0, 32.0), 14.0).clicked();
            if !submit {
                return;
            }
            let when = if form.when.trim().is_empty() {
                Ok(now)
            } else {
                NaiveDateTime::parse_from_str(form.when.trim(), DATETIME_FORMAT)
            };
            match (parsed, when) {
                (None, _) => form.error = "金额要填大于 0 的数字。".to_owned(),
                (_, Err(_)) => form.error = "时间格式不对，例子：2026-11-04 12:30".to_owned(),
                (Some(amount), Ok(at)) => {
                    e.add_expense(at, amount, &form.category, &form.note);
                    let category = std::mem::take(&mut form.category);
                    *form = ExpenseForm { category, ..ExpenseForm::default() };
                    added = Some((at, amount));
                }
            }
        });
        if let Some((at, amount)) = added {
            self.ledger_month = (at.year(), at.month());
            self.set_notice(format!("记下了：{}", widgets::money(&e.cfg, amount)));
        }
    }

    // ---------- 设置 ----------

    pub fn page_settings(&mut self, ui: &mut Ui, e: &mut Engine) {
        let pal = self.palette;
        ui.heading("设置");
        ui.add_space(4.0);
        let before = e.cfg.clone();

        card(ui, &pal, |ui| {
            section(ui, &pal, "赚钱表", "数据只存在本机。");
            let s = &mut e.cfg.salary;
            let lunch_quiet = &mut e.cfg.lunch_quiet;
            egui::Grid::new("salary").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label("月薪");
                ui.add(egui::DragValue::new(&mut s.monthly).range(0.0..=100_000_000.0).speed(100.0).prefix(format!("{} ", s.currency)));
                ui.end_row();
                ui.label("月计薪天数");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut s.paid_days).range(1.0..=31.0).speed(0.25));
                    ui.label(RichText::new("国家标准 21.75").size(12.0).color(pal.muted));
                });
                ui.end_row();
                ui.label("上班时间");
                time_field(ui, &mut s.work_start, "work-start");
                ui.end_row();
                ui.label("下班时间");
                time_field(ui, &mut s.work_end, "work-end");
                ui.end_row();
                ui.label("午休");
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        time_field(ui, &mut s.lunch_start, "lunch-start");
                        ui.label("到");
                        time_field(ui, &mut s.lunch_end, "lunch-end");
                    });
                    ui.checkbox(&mut s.lunch_unpaid, "午休不计薪");
                    ui.checkbox(lunch_quiet, "午休模式：这段时间不弹提醒，循环倒计时暂停")
                        .on_hover_text("上班时到点自动进入。吃药提醒等午休结束再响。\n别的时间吃饭，可以在“今日”页或托盘菜单手动开始午休。");
                });
                ui.end_row();
                ui.label("跳字频率");
                ui.horizontal(|ui| {
                    for unit in [TickUnit::Second, TickUnit::Minute, TickUnit::Hour] {
                        if chip(ui, &pal, unit.label(), s.tick_unit == unit, pal.accent).clicked() {
                            s.tick_unit = unit;
                        }
                    }
                });
                ui.end_row();
                ui.label("加班倍率");
                ui.horizontal_wrapped(|ui| {
                    for (label, value) in [("白嫖", 0.0), ("1 倍", 1.0), ("1.5 倍", 1.5), ("2 倍", 2.0), ("3 倍", 3.0)] {
                        if chip(ui, &pal, label, (s.overtime_multiplier - value).abs() < 1e-6, pal.accent).clicked() {
                            s.overtime_multiplier = value;
                        }
                    }
                });
                ui.end_row();
                ui.label("发薪日");
                ui.add(egui::DragValue::new(&mut s.payday).range(1..=31).prefix("每月 ").suffix(" 号"));
                ui.end_row();
                ui.label("货币符号");
                ui.add(egui::TextEdit::singleline(&mut s.currency).desired_width(50.0).char_limit(3));
                ui.end_row();
            });
            let daily = salary::standard_daily_secs(s);
            let rate = salary::rate_per_sec(s);
            let text = if rate > 0.0 {
                format!(
                    "标准工时每天 {:.1} 小时，日薪 {}{:.2}，每小时 {}{:.2}。跳字只影响显示，下班按实际秒数结算。",
                    daily / 3600.0,
                    s.currency,
                    rate * daily,
                    s.currency,
                    rate * 3600.0
                )
            } else if s.monthly <= 0.0 {
                "填写月薪后开始计算。".to_owned()
            } else {
                "上下班时间扣除午休后没有计薪时长，请检查设置。".to_owned()
            };
            let color = if rate > 0.0 || s.monthly <= 0.0 { pal.muted } else { pal.danger };
            ui.label(RichText::new(text).size(12.0).color(color));
        });
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            section(ui, &pal, "提醒", "");
            ui.horizontal(|ui| {
                ui.label("提醒方式");
                for (label, style) in [("置顶卡片", AlertStyle::Card), ("系统通知", AlertStyle::Toast), ("两者都要", AlertStyle::Both)] {
                    if chip(ui, &pal, label, e.cfg.alert_style == style, pal.accent).clicked() {
                        e.cfg.alert_style = style;
                    }
                }
            });
            ui.label(RichText::new("系统通知可能被勿扰模式拦截，发出后不等确认直接进入下一轮。").size(12.0).color(pal.muted));
            ui.horizontal(|ui| {
                ui.label("稍后提醒");
                ui.add(egui::DragValue::new(&mut e.cfg.snooze_minutes).range(1..=120).suffix(" 分钟"));
            });
            ui.checkbox(&mut e.cfg.sound, "提示音");
            ui.checkbox(&mut e.cfg.pause_when_away, "锁屏或休眠时暂停循环倒计时（吃药提醒不暂停）");
            ui.checkbox(&mut e.cfg.work_end_reminder, "到下班时间提醒我打卡");
        });
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            section(ui, &pal, "下班特效", "");
            ui.checkbox(&mut e.cfg.effect_enabled, "下班打卡时播放全屏特效");
            ui.horizontal(|ui| {
                ui.label("时长");
                ui.add(egui::Slider::new(&mut e.cfg.effect_seconds, 3.0..=15.0).suffix(" 秒"));
            });
            if ui.button("预览特效").clicked() {
                self.preview_effect(e);
            }
        });
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            section(ui, &pal, "桌面与外观", "");
            ui.checkbox(&mut e.cfg.floating_bar, "显示悬浮赚钱条（全屏或演示时自动隐藏）");
            let mut boss = e.cfg.boss_key;
            if ui.checkbox(&mut boss, "老板键 Ctrl + Alt + B：隐藏金额和悬浮条").changed() {
                self.set_boss_key(e, boss);
            }
            if let Some(error) = &self.boss_key_error {
                ui.label(RichText::new(error.as_str()).size(12.0).color(pal.danger));
            }
            let mut autostart = self.autostart;
            if ui.checkbox(&mut autostart, "开机自动启动（启动后待在托盘）").changed() {
                if platform::set_autostart(autostart) {
                    self.autostart = autostart;
                } else {
                    self.set_notice("设置开机自启失败");
                }
            }
            if e.cfg.floating_bar && ui.button("悬浮条放回右下角").clicked() {
                e.cfg.floating_pos = None;
                self.floating_placed = false;
            }
            ui.horizontal(|ui| {
                ui.label("主题");
                for (label, mode) in [("跟随系统", ThemeMode::System), ("浅色", ThemeMode::Light), ("深色", ThemeMode::Dark)] {
                    if chip(ui, &pal, label, e.cfg.theme == mode, pal.accent).clicked() {
                        e.cfg.theme = mode;
                    }
                }
            });
        });
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            let theme = if pal.dark { "深色" } else { "浅色" };
            section(ui, &pal, "调色板", &format!("下面改的是当前{theme}主题的颜色，浅色和深色各存一套。一键配色会同时换掉两套。"));
            ui.horizontal_wrapped(|ui| {
                ui.label("一键配色");
                for (name, light, dark) in widgets::presets() {
                    let selected = e.cfg.colors_light == light && e.cfg.colors_dark == dark;
                    let swatch = if pal.dark { dark } else { light };
                    let base = if pal.dark { Palette::DARK } else { Palette::LIGHT };
                    let color = base.with_overrides(&swatch).accent;
                    if chip(ui, &pal, name, selected, color).clicked() {
                        e.cfg.colors_light = light;
                        e.cfg.colors_dark = dark;
                    }
                }
            });
            let base = if pal.dark { Palette::DARK } else { Palette::LIGHT };
            let overrides = if pal.dark { &mut e.cfg.colors_dark } else { &mut e.cfg.colors_light };
            let current = |o: &ColorOverrides, slot: ColorSlot| {
                slot.get(o).unwrap_or_else(|| {
                    let c = base.slot(slot);
                    [c.r(), c.g(), c.b()]
                })
            };
            ui.label(RichText::new("点色块换颜色。").size(12.0).color(pal.muted));
            egui::Grid::new("palette").num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
                for slot in ColorSlot::ALL {
                    ui.label(slot.label());
                    let open = self.palette_open == Some(slot);
                    if swatch(ui, &pal, current(overrides, slot), Vec2::new(40.0, 22.0), open).clicked() {
                        self.palette_open = if open { None } else { Some(slot) };
                        self.palette_hex = hex(current(overrides, slot));
                    }
                    if slot.get(overrides).is_some() {
                        if ui.small_button("恢复默认").clicked() {
                            slot.set(overrides, None);
                            self.palette_hex = hex(current(overrides, slot));
                        }
                    } else {
                        ui.label(RichText::new("默认").size(12.0).color(pal.muted));
                    }
                    ui.end_row();
                }
            });
            if let Some(slot) = self.palette_open {
                Frame::new()
                    .fill(pal.sunken)
                    .corner_radius(10)
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(bold(format!("选一个{}", slot.label()), 13.0).color(pal.ink));
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                            for color in SWATCHES {
                                let selected = current(overrides, slot) == color;
                                if swatch(ui, &pal, color, Vec2::splat(26.0), selected).clicked() {
                                    slot.set(overrides, Some(color));
                                    self.palette_hex = hex(color);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("自定义 #");
                            let edit = ui.add(egui::TextEdit::singleline(&mut self.palette_hex).char_limit(7).desired_width(80.0));
                            if edit.changed()
                                && let Some(color) = parse_hex(&self.palette_hex)
                            {
                                slot.set(overrides, Some(color));
                            }
                            ui.label(RichText::new("6 位十六进制，如 2F6FC9").size(12.0).color(pal.muted));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("收起").clicked() {
                                    self.palette_open = None;
                                }
                            });
                        });
                    });
            }
            if pal.bg_card_clash() {
                ui.label(RichText::new("窗口背景和卡片底色一深一浅，页面标题可能看不清，建议调成同一深浅。").size(12.0).color(pal.danger));
            }
            if ui.button("全部恢复默认配色").clicked() {
                e.cfg.colors_light = ColorOverrides::default();
                e.cfg.colors_dark = ColorOverrides::default();
            }
        });
        ui.add_space(4.0);

        card(ui, &pal, |ui| {
            section(ui, &pal, "数据", "全部保存在本机，不联网。关闭窗口后程序留在托盘，要退出请右键托盘图标。");
            ui.label(RichText::new(self.dir.display().to_string()).size(12.0).color(pal.muted));
            if ui.button("打开数据文件夹").clicked() {
                platform::open_folder(&self.dir);
            }
        });

        if e.cfg != before {
            e.cfg_dirty = true;
        }
    }

    // ---------- 弹窗 ----------

    pub fn modals(&mut self, ui: &mut Ui, e: &mut Engine, now: NaiveDateTime) {
        let ctx = ui.ctx().clone();
        let pal = self.palette;

        if let Some(start) = e.forgotten(now) {
            egui::Modal::new(egui::Id::unique("forgot")).show(&ctx, |ui| {
                ui.set_width(380.0);
                ui.heading("是不是忘打下班卡了？");
                ui.label(format!("你在 {} 打了上班卡，一直没有下班。", start.format("%m-%d %H:%M")));
                let planned = salary::work_end_at(start.date(), &e.cfg.salary);
                ui.add_space(4.0);
                if planned > start
                    && widgets::wide_button(ui, &format!("按下班时间 {} 补卡", planned.format("%m-%d %H:%M")), pal.accent, pal.on_accent, 34.0, 14.0).clicked()
                {
                    e.clock_out(planned);
                }
                ui.horizontal(|ui| {
                    if self.forgot_custom.is_empty() {
                        self.forgot_custom = planned.format(DATETIME_FORMAT).to_string();
                    }
                    ui.add(egui::TextEdit::singleline(&mut self.forgot_custom).desired_width(170.0));
                    if ui.button("按这个时间补卡").clicked() {
                        match NaiveDateTime::parse_from_str(self.forgot_custom.trim(), DATETIME_FORMAT) {
                            Ok(end) if end > start && end <= now => {
                                e.clock_out(end);
                                self.forgot_error.clear();
                            }
                            Ok(_) => self.forgot_error = "时间要晚于上班时间，且不能晚于现在。".to_owned(),
                            Err(_) => self.forgot_error = "格式不对，例子：2026-11-02 18:00".to_owned(),
                        }
                    }
                });
                if !self.forgot_error.is_empty() {
                    ui.label(RichText::new(self.forgot_error.as_str()).size(12.0).color(pal.danger));
                }
                if ui.button("没忘，我真的还在上班").clicked() {
                    e.ack_forgotten(start);
                }
            });
            return;
        }

        let mut close_month = false;
        if let Some(edit) = &mut self.month_edit {
            let stats = e.stats(now);
            let day = stats.work_day.unwrap_or(now.date());
            egui::Modal::new(egui::Id::unique("month-earned")).show(&ctx, |ui| {
                ui.set_width(380.0);
                ui.heading(format!("修改 {} 月已赚", day.month()));
                ui.label("工资不是从装软件那天开始算的？比如 1 号开始算工资、3 号才装上，就在这里填上到现在为止这个月实际已经赚了多少。");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("本月到现在已赚");
                    ui.add(egui::DragValue::new(&mut edit.amount).range(0.0..=100_000_000.0).speed(10.0).max_decimals(2).prefix(format!("{} ", e.cfg.salary.currency)));
                });
                let recorded = stats.month_earned - stats.month_adjust;
                let diff = edit.amount - recorded;
                let note = if diff.abs() < 0.005 {
                    "和打卡记录算出来的一致，不需要补记。".to_owned()
                } else {
                    format!("打卡记录算出 {}，将补记 {}。之后照常按秒累加，下个月从零开始。", widgets::money(&e.cfg, recorded), signed_money(&e.cfg, diff))
                };
                ui.label(RichText::new(note).size(12.0).color(pal.muted));
                if let Some(days) = salary_days_hint(&e.cfg, edit.amount) {
                    ui.label(RichText::new(days).size(12.0).color(pal.muted));
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if widgets::solid_button(ui, "保存", pal.accent, pal.on_accent, Vec2::new(70.0, 30.0), 14.0).clicked() {
                        e.set_month_earned(now, edit.amount);
                        close_month = true;
                    }
                    if stats.month_adjust.abs() >= 0.005 && ui.button("清除补记").clicked() {
                        e.clear_month_adjust(day.year(), day.month());
                        close_month = true;
                    }
                    if ui.button("取消").clicked() {
                        close_month = true;
                    }
                });
            });
        }
        if close_month {
            self.month_edit = None;
        }

        let mut close_edit = false;
        if let Some(edit) = &mut self.session_edit {
            egui::Modal::new(egui::Id::unique("edit-session")).show(&ctx, |ui| {
                ui.set_width(340.0);
                ui.heading("修改打卡记录");
                egui::Grid::new("edit-grid").num_columns(2).show(ui, |ui| {
                    ui.label("上班");
                    ui.add(egui::TextEdit::singleline(&mut edit.start).desired_width(170.0));
                    ui.end_row();
                    ui.label("下班");
                    ui.add(egui::TextEdit::singleline(&mut edit.end).desired_width(170.0));
                    ui.end_row();
                });
                ui.label(RichText::new("格式：2026-11-02 18:00").size(12.0).color(pal.muted));
                if !edit.error.is_empty() {
                    ui.label(RichText::new(edit.error.as_str()).size(12.0).color(pal.danger));
                }
                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        let start = NaiveDateTime::parse_from_str(edit.start.trim(), DATETIME_FORMAT);
                        let end = NaiveDateTime::parse_from_str(edit.end.trim(), DATETIME_FORMAT);
                        match (start, end) {
                            (Ok(start), Ok(end)) if e.edit_session(edit.index, start, end) => close_edit = true,
                            (Ok(_), Ok(_)) => edit.error = "下班时间要晚于上班时间。".to_owned(),
                            _ => edit.error = "时间格式不对。".to_owned(),
                        }
                    }
                    if ui.button("取消").clicked() {
                        close_edit = true;
                    }
                });
            });
        }
        if close_edit {
            self.session_edit = None;
        }

        if let Some(index) = self.pending_delete {
            let mut close = false;
            egui::Modal::new(egui::Id::unique("delete-session")).show(&ctx, |ui| {
                ui.set_width(300.0);
                ui.heading("删除这条记录？");
                if let Some(s) = e.data.sessions.get(index) {
                    ui.label(format!("{} 的打卡记录，删除后不能恢复。", s.start.format("%Y-%m-%d %H:%M")));
                }
                ui.horizontal(|ui| {
                    if widgets::solid_button(ui, "删除", pal.danger, pal.on_accent_for(pal.danger), Vec2::new(70.0, 30.0), 14.0).clicked() {
                        e.delete_session(index);
                        close = true;
                    }
                    if ui.button("取消").clicked() {
                        close = true;
                    }
                });
            });
            if close {
                self.pending_delete = None;
            }
        }
    }
}

fn hp_color(pal: &Palette, hp: i32) -> egui::Color32 {
    match hp {
        70.. => pal.green,
        40..=69 => if pal.dark { pal.gold } else { egui::Color32::from_rgb(0xB5, 0x7A, 0x10) },
        _ => pal.danger,
    }
}

fn countdown_text(stats: &TodayStats, cfg: &Config) -> RichText {
    let text = if !stats.on_duty {
        "未在上班".to_owned()
    } else {
        match stats.to_end_secs {
            Some(left) if left > 0 => format!("距下班 {}", widgets::clock_text(left)),
            Some(left) => {
                let word = if cfg.salary.overtime_multiplier <= 0.0 { "白嫖中" } else { "加班中" };
                format!("{word} {}", widgets::clock_text(-left))
            }
            None => format!("计划外上班 {}", widgets::clock_text(stats.worked_secs as i64)),
        }
    };
    num(text, 14.0)
}

fn loop_progress(ui: &mut Ui, l: &ActiveLoop, now: NaiveDateTime, pal: &Palette) {
    let total = (l.interval_min as i64 * 60).max(1);
    let (right, frac, color) = if l.pending_since.is_some() {
        ("到点了".to_owned(), 1.0, pal.danger)
    } else if let Some(left) = l.paused_left {
        (format!("已暂停 {}", widgets::short_clock(left)), 1.0 - left as f32 / total as f32, pal.muted)
    } else {
        let left = (l.next_fire - now).num_seconds().max(0);
        (widgets::short_clock(left), 1.0 - left as f32 / total as f32, pal.accent)
    };
    ui.horizontal(|ui| {
        ui.label(bold(l.title.as_str(), 14.0).color(pal.ink));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(num(right, 14.0).color(color));
        });
    });
    bar(ui, frac, 5.0, pal.sunken, color);
}

fn time_field(ui: &mut Ui, time: &mut NaiveTime, id: &str) {
    ui.push_id(id, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let mut hour = time.hour();
            let mut minute = time.minute();
            let a = ui.add(egui::DragValue::new(&mut hour).range(0..=23).custom_formatter(|n, _| format!("{n:02}")));
            ui.label(":");
            let b = ui.add(egui::DragValue::new(&mut minute).range(0..=59).custom_formatter(|n, _| format!("{n:02}")));
            if (a.changed() || b.changed())
                && let Some(updated) = NaiveTime::from_hms_opt(hour, minute, 0)
            {
                *time = updated;
            }
        });
    });
}

fn month_chart(ui: &mut Ui, e: &Engine, year: i32, month: u32, pal: &Palette) {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("合法月份");
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("合法月份");
    let days = (next - first).num_days() as usize;

    let mut worked = vec![0.0f32; days];
    let mut overtime = vec![0.0f32; days];
    for s in &e.data.sessions {
        let Some(end) = s.end else { continue };
        if s.start.date() >= first && s.start.date() < next {
            let i = (s.start.date() - first).num_days() as usize;
            worked[i] += (end - s.start).num_seconds() as f32 / 3600.0;
            overtime[i] += s.overtime_secs as f32 / 3600.0;
        }
    }
    let max = worked.iter().copied().fold(10.0f32, f32::max);

    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 88.0), Sense::hover());
    let slot = rect.width() / days as f32;
    let baseline = rect.bottom() - 14.0;
    let height = rect.height() - 18.0;
    for i in 0..days {
        let x = rect.left() + slot * i as f32;
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(x + 1.5, baseline - height * worked[i] / max),
            egui::pos2(x + slot - 1.5, baseline),
        );
        let hover = ui.interact(
            egui::Rect::from_min_max(egui::pos2(x, rect.top()), egui::pos2(x + slot, rect.bottom())),
            ui.scope_id().with(("day", i)),
            Sense::hover(),
        );
        let painter = ui.painter();
        if worked[i] > 0.0 {
            painter.rect_filled(bar_rect, 2, pal.green);
            if overtime[i] > 0.0 {
                let top = egui::Rect::from_min_max(
                    bar_rect.min,
                    egui::pos2(bar_rect.right(), bar_rect.top() + height * overtime[i] / max),
                );
                painter.rect_filled(top, 2, pal.accent);
            }
        } else {
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x + 1.5, baseline - 2.0), egui::pos2(x + slot - 1.5, baseline)),
                1,
                pal.sunken,
            );
        }
        if (i + 1) % 5 == 0 || i == 0 {
            painter.text(
                egui::pos2(x + slot / 2.0, rect.bottom()),
                egui::Align2::CENTER_BOTTOM,
                (i + 1).to_string(),
                egui::FontId::proportional(10.0),
                pal.muted,
            );
        }
        if worked[i] > 0.0 {
            hover.on_hover_text(format!("{month} 月 {} 日：{:.1} 小时", i + 1, worked[i]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        assert_eq!(parse_hex("2F6FC9"), Some([0x2F, 0x6F, 0xC9]));
        assert_eq!(parse_hex(" #2f6fc9 "), Some([0x2F, 0x6F, 0xC9]));
        assert_eq!(parse_hex("2F6FC"), None);
        assert_eq!(parse_hex("GG0000"), None);
        assert_eq!(parse_hex("牛马牛"), None);
        assert_eq!(hex([0x2F, 0x6F, 0xC9]), "2F6FC9");
    }

    #[test]
    fn work_equivalent_reads_naturally() {
        let mut cfg = Config::default();
        assert_eq!(work_equivalent(&cfg, 10.0), None, "没设月薪");
        cfg.salary.monthly = 21750.0; // 日薪 1000，8 小时，每小时 125
        assert_eq!(work_equivalent(&cfg, 0.5).as_deref(), Some("不到 1 分钟的班"));
        assert_eq!(work_equivalent(&cfg, 25.0).as_deref(), Some("12 分钟的班"));
        assert_eq!(work_equivalent(&cfg, 250.0).as_deref(), Some("2 小时的班"));
        assert_eq!(work_equivalent(&cfg, 300.0).as_deref(), Some("2 小时 24 分钟的班"));
    }
}
