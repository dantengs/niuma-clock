package io.github.dantengs.niumaclock.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.time.LocalDate
import java.time.LocalDateTime

/** 与电脑版 salary.rs 的测试一一对应，保证两边算出来的钱一样。 */
class SalaryTest {
    private fun dt(d: Int, h: Int, m: Int): LocalDateTime = LocalDateTime.of(2026, 11, d, h, m)
    private fun cfg() = SalaryConfig(monthly = 21750.0)

    @Test fun standardDayExcludesLunch() {
        assertEquals(8.0 * 3600, Salary.standardDailySecs(cfg()), 0.0)
        assertEquals(9.0 * 3600, Salary.standardDailySecs(cfg().copy(lunchUnpaid = false)), 0.0)
    }

    @Test fun fullDayPaysExactlyDailyWage() {
        val s = cfg()
        assertEquals(1000.0, Salary.earned(Salary.paidSecs(dt(2, 9, 0), dt(2, 18, 0), s), s), 1e-6)
    }

    @Test fun lunchOverlapIsPartial() {
        val s = cfg()
        assertEquals(1800.0, Salary.paidSecs(dt(2, 11, 30), dt(2, 12, 30), s), 0.0)
        assertEquals(0.0, Salary.paidSecs(dt(2, 12, 10), dt(2, 12, 50), s), 0.0)
        assertEquals(1800.0, Salary.paidSecs(dt(2, 12, 30), dt(2, 13, 30), s), 0.0)
    }

    @Test fun multiDaySessionSubtractsEachLunch() {
        assertEquals(33.0 * 3600 - 2 * 3600, Salary.paidSecs(dt(2, 9, 0), dt(3, 18, 0), cfg()), 0.0)
    }

    @Test fun reversedRangeIsZero() {
        assertEquals(0.0, Salary.paidSecs(dt(2, 18, 0), dt(2, 9, 0), cfg()), 0.0)
    }

    @Test fun overtimeMultiplierAppliesAfterStandardHours() {
        val hour = 1000.0 / 8
        assertEquals(1000.0 + 2 * hour, Salary.earned(9.0 * 3600, cfg().copy(overtimeMultiplier = 2.0)), 1e-6)
        assertEquals(1000.0, Salary.earned(12.0 * 3600, cfg().copy(overtimeMultiplier = 0.0)), 1e-6)
    }

    @Test fun overnightShiftHasPositiveDay() {
        val s = cfg().copy(workStart = hm(22, 0), workEnd = hm(6, 0))
        assertEquals(8.0 * 3600, Salary.standardDailySecs(s), 0.0)
        assertEquals(dt(3, 6, 0), Salary.workEndAt(LocalDate.of(2026, 11, 2), s))
    }

    @Test fun invalidConfigHasZeroRate() {
        assertEquals(0.0, Salary.ratePerSec(SalaryConfig()), 0.0)
        assertEquals(0.0, Salary.ratePerSec(cfg().copy(paidDays = 0.0)), 0.0)
        assertEquals(0.0, Salary.ratePerSec(cfg().copy(workStart = hm(12, 0), workEnd = hm(13, 0))), 0.0)
    }

    @Test fun snapFloorsToUnit() {
        assertEquals(61.0, Salary.snap(61.9, TickUnit.Second), 0.0)
        assertEquals(60.0, Salary.snap(61.9, TickUnit.Minute), 0.0)
        assertEquals(0.0, Salary.snap(3599.0, TickUnit.Hour), 0.0)
        assertEquals(3600.0, Salary.snap(3600.0, TickUnit.Hour), 0.0)
    }

    @Test fun paydayRollsOverAndClamps() {
        fun d(y: Int, m: Int, day: Int) = LocalDate.of(y, m, day)
        assertEquals(0, Salary.daysToPayday(d(2026, 11, 15), 15))
        assertEquals(14, Salary.daysToPayday(d(2026, 11, 1), 15))
        assertEquals(29, Salary.daysToPayday(d(2026, 11, 16), 15))
        assertEquals(26, Salary.daysToPayday(d(2026, 12, 20), 15))
        assertEquals(27, Salary.daysToPayday(d(2026, 2, 1), 31))
    }

    @Test fun timesParsingAcceptsChineseSeparators() {
        assertEquals(listOf(hm(8, 0), hm(12, 30), hm(20, 0)), Salary.parseTimes("20:00，08:00、12:30 08:00"))
        assertNull(Salary.parseTimes("  "))
        assertNull(Salary.parseTimes("8点"))
        assertEquals(hm(8, 30), Salary.parseTime("08：30"))
    }
}
