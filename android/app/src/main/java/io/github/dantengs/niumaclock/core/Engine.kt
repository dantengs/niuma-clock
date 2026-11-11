package io.github.dantengs.niumaclock.core

import java.time.Duration
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.YearMonth

/*
 * 中央状态机：打卡、循环、吃药、提醒确认、午休。时间都由调用方传入，便于测试。
 * 与电脑版 engine.rs 对应，区别是手机上没有 250 ms 心跳：调用方在 nextWake() 给出的时刻用闹钟唤醒，再调 tick()。
 * 因此午休改成“开始时冻结、结束时恢复”，而不是每次心跳顺延；锁屏不暂停（手机锁屏是常态）。
 */

/** 打卡跨天且超过这个时长，才怀疑忘了打下班卡，避免误伤正常夜班。 */
private const val FORGOT_AFTER_HOURS = 16L
private const val WORK_END_SNOOZE_MIN = 30L

sealed class AlertKind {
    data class Loop(val id: Long) : AlertKind()
    data class Med(val planId: Long) : AlertKind()
    data object WorkEnd : AlertKind()
    data object Sick : AlertKind()

    /** 稳定的字符串键，用于通知 id 和广播参数。 */
    val key: String
        get() = when (this) {
            is Loop -> "loop:$id"
            is Med -> "med:$planId"
            WorkEnd -> "work-end"
            Sick -> "sick"
        }

    companion object {
        fun parse(key: String): AlertKind? = when {
            key.startsWith("loop:") -> key.removePrefix("loop:").toLongOrNull()?.let { Loop(it) }
            key.startsWith("med:") -> key.removePrefix("med:").toLongOrNull()?.let { Med(it) }
            key == "work-end" -> WorkEnd
            key == "sick" -> Sick
            else -> null
        }
    }
}

data class Notice(
    val kind: AlertKind,
    val title: String,
    val body: String,
    /** 需要用户确认（通知带按钮，确认前不进入下一轮）。 */
    val needsAck: Boolean,
)

data class Alert(val kind: AlertKind, val title: String, val body: String)

enum class AckAction { Done, Skip, Snooze }

data class TodayStats(
    val workDay: LocalDate,
    val onDuty: Boolean,
    val sessionStart: LocalDateTime?,
    /** 按跳字粒度显示的收入。 */
    val earnedDisplay: Double,
    val earnedExact: Double,
    val workedSecs: Double,
    val paidSecs: Double,
    val overtimeSecs: Double,
    val progress: Float,
    val toEndSecs: Long?,
    /** 含手动调整。 */
    val monthEarned: Double,
    val monthAdjust: Double,
    val done: Int,
    val skipped: Int,
    val hp: Int,
    /** 正在午休时为午休结束时刻。 */
    val lunchUntil: LocalDateTime?,
)

data class DaySummary(
    val date: LocalDate,
    val income: Double,
    val workedSecs: Double,
    val overtimeSecs: Double,
    val done: Int,
    val skipped: Int,
    val hp: Int,
    val freeLabor: Boolean,
)

data class MonthSummary(
    val days: Int = 0,
    val workedSecs: Double = 0.0,
    val overtimeSecs: Double = 0.0,
    /** 含手动调整。 */
    val income: Double = 0.0,
    val adjust: Double = 0.0,
)

data class ExpenseSummary(
    val total: Double = 0.0,
    val count: Int = 0,
    /** 按金额从大到小。 */
    val byCategory: List<Pair<String, Double>> = emptyList(),
)

fun hpScore(done: Int, skipped: Int, overtimeSecs: Double, health: Health): Int {
    val penalty = when (health) {
        Health.Healthy -> 0
        Health.SubHealthy -> 10
        Health.Unwell -> 20
    }
    val overtime = (overtimeSecs / 600.0).toInt()
    return (100 + 2 * done - 5 * skipped - overtime - penalty).coerceIn(0, 100)
}

fun hpRank(hp: Int): String = when {
    hp >= 90 -> "金牌牛马"
    hp >= 70 -> "健壮牛马"
    hp >= 50 -> "疲惫牛马"
    hp >= 25 -> "濒危牛马"
    else -> "牛马已躺平"
}

private fun secs(a: LocalDateTime, b: LocalDateTime): Long = Duration.between(a, b).seconds

class Engine(var cfg: Config, var data: Data) {
    var cfgDirty = false
    var dataDirty = false

    init {
        // 默认模板占了 1、2 号，保证后续 id 不会撞车。
        val maxTpl = cfg.reminders.maxOfOrNull { it.id } ?: 0
        val maxMed = cfg.meds.maxOfOrNull { it.id } ?: 0
        data.nextId = maxOf(data.nextId, maxOf(maxTpl, maxMed) + 1)
    }

    private fun newId(): Long = data.nextId++

    fun openSession(): WorkSession? = data.sessions.lastOrNull { it.end == null }

    fun onDuty(): Boolean = openSession() != null

    private fun workDay(now: LocalDateTime): LocalDate = openSession()?.start?.toLocalDate() ?: now.toLocalDate()

    private fun dayMut(date: LocalDate): DayLog = data.days.getOrPut(date) { DayLog() }

    // ---------- 打卡 ----------

    fun clockIn(now: LocalDateTime): Boolean {
        if (onDuty()) return false
        data.sessions.add(WorkSession(start = now))
        for (tpl in cfg.reminders.filter { it.autoStart }) startTemplateLoop(tpl, now)
        syncLunchFreeze(now)
        // 重新打卡说明上一段已经下班，当天不再催。
        if (!Salary.workEndAt(now.toLocalDate(), cfg.salary).isAfter(now)) {
            data.workEndNotified = now.toLocalDate()
        }
        dataDirty = true
        return true
    }

    /** 下班结算，并清除所有运行中的循环。常用模板留在配置里，下次上班自动恢复。 */
    fun clockOut(now: LocalDateTime): DaySummary? {
        val index = data.sessions.indexOfLast { it.end == null }
        if (index < 0) return null
        val session = data.sessions[index]
        val date = session.start.toLocalDate()
        session.end = maxOf(now, session.start)
        recalcDay(date)
        data.loops.clear()
        data.lunchFrozen.clear()
        data.lunchManualUntil = null
        data.lunchManualFrom = null
        data.workEndPending = false
        data.workEndSnoozeUntil = null
        dataDirty = true
        return summaryOf(date)
    }

    /** 按当前工资设置重算某天所有已结束的打卡段。 */
    private fun recalcDay(date: LocalDate) {
        val s = cfg.salary
        val daily = Salary.standardDailySecs(s)
        var before = 0.0
        for (session in data.sessions.filter { it.start.toLocalDate() == date }) {
            val end = session.end ?: continue
            val paid = Salary.paidSecs(session.start, end, s)
            session.paidSecs = paid
            session.income = Salary.earned(before + paid, s) - Salary.earned(before, s)
            session.overtimeSecs = (before + paid - maxOf(daily, before)).coerceAtLeast(0.0)
            before += paid
        }
    }

    fun summaryOf(date: LocalDate): DaySummary {
        var income = 0.0
        var worked = 0.0
        var overtime = 0.0
        for (session in data.sessions.filter { it.start.toLocalDate() == date }) {
            val end = session.end ?: continue
            income += session.income
            worked += secs(session.start, end).toDouble()
            overtime += session.overtimeSecs
        }
        val log = data.days[date] ?: DayLog()
        return DaySummary(
            date = date,
            income = income,
            workedSecs = worked,
            overtimeSecs = overtime,
            done = log.done,
            skipped = log.skipped,
            hp = hpScore(log.done, log.skipped, overtime, log.health ?: data.health),
            freeLabor = overtime > 0.0 && cfg.salary.overtimeMultiplier <= 0.0,
        )
    }

    fun stats(now: LocalDateTime): TodayStats {
        val s = cfg.salary
        val workDay = workDay(now)
        var closedPaid = 0.0
        var closedIncome = 0.0
        var closedWorked = 0.0
        for (session in data.sessions.filter { it.start.toLocalDate() == workDay && it.end != null }) {
            closedPaid += session.paidSecs
            closedIncome += session.income
            closedWorked += secs(session.start, session.end!!).toDouble()
        }
        val open = openSession()
        val currentPaid = open?.let { Salary.paidSecs(it.start, now, s) } ?: 0.0
        val currentWorked = open?.let { secs(it.start, now).coerceAtLeast(0).toDouble() } ?: 0.0
        fun live(paid: Double) = Salary.earned(closedPaid + paid, s) - Salary.earned(closedPaid, s)
        val earnedExact = closedIncome + live(currentPaid)
        val earnedDisplay = closedIncome + live(Salary.snap(currentPaid, s.tickUnit))

        val daily = Salary.standardDailySecs(s)
        val totalPaid = closedPaid + currentPaid
        val overtime = (totalPaid - daily).coerceAtLeast(0.0)
        val progress = if (daily > 0.0) minOf(totalPaid / daily, 1.0).toFloat() else 0f
        val toEnd = open?.let {
            val end = Salary.workEndAt(it.start.toLocalDate(), s)
            if (end.isAfter(it.start)) secs(now, end) else null
        }

        val monthClosed = data.sessions
            .filter { it.end != null && it.start.year == workDay.year && it.start.monthValue == workDay.monthValue }
            .sumOf { it.income }
        val adjust = monthAdjust(workDay.year, workDay.monthValue)
        val log = data.days[workDay] ?: DayLog()
        return TodayStats(
            workDay = workDay,
            onDuty = open != null,
            sessionStart = open?.start,
            earnedDisplay = earnedDisplay,
            earnedExact = earnedExact,
            workedSecs = closedWorked + currentWorked,
            paidSecs = totalPaid,
            overtimeSecs = overtime,
            progress = progress,
            toEndSecs = toEnd,
            monthEarned = monthClosed + live(currentPaid) + adjust,
            monthAdjust = adjust,
            done = log.done,
            skipped = log.skipped,
            hp = hpScore(log.done, log.skipped, overtime, data.health),
            lunchUntil = lunchUntil(now),
        )
    }

    // ---------- 忘打卡 ----------

    /** 如果疑似忘打下班卡，返回那次上班时间。 */
    fun forgotten(now: LocalDateTime): LocalDateTime? {
        val start = openSession()?.start ?: return null
        val suspicious = start.toLocalDate() < now.toLocalDate() &&
            Duration.between(start, now) > Duration.ofHours(FORGOT_AFTER_HOURS) &&
            data.forgotAck != start
        return if (suspicious) start else null
    }

    fun ackForgotten(start: LocalDateTime) {
        data.forgotAck = start
        dataDirty = true
    }

    // ---------- 记录修改 ----------

    fun editSession(index: Int, start: LocalDateTime, end: LocalDateTime): Boolean {
        val session = data.sessions.getOrNull(index) ?: return false
        if (!end.isAfter(start) || session.end == null) return false
        val oldDate = session.start.toLocalDate()
        session.start = start
        session.end = end
        data.sessions.sortBy { it.start }
        recalcDay(oldDate)
        recalcDay(start.toLocalDate())
        dataDirty = true
        return true
    }

    fun deleteSession(index: Int): Boolean {
        val session = data.sessions.getOrNull(index) ?: return false
        if (session.end == null) return false
        data.sessions.removeAt(index)
        recalcDay(session.start.toLocalDate())
        dataDirty = true
        return true
    }

    fun monthSummary(year: Int, month: Int): MonthSummary {
        val sessions = data.sessions.filter { it.end != null && it.start.year == year && it.start.monthValue == month }
        val adjust = monthAdjust(year, month)
        return MonthSummary(
            days = sessions.map { it.start.toLocalDate() }.distinct().size,
            workedSecs = sessions.sumOf { secs(it.start, it.end!!).toDouble() },
            overtimeSecs = sessions.sumOf { it.overtimeSecs },
            income = sessions.sumOf { it.income } + adjust,
            adjust = adjust,
        )
    }

    /** 某月每天的工时（秒），用于柱状图。 */
    fun dailyWorked(year: Int, month: Int): Map<LocalDate, Double> =
        data.sessions.filter { it.end != null && it.start.year == year && it.start.monthValue == month }
            .groupBy { it.start.toLocalDate() }
            .mapValues { (_, list) -> list.sumOf { secs(it.start, it.end!!).toDouble() } }

    // ---------- 本月已赚的手动调整 ----------

    fun monthAdjust(year: Int, month: Int): Double = data.monthAdjust[monthKey(year, month)] ?: 0.0

    /** 把当前工作月的“本月已赚”改成 target。存的是差额，之后照常累加。 */
    fun setMonthEarned(now: LocalDateTime, target: Double) {
        val stats = stats(now)
        val offset = target.coerceAtLeast(0.0) - (stats.monthEarned - stats.monthAdjust)
        val key = monthKey(stats.workDay.year, stats.workDay.monthValue)
        if (Math.abs(offset) < 0.005) data.monthAdjust.remove(key) else data.monthAdjust[key] = offset
        dataDirty = true
    }

    fun clearMonthAdjust(year: Int, month: Int) {
        if (data.monthAdjust.remove(monthKey(year, month)) != null) dataDirty = true
    }

    // ---------- 记账 ----------

    fun addExpense(at: LocalDateTime, amount: Double, category: String, note: String): Boolean {
        if (!(amount > 0.0 && amount.isFinite())) return false
        val cat = category.trim().ifEmpty { "其他" }
        data.expenses.add(Expense(newId(), at, Math.round(amount * 100.0) / 100.0, cat, note.trim()))
        data.expenses.sortBy { it.at }
        dataDirty = true
        return true
    }

    fun deleteExpense(id: Long): Expense? {
        val index = data.expenses.indexOfFirst { it.id == id }
        if (index < 0) return null
        dataDirty = true
        return data.expenses.removeAt(index)
    }

    /** 撤销删除。 */
    fun restoreExpense(expense: Expense) {
        if (data.expenses.any { it.id == expense.id }) return
        data.expenses.add(expense)
        data.expenses.sortBy { it.at }
        dataDirty = true
    }

    fun expenseSummary(from: LocalDate, to: LocalDate): ExpenseSummary {
        val list = data.expenses.filter { !it.at.toLocalDate().isBefore(from) && !it.at.toLocalDate().isAfter(to) }
        val by = list.groupBy { it.category }.mapValues { (_, v) -> v.sumOf { it.amount } }
            .toList().sortedByDescending { it.second }
        return ExpenseSummary(list.sumOf { it.amount }, list.size, by)
    }

    fun monthExpenses(year: Int, month: Int): ExpenseSummary {
        val ym = YearMonth.of(year, month)
        return expenseSummary(ym.atDay(1), ym.atEndOfMonth())
    }

    fun monthExpenseList(year: Int, month: Int): List<Expense> =
        data.expenses.filter { it.at.year == year && it.at.monthValue == month }.sortedByDescending { it.at }

    fun expensesCsv(): String = buildString {
        append("时间,金额,分类,备注\n")
        for (x in data.expenses) {
            val note = x.note.replace("\"", "\"\"")
            append("${x.at.format(DT_MIN)},${"%.2f".format(x.amount)},${x.category},\"$note\"\n")
        }
    }

    fun toCsv(): String = buildString {
        append("日期,上班,下班,工作时长(小时),计薪时长(小时),加班时长(小时),收入,完成提醒,跳过提醒,健康,心情\n")
        for (s in data.sessions) {
            val end = s.end ?: continue
            val log = data.days[s.start.toLocalDate()] ?: DayLog()
            append(
                listOf(
                    s.start.toLocalDate().toString(),
                    s.start.toLocalTime().format(T_SEC),
                    end.format(DT_SEC),
                    "%.2f".format(secs(s.start, end) / 3600.0),
                    "%.2f".format(s.paidSecs / 3600.0),
                    "%.2f".format(s.overtimeSecs / 3600.0),
                    "%.2f".format(s.income),
                    log.done.toString(),
                    log.skipped.toString(),
                    log.health?.label ?: "",
                    log.mood?.label ?: "",
                ).joinToString(","),
            )
            append('\n')
        }
    }

    // ---------- 循环事项 ----------

    private fun startTemplateLoop(tpl: ReminderTemplate, now: LocalDateTime) {
        if (data.loops.any { it.templateId == tpl.id }) return
        val interval = tpl.intervalMin.coerceAtLeast(1)
        data.loops.add(
            ActiveLoop(
                id = newId(), templateId = tpl.id, title = tpl.title, note = tpl.note,
                intervalMin = interval, nextFire = now.plusMinutes(interval.toLong()),
            ),
        )
    }

    /** 手动开始某个常用事项，仅上班期间有效。 */
    fun startTemplate(templateId: Long, now: LocalDateTime): Boolean {
        if (!onDuty()) return false
        val tpl = cfg.reminders.firstOrNull { it.id == templateId } ?: return false
        startTemplateLoop(tpl, now)
        syncLunchFreeze(now)
        dataDirty = true
        return true
    }

    /** 添加今日临时事项，下班后消失。 */
    fun addTempLoop(title: String, note: String, intervalMin: Int, now: LocalDateTime): Boolean {
        if (!onDuty()) return false
        val interval = intervalMin.coerceAtLeast(1)
        data.loops.add(
            ActiveLoop(
                id = newId(), templateId = null, title = title, note = note,
                intervalMin = interval, nextFire = now.plusMinutes(interval.toLong()),
            ),
        )
        syncLunchFreeze(now)
        dataDirty = true
        return true
    }

    fun addTemplate(title: String, note: String, intervalMin: Int, autoStart: Boolean): Long {
        val id = newId()
        cfg.reminders.add(ReminderTemplate(id, title, note, intervalMin.coerceAtLeast(1), autoStart))
        cfgDirty = true
        dataDirty = true
        return id
    }

    /** 修改模板并同步到正在运行的循环。周期变了才重新计时。 */
    fun updateTemplate(updated: ReminderTemplate, now: LocalDateTime) {
        val index = cfg.reminders.indexOfFirst { it.id == updated.id }
        if (index >= 0) {
            cfg.reminders[index] = updated.copy(intervalMin = updated.intervalMin.coerceAtLeast(1))
            cfgDirty = true
        }
        val l = data.loops.firstOrNull { it.templateId == updated.id } ?: return
        l.title = updated.title
        l.note = updated.note
        val interval = updated.intervalMin.coerceAtLeast(1)
        if (l.intervalMin != interval) {
            l.intervalMin = interval
            l.nextFire = now.plusMinutes(interval.toLong())
            l.pausedLeft = null
            data.lunchFrozen.remove(l.id)
        }
        dataDirty = true
    }

    fun removeTemplate(id: Long) {
        cfg.reminders.removeAll { it.id == id }
        data.loops.removeAll { it.templateId == id }
        cfgDirty = true
        dataDirty = true
    }

    fun removeLoop(id: Long) {
        data.loops.removeAll { it.id == id }
        data.lunchFrozen.remove(id)
        dataDirty = true
    }

    /** 手动暂停 / 继续。午休冻结的循环交给午休结束时恢复。 */
    fun togglePause(id: Long, now: LocalDateTime) {
        val l = data.loops.firstOrNull { it.id == id } ?: return
        data.lunchFrozen.remove(id)
        val left = l.pausedLeft
        if (left != null) {
            l.pausedLeft = null
            l.nextFire = now.plusSeconds(left)
        } else {
            l.pausedLeft = secs(now, l.nextFire).coerceAtLeast(0)
        }
        dataDirty = true
    }

    fun isLunchFrozen(id: Long) = id in data.lunchFrozen

    fun restartLoop(id: Long, now: LocalDateTime) {
        val l = data.loops.firstOrNull { it.id == id } ?: return
        l.nextFire = now.plusMinutes(l.intervalMin.toLong())
        l.pausedLeft = null
        l.pendingSince = null
        data.lunchFrozen.remove(id)
        dataDirty = true
    }

    // ---------- 健康与心情 ----------

    fun setHealth(health: Health, now: LocalDateTime) {
        if (data.health == health) return
        data.health = health
        data.statusLog.add(StatusEntry(now, StatusChange(health = health)))
        dayMut(workDay(now)).health = health
        if (health == Health.Unwell) {
            data.sickPromptPending = true
            for (plan in cfg.meds.toList()) startMed(plan, now)
        } else {
            data.sickPromptPending = false
            data.meds.clear()
        }
        dataDirty = true
    }

    fun setMood(mood: Mood, now: LocalDateTime) {
        if (data.mood == mood) return
        data.mood = mood
        data.statusLog.add(StatusEntry(now, StatusChange(mood = mood)))
        dayMut(workDay(now)).mood = mood
        dataDirty = true
    }

    private fun startMed(plan: MedPlan, now: LocalDateTime) {
        if (data.meds.any { it.planId == plan.id }) return
        // 按间隔吃的药先问一次，定时吃的药等下一个时间点。
        val next = if (plan.schedule.interval != null) now else nextMedFire(plan, now)
        data.meds.add(MedRuntime(plan.id, next))
    }

    fun addMed(name: String, dose: String, note: String, schedule: MedSchedule, now: LocalDateTime) {
        val plan = MedPlan(newId(), name, dose, note, schedule)
        cfg.meds.add(plan)
        if (data.health == Health.Unwell) {
            data.sickPromptPending = false
            startMed(plan, now)
        }
        cfgDirty = true
        dataDirty = true
    }

    fun removeMed(id: Long) {
        cfg.meds.removeAll { it.id == id }
        data.meds.removeAll { it.planId == id }
        cfgDirty = true
        dataDirty = true
    }

    // ---------- 提醒确认 ----------

    fun alerts(): List<Alert> = buildList {
        if (data.workEndPending) add(Alert(AlertKind.WorkEnd, WORK_END_TITLE, WORK_END_BODY))
        if (data.sickPromptPending) {
            val body = if (cfg.meds.isEmpty()) "不舒服就别硬扛。设置一下吃药提醒，我来盯着。" else "不舒服就别硬扛，记得按时吃药，多喝热水。"
            add(Alert(AlertKind.Sick, "牛马身体抱恙", body))
        }
        for (med in data.meds.filter { it.pendingSince != null }) {
            val plan = cfg.meds.firstOrNull { it.id == med.planId } ?: continue
            add(Alert(AlertKind.Med(plan.id), "该吃药了：${plan.name}", medBody(plan)))
        }
        for (l in data.loops.filter { it.pendingSince != null }) add(Alert(AlertKind.Loop(l.id), l.title, l.note))
    }

    fun ack(kind: AlertKind, action: AckAction, now: LocalDateTime) {
        val snooze = cfg.snoozeMinutes.coerceAtLeast(1).toLong()
        when (kind) {
            is AlertKind.Loop -> {
                val date = workDay(now)
                val l = data.loops.firstOrNull { it.id == kind.id } ?: return
                if (l.pendingSince == null) return
                l.pendingSince = null
                l.nextFire = if (action == AckAction.Snooze) now.plusMinutes(snooze) else now.plusMinutes(l.intervalMin.toLong())
                when (action) {
                    AckAction.Done -> { l.done++; dayMut(date).done++ }
                    AckAction.Skip -> { l.skipped++; dayMut(date).skipped++ }
                    AckAction.Snooze -> {}
                }
            }
            is AlertKind.Med -> {
                val plan = cfg.meds.firstOrNull { it.id == kind.planId }
                val date = workDay(now)
                val med = data.meds.firstOrNull { it.planId == kind.planId } ?: return
                if (med.pendingSince == null) return
                med.pendingSince = null
                med.nextFire = if (action == AckAction.Snooze || plan == null) now.plusMinutes(snooze) else nextMedFire(plan, now)
                if (action == AckAction.Done) dayMut(date).medsTaken++
            }
            AlertKind.WorkEnd -> {
                if (!data.workEndPending) return
                data.workEndPending = false
                if (action == AckAction.Snooze) {
                    // 再卷一会：半小时后再催一次。
                    data.workEndNotified = null
                    data.workEndSnoozeUntil = now.plusMinutes(WORK_END_SNOOZE_MIN)
                }
            }
            AlertKind.Sick -> data.sickPromptPending = false
        }
        dataDirty = true
    }

    // ---------- 午休 ----------

    /** 自动午休的时间段（今天），没开午休模式、没上班或时间设反时为 null。 */
    private fun autoLunchWindow(now: LocalDateTime): Pair<LocalDateTime, LocalDateTime>? {
        val s = cfg.salary
        if (!cfg.lunchQuiet || !onDuty() || s.lunchStart >= s.lunchEnd) return null
        if (data.lunchAutoOff == now.toLocalDate()) return null
        return now.toLocalDate().atTime(s.lunchStart) to now.toLocalDate().atTime(s.lunchEnd)
    }

    /** 当前所在的午休：开始和结束时刻。 */
    private fun lunchSpan(now: LocalDateTime): Pair<LocalDateTime, LocalDateTime>? {
        val manualUntil = data.lunchManualUntil
        if (manualUntil != null && now.isBefore(manualUntil)) return (data.lunchManualFrom ?: now) to manualUntil
        val (start, end) = autoLunchWindow(now) ?: return null
        return if (!now.isBefore(start) && now.isBefore(end)) start to end else null
    }

    /** 正在午休时返回午休结束时刻。手动午休随时可开；自动午休只在上班期间、设置的午休时间段内生效。 */
    fun lunchUntil(now: LocalDateTime): LocalDateTime? = lunchSpan(now)?.second

    /** 现在开始午休，时长取设置里的午休时长（没设好时按 1 小时）。 */
    fun startLunch(now: LocalDateTime) {
        val s = cfg.salary
        val minutes = if (s.lunchStart < s.lunchEnd) Duration.between(s.lunchStart, s.lunchEnd).toMinutes() else 60
        data.lunchManualFrom = now
        data.lunchManualUntil = now.plusMinutes(minutes)
        data.lunchAutoOff = null
        dataDirty = true
        syncLunchFreeze(now)
    }

    /** 提前结束午休。如果正处在自动午休时间段，当天不再自动进入。 */
    fun endLunch(now: LocalDateTime) {
        data.lunchManualUntil = null
        data.lunchManualFrom = null
        if (lunchUntil(now) != null) data.lunchAutoOff = now.toLocalDate()
        dataDirty = true
        syncLunchFreeze(now)
    }

    /**
     * 午休开始时冻结运行中的循环（剩余时间从午休开始算起），结束时从冻结时的剩余时间继续。
     * 闹钟可能晚到一会儿，所以剩余时间按午休开始时刻算，而不是按 now。
     */
    private fun syncLunchFreeze(now: LocalDateTime) {
        val span = lunchSpan(now)
        if (span != null) {
            for (l in data.loops.filter { it.pausedLeft == null && it.pendingSince == null }) {
                // 午休中途才开始的循环，剩余时间不超过一个周期。
                l.pausedLeft = secs(span.first, l.nextFire).coerceIn(0, l.intervalMin * 60L)
                data.lunchFrozen.add(l.id)
                dataDirty = true
            }
        } else if (data.lunchFrozen.isNotEmpty()) {
            for (l in data.loops.filter { it.id in data.lunchFrozen }) {
                l.nextFire = now.plusSeconds(l.pausedLeft ?: 0)
                l.pausedLeft = null
            }
            data.lunchFrozen.clear()
            dataDirty = true
        }
    }

    // ---------- 唤醒 ----------

    /**
     * 处理到点的事。调用方在 nextWake() 给出的时刻（和打开应用、点通知按钮时）调用。
     * 返回需要发出的通知。
     */
    fun tick(now: LocalDateTime): List<Notice> {
        data.lastSeen = now
        val manualUntil = data.lunchManualUntil
        if (manualUntil != null && !now.isBefore(manualUntil)) {
            data.lunchManualUntil = null
            data.lunchManualFrom = null
            dataDirty = true
        }
        syncLunchFreeze(now)
        val notices = mutableListOf<Notice>()
        // 午休时什么都不弹（吃药提醒也等午休结束再响）。
        if (lunchUntil(now) != null) return notices
        val needsAck = cfg.alertStyle != AlertStyle.Toast

        // 疑似忘打卡时不再催活动，等用户处理完。
        if (forgotten(now) == null) {
            for (l in data.loops.filter { it.pausedLeft == null && it.pendingSince == null && !now.isBefore(it.nextFire) }) {
                notices.add(Notice(AlertKind.Loop(l.id), l.title, l.note, needsAck))
                if (needsAck) l.pendingSince = now else l.nextFire = now.plusMinutes(l.intervalMin.toLong())
                dataDirty = true
            }
            val open = openSession()
            if (open != null) {
                val start = open.start
                val end = Salary.workEndAt(start.toLocalDate(), cfg.salary)
                val notified = data.workEndNotified == start.toLocalDate()
                val snoozed = data.workEndSnoozeUntil?.let { now.isBefore(it) } ?: false
                if (cfg.workEndReminder && start.isBefore(end) && !now.isBefore(end) && !notified && !snoozed) {
                    data.workEndNotified = start.toLocalDate()
                    if (needsAck) data.workEndPending = true
                    notices.add(Notice(AlertKind.WorkEnd, WORK_END_TITLE, WORK_END_BODY, needsAck))
                    dataDirty = true
                }
            }
        }

        // 吃药提醒不看打卡状态。
        for (med in data.meds.filter { it.pendingSince == null && !now.isBefore(it.nextFire) }) {
            val plan = cfg.meds.firstOrNull { it.id == med.planId } ?: continue
            notices.add(Notice(AlertKind.Med(plan.id), "该吃药了：${plan.name}", medBody(plan), needsAck))
            if (needsAck) med.pendingSince = now else med.nextFire = nextMedFire(plan, now)
            dataDirty = true
        }
        return notices
    }

    /** 下一次需要调用 tick() 的时刻；没有待办时为 null。 */
    fun nextWake(now: LocalDateTime): LocalDateTime? {
        val times = mutableListOf<LocalDateTime>()
        val lunch = lunchSpan(now)
        if (lunch != null) {
            times.add(lunch.second)
        } else {
            data.loops.filter { it.pausedLeft == null && it.pendingSince == null }.mapTo(times) { it.nextFire }
            data.meds.filter { it.pendingSince == null }.mapTo(times) { it.nextFire }
            autoLunchWindow(now)?.let { (start, _) -> if (start.isAfter(now)) times.add(start) }
            val open = openSession()
            if (open != null && cfg.workEndReminder && data.workEndNotified != open.start.toLocalDate()) {
                val end = Salary.workEndAt(open.start.toLocalDate(), cfg.salary)
                times.add(maxOf(end, data.workEndSnoozeUntil ?: end))
            }
        }
        val next = times.minOrNull() ?: return null
        return if (next.isAfter(now)) next else now.plusSeconds(1)
    }

    companion object {
        const val WORK_END_TITLE = "到点了，该下班了"
        const val WORK_END_BODY = "工作是老板的，命是自己的。"
        val DT_MIN: java.time.format.DateTimeFormatter = java.time.format.DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm")
        val DT_SEC: java.time.format.DateTimeFormatter = java.time.format.DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")
        val T_SEC: java.time.format.DateTimeFormatter = java.time.format.DateTimeFormatter.ofPattern("HH:mm:ss")
    }
}

fun medBody(plan: MedPlan): String = when {
    plan.dose.isNotEmpty() && plan.note.isNotEmpty() -> "${plan.dose} · ${plan.note}"
    plan.dose.isNotEmpty() -> plan.dose
    plan.note.isNotEmpty() -> plan.note
    else -> "按时吃药，早点好起来。"
}

/** 下一次吃药时间。定时方案找今天剩下的时间点，没有就取明天第一个。 */
fun nextMedFire(plan: MedPlan, now: LocalDateTime): LocalDateTime {
    plan.schedule.interval?.let { return now.plusMinutes(it.minutes.coerceAtLeast(1).toLong()) }
    val times = plan.schedule.times?.times.orEmpty()
    val laterToday = times.map { now.toLocalDate().atTime(it) }.filter { it.isAfter(now) }.minOrNull()
    if (laterToday != null) return laterToday
    val first = times.minOrNull() ?: return now.plusHours(24)
    return now.toLocalDate().plusDays(1).atTime(first)
}
