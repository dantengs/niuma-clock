package io.github.dantengs.niumaclock.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.LocalDate
import java.time.LocalDateTime

class EngineTest {
    private fun dt(d: Int, h: Int, m: Int): LocalDateTime = LocalDateTime.of(2026, 11, d, h, m)
    private fun day(d: Int): LocalDate = LocalDate.of(2026, 11, d)

    private fun engine() = Engine(Config(salary = SalaryConfig(monthly = 21750.0)), Data())

    /** 每 30 秒 tick 一次，返回发出的通知数。 */
    private fun walk(e: Engine, from: LocalDateTime, to: LocalDateTime): Int {
        var count = 0
        var now = from
        while (!now.isAfter(to)) {
            count += e.tick(now).size
            now = now.plusSeconds(30)
        }
        return count
    }

    /** 只在 nextWake() 给出的时刻 tick，模拟手机上的闹钟。 */
    private fun wakeUps(e: Engine, from: LocalDateTime, to: LocalDateTime): List<Pair<LocalDateTime, Notice>> {
        val out = mutableListOf<Pair<LocalDateTime, Notice>>()
        var now = from
        e.tick(now).forEach { out += now to it }
        while (true) {
            now = e.nextWake(now) ?: break
            if (now.isAfter(to)) break
            e.tick(now).forEach { out += now to it }
        }
        return out
    }

    @Test fun clockInTwiceIsRejected() {
        val e = engine()
        assertTrue(e.clockIn(dt(2, 9, 0)))
        assertFalse(e.clockIn(dt(2, 9, 1)))
        assertEquals(1, e.data.sessions.size)
    }

    @Test fun clockOutWithoutClockInDoesNothing() {
        assertNull(engine().clockOut(dt(2, 18, 0)))
    }

    @Test fun fullDaySettlesToDailyWage() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        val summary = e.clockOut(dt(2, 18, 0))!!
        assertEquals(1000.0, summary.income, 1e-6)
        assertEquals(9.0 * 3600, summary.workedSecs, 0.0)
        assertTrue(e.data.loops.isEmpty())
    }

    @Test fun twoSessionsShareStandardHours() {
        val e = engine()
        e.clockIn(dt(2, 9, 0)); e.clockOut(dt(2, 12, 0))
        e.clockIn(dt(2, 13, 0)); e.clockOut(dt(2, 19, 0))
        val summary = e.summaryOf(day(2))
        assertEquals(1000.0 + 125.0, summary.income, 1e-6)
        assertEquals(3600.0, summary.overtimeSecs, 1e-6)
    }

    @Test fun liveStatsMatchSettlement() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        val live = e.stats(dt(2, 15, 0)).earnedExact
        assertEquals(live, e.clockOut(dt(2, 15, 0))!!.income, 1e-6)
    }

    @Test fun displaySnapsButSettlementIsExact() {
        val e = engine()
        e.cfg.salary.tickUnit = TickUnit.Hour
        e.clockIn(dt(2, 9, 0))
        val stats = e.stats(dt(2, 9, 59))
        assertEquals(0.0, stats.earnedDisplay, 1e-9)
        assertTrue(stats.earnedExact > 0.0)
    }

    @Test fun changingSalaryDoesNotRewriteHistory() {
        val e = engine()
        e.clockIn(dt(2, 9, 0)); e.clockOut(dt(2, 18, 0))
        e.cfg.salary.monthly = 43500.0
        assertEquals(1000.0, e.summaryOf(day(2)).income, 1e-6)
    }

    @Test fun clockInStartsAutoTemplatesOnly() {
        val e = engine()
        e.cfg.reminders[1].autoStart = false
        e.clockIn(dt(2, 9, 0))
        assertEquals(listOf(1L), e.data.loops.map { it.templateId })
    }

    @Test fun tempLoopNeedsDuty() {
        val e = engine()
        assertFalse(e.addTempLoop("开会", "", 30, dt(2, 9, 0)))
        e.clockIn(dt(2, 9, 0))
        assertTrue(e.addTempLoop("开会", "", 30, dt(2, 9, 0)))
    }

    @Test fun reminderFiresOnceAndWaitsForAck() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        assertEquals("喝水到点只响一次，没确认不叠加", 1, walk(e, dt(2, 9, 0), dt(2, 9, 50)))
        assertEquals(1, e.alerts().size)
        val id = e.data.loops[0].id
        e.ack(AlertKind.Loop(id), AckAction.Done, dt(2, 9, 50))
        assertTrue(e.alerts().isEmpty())
        assertEquals(dt(2, 10, 35), e.data.loops[0].nextFire)
        assertEquals(1, e.data.days[day(2)]!!.done)
        e.ack(AlertKind.Loop(id), AckAction.Done, dt(2, 9, 51))
        assertEquals("重复确认（通知和应用里各点一次）只算一次", 1, e.data.days[day(2)]!!.done)
    }

    @Test fun snoozeAndSkipRearmCorrectly() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        walk(e, dt(2, 9, 0), dt(2, 9, 46))
        val id = e.data.loops[0].id
        e.ack(AlertKind.Loop(id), AckAction.Snooze, dt(2, 9, 46))
        assertEquals(dt(2, 9, 51), e.data.loops[0].nextFire)
        assertFalse(e.data.days.containsKey(day(2)))
        walk(e, dt(2, 9, 46), dt(2, 9, 52))
        e.ack(AlertKind.Loop(id), AckAction.Skip, dt(2, 9, 52))
        assertEquals(dt(2, 10, 37), e.data.loops[0].nextFire)
        assertEquals(1, e.data.days[day(2)]!!.skipped)
    }

    @Test fun toastOnlyStyleRearmsWithoutAck() {
        val e = engine()
        e.cfg.alertStyle = AlertStyle.Toast
        e.clockIn(dt(2, 9, 0))
        assertEquals("喝水 9:45、10:30，活动 10:00", 3, walk(e, dt(2, 9, 0), dt(2, 10, 31)))
        assertTrue(e.alerts().isEmpty())
    }

    @Test fun alarmDrivenWakeUpsHitEveryReminder() {
        val e = engine()
        e.cfg.alertStyle = AlertStyle.Toast
        e.clockIn(dt(2, 9, 0))
        val fired = wakeUps(e, dt(2, 9, 0), dt(2, 10, 31)).map { it.first }
        assertEquals(listOf(dt(2, 9, 45), dt(2, 10, 0), dt(2, 10, 30)), fired)
    }

    @Test fun manualPauseFreezesRemainingTime() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        val id = e.data.loops[0].id
        e.togglePause(id, dt(2, 9, 15))
        assertEquals(30L * 60, e.data.loops[0].pausedLeft)
        walk(e, dt(2, 9, 15), dt(2, 11, 0))
        assertNull("暂停的循环不会到点", e.data.loops.first { it.id == id }.pendingSince)
        e.togglePause(id, dt(2, 11, 0))
        assertEquals(dt(2, 11, 30), e.data.loops[0].nextFire)
    }

    @Test fun lunchSilencesAndFreezesLoops() {
        val e = engine()
        e.clockIn(dt(2, 11, 50))
        val before = e.data.loops.associate { it.id to it.nextFire }
        // 闹钟驱动：12:00 午休开始冻结，13:00 结束恢复。
        val fired = wakeUps(e, dt(2, 11, 50), dt(2, 12, 59))
        assertTrue("午休中不弹提醒", fired.isEmpty())
        assertTrue(e.alerts().isEmpty())
        assertEquals(dt(2, 13, 0), e.lunchUntil(dt(2, 12, 30)))
        e.tick(dt(2, 13, 0))
        for (l in e.data.loops) {
            val leftAtNoon = java.time.Duration.between(dt(2, 12, 0), before[l.id]!!)
            assertEquals("从冻结时的剩余时间继续", dt(2, 13, 0).plus(leftAtNoon), l.nextFire)
            assertNull(l.pausedLeft)
        }
        assertTrue(e.data.lunchFrozen.isEmpty())
        assertTrue("午休结束后照常提醒", walk(e, dt(2, 13, 0), dt(2, 14, 0)) > 0)
    }

    @Test fun lunchFreezeUsesLunchStartEvenIfAlarmIsLate() {
        val e = engine()
        e.clockIn(dt(2, 11, 50)) // 喝水 12:35
        e.tick(dt(2, 12, 20)) // 闹钟晚了 20 分钟
        val water = e.data.loops.first { it.templateId == 1L }
        assertEquals(35L * 60, water.pausedLeft)
    }

    @Test fun manualPauseSurvivesLunch() {
        val e = engine()
        e.clockIn(dt(2, 11, 0))
        val id = e.data.loops[0].id
        e.togglePause(id, dt(2, 11, 30))
        e.tick(dt(2, 12, 10))
        e.tick(dt(2, 13, 0))
        assertEquals("手动暂停的不被午休恢复", 15L * 60, e.data.loops.first { it.id == id }.pausedLeft)
    }

    @Test fun lunchDefersMedsUntilItEnds() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        e.addMed("布洛芬", "", "", MedSchedule.at(listOf(hm(12, 10))), dt(2, 9, 0))
        e.setHealth(Health.Unwell, dt(2, 9, 0))
        e.data.sickPromptPending = false
        e.data.loops.clear()
        val fired = wakeUps(e, dt(2, 11, 0), dt(2, 13, 30))
        assertEquals(1, fired.size)
        assertEquals(dt(2, 13, 0), fired[0].first)
        assertTrue(fired[0].second.title.contains("布洛芬"))
    }

    @Test fun autoLunchNeedsDutyAndSetting() {
        val e = engine()
        assertNull("没上班不自动午休", e.lunchUntil(dt(2, 12, 30)))
        e.clockIn(dt(2, 9, 0))
        assertNull(e.lunchUntil(dt(2, 11, 59)))
        assertEquals(dt(2, 13, 0), e.lunchUntil(dt(2, 12, 0)))
        e.cfg.lunchQuiet = false
        assertNull(e.lunchUntil(dt(2, 12, 30)))
    }

    @Test fun manualLunchAndEarlyEnd() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        e.startLunch(dt(2, 14, 0))
        assertEquals("手动午休按设置的午休时长", dt(2, 15, 0), e.lunchUntil(dt(2, 14, 30)))
        assertTrue(e.data.loops.all { it.pausedLeft != null })
        e.endLunch(dt(2, 14, 20))
        assertNull(e.lunchUntil(dt(2, 14, 21)))
        assertTrue("提前结束就恢复倒计时", e.data.loops.all { it.pausedLeft == null })
        assertEquals("手动结束不影响自动午休", dt(2, 13, 0), e.lunchUntil(dt(2, 12, 30)))

        e.endLunch(dt(2, 12, 10))
        assertNull("提前结束自动午休，当天不再进入", e.lunchUntil(dt(2, 12, 30)))
        e.clockOut(dt(2, 18, 0))
        e.clockIn(dt(3, 9, 0))
        assertEquals("第二天照常", dt(3, 13, 0), e.lunchUntil(dt(3, 12, 30)))

        e.startLunch(dt(3, 15, 0))
        e.tick(dt(3, 16, 0))
        assertNull("到时自动清掉", e.data.lunchManualUntil)
    }

    @Test fun nextWakeCoversLunchStartAndWorkEnd() {
        val e = engine()
        e.clockIn(dt(2, 11, 59))
        e.data.loops.clear()
        assertEquals(dt(2, 12, 0), e.nextWake(dt(2, 11, 59)))
        assertEquals(dt(2, 13, 0), e.nextWake(dt(2, 12, 0)))
        assertEquals(dt(2, 18, 0), e.nextWake(dt(2, 13, 0)))
        assertNull("下班后没有待办", run { e.clockOut(dt(2, 18, 0)); e.nextWake(dt(2, 18, 0)) })
    }

    @Test fun unwellStartsMedsAndSurvivesClockOut() {
        val e = engine()
        e.addMed("布洛芬", "1 粒", "饭后", MedSchedule.every(240), dt(2, 8, 0))
        assertTrue("健康时不启动吃药提醒", e.data.meds.isEmpty())
        e.clockIn(dt(2, 9, 0))
        e.setHealth(Health.Unwell, dt(2, 10, 0))
        assertTrue(e.alerts().any { it.kind == AlertKind.Sick })
        assertTrue(e.tick(dt(2, 10, 0)).any { it.title.contains("布洛芬") })
        val planId = e.cfg.meds[0].id
        e.ack(AlertKind.Med(planId), AckAction.Done, dt(2, 10, 1))
        assertEquals(dt(2, 14, 1), e.data.meds[0].nextFire)
        assertEquals(1, e.data.days[day(2)]!!.medsTaken)
        e.clockOut(dt(2, 12, 0))
        assertEquals("下班后吃药提醒继续", 1, e.data.meds.size)
        e.setHealth(Health.Healthy, dt(2, 13, 0))
        assertTrue("恢复健康后停止", e.data.meds.isEmpty())
    }

    @Test fun scheduledMedWaitsForNextTimePoint() {
        val plan = MedPlan(1, "维C", "", "", MedSchedule.at(listOf(hm(8, 0), hm(20, 0))))
        assertEquals(dt(2, 20, 0), nextMedFire(plan, dt(2, 9, 0)))
        assertEquals(dt(3, 8, 0), nextMedFire(plan, dt(2, 21, 0)))
    }

    @Test fun workEndAlertFiresOncePerDayAndSnoozes() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        e.data.loops.clear()
        assertEquals(1, walk(e, dt(2, 17, 59), dt(2, 18, 5)))
        e.ack(AlertKind.WorkEnd, AckAction.Snooze, dt(2, 18, 5))
        assertEquals(0, walk(e, dt(2, 18, 5), dt(2, 18, 34)))
        assertEquals("再卷半小时后再催一次", 1, walk(e, dt(2, 18, 35), dt(2, 18, 40)))
    }

    @Test fun overtimeClockInAfterEndDoesNotNag() {
        val e = engine()
        e.clockIn(dt(2, 19, 0))
        e.data.loops.clear()
        assertEquals(0, walk(e, dt(2, 19, 0), dt(2, 19, 10)))
    }

    @Test fun forgottenClockOutIsDetectedAndSilencesLoops() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        assertNull(e.forgotten(dt(2, 23, 0)))
        assertEquals(dt(2, 9, 0), e.forgotten(dt(3, 9, 0)))
        assertTrue(e.tick(dt(3, 9, 0)).isEmpty())
        assertEquals(1000.0, e.clockOut(dt(2, 18, 0))!!.income, 1e-6)
        assertNull(e.forgotten(dt(3, 9, 0)))
    }

    @Test fun nightShiftIsNotFlaggedForgotten() {
        val e = engine()
        e.clockIn(dt(2, 22, 0))
        assertNull(e.forgotten(dt(3, 6, 0)))
    }

    @Test fun editAndDeleteSessionRecalculate() {
        val e = engine()
        e.clockIn(dt(2, 9, 0)); e.clockOut(dt(2, 13, 0))
        assertTrue(e.editSession(0, dt(2, 9, 0), dt(2, 18, 0)))
        assertEquals(1000.0, e.summaryOf(day(2)).income, 1e-6)
        e.clockIn(dt(3, 9, 0))
        assertFalse("进行中的不能改", e.editSession(1, dt(3, 9, 0), dt(3, 10, 0)))
        assertTrue(e.deleteSession(0))
        assertEquals(0.0, e.summaryOf(day(2)).income, 0.0)
    }

    @Test fun monthEarnedCanBeSetAndKeepsGrowing() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        e.setMonthEarned(dt(2, 10, 0), 1500.0)
        assertEquals(1500.0, e.stats(dt(2, 10, 0)).monthEarned, 1e-6)
        assertTrue(e.stats(dt(2, 11, 0)).monthEarned > 1500.0)
        val m = e.monthSummary(2026, 11)
        assertEquals(m.adjust, e.monthAdjust(2026, 11), 0.0)
        e.setMonthEarned(dt(2, 10, 0), e.stats(dt(2, 10, 0)).monthEarned - e.monthAdjust(2026, 11))
        assertEquals("对上记录时自动清掉差额", 0.0, e.monthAdjust(2026, 11), 0.0)
    }

    @Test fun expensesAddDeleteRestoreAndSummarise() {
        val e = engine()
        assertFalse(e.addExpense(dt(2, 12, 0), 0.0, "餐饮", ""))
        assertTrue(e.addExpense(dt(2, 12, 0), 25.456, "餐饮", " 午饭 "))
        assertTrue(e.addExpense(dt(2, 19, 0), 100.0, "购物", ""))
        assertTrue(e.addExpense(dt(3, 8, 0), 4.0, "", "地铁"))
        val lunch = e.data.expenses.first { it.note == "午饭" }
        assertEquals(25.46, lunch.amount, 0.0)
        val sum = e.monthExpenses(2026, 11)
        assertEquals(129.46, sum.total, 1e-9)
        assertEquals("购物", sum.byCategory.first().first)
        assertEquals("其他", e.data.expenses.last().category)
        val removed = e.deleteExpense(lunch.id)!!
        assertNull(e.deleteExpense(lunch.id))
        e.restoreExpense(removed)
        assertEquals(3, e.data.expenses.size)
        assertTrue(e.expensesCsv().contains("2026-11-02 12:00,25.46,餐饮,\"午饭\""))
    }

    @Test fun hpChangesWithBehaviour() {
        assertEquals(100, hpScore(0, 0, 0.0, Health.Healthy))
        assertEquals(75, hpScore(0, 5, 0.0, Health.Healthy))
        assertEquals(78, hpScore(2, 0, 3600.0, Health.Unwell))
        assertEquals("金牌牛马", hpRank(95))
        assertEquals("牛马已躺平", hpRank(10))
    }

    @Test fun csvContainsFinishedSessionsOnly() {
        val e = engine()
        e.clockIn(dt(2, 9, 0)); e.clockOut(dt(2, 18, 0))
        e.clockIn(dt(3, 9, 0))
        val lines = e.toCsv().trim().lines()
        assertEquals(2, lines.size)
        assertTrue(lines[1].startsWith("2026-11-02,09:00:00,2026-11-02 18:00:00,9.00,8.00,0.00,1000.00"))
    }

    @Test fun newIdsDoNotCollideWithDefaultTemplates() {
        val e = engine()
        val id = e.addTemplate("看远处", "", 20, false)
        assertTrue(id > 2)
    }

    @Test fun templateEditSyncsToRunningLoop() {
        val e = engine()
        e.clockIn(dt(2, 9, 0))
        val tpl = e.cfg.reminders[0]
        e.updateTemplate(tpl.copy(title = "喝水！", intervalMin = 30), dt(2, 9, 10))
        val l = e.data.loops.first { it.templateId == tpl.id }
        assertEquals("喝水！", l.title)
        assertEquals(dt(2, 9, 40), l.nextFire)
    }

    @Test fun alertKindKeysRoundTrip() {
        for (k in listOf(AlertKind.Loop(7), AlertKind.Med(9), AlertKind.WorkEnd, AlertKind.Sick)) {
            assertEquals(k, AlertKind.parse(k.key))
        }
        assertNotNull(AlertKind.parse("loop:1"))
        assertNull(AlertKind.parse("bogus"))
    }
}
