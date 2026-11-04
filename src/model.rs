//! 持久化的数据结构。所有字段都带 serde 默认值，旧版本文件可以直接读入。
use std::collections::BTreeMap;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

pub fn hm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).expect("合法的时分")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TickUnit {
    #[default]
    Second,
    Minute,
    Hour,
}

impl TickUnit {
    pub fn secs(self) -> f64 {
        match self {
            TickUnit::Second => 1.0,
            TickUnit::Minute => 60.0,
            TickUnit::Hour => 3600.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TickUnit::Second => "每秒",
            TickUnit::Minute => "每分钟",
            TickUnit::Hour => "每小时",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SalaryConfig {
    pub monthly: f64,
    /// 月计薪天数，国家标准 21.75。
    pub paid_days: f64,
    pub work_start: NaiveTime,
    pub work_end: NaiveTime,
    /// 午休不计薪。关闭后午休也算钱。
    pub lunch_unpaid: bool,
    pub lunch_start: NaiveTime,
    pub lunch_end: NaiveTime,
    pub tick_unit: TickUnit,
    /// 超过标准工时后的倍率。0 表示白嫖模式：表停，只记白干的时长。
    pub overtime_multiplier: f64,
    pub payday: u32,
    pub currency: String,
}

impl Default for SalaryConfig {
    fn default() -> Self {
        Self {
            monthly: 0.0,
            paid_days: 21.75,
            work_start: hm(9, 0),
            work_end: hm(18, 0),
            lunch_unpaid: true,
            lunch_start: hm(12, 0),
            lunch_end: hm(13, 0),
            tick_unit: TickUnit::Second,
            overtime_multiplier: 1.0,
            payday: 15,
            currency: "¥".to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderTemplate {
    pub id: u64,
    pub title: String,
    pub note: String,
    pub interval_min: u32,
    /// 上班打卡时自动开始。
    pub auto_start: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MedSchedule {
    Interval { minutes: u32 },
    Times { times: Vec<NaiveTime> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MedPlan {
    pub id: u64,
    pub name: String,
    pub dose: String,
    pub note: String,
    pub schedule: MedSchedule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlertStyle {
    /// 置顶卡片，等你确认。
    #[default]
    Card,
    /// 只发系统通知，发完自动进入下一轮。
    Toast,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

/// 调色板：用户改过的颜色，没改的用内置主题色。浅色和深色主题各存一份。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorOverrides {
    pub accent: Option<[u8; 3]>,
    pub gold: Option<[u8; 3]>,
    pub hero: Option<[u8; 3]>,
    pub bg: Option<[u8; 3]>,
    pub card: Option<[u8; 3]>,
}

pub const EXPENSE_CATEGORIES: [&str; 8] = ["餐饮", "交通", "购物", "娱乐", "居住", "医疗", "人情", "其他"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Expense {
    pub id: u64,
    pub at: NaiveDateTime,
    pub amount: f64,
    pub category: String,
    #[serde(default)]
    pub note: String,
}

pub fn month_key(year: i32, month: u32) -> String {
    format!("{year:04}-{month:02}")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub salary: SalaryConfig,
    pub reminders: Vec<ReminderTemplate>,
    pub meds: Vec<MedPlan>,
    pub alert_style: AlertStyle,
    pub sound: bool,
    pub snooze_minutes: u32,
    /// 锁屏、休眠期间暂停活动类倒计时。吃药提醒不受影响。
    pub pause_when_away: bool,
    pub work_end_reminder: bool,
    pub floating_bar: bool,
    pub floating_pos: Option<[f32; 2]>,
    pub hide_money: bool,
    pub boss_key: bool,
    pub effect_enabled: bool,
    pub effect_seconds: f32,
    pub theme: ThemeMode,
    pub colors_light: ColorOverrides,
    pub colors_dark: ColorOverrides,
    pub tray_hint_shown: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            salary: SalaryConfig::default(),
            reminders: vec![
                ReminderTemplate {
                    id: 1,
                    title: "喝口水".to_owned(),
                    note: "牛马也要补水，别等渴了才想起来。".to_owned(),
                    interval_min: 45,
                    auto_start: true,
                },
                ReminderTemplate {
                    id: 2,
                    title: "起来动一动".to_owned(),
                    note: "站起来走两步，活动一下肩颈和腰。".to_owned(),
                    interval_min: 60,
                    auto_start: true,
                },
            ],
            meds: Vec::new(),
            alert_style: AlertStyle::Card,
            sound: true,
            snooze_minutes: 5,
            pause_when_away: true,
            work_end_reminder: true,
            floating_bar: true,
            floating_pos: None,
            hide_money: false,
            boss_key: true,
            effect_enabled: true,
            effect_seconds: 7.0,
            theme: ThemeMode::System,
            colors_light: ColorOverrides::default(),
            colors_dark: ColorOverrides::default(),
            tray_hint_shown: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Health {
    #[default]
    Healthy,
    SubHealthy,
    Unwell,
}

impl Health {
    pub const ALL: [Health; 3] = [Health::Healthy, Health::SubHealthy, Health::Unwell];

    pub fn label(self) -> &'static str {
        match self {
            Health::Healthy => "健康",
            Health::SubHealthy => "亚健康",
            Health::Unwell => "不健康",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Mood {
    Happy,
    #[default]
    Calm,
    Slacking,
    Tired,
    Annoyed,
    Broken,
}

impl Mood {
    pub const ALL: [Mood; 6] = [
        Mood::Happy,
        Mood::Calm,
        Mood::Slacking,
        Mood::Tired,
        Mood::Annoyed,
        Mood::Broken,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mood::Happy => "开心",
            Mood::Calm => "平静",
            Mood::Slacking => "想摸鱼",
            Mood::Tired => "累了",
            Mood::Annoyed => "烦躁",
            Mood::Broken => "崩溃",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkSession {
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    /// 以下三项在下班时结算并固化，之后改工资不影响历史。
    #[serde(default)]
    pub paid_secs: f64,
    #[serde(default)]
    pub overtime_secs: f64,
    #[serde(default)]
    pub income: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DayLog {
    pub done: u32,
    pub skipped: u32,
    pub meds_taken: u32,
    pub health: Option<Health>,
    pub mood: Option<Mood>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveLoop {
    pub id: u64,
    /// None 表示今日临时事项。
    pub template_id: Option<u64>,
    pub title: String,
    pub note: String,
    pub interval_min: u32,
    pub next_fire: NaiveDateTime,
    /// 暂停时保存的剩余秒数。
    pub paused_left: Option<i64>,
    pub pending_since: Option<NaiveDateTime>,
    pub done: u32,
    pub skipped: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MedRuntime {
    pub plan_id: u64,
    pub next_fire: NaiveDateTime,
    pub pending_since: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusChange {
    Health(Health),
    Mood(Mood),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusEntry {
    pub at: NaiveDateTime,
    pub change: StatusChange,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Data {
    pub next_id: u64,
    pub sessions: Vec<WorkSession>,
    pub days: BTreeMap<NaiveDate, DayLog>,
    pub loops: Vec<ActiveLoop>,
    pub meds: Vec<MedRuntime>,
    pub health: Health,
    pub mood: Mood,
    pub status_log: Vec<StatusEntry>,
    pub work_end_notified: Option<NaiveDate>,
    pub work_end_pending: bool,
    pub work_end_snooze_until: Option<NaiveDateTime>,
    pub sick_prompt_pending: bool,
    /// 用户确认“真的还在上班”的那次打卡，不再追问。
    pub forgot_ack: Option<NaiveDateTime>,
    pub last_seen: Option<NaiveDateTime>,
    /// 每月已赚的手动调整（键为 "2026-11"），用于补上装软件之前已经赚到的部分。
    pub month_adjust: BTreeMap<String, f64>,
    pub expenses: Vec<Expense>,
}
