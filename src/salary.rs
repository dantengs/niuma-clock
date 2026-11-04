//! 工资计算。全部是纯函数，时间由调用方传入。
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};

use crate::model::{SalaryConfig, TickUnit};

fn secs_between(a: NaiveDateTime, b: NaiveDateTime) -> f64 {
    (b - a).num_milliseconds() as f64 / 1000.0
}

/// 打卡区间内应计薪的秒数：总时长减去与每天午休的重叠部分。
pub fn paid_secs(start: NaiveDateTime, end: NaiveDateTime, s: &SalaryConfig) -> f64 {
    if end <= start {
        return 0.0;
    }
    let mut total = secs_between(start, end);
    if s.lunch_unpaid && s.lunch_start < s.lunch_end {
        let mut day = start.date();
        while day <= end.date() {
            let lunch_a = day.and_time(s.lunch_start).max(start);
            let lunch_b = day.and_time(s.lunch_end).min(end);
            if lunch_b > lunch_a {
                total -= secs_between(lunch_a, lunch_b);
            }
            day += Duration::days(1);
        }
    }
    total.max(0.0)
}

/// 下班时刻。下班时间不晚于上班时间视为跨夜班。
pub fn work_end_at(work_day: NaiveDate, s: &SalaryConfig) -> NaiveDateTime {
    let end = work_day.and_time(s.work_end);
    if s.work_end <= s.work_start {
        end + Duration::days(1)
    } else {
        end
    }
}

/// 一天的标准计薪秒数。
pub fn standard_daily_secs(s: &SalaryConfig) -> f64 {
    let base = NaiveDate::from_ymd_opt(2024, 1, 1).expect("固定日期");
    paid_secs(base.and_time(s.work_start), work_end_at(base, s), s)
}

/// 基础费率，元/秒。配置不完整时为 0。
pub fn rate_per_sec(s: &SalaryConfig) -> f64 {
    let daily = standard_daily_secs(s);
    if s.monthly <= 0.0 || s.paid_days <= 0.0 || daily <= 0.0 {
        return 0.0;
    }
    s.monthly / s.paid_days / daily
}

/// 当天累计计薪 `paid` 秒时的总收入，超过标准工时的部分按加班倍率。
pub fn earned(paid: f64, s: &SalaryConfig) -> f64 {
    let daily = standard_daily_secs(s);
    let regular = paid.min(daily).max(0.0);
    let overtime = (paid - daily).max(0.0);
    rate_per_sec(s) * (regular + overtime * s.overtime_multiplier.max(0.0))
}

/// 按跳字粒度向下取整。只影响显示，结算用实际秒数。
pub fn snap(secs: f64, unit: TickUnit) -> f64 {
    (secs / unit.secs()).floor() * unit.secs()
}

/// 这一跳增加多少钱。
pub fn tick_amount(s: &SalaryConfig) -> f64 {
    rate_per_sec(s) * s.tick_unit.secs()
}

/// 到下一个发薪日还有几天，当天为 0。发薪日超过当月天数时取月末。
pub fn days_to_payday(today: NaiveDate, payday: u32) -> i64 {
    let in_month = |year: i32, month: u32| {
        let next = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .expect("合法月份");
        let last = (next - Duration::days(1)).day();
        NaiveDate::from_ymd_opt(year, month, payday.clamp(1, last)).expect("合法日期")
    };
    let this = in_month(today.year(), today.month());
    if this >= today {
        return (this - today).num_days();
    }
    let (y, m) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    (in_month(y, m) - today).num_days()
}

/// 解析 "08:00, 12:30 20:00" 这样的时间列表，去重排序。
pub fn parse_times(text: &str) -> Option<Vec<NaiveTime>> {
    let mut times = Vec::new();
    for part in text
        .split(|c: char| c == ',' || c == '，' || c == '、' || c.is_whitespace())
        .filter(|p| !p.is_empty())
    {
        times.push(parse_time(part)?);
    }
    times.sort();
    times.dedup();
    (!times.is_empty()).then_some(times)
}

pub fn parse_time(text: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(text.trim().replace('：', ":").as_str(), "%H:%M").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::hm;

    fn dt(d: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap().and_time(hm(h, m))
    }

    fn cfg() -> SalaryConfig {
        SalaryConfig {
            monthly: 21750.0,
            ..SalaryConfig::default()
        }
    }

    #[test]
    fn standard_day_excludes_lunch() {
        assert_eq!(standard_daily_secs(&cfg()), 8.0 * 3600.0);
        let paid_lunch = SalaryConfig {
            lunch_unpaid: false,
            ..cfg()
        };
        assert_eq!(standard_daily_secs(&paid_lunch), 9.0 * 3600.0);
    }

    #[test]
    fn full_day_pays_exactly_daily_wage() {
        let s = cfg();
        let paid = paid_secs(dt(2, 9, 0), dt(2, 18, 0), &s);
        assert!((earned(paid, &s) - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn lunch_overlap_is_partial() {
        let s = cfg();
        assert_eq!(paid_secs(dt(2, 11, 30), dt(2, 12, 30), &s), 1800.0);
        assert_eq!(paid_secs(dt(2, 12, 10), dt(2, 12, 50), &s), 0.0);
        assert_eq!(paid_secs(dt(2, 12, 30), dt(2, 13, 30), &s), 1800.0);
    }

    #[test]
    fn multi_day_session_subtracts_each_lunch() {
        let s = cfg();
        assert_eq!(
            paid_secs(dt(2, 9, 0), dt(3, 18, 0), &s),
            33.0 * 3600.0 - 2.0 * 3600.0
        );
    }

    #[test]
    fn reversed_range_is_zero() {
        assert_eq!(paid_secs(dt(2, 18, 0), dt(2, 9, 0), &cfg()), 0.0);
    }

    #[test]
    fn overtime_multiplier_applies_after_standard_hours() {
        let double = SalaryConfig {
            overtime_multiplier: 2.0,
            ..cfg()
        };
        let hour = 1000.0 / 8.0;
        assert!((earned(9.0 * 3600.0, &double) - (1000.0 + 2.0 * hour)).abs() < 1e-6);
        let free = SalaryConfig {
            overtime_multiplier: 0.0,
            ..cfg()
        };
        assert!((earned(12.0 * 3600.0, &free) - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn overnight_shift_has_positive_day() {
        let s = SalaryConfig {
            work_start: hm(22, 0),
            work_end: hm(6, 0),
            ..cfg()
        };
        assert_eq!(standard_daily_secs(&s), 8.0 * 3600.0);
        assert_eq!(work_end_at(dt(2, 0, 0).date(), &s), dt(3, 6, 0));
    }

    #[test]
    fn invalid_config_has_zero_rate() {
        assert_eq!(rate_per_sec(&SalaryConfig::default()), 0.0);
        let no_days = SalaryConfig {
            paid_days: 0.0,
            ..cfg()
        };
        assert_eq!(rate_per_sec(&no_days), 0.0);
        let empty_day = SalaryConfig {
            work_start: hm(12, 0),
            work_end: hm(13, 0),
            ..cfg()
        };
        assert_eq!(rate_per_sec(&empty_day), 0.0);
    }

    #[test]
    fn snap_floors_to_unit() {
        assert_eq!(snap(61.9, TickUnit::Second), 61.0);
        assert_eq!(snap(61.9, TickUnit::Minute), 60.0);
        assert_eq!(snap(3599.0, TickUnit::Hour), 0.0);
        assert_eq!(snap(3600.0, TickUnit::Hour), 3600.0);
    }

    #[test]
    fn payday_rolls_over_and_clamps() {
        let d = |y, m, day| NaiveDate::from_ymd_opt(y, m, day).unwrap();
        assert_eq!(days_to_payday(d(2026, 11, 15), 15), 0);
        assert_eq!(days_to_payday(d(2026, 11, 1), 15), 14);
        assert_eq!(days_to_payday(d(2026, 11, 16), 15), 29);
        assert_eq!(days_to_payday(d(2026, 12, 20), 15), 26);
        assert_eq!(days_to_payday(d(2026, 2, 1), 31), 27);
    }

    #[test]
    fn times_parsing_accepts_chinese_separators() {
        assert_eq!(
            parse_times("20:00，08:00、12:30 08:00"),
            Some(vec![hm(8, 0), hm(12, 30), hm(20, 0)])
        );
        assert_eq!(parse_times("  "), None);
        assert_eq!(parse_times("8点"), None);
        assert_eq!(parse_time("08：30"), Some(hm(8, 30)));
    }
}
