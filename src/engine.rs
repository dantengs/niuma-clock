//! 中央状态机：打卡、循环提醒、吃药、状态变更都走这里。
//! 时间全部由调用方传入，方便测试，也让后台线程和界面共用同一份逻辑。
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};

use crate::model::*;
use crate::salary;

/// 两次心跳间隔超过它，就认为电脑睡过或者程序没运行。
const AWAY_GAP_SECS: i64 = 60;
/// 打卡跨天且超过这个时长，才怀疑忘了打下班卡，避免误伤正常夜班。
const FORGOT_AFTER_HOURS: i64 = 16;
const WORK_END_SNOOZE_MIN: i64 = 30;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub body: String,
    pub toast: bool,
    pub sound: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TickEnv {
    /// 已锁屏。
    pub away: bool,
    /// 正在全屏或演示，新提醒先推迟。
    pub quiet: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlertKind {
    Loop(u64),
    Med(u64),
    WorkEnd,
    Sick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alert {
    pub kind: AlertKind,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AckAction {
    Done,
    Skip,
    Snooze,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TodayStats {
    pub work_day: Option<NaiveDate>,
    pub on_duty: bool,
    pub session_start: Option<NaiveDateTime>,
    /// 按跳字粒度显示的收入。
    pub earned_display: f64,
    pub earned_exact: f64,
    pub worked_secs: f64,
    pub paid_secs: f64,
    pub overtime_secs: f64,
    pub progress: f32,
    pub to_end_secs: Option<i64>,
    /// 含手动调整。
    pub month_earned: f64,
    pub month_adjust: f64,
    pub done: u32,
    pub skipped: u32,
    pub hp: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DaySummary {
    pub date: NaiveDate,
    pub income: f64,
    pub worked_secs: f64,
    pub overtime_secs: f64,
    pub done: u32,
    pub skipped: u32,
    pub hp: i32,
    pub free_labor: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MonthSummary {
    pub days: u32,
    pub worked_secs: f64,
    pub overtime_secs: f64,
    /// 含手动调整。
    pub income: f64,
    pub adjust: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExpenseSummary {
    pub total: f64,
    pub count: usize,
    /// 按金额从大到小。
    pub by_category: Vec<(String, f64)>,
}

pub fn hp_score(done: u32, skipped: u32, overtime_secs: f64, health: Health) -> i32 {
    let penalty = match health {
        Health::Healthy => 0,
        Health::SubHealthy => 10,
        Health::Unwell => 20,
    };
    let overtime = (overtime_secs / 600.0) as i32;
    (100 + 2 * done as i32 - 5 * skipped as i32 - overtime - penalty).clamp(0, 100)
}

pub fn hp_rank(hp: i32) -> &'static str {
    match hp {
        90.. => "金牌牛马",
        70..=89 => "健壮牛马",
        50..=69 => "疲惫牛马",
        25..=49 => "濒危牛马",
        _ => "牛马已躺平",
    }
}

#[derive(Debug, Default)]
pub struct Engine {
    pub cfg: Config,
    pub data: Data,
    pub cfg_dirty: bool,
    pub data_dirty: bool,
    last_seen_saved: Option<NaiveDateTime>,
}

impl Engine {
    pub fn new(cfg: Config, mut data: Data) -> Self {
        // 默认模板占了 1、2 号，保证后续 id 不会撞车。
        let max_tpl = cfg.reminders.iter().map(|r| r.id).max().unwrap_or(0);
        let max_med = cfg.meds.iter().map(|m| m.id).max().unwrap_or(0);
        data.next_id = data.next_id.max(max_tpl.max(max_med) + 1);
        Self {
            cfg,
            data,
            cfg_dirty: false,
            data_dirty: false,
            last_seen_saved: None,
        }
    }

    fn new_id(&mut self) -> u64 {
        let id = self.data.next_id;
        self.data.next_id += 1;
        id
    }

    pub fn open_session(&self) -> Option<&WorkSession> {
        self.data.sessions.iter().rev().find(|s| s.end.is_none())
    }

    pub fn on_duty(&self) -> bool {
        self.open_session().is_some()
    }

    fn work_day(&self, now: NaiveDateTime) -> NaiveDate {
        self.open_session()
            .map_or(now.date(), |session| session.start.date())
    }

    fn day_mut(&mut self, date: NaiveDate) -> &mut DayLog {
        self.data.days.entry(date).or_default()
    }

    // ---------- 打卡 ----------

    pub fn clock_in(&mut self, now: NaiveDateTime) -> bool {
        if self.on_duty() {
            return false;
        }
        self.data.sessions.push(WorkSession {
            start: now,
            end: None,
            paid_secs: 0.0,
            overtime_secs: 0.0,
            income: 0.0,
        });
        let autos: Vec<ReminderTemplate> = self
            .cfg
            .reminders
            .iter()
            .filter(|t| t.auto_start)
            .cloned()
            .collect();
        for tpl in autos {
            self.start_template_loop(&tpl, now);
        }
        // 重新打卡说明上一段已经下班，当天不再催。
        if salary::work_end_at(now.date(), &self.cfg.salary) <= now {
            self.data.work_end_notified = Some(now.date());
        }
        self.data_dirty = true;
        true
    }

    /// 下班结算，并清除所有运行中的循环。常用模板留在配置里，下次上班自动恢复。
    pub fn clock_out(&mut self, now: NaiveDateTime) -> Option<DaySummary> {
        let index = self.data.sessions.iter().rposition(|s| s.end.is_none())?;
        let date = self.data.sessions[index].start.date();
        let end = now.max(self.data.sessions[index].start);
        self.data.sessions[index].end = Some(end);
        self.recalc_day(date);
        self.data.loops.clear();
        self.data.work_end_pending = false;
        self.data.work_end_snooze_until = None;
        self.data_dirty = true;
        Some(self.summary_of(date))
    }

    /// 按当前工资设置重算某天所有已结束的打卡段。
    fn recalc_day(&mut self, date: NaiveDate) {
        let salary_cfg = self.cfg.salary.clone();
        let daily = salary::standard_daily_secs(&salary_cfg);
        let mut before = 0.0;
        for session in self
            .data
            .sessions
            .iter_mut()
            .filter(|s| s.start.date() == date)
        {
            let Some(end) = session.end else { continue };
            let paid = salary::paid_secs(session.start, end, &salary_cfg);
            session.paid_secs = paid;
            session.income =
                salary::earned(before + paid, &salary_cfg) - salary::earned(before, &salary_cfg);
            session.overtime_secs = (before + paid - daily.max(before)).max(0.0);
            before += paid;
        }
    }

    pub fn summary_of(&self, date: NaiveDate) -> DaySummary {
        let mut income = 0.0;
        let mut worked = 0.0;
        let mut overtime = 0.0;
        for session in self.data.sessions.iter().filter(|s| s.start.date() == date) {
            if let Some(end) = session.end {
                income += session.income;
                worked += (end - session.start).num_seconds() as f64;
                overtime += session.overtime_secs;
            }
        }
        let log = self.data.days.get(&date).cloned().unwrap_or_default();
        DaySummary {
            date,
            income,
            worked_secs: worked,
            overtime_secs: overtime,
            done: log.done,
            skipped: log.skipped,
            hp: hp_score(
                log.done,
                log.skipped,
                overtime,
                log.health.unwrap_or(self.data.health),
            ),
            free_labor: overtime > 0.0 && self.cfg.salary.overtime_multiplier <= 0.0,
        }
    }

    pub fn stats(&self, now: NaiveDateTime) -> TodayStats {
        let s = &self.cfg.salary;
        let work_day = self.work_day(now);
        let mut closed_paid = 0.0;
        let mut closed_income = 0.0;
        let mut closed_worked = 0.0;
        for session in self
            .data
            .sessions
            .iter()
            .filter(|x| x.start.date() == work_day && x.end.is_some())
        {
            closed_paid += session.paid_secs;
            closed_income += session.income;
            closed_worked += (session.end.unwrap_or(session.start) - session.start).num_seconds() as f64;
        }

        let open = self.open_session();
        let current_paid = open.map_or(0.0, |o| salary::paid_secs(o.start, now, s));
        let current_worked = open.map_or(0.0, |o| (now - o.start).num_seconds().max(0) as f64);
        let live = |paid: f64| salary::earned(closed_paid + paid, s) - salary::earned(closed_paid, s);
        let earned_exact = closed_income + live(current_paid);
        let earned_display = closed_income + live(salary::snap(current_paid, s.tick_unit));

        let daily = salary::standard_daily_secs(s);
        let total_paid = closed_paid + current_paid;
        let overtime_secs = (total_paid - daily).max(0.0);
        let progress = if daily > 0.0 {
            (total_paid / daily).min(1.0) as f32
        } else {
            0.0
        };
        let to_end_secs = open.and_then(|o| {
            let end = salary::work_end_at(o.start.date(), s);
            (end > o.start).then(|| (end - now).num_seconds())
        });

        let month_closed: f64 = self
            .data
            .sessions
            .iter()
            .filter(|x| {
                x.end.is_some()
                    && x.start.year() == work_day.year()
                    && x.start.month() == work_day.month()
            })
            .map(|x| x.income)
            .sum();
        let month_adjust = self.month_adjust(work_day.year(), work_day.month());

        let log = self.data.days.get(&work_day).cloned().unwrap_or_default();
        TodayStats {
            work_day: Some(work_day),
            on_duty: open.is_some(),
            session_start: open.map(|o| o.start),
            earned_display,
            earned_exact,
            worked_secs: closed_worked + current_worked,
            paid_secs: total_paid,
            overtime_secs,
            progress,
            to_end_secs,
            month_earned: month_closed + live(current_paid) + month_adjust,
            month_adjust,
            done: log.done,
            skipped: log.skipped,
            hp: hp_score(log.done, log.skipped, overtime_secs, self.data.health),
        }
    }

    // ---------- 忘打卡 ----------

    /// 如果疑似忘打下班卡，返回那次上班时间。
    pub fn forgotten(&self, now: NaiveDateTime) -> Option<NaiveDateTime> {
        let start = self.open_session()?.start;
        let suspicious = start.date() < now.date()
            && now - start > Duration::hours(FORGOT_AFTER_HOURS)
            && self.data.forgot_ack != Some(start);
        suspicious.then_some(start)
    }

    pub fn ack_forgotten(&mut self, start: NaiveDateTime) {
        self.data.forgot_ack = Some(start);
        self.data_dirty = true;
    }

    // ---------- 记录修改 ----------

    pub fn edit_session(&mut self, index: usize, start: NaiveDateTime, end: NaiveDateTime) -> bool {
        if end <= start || self.data.sessions.get(index).is_none_or(|s| s.end.is_none()) {
            return false;
        }
        let old_date = self.data.sessions[index].start.date();
        self.data.sessions[index].start = start;
        self.data.sessions[index].end = Some(end);
        self.data.sessions.sort_by_key(|s| s.start);
        self.recalc_day(old_date);
        self.recalc_day(start.date());
        self.data_dirty = true;
        true
    }

    pub fn delete_session(&mut self, index: usize) -> bool {
        if self.data.sessions.get(index).is_none_or(|s| s.end.is_none()) {
            return false;
        }
        let date = self.data.sessions.remove(index).start.date();
        self.recalc_day(date);
        self.data_dirty = true;
        true
    }

    pub fn month_summary(&self, year: i32, month: u32) -> MonthSummary {
        let mut out = MonthSummary::default();
        let mut days = std::collections::BTreeSet::new();
        for session in &self.data.sessions {
            let Some(end) = session.end else { continue };
            if session.start.year() == year && session.start.month() == month {
                days.insert(session.start.date());
                out.worked_secs += (end - session.start).num_seconds() as f64;
                out.overtime_secs += session.overtime_secs;
                out.income += session.income;
            }
        }
        out.days = days.len() as u32;
        out.adjust = self.month_adjust(year, month);
        out.income += out.adjust;
        out
    }

    // ---------- 本月已赚的手动调整 ----------

    pub fn month_adjust(&self, year: i32, month: u32) -> f64 {
        self.data.month_adjust.get(&month_key(year, month)).copied().unwrap_or(0.0)
    }

    /// 把当前工作月的“本月已赚”改成 target。存的是差额，之后照常累加。
    pub fn set_month_earned(&mut self, now: NaiveDateTime, target: f64) {
        let stats = self.stats(now);
        let day = stats.work_day.unwrap_or(now.date());
        let offset = target.max(0.0) - (stats.month_earned - stats.month_adjust);
        let key = month_key(day.year(), day.month());
        if offset.abs() < 0.005 {
            self.data.month_adjust.remove(&key);
        } else {
            self.data.month_adjust.insert(key, offset);
        }
        self.data_dirty = true;
    }

    pub fn clear_month_adjust(&mut self, year: i32, month: u32) {
        if self.data.month_adjust.remove(&month_key(year, month)).is_some() {
            self.data_dirty = true;
        }
    }

    // ---------- 记账 ----------

    pub fn add_expense(&mut self, at: NaiveDateTime, amount: f64, category: &str, note: &str) -> bool {
        if !(amount > 0.0 && amount.is_finite()) {
            return false;
        }
        let id = self.new_id();
        let category = if category.trim().is_empty() { "其他" } else { category.trim() };
        self.data.expenses.push(Expense {
            id,
            at,
            amount: (amount * 100.0).round() / 100.0,
            category: category.to_owned(),
            note: note.trim().to_owned(),
        });
        self.data.expenses.sort_by_key(|x| x.at);
        self.data_dirty = true;
        true
    }

    pub fn delete_expense(&mut self, id: u64) -> Option<Expense> {
        let index = self.data.expenses.iter().position(|x| x.id == id)?;
        self.data_dirty = true;
        Some(self.data.expenses.remove(index))
    }

    /// 撤销删除。
    pub fn restore_expense(&mut self, expense: Expense) {
        if self.data.expenses.iter().any(|x| x.id == expense.id) {
            return;
        }
        self.data.expenses.push(expense);
        self.data.expenses.sort_by_key(|x| x.at);
        self.data_dirty = true;
    }

    pub fn expense_summary(&self, from: NaiveDate, to: NaiveDate) -> ExpenseSummary {
        let mut out = ExpenseSummary::default();
        let mut by: std::collections::BTreeMap<&str, f64> = std::collections::BTreeMap::new();
        for x in self.data.expenses.iter().filter(|x| x.at.date() >= from && x.at.date() <= to) {
            out.total += x.amount;
            out.count += 1;
            *by.entry(x.category.as_str()).or_default() += x.amount;
        }
        out.by_category = by.into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
        out.by_category.sort_by(|a, b| b.1.total_cmp(&a.1));
        out
    }

    pub fn month_expenses(&self, year: i32, month: u32) -> ExpenseSummary {
        let first = NaiveDate::from_ymd_opt(year, month, 1).expect("合法月份");
        let last = first + chrono::Months::new(1) - Duration::days(1);
        self.expense_summary(first, last)
    }

    pub fn expenses_csv(&self) -> String {
        let mut out = String::from("时间,金额,分类,备注\n");
        for x in &self.data.expenses {
            let note = x.note.replace('"', "\"\"");
            out += &format!("{},{:.2},{},\"{note}\"\n", x.at.format("%Y-%m-%d %H:%M"), x.amount, x.category);
        }
        out
    }

    pub fn to_csv(&self) -> String {
        let mut out = String::from("日期,上班,下班,工作时长(小时),计薪时长(小时),加班时长(小时),收入,完成提醒,跳过提醒,健康,心情\n");
        for s in &self.data.sessions {
            let Some(end) = s.end else { continue };
            let log = self.data.days.get(&s.start.date()).cloned().unwrap_or_default();
            out += &format!(
                "{},{},{},{:.2},{:.2},{:.2},{:.2},{},{},{},{}\n",
                s.start.format("%Y-%m-%d"),
                s.start.format("%H:%M:%S"),
                end.format("%Y-%m-%d %H:%M:%S"),
                (end - s.start).num_seconds() as f64 / 3600.0,
                s.paid_secs / 3600.0,
                s.overtime_secs / 3600.0,
                s.income,
                log.done,
                log.skipped,
                log.health.map_or("", Health::label),
                log.mood.map_or("", Mood::label),
            );
        }
        out
    }

    // ---------- 循环事项 ----------

    fn start_template_loop(&mut self, tpl: &ReminderTemplate, now: NaiveDateTime) {
        if self.data.loops.iter().any(|l| l.template_id == Some(tpl.id)) {
            return;
        }
        let id = self.new_id();
        self.data.loops.push(ActiveLoop {
            id,
            template_id: Some(tpl.id),
            title: tpl.title.clone(),
            note: tpl.note.clone(),
            interval_min: tpl.interval_min.max(1),
            next_fire: now + Duration::minutes(tpl.interval_min.max(1) as i64),
            paused_left: None,
            pending_since: None,
            done: 0,
            skipped: 0,
        });
    }

    /// 手动开始某个常用事项，仅上班期间有效。
    pub fn start_template(&mut self, template_id: u64, now: NaiveDateTime) -> bool {
        if !self.on_duty() {
            return false;
        }
        let Some(tpl) = self.cfg.reminders.iter().find(|t| t.id == template_id).cloned() else {
            return false;
        };
        self.start_template_loop(&tpl, now);
        self.data_dirty = true;
        true
    }

    /// 添加今日临时事项，下班后消失。
    pub fn add_temp_loop(
        &mut self,
        title: String,
        note: String,
        interval_min: u32,
        now: NaiveDateTime,
    ) -> bool {
        if !self.on_duty() {
            return false;
        }
        let interval = interval_min.max(1);
        let id = self.new_id();
        self.data.loops.push(ActiveLoop {
            id,
            template_id: None,
            title,
            note,
            interval_min: interval,
            next_fire: now + Duration::minutes(interval as i64),
            paused_left: None,
            pending_since: None,
            done: 0,
            skipped: 0,
        });
        self.data_dirty = true;
        true
    }

    pub fn add_template(
        &mut self,
        title: String,
        note: String,
        interval_min: u32,
        auto_start: bool,
    ) -> u64 {
        let id = self.new_id();
        self.cfg.reminders.push(ReminderTemplate {
            id,
            title,
            note,
            interval_min: interval_min.max(1),
            auto_start,
        });
        self.cfg_dirty = true;
        self.data_dirty = true;
        id
    }

    /// 修改模板并同步到正在运行的循环。周期变了才重新计时。
    pub fn update_template(&mut self, updated: ReminderTemplate, now: NaiveDateTime) {
        if let Some(tpl) = self.cfg.reminders.iter_mut().find(|t| t.id == updated.id) {
            *tpl = updated.clone();
            tpl.interval_min = tpl.interval_min.max(1);
            self.cfg_dirty = true;
        }
        if let Some(l) = self
            .data
            .loops
            .iter_mut()
            .find(|l| l.template_id == Some(updated.id))
        {
            l.title = updated.title;
            l.note = updated.note;
            let interval = updated.interval_min.max(1);
            if l.interval_min != interval {
                l.interval_min = interval;
                l.next_fire = now + Duration::minutes(interval as i64);
                l.paused_left = None;
            }
            self.data_dirty = true;
        }
    }

    pub fn remove_template(&mut self, id: u64) {
        self.cfg.reminders.retain(|t| t.id != id);
        self.data.loops.retain(|l| l.template_id != Some(id));
        self.cfg_dirty = true;
        self.data_dirty = true;
    }

    pub fn remove_loop(&mut self, id: u64) {
        self.data.loops.retain(|l| l.id != id);
        self.data_dirty = true;
    }

    pub fn toggle_pause(&mut self, id: u64, now: NaiveDateTime) {
        if let Some(l) = self.data.loops.iter_mut().find(|l| l.id == id) {
            match l.paused_left.take() {
                Some(left) => l.next_fire = now + Duration::seconds(left),
                None => l.paused_left = Some((l.next_fire - now).num_seconds().max(0)),
            }
            self.data_dirty = true;
        }
    }

    pub fn restart_loop(&mut self, id: u64, now: NaiveDateTime) {
        if let Some(l) = self.data.loops.iter_mut().find(|l| l.id == id) {
            l.next_fire = now + Duration::minutes(l.interval_min as i64);
            l.paused_left = None;
            l.pending_since = None;
            self.data_dirty = true;
        }
    }

    // ---------- 健康与心情 ----------

    pub fn set_health(&mut self, health: Health, now: NaiveDateTime) {
        if self.data.health == health {
            return;
        }
        self.data.health = health;
        self.data.status_log.push(StatusEntry {
            at: now,
            change: StatusChange::Health(health),
        });
        let date = self.work_day(now);
        self.day_mut(date).health = Some(health);
        if health == Health::Unwell {
            self.data.sick_prompt_pending = true;
            for plan in self.cfg.meds.clone() {
                self.start_med(&plan, now);
            }
        } else {
            self.data.sick_prompt_pending = false;
            self.data.meds.clear();
        }
        self.data_dirty = true;
    }

    pub fn set_mood(&mut self, mood: Mood, now: NaiveDateTime) {
        if self.data.mood == mood {
            return;
        }
        self.data.mood = mood;
        self.data.status_log.push(StatusEntry {
            at: now,
            change: StatusChange::Mood(mood),
        });
        let date = self.work_day(now);
        self.day_mut(date).mood = Some(mood);
        self.data_dirty = true;
    }

    fn start_med(&mut self, plan: &MedPlan, now: NaiveDateTime) {
        if self.data.meds.iter().any(|m| m.plan_id == plan.id) {
            return;
        }
        // 按间隔吃的药先问一次，定时吃的药等下一个时间点。
        let next_fire = match plan.schedule {
            MedSchedule::Interval { .. } => now,
            MedSchedule::Times { .. } => next_med_fire(plan, now),
        };
        self.data.meds.push(MedRuntime {
            plan_id: plan.id,
            next_fire,
            pending_since: None,
        });
    }

    pub fn add_med(&mut self, name: String, dose: String, note: String, schedule: MedSchedule, now: NaiveDateTime) {
        let plan = MedPlan {
            id: self.new_id(),
            name,
            dose,
            note,
            schedule,
        };
        self.cfg.meds.push(plan.clone());
        if self.data.health == Health::Unwell {
            self.data.sick_prompt_pending = false;
            self.start_med(&plan, now);
        }
        self.cfg_dirty = true;
        self.data_dirty = true;
    }

    pub fn remove_med(&mut self, id: u64) {
        self.cfg.meds.retain(|m| m.id != id);
        self.data.meds.retain(|m| m.plan_id != id);
        self.cfg_dirty = true;
        self.data_dirty = true;
    }

    // ---------- 提醒确认 ----------

    pub fn alerts(&self) -> Vec<Alert> {
        let mut out = Vec::new();
        if self.data.work_end_pending {
            out.push(Alert {
                kind: AlertKind::WorkEnd,
                title: "到点了，该下班了".to_owned(),
                body: "工作是老板的，命是自己的。".to_owned(),
            });
        }
        if self.data.sick_prompt_pending {
            let body = if self.cfg.meds.is_empty() {
                "不舒服就别硬扛。设置一下吃药提醒，我来盯着。"
            } else {
                "不舒服就别硬扛，记得按时吃药，多喝热水。"
            };
            out.push(Alert {
                kind: AlertKind::Sick,
                title: "牛马身体抱恙".to_owned(),
                body: body.to_owned(),
            });
        }
        for med in self.data.meds.iter().filter(|m| m.pending_since.is_some()) {
            if let Some(plan) = self.cfg.meds.iter().find(|p| p.id == med.plan_id) {
                out.push(Alert {
                    kind: AlertKind::Med(plan.id),
                    title: format!("该吃药了：{}", plan.name),
                    body: med_body(plan),
                });
            }
        }
        for l in self.data.loops.iter().filter(|l| l.pending_since.is_some()) {
            out.push(Alert {
                kind: AlertKind::Loop(l.id),
                title: l.title.clone(),
                body: l.note.clone(),
            });
        }
        out
    }

    pub fn ack(&mut self, kind: AlertKind, action: AckAction, now: NaiveDateTime) {
        let snooze = Duration::minutes(self.cfg.snooze_minutes.max(1) as i64);
        match kind {
            AlertKind::Loop(id) => {
                let date = self.work_day(now);
                let Some(l) = self.data.loops.iter_mut().find(|l| l.id == id) else {
                    return;
                };
                l.pending_since = None;
                l.next_fire = match action {
                    AckAction::Snooze => now + snooze,
                    _ => now + Duration::minutes(l.interval_min as i64),
                };
                match action {
                    AckAction::Done => {
                        l.done += 1;
                        self.day_mut(date).done += 1;
                    }
                    AckAction::Skip => {
                        l.skipped += 1;
                        self.day_mut(date).skipped += 1;
                    }
                    AckAction::Snooze => {}
                }
            }
            AlertKind::Med(plan_id) => {
                let plan = self.cfg.meds.iter().find(|p| p.id == plan_id).cloned();
                let date = self.work_day(now);
                let Some(med) = self.data.meds.iter_mut().find(|m| m.plan_id == plan_id) else {
                    return;
                };
                med.pending_since = None;
                med.next_fire = match (action, plan) {
                    (AckAction::Snooze, _) | (_, None) => now + snooze,
                    (_, Some(plan)) => next_med_fire(&plan, now),
                };
                if action == AckAction::Done {
                    self.day_mut(date).meds_taken += 1;
                }
            }
            AlertKind::WorkEnd => {
                self.data.work_end_pending = false;
                if action == AckAction::Snooze {
                    // 再卷一会：半小时后再催一次。
                    self.data.work_end_notified = None;
                    self.data.work_end_snooze_until = Some(now + Duration::minutes(WORK_END_SNOOZE_MIN));
                }
            }
            AlertKind::Sick => self.data.sick_prompt_pending = false,
        }
        self.data_dirty = true;
    }

    // ---------- 心跳 ----------

    pub fn tick(&mut self, now: NaiveDateTime, env: TickEnv) -> Vec<Notice> {
        // 按毫秒算：心跳只隔 250 毫秒，按秒取整会把锁屏时间全部丢掉。
        let gap_ms = self
            .data
            .last_seen
            .map_or(0, |last| (now - last).num_milliseconds());
        let long_gap = gap_ms >= AWAY_GAP_SECS * 1000;
        self.data.last_seen = Some(now);
        // last_seen 每 30 秒落盘一次，重启时才能算出程序停了多久。
        if self
            .last_seen_saved
            .is_none_or(|saved| now - saved >= Duration::seconds(30))
        {
            self.last_seen_saved = Some(now);
            self.data_dirty = true;
        }

        // 锁屏、休眠或程序没运行的这段时间，活动类倒计时顺延，不算作没动。
        if self.cfg.pause_when_away && gap_ms > 0 && (env.away || long_gap) {
            let shift = Duration::milliseconds(gap_ms);
            for l in self
                .data
                .loops
                .iter_mut()
                .filter(|l| l.paused_left.is_none() && l.pending_since.is_none())
            {
                l.next_fire += shift;
            }
        }

        let mut notices = Vec::new();
        if env.quiet {
            return notices;
        }
        let style = self.cfg.alert_style;
        let toast = matches!(style, AlertStyle::Toast | AlertStyle::Both);
        let sound = self.cfg.sound && !env.away;
        let card = !matches!(style, AlertStyle::Toast);

        // 疑似忘打卡时不再催活动，等用户处理完。
        if self.forgotten(now).is_none() {
            for l in self.data.loops.iter_mut().filter(|l| {
                l.paused_left.is_none() && l.pending_since.is_none() && now >= l.next_fire
            }) {
                if env.away && self.cfg.pause_when_away {
                    continue;
                }
                notices.push(Notice {
                    title: l.title.clone(),
                    body: l.note.clone(),
                    toast,
                    sound,
                });
                if card {
                    l.pending_since = Some(now);
                } else {
                    l.next_fire = now + Duration::minutes(l.interval_min as i64);
                }
                self.data_dirty = true;
            }

            if let Some(open) = self.open_session() {
                let start = open.start;
                let end = salary::work_end_at(start.date(), &self.cfg.salary);
                let notified = self.data.work_end_notified == Some(start.date());
                let snoozed = self.data.work_end_snooze_until.is_some_and(|until| now < until);
                if self.cfg.work_end_reminder && start < end && now >= end && !notified && !snoozed {
                    self.data.work_end_notified = Some(start.date());
                    if card {
                        self.data.work_end_pending = true;
                    }
                    notices.push(Notice {
                        title: "到点了，该下班了".to_owned(),
                        body: "工作是老板的，命是自己的。".to_owned(),
                        toast,
                        sound,
                    });
                    self.data_dirty = true;
                }
            }
        }

        // 吃药提醒不看打卡状态，锁屏时也照常到期，解锁就能看到。
        let plans = self.cfg.meds.clone();
        for med in self.data.meds.iter_mut().filter(|m| m.pending_since.is_none() && now >= m.next_fire) {
            let Some(plan) = plans.iter().find(|p| p.id == med.plan_id) else {
                continue;
            };
            notices.push(Notice {
                title: format!("该吃药了：{}", plan.name),
                body: med_body(plan),
                toast,
                sound,
            });
            if card {
                med.pending_since = Some(now);
            } else {
                med.next_fire = next_med_fire(plan, now);
            }
            self.data_dirty = true;
        }
        notices
    }
}

fn med_body(plan: &MedPlan) -> String {
    match (plan.dose.is_empty(), plan.note.is_empty()) {
        (false, false) => format!("{} · {}", plan.dose, plan.note),
        (false, true) => plan.dose.clone(),
        (true, false) => plan.note.clone(),
        (true, true) => "按时吃药，早点好起来。".to_owned(),
    }
}

/// 下一次吃药时间。定时方案找今天剩下的时间点，没有就取明天第一个。
pub fn next_med_fire(plan: &MedPlan, now: NaiveDateTime) -> NaiveDateTime {
    match &plan.schedule {
        MedSchedule::Interval { minutes } => now + Duration::minutes((*minutes).max(1) as i64),
        MedSchedule::Times { times } => {
            let later_today = times
                .iter()
                .map(|t| now.date().and_time(*t))
                .filter(|at| *at > now)
                .min();
            later_today.unwrap_or_else(|| match times.iter().min() {
                Some(first) => (now.date() + Duration::days(1)).and_time(*first),
                None => now + Duration::hours(24),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(d: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap().and_time(hm(h, m))
    }

    fn engine() -> Engine {
        let mut cfg = Config::default();
        cfg.salary.monthly = 21750.0;
        Engine::new(cfg, Data::default())
    }

    fn tick(e: &mut Engine, now: NaiveDateTime) -> Vec<Notice> {
        e.tick(now, TickEnv::default())
    }

    /// 连续心跳到 target，避免被当成休眠。
    fn walk(e: &mut Engine, from: NaiveDateTime, to: NaiveDateTime) -> usize {
        let mut count = 0;
        let mut now = from;
        while now <= to {
            count += tick(e, now).len();
            now += Duration::seconds(30);
        }
        count
    }

    #[test]
    fn clock_in_twice_is_rejected() {
        let mut e = engine();
        assert!(e.clock_in(dt(2, 9, 0)));
        assert!(!e.clock_in(dt(2, 9, 1)));
        assert_eq!(e.data.sessions.len(), 1);
    }

    #[test]
    fn clock_out_without_clock_in_does_nothing() {
        assert!(engine().clock_out(dt(2, 18, 0)).is_none());
    }

    #[test]
    fn full_day_settles_to_daily_wage() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        let summary = e.clock_out(dt(2, 18, 0)).unwrap();
        assert!((summary.income - 1000.0).abs() < 1e-6);
        assert_eq!(summary.worked_secs, 9.0 * 3600.0);
        assert_eq!(summary.overtime_secs, 0.0);
    }

    #[test]
    fn two_sessions_share_standard_hours() {
        let mut e = engine();
        e.cfg.salary.overtime_multiplier = 2.0;
        e.clock_in(dt(2, 9, 0));
        e.clock_out(dt(2, 18, 0));
        e.clock_in(dt(2, 19, 0));
        let summary = e.clock_out(dt(2, 20, 0)).unwrap();
        assert!((summary.income - (1000.0 + 2.0 * 125.0)).abs() < 1e-6);
        assert_eq!(summary.overtime_secs, 3600.0);
    }

    #[test]
    fn live_stats_match_settlement() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        let stats = e.stats(dt(2, 11, 0));
        assert!((stats.earned_exact - 250.0).abs() < 1e-6);
        assert!((stats.progress - 0.25).abs() < 1e-6);
        assert_eq!(stats.to_end_secs, Some(7 * 3600));
        let settled = e.clock_out(dt(2, 11, 0)).unwrap();
        assert!((settled.income - stats.earned_exact).abs() < 1e-6);
    }

    #[test]
    fn display_snaps_but_settlement_is_exact() {
        let mut e = engine();
        e.cfg.salary.tick_unit = TickUnit::Hour;
        e.clock_in(dt(2, 9, 0));
        let now = dt(2, 10, 59);
        let stats = e.stats(now);
        assert!((stats.earned_display - 125.0).abs() < 1e-6);
        assert!(stats.earned_exact > 240.0);
        let settled = e.clock_out(now).unwrap();
        assert!((settled.income - stats.earned_exact).abs() < 1e-6);
    }

    #[test]
    fn changing_salary_does_not_rewrite_history() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        e.clock_out(dt(2, 18, 0));
        e.cfg.salary.monthly = 1.0;
        assert!((e.summary_of(dt(2, 0, 0).date()).income - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn clock_in_starts_auto_templates_only() {
        let mut e = engine();
        e.add_template("护眼".into(), String::new(), 20, false);
        e.clock_in(dt(2, 9, 0));
        assert_eq!(e.data.loops.len(), 2);
        assert_eq!(e.data.loops[0].next_fire, dt(2, 9, 45));
    }

    #[test]
    fn clock_out_clears_loops_but_keeps_templates() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        e.add_temp_loop("给客户回电".into(), "记得说好话".into(), 30, dt(2, 9, 1));
        assert_eq!(e.data.loops.len(), 3);
        e.clock_out(dt(2, 18, 0));
        assert!(e.data.loops.is_empty());
        assert_eq!(e.cfg.reminders.len(), 2);
        e.clock_in(dt(3, 9, 0));
        assert_eq!(e.data.loops.len(), 2);
        assert!(e.data.loops.iter().all(|l| l.title != "给客户回电"));
    }

    #[test]
    fn temp_loop_needs_duty() {
        let mut e = engine();
        assert!(!e.add_temp_loop("x".into(), String::new(), 5, dt(2, 9, 0)));
    }

    #[test]
    fn reminder_fires_once_and_waits_for_ack() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        let notices = walk(&mut e, dt(2, 9, 0), dt(2, 9, 50));
        assert_eq!(notices, 1, "喝水到点只响一次，没确认不叠加");
        assert_eq!(e.alerts().len(), 1);

        let id = e.data.loops[0].id;
        e.ack(AlertKind::Loop(id), AckAction::Done, dt(2, 9, 50));
        assert!(e.alerts().is_empty());
        assert_eq!(e.data.loops[0].next_fire, dt(2, 10, 35));
        assert_eq!(e.data.days[&dt(2, 0, 0).date()].done, 1);
    }

    #[test]
    fn snooze_and_skip_rearm_correctly() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        walk(&mut e, dt(2, 9, 0), dt(2, 9, 46));
        let id = e.data.loops[0].id;
        e.ack(AlertKind::Loop(id), AckAction::Snooze, dt(2, 9, 46));
        assert_eq!(e.data.loops[0].next_fire, dt(2, 9, 51));
        assert!(!e.data.days.contains_key(&dt(2, 0, 0).date()), "稍后不计入完成或跳过");

        walk(&mut e, dt(2, 9, 46), dt(2, 9, 52));
        e.ack(AlertKind::Loop(id), AckAction::Skip, dt(2, 9, 52));
        assert_eq!(e.data.loops[0].next_fire, dt(2, 10, 37));
        assert_eq!(e.data.days[&dt(2, 0, 0).date()].skipped, 1);
    }

    #[test]
    fn toast_only_style_rearms_without_card() {
        let mut e = engine();
        e.cfg.alert_style = AlertStyle::Toast;
        e.clock_in(dt(2, 9, 0));
        let count = walk(&mut e, dt(2, 9, 0), dt(2, 10, 31));
        assert_eq!(count, 3, "喝水 9:45、10:30，活动 10:00");
        assert!(e.alerts().is_empty());
    }

    #[test]
    fn sleep_gap_shifts_countdown() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        tick(&mut e, dt(2, 9, 0));
        // 电脑睡了一小时，醒来不应该立刻喝水。
        assert!(tick(&mut e, dt(2, 10, 0)).is_empty());
        assert_eq!(e.data.loops[0].next_fire, dt(2, 10, 45));
    }

    #[test]
    fn locked_session_pauses_countdown() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        tick(&mut e, dt(2, 9, 0));
        let before = e.data.loops[0].next_fire;
        let away = TickEnv {
            away: true,
            quiet: false,
        };
        assert!(e.tick(dt(2, 9, 0) + Duration::seconds(30), away).is_empty());
        assert_eq!(e.data.loops[0].next_fire, before + Duration::seconds(30));
    }

    #[test]
    fn locked_time_accumulates_with_real_heartbeat() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        tick(&mut e, dt(2, 9, 0));
        let before = e.data.loops[0].next_fire;
        let away = TickEnv {
            away: true,
            quiet: false,
        };
        let mut now = dt(2, 9, 0);
        for _ in 0..40 {
            now += Duration::milliseconds(250);
            e.tick(now, away);
        }
        assert_eq!(e.data.loops[0].next_fire, before + Duration::seconds(10));
    }

    #[test]
    fn last_seen_is_persisted_every_half_minute() {
        let mut e = engine();
        let mut now = dt(2, 9, 0);
        let mut saves = 0;
        for _ in 0..(4 * 65) {
            e.data_dirty = false;
            tick(&mut e, now);
            saves += e.data_dirty as u32;
            now += Duration::milliseconds(250);
        }
        assert_eq!(saves, 3, "0 秒、30 秒、60 秒各一次");
    }

    #[test]
    fn pause_when_away_can_be_disabled() {
        let mut e = engine();
        e.cfg.pause_when_away = false;
        e.clock_in(dt(2, 9, 0));
        tick(&mut e, dt(2, 9, 0));
        assert_eq!(tick(&mut e, dt(2, 10, 0)).len(), 2, "两个默认循环都已到期");
    }

    #[test]
    fn quiet_mode_defers_until_it_ends() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        walk(&mut e, dt(2, 9, 0), dt(2, 9, 45) - Duration::seconds(30));
        let quiet = TickEnv {
            away: false,
            quiet: true,
        };
        assert!(e.tick(dt(2, 9, 45) + Duration::seconds(10), quiet).is_empty());
        assert_eq!(tick(&mut e, dt(2, 9, 45) + Duration::seconds(40)).len(), 1);
    }

    #[test]
    fn manual_pause_freezes_remaining_time() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        let id = e.data.loops[0].id;
        e.toggle_pause(id, dt(2, 9, 15));
        assert_eq!(e.data.loops[0].paused_left, Some(30 * 60));
        assert_eq!(walk(&mut e, dt(2, 9, 15), dt(2, 9, 50)), 0, "暂停的喝水过了 9:45 也不响");
        e.toggle_pause(id, dt(2, 10, 0));
        assert_eq!(e.data.loops[0].next_fire, dt(2, 10, 30));
    }

    #[test]
    fn unwell_starts_meds_and_survives_clock_out() {
        let mut e = engine();
        e.add_med(
            "布洛芬".into(),
            "1 粒".into(),
            "饭后".into(),
            MedSchedule::Interval { minutes: 240 },
            dt(2, 8, 0),
        );
        assert!(e.data.meds.is_empty(), "健康时不启动吃药提醒");
        e.clock_in(dt(2, 9, 0));
        e.set_health(Health::Unwell, dt(2, 10, 0));
        assert!(e.alerts().iter().any(|a| a.kind == AlertKind::Sick));
        assert!(tick(&mut e, dt(2, 10, 0)).iter().any(|n| n.title.contains("布洛芬")));
        let plan_id = e.cfg.meds[0].id;
        e.ack(AlertKind::Med(plan_id), AckAction::Done, dt(2, 10, 1));
        assert_eq!(e.data.meds[0].next_fire, dt(2, 14, 1));
        assert_eq!(e.data.days[&dt(2, 0, 0).date()].meds_taken, 1);

        e.clock_out(dt(2, 12, 0));
        assert_eq!(e.data.meds.len(), 1, "下班后吃药提醒继续");

        e.set_health(Health::Healthy, dt(2, 13, 0));
        assert!(e.data.meds.is_empty(), "恢复健康后停止");
    }

    #[test]
    fn scheduled_med_waits_for_next_time_point() {
        let plan = MedPlan {
            id: 9,
            name: "感冒灵".into(),
            dose: String::new(),
            note: String::new(),
            schedule: MedSchedule::Times {
                times: vec![hm(8, 0), hm(12, 0), hm(20, 0)],
            },
        };
        assert_eq!(next_med_fire(&plan, dt(2, 9, 0)), dt(2, 12, 0));
        assert_eq!(next_med_fire(&plan, dt(2, 12, 0)), dt(2, 20, 0));
        assert_eq!(next_med_fire(&plan, dt(2, 21, 0)), dt(3, 8, 0));
    }

    #[test]
    fn adding_med_while_unwell_starts_it_immediately() {
        let mut e = engine();
        e.set_health(Health::Unwell, dt(2, 10, 0));
        assert!(e.alerts()[0].body.contains("设置"));
        e.add_med("药".into(), String::new(), String::new(), MedSchedule::Interval { minutes: 60 }, dt(2, 10, 1));
        assert_eq!(e.data.meds.len(), 1);
        assert_eq!(tick(&mut e, dt(2, 10, 1)).len(), 1);
    }

    #[test]
    fn removing_med_stops_its_alert() {
        let mut e = engine();
        e.add_med("药".into(), String::new(), String::new(), MedSchedule::Interval { minutes: 60 }, dt(2, 10, 0));
        e.set_health(Health::Unwell, dt(2, 10, 0));
        tick(&mut e, dt(2, 10, 0));
        let id = e.cfg.meds[0].id;
        e.remove_med(id);
        assert!(!e.alerts().iter().any(|a| a.kind == AlertKind::Med(id)));
    }

    #[test]
    fn status_changes_are_logged_once() {
        let mut e = engine();
        e.set_mood(Mood::Broken, dt(2, 10, 0));
        e.set_mood(Mood::Broken, dt(2, 10, 1));
        assert_eq!(e.data.status_log.len(), 1);
        assert_eq!(e.data.days[&dt(2, 0, 0).date()].mood, Some(Mood::Broken));
    }

    #[test]
    fn work_end_alert_fires_once_per_day() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        let id: Vec<u64> = e.data.loops.iter().map(|l| l.id).collect();
        id.iter().for_each(|i| e.remove_loop(*i));
        assert_eq!(walk(&mut e, dt(2, 17, 59), dt(2, 18, 2)), 1);
        assert!(e.alerts().iter().any(|a| a.kind == AlertKind::WorkEnd));
        e.ack(AlertKind::WorkEnd, AckAction::Skip, dt(2, 18, 3));
        assert_eq!(walk(&mut e, dt(2, 18, 3), dt(2, 18, 10)), 0);
    }

    #[test]
    fn work_end_snooze_waits_half_an_hour() {
        let mut e = engine();
        e.cfg.reminders.clear();
        e.clock_in(dt(2, 9, 0));
        assert_eq!(walk(&mut e, dt(2, 18, 0), dt(2, 18, 0)), 1);
        e.ack(AlertKind::WorkEnd, AckAction::Snooze, dt(2, 18, 0));
        assert_eq!(walk(&mut e, dt(2, 18, 0), dt(2, 18, 29)), 0);
        assert_eq!(walk(&mut e, dt(2, 18, 29), dt(2, 18, 31)), 1);
    }

    #[test]
    fn overtime_clock_in_after_end_does_not_nag() {
        let mut e = engine();
        e.cfg.reminders.clear();
        e.clock_in(dt(2, 19, 0));
        assert!(tick(&mut e, dt(2, 19, 1)).is_empty());
        assert_eq!(e.stats(dt(2, 19, 1)).to_end_secs, None);
    }

    #[test]
    fn forgotten_clock_out_is_detected_and_silences_loops() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        assert_eq!(e.forgotten(dt(2, 23, 0)), None);
        assert_eq!(e.forgotten(dt(3, 9, 0)), Some(dt(2, 9, 0)));
        assert!(tick(&mut e, dt(3, 9, 0)).is_empty());

        let summary = e.clock_out(dt(2, 18, 0)).unwrap();
        assert!((summary.income - 1000.0).abs() < 1e-6);
        assert_eq!(e.forgotten(dt(3, 9, 0)), None);
    }

    #[test]
    fn night_shift_is_not_flagged_forgotten() {
        let mut e = engine();
        e.clock_in(dt(2, 22, 0));
        assert_eq!(e.forgotten(dt(3, 6, 0)), None);
    }

    #[test]
    fn ack_forgotten_stops_asking_and_keeps_work_day() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        e.ack_forgotten(dt(2, 9, 0));
        assert_eq!(e.forgotten(dt(3, 9, 0)), None);
        assert_eq!(e.stats(dt(3, 9, 0)).work_day, Some(dt(2, 0, 0).date()));
    }

    #[test]
    fn edit_session_recalculates_income() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        e.clock_out(dt(2, 13, 0));
        assert!(e.edit_session(0, dt(2, 9, 0), dt(2, 18, 0)));
        assert!((e.data.sessions[0].income - 1000.0).abs() < 1e-6);
        assert!(!e.edit_session(0, dt(2, 18, 0), dt(2, 9, 0)), "结束早于开始要拒绝");
    }

    #[test]
    fn edit_moves_session_across_days() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        e.clock_out(dt(2, 18, 0));
        e.clock_in(dt(3, 9, 0));
        e.clock_out(dt(3, 18, 0));
        assert!(e.edit_session(1, dt(4, 9, 0), dt(4, 13, 0)));
        assert!((e.summary_of(dt(3, 0, 0).date()).income).abs() < 1e-6);
        assert!((e.summary_of(dt(4, 0, 0).date()).income - 375.0).abs() < 1e-6, "9 点到 13 点扣掉午休是 3 小时");
    }

    #[test]
    fn open_session_cannot_be_edited_or_deleted() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        assert!(!e.edit_session(0, dt(2, 8, 0), dt(2, 18, 0)));
        assert!(!e.delete_session(0));
    }

    #[test]
    fn delete_session_recalculates_the_rest() {
        let mut e = engine();
        e.cfg.salary.overtime_multiplier = 2.0;
        e.clock_in(dt(2, 9, 0));
        e.clock_out(dt(2, 18, 0));
        e.clock_in(dt(2, 19, 0));
        e.clock_out(dt(2, 20, 0));
        assert!(e.delete_session(0));
        assert!((e.data.sessions[0].income - 125.0).abs() < 1e-6, "只剩一段时不再算加班");
    }

    #[test]
    fn month_summary_counts_distinct_days() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        e.clock_out(dt(2, 12, 0));
        e.clock_in(dt(2, 13, 0));
        e.clock_out(dt(2, 18, 0));
        e.clock_in(dt(3, 9, 0));
        e.clock_out(dt(3, 18, 0));
        let summary = e.month_summary(2026, 11);
        assert_eq!(summary.days, 2);
        assert!((summary.income - 2000.0).abs() < 1e-6);
        assert_eq!(e.month_summary(2026, 10).days, 0);
    }

    #[test]
    fn month_earned_can_be_set_and_keeps_growing() {
        let mut e = engine();
        e.clock_in(dt(3, 9, 0));
        // 11 月 1 日起算工资，3 日才装软件：上午 10 点时实际已赚 2100。
        e.set_month_earned(dt(3, 10, 0), 2100.0);
        let stats = e.stats(dt(3, 10, 0));
        assert!((stats.month_earned - 2100.0).abs() < 1e-6);
        assert!((stats.month_adjust - 1975.0).abs() < 1e-6, "调整额 = 2100 - 已打卡的 125");
        // 之后照常累加。
        let later = e.stats(dt(3, 11, 0));
        assert!((later.month_earned - 2225.0).abs() < 1e-6);
        e.clock_out(dt(3, 18, 0));
        let summary = e.month_summary(2026, 11);
        assert!((summary.income - 2975.0).abs() < 1e-6, "当天 1000 + 调整 1975");
        assert!((summary.adjust - 1975.0).abs() < 1e-6);
        assert_eq!(e.month_adjust(2026, 12), 0.0, "下个月不受影响");
    }

    #[test]
    fn month_adjust_resets_when_target_matches_records() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        e.clock_out(dt(2, 18, 0));
        e.set_month_earned(dt(2, 19, 0), 500.0);
        assert!((e.month_adjust(2026, 11) + 500.0).abs() < 1e-6, "也可以往下调");
        e.set_month_earned(dt(2, 19, 0), 1000.0);
        assert!(e.data.month_adjust.is_empty());
        e.set_month_earned(dt(2, 19, 0), 1200.0);
        e.clear_month_adjust(2026, 11);
        assert!((e.stats(dt(2, 19, 0)).month_earned - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn expenses_add_delete_restore_and_summarise() {
        let mut e = engine();
        assert!(!e.add_expense(dt(2, 12, 0), 0.0, "餐饮", ""), "金额必须大于 0");
        assert!(!e.add_expense(dt(2, 12, 0), f64::NAN, "餐饮", ""));
        assert!(e.add_expense(dt(2, 12, 0), 25.456, "餐饮", " 午饭 "));
        assert!(e.add_expense(dt(3, 8, 0), 4.0, "交通", ""));
        assert!(e.add_expense(dt(1, 19, 0), 40.0, "餐饮", ""));
        assert!(e.add_expense(NaiveDate::from_ymd_opt(2026, 10, 31).unwrap().and_time(hm(20, 0)), 99.0, "购物", ""));
        assert!(e.add_expense(dt(4, 9, 0), 1.0, " ", ""));
        assert_eq!(e.data.expenses[0].category, "购物", "按时间排序");
        assert_eq!(e.data.expenses.last().unwrap().category, "其他", "空分类归到其他");
        let lunch = e.data.expenses.iter().find(|x| x.note == "午饭").unwrap().clone();
        assert_eq!(lunch.amount, 25.46);

        let month = e.month_expenses(2026, 11);
        assert_eq!(month.count, 4);
        assert!((month.total - 70.46).abs() < 1e-9);
        assert_eq!(month.by_category[0].0, "餐饮");
        assert!((month.by_category[0].1 - 65.46).abs() < 1e-9);
        assert_eq!(e.month_expenses(2026, 10).count, 1);
        assert_eq!(e.month_expenses(2026, 12).count, 0);

        let removed = e.delete_expense(lunch.id).unwrap();
        assert!(e.delete_expense(lunch.id).is_none());
        assert_eq!(e.month_expenses(2026, 11).count, 3);
        e.restore_expense(removed.clone());
        e.restore_expense(removed);
        assert_eq!(e.month_expenses(2026, 11).count, 4, "撤销只恢复一次");
        assert!(e.expenses_csv().contains("2026-11-02 12:00,25.46,餐饮,\"午饭\""));
    }

    #[test]
    fn hp_changes_with_behaviour() {
        assert_eq!(hp_score(0, 0, 0.0, Health::Healthy), 100);
        assert_eq!(hp_score(5, 0, 0.0, Health::Healthy), 100);
        assert_eq!(hp_score(0, 4, 0.0, Health::Healthy), 80);
        assert_eq!(hp_score(0, 0, 3600.0, Health::Healthy), 94);
        assert_eq!(hp_score(0, 0, 0.0, Health::Unwell), 80);
        assert_eq!(hp_score(0, 100, 0.0, Health::Unwell), 0);
    }

    #[test]
    fn csv_contains_finished_sessions_only() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        e.clock_out(dt(2, 18, 0));
        e.clock_in(dt(3, 9, 0));
        let csv = e.to_csv();
        assert_eq!(csv.lines().count(), 2);
        assert!(csv.contains("2026-11-02,09:00:00,2026-11-02 18:00:00,9.00,8.00,0.00,1000.00"));
    }

    #[test]
    fn new_ids_do_not_collide_with_default_templates() {
        let mut e = engine();
        let id = e.add_template("x".into(), String::new(), 5, false);
        assert!(id > 2);
    }

    #[test]
    fn template_edit_syncs_to_running_loop() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        let mut tpl = e.cfg.reminders[0].clone();
        tpl.title = "喝热水".into();
        e.update_template(tpl.clone(), dt(2, 9, 10));
        assert_eq!(e.data.loops[0].title, "喝热水");
        assert_eq!(e.data.loops[0].next_fire, dt(2, 9, 45), "周期没变不重新计时");
        tpl.interval_min = 10;
        e.update_template(tpl, dt(2, 9, 10));
        assert_eq!(e.data.loops[0].next_fire, dt(2, 9, 20));
    }

    #[test]
    fn data_survives_json_round_trip() {
        let mut e = engine();
        e.clock_in(dt(2, 9, 0));
        e.set_health(Health::Unwell, dt(2, 9, 1));
        let text = serde_json::to_string(&e.data).unwrap();
        let back: Data = serde_json::from_str(&text).unwrap();
        assert_eq!(back, e.data);
        let old: Config = serde_json::from_str(r#"{"sound":false}"#).unwrap();
        assert!(!old.sound);
        assert_eq!(old.salary.paid_days, 21.75);
    }
}
