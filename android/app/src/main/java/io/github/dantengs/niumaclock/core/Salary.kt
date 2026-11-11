package io.github.dantengs.niumaclock.core

import java.time.Duration
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.temporal.ChronoUnit
import java.time.YearMonth

/** 工资计算。全部是纯函数，时间由调用方传入。与电脑版 salary.rs 逐条对应。 */
object Salary {
    fun secsBetween(a: LocalDateTime, b: LocalDateTime): Double = Duration.between(a, b).toMillis() / 1000.0

    /** 打卡区间内应计薪的秒数：总时长减去与每天午休的重叠部分。 */
    fun paidSecs(start: LocalDateTime, end: LocalDateTime, s: SalaryConfig): Double {
        if (!end.isAfter(start)) return 0.0
        var total = secsBetween(start, end)
        if (s.lunchUnpaid && s.lunchStart < s.lunchEnd) {
            var day = start.toLocalDate()
            while (!day.isAfter(end.toLocalDate())) {
                val a = maxOf(day.atTime(s.lunchStart), start)
                val b = minOf(day.atTime(s.lunchEnd), end)
                if (b.isAfter(a)) total -= secsBetween(a, b)
                day = day.plusDays(1)
            }
        }
        return total.coerceAtLeast(0.0)
    }

    /** 下班时刻。下班时间不晚于上班时间视为跨夜班。 */
    fun workEndAt(workDay: LocalDate, s: SalaryConfig): LocalDateTime {
        val end = workDay.atTime(s.workEnd)
        return if (s.workEnd <= s.workStart) end.plusDays(1) else end
    }

    /** 一天的标准计薪秒数。 */
    fun standardDailySecs(s: SalaryConfig): Double {
        val base = LocalDate.of(2024, 1, 1)
        return paidSecs(base.atTime(s.workStart), workEndAt(base, s), s)
    }

    /** 基础费率，元/秒。配置不完整时为 0。 */
    fun ratePerSec(s: SalaryConfig): Double {
        val daily = standardDailySecs(s)
        if (s.monthly <= 0.0 || s.paidDays <= 0.0 || daily <= 0.0) return 0.0
        return s.monthly / s.paidDays / daily
    }

    /** 当天累计计薪 paid 秒时的总收入，超过标准工时的部分按加班倍率。 */
    fun earned(paid: Double, s: SalaryConfig): Double {
        val daily = standardDailySecs(s)
        val regular = paid.coerceIn(0.0, maxOf(daily, 0.0))
        val overtime = (paid - daily).coerceAtLeast(0.0)
        return ratePerSec(s) * (regular + overtime * s.overtimeMultiplier.coerceAtLeast(0.0))
    }

    /** 按跳字粒度向下取整。只影响显示，结算用实际秒数。 */
    fun snap(secs: Double, unit: TickUnit): Double = Math.floor(secs / unit.secs) * unit.secs

    /** 这一跳增加多少钱。 */
    fun tickAmount(s: SalaryConfig): Double = ratePerSec(s) * s.tickUnit.secs

    /** 到下一个发薪日还有几天，当天为 0。发薪日超过当月天数时取月末。 */
    fun daysToPayday(today: LocalDate, payday: Int): Long {
        fun inMonth(ym: YearMonth): LocalDate = ym.atDay(payday.coerceIn(1, ym.lengthOfMonth()))
        val ym = YearMonth.from(today)
        val thisMonth = inMonth(ym)
        if (!thisMonth.isBefore(today)) return ChronoUnit.DAYS.between(today, thisMonth)
        return ChronoUnit.DAYS.between(today, inMonth(ym.plusMonths(1)))
    }

    /** 解析 "08:00, 12:30 20:00" 这样的时间列表，去重排序。 */
    fun parseTimes(text: String): List<LocalTime>? {
        val parts = text.split(',', '，', '、', ' ', '\t', '\n').filter { it.isNotBlank() }
        if (parts.isEmpty()) return null
        val times = parts.map { parseTime(it) ?: return null }
        return times.distinct().sorted()
    }

    fun parseTime(text: String): LocalTime? {
        val m = Regex("""^(\d{1,2})[:：](\d{2})$""").matchEntire(text.trim()) ?: return null
        val (h, min) = m.destructured
        return runCatching { LocalTime.of(h.toInt(), min.toInt()) }.getOrNull()
    }
}
