package io.github.dantengs.niumaclock.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.dantengs.niumaclock.Fmt
import io.github.dantengs.niumaclock.core.AckAction
import io.github.dantengs.niumaclock.core.ActiveLoop
import io.github.dantengs.niumaclock.core.AlertKind
import io.github.dantengs.niumaclock.core.DaySummary
import io.github.dantengs.niumaclock.core.Health
import io.github.dantengs.niumaclock.core.Mood
import io.github.dantengs.niumaclock.core.Quotes
import io.github.dantengs.niumaclock.core.Salary
import io.github.dantengs.niumaclock.core.TickUnit
import io.github.dantengs.niumaclock.core.hpRank
import java.time.Duration

@Composable
fun TodayScreen(ui: Ui, onClockOut: (DaySummary) -> Unit, openSettings: () -> Unit, openHealth: () -> Unit) {
    val p = LocalPalette.current
    val stats = ui.e.stats(ui.now)
    var monthEdit by remember { mutableStateOf(false) }

    Hero(ui, onClockOut, openSettings)
    Spacer(Modifier.height(10.dp))
    PendingAlerts(ui)

    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        NCard(Modifier.weight(1f)) {
            Muted("牛马血条", 12.sp)
            Num(stats.hp.toString(), 22.sp, hpColor(stats.hp))
            Bar(stats.hp / 100f, 5, p.sunken, hpColor(stats.hp))
            Muted(hpRank(stats.hp), 12.sp)
        }
        NCard(Modifier.weight(1f)) {
            Muted("距发薪日", 12.sp)
            val days = Salary.daysToPayday(ui.now.toLocalDate(), ui.cfg.salary.payday)
            if (days == 0L) Text("今天！", fontSize = 20.sp, fontWeight = FontWeight.Bold, color = p.accent)
            else Row(verticalAlignment = Alignment.Bottom) { Num("$days", 22.sp, p.ink); Muted(" 天", 12.sp) }
            Muted("每月 ${ui.cfg.salary.payday} 号", 12.sp)
        }
    }
    Spacer(Modifier.height(8.dp))
    NCard {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Muted("本月已赚", 12.sp)
            Spacer(Modifier.weight(1f))
            Text("修改", color = p.accent, fontSize = 13.sp, modifier = Modifier.clickable { monthEdit = true })
        }
        Num(ui.money(stats.monthEarned), 22.sp, p.ink)
        Muted(
            if (Math.abs(stats.monthAdjust) >= 0.005 && !ui.cfg.hideMoney) "含手动补记 ${Fmt.signedMoney(ui.cfg, stats.monthAdjust)}"
            else "今日完成提醒 ${stats.done} 次",
            12.sp,
        )
    }
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("今天状态如何")
        Row(verticalAlignment = Alignment.CenterVertically) {
            Muted("身体  ")
            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                for (h in Health.entries) {
                    val color = when (h) { Health.Healthy -> p.green; Health.SubHealthy -> p.gold; Health.Unwell -> p.danger }
                    Chip(h.label, ui.e.data.health == h, color) { ui.update { it.setHealth(h, ui.now) } }
                }
            }
        }
        Row(verticalAlignment = Alignment.Top) {
            Muted("心情  ", modifier = Modifier.padding(top = 7.dp))
            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                for (m in Mood.entries) Chip(m.label, ui.e.data.mood == m) { ui.update { it.setMood(m, ui.now) } }
            }
        }
        if (ui.e.data.health == Health.Unwell) {
            Text(
                if (ui.cfg.meds.isEmpty()) "还没有吃药提醒，点这里设置" else "查看吃药提醒",
                color = p.danger, fontSize = 13.sp, modifier = Modifier.clickable(onClick = openHealth),
            )
        }
    }
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("运行中的循环")
        if (ui.e.data.loops.isEmpty()) {
            Muted(if (stats.onDuty) "当前没有循环事项，去“循环”页添加一个吧。" else "上班打卡后，常用事项会自动开始计时。")
        }
        for (l in ui.e.data.loops) LoopProgress(ui, l)
    }
    Spacer(Modifier.height(10.dp))
    Text(
        Quotes.daily(ui.now.toLocalDate(), ui.e.data.mood), fontSize = 13.sp, fontStyle = FontStyle.Italic, color = p.muted,
        modifier = Modifier.fillMaxWidth(), textAlign = androidx.compose.ui.text.style.TextAlign.Center,
    )

    if (monthEdit) MonthEditDialog(ui, stats.monthEarned) { monthEdit = false }
}

@Composable
private fun Muted(text: String, modifier: Modifier) = Text(text, fontSize = 13.sp, color = LocalPalette.current.muted, modifier = modifier)

@Composable
fun hpColor(hp: Int): Color {
    val p = LocalPalette.current
    return when {
        hp >= 70 -> p.green
        hp >= 40 -> if (p.dark) p.gold else Color(0xFFB57A10)
        else -> p.danger
    }
}

@Composable
private fun Hero(ui: Ui, onClockOut: (DaySummary) -> Unit, openSettings: () -> Unit) {
    val p = LocalPalette.current
    val stats = ui.e.stats(ui.now)
    val s = ui.cfg.salary
    Column(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(20.dp)).background(p.hero).padding(18.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("今日已赚 · 窝囊费实时到账", fontSize = 13.sp, color = p.heroMuted, modifier = Modifier.weight(1f))
            HeroPill(if (ui.cfg.hideMoney) "显示金额" else "隐藏金额") { ui.update { it.cfg.hideMoney = !it.cfg.hideMoney; it.cfgDirty = true } }
        }
        if (s.monthly <= 0.0) {
            Text("先告诉我你月薪多少", fontSize = 24.sp, fontWeight = FontWeight.Bold, color = p.gold)
            Text("数据只存在这台手机上，不联网、不上传。", fontSize = 13.sp, color = p.heroMuted)
            Button(onClick = openSettings, colors = ButtonDefaults.buttonColors(containerColor = p.gold, contentColor = Color(0xFF2B2008))) { Text("去设置月薪") }
        } else {
            Row(verticalAlignment = Alignment.Bottom) {
                if (s.tickUnit == TickUnit.Second) {
                    val (head, tail) = Fmt.moneyParts(ui.cfg, stats.earnedDisplay)
                    Num(head, 40.sp, p.gold)
                    Num(tail, 20.sp, p.heroMuted, Modifier.padding(bottom = 5.dp))
                } else {
                    Num(ui.money(stats.earnedDisplay), 40.sp, p.gold)
                }
            }
            val rate = if (ui.cfg.hideMoney) "${s.tickUnit.label} +***" else "${s.tickUnit.label} +${s.currency}${"%.4f".format(Salary.tickAmount(s))}"
            Text(rate, fontSize = 13.sp, color = p.heroMuted)
        }
        Spacer(Modifier.height(2.dp))
        Bar(stats.progress, 8, Color.White.copy(alpha = 0.11f), p.gold)
        Row {
            Text("今日进度 ${"%.0f".format(stats.progress * 100)}%", fontSize = 12.sp, color = p.heroMuted, modifier = Modifier.weight(1f))
            Num(countdownText(ui), 14.sp, p.heroInk)
        }
        Button(
            onClick = {
                if (stats.onDuty) ui.update { e -> e.clockOut(ui.now)?.let(onClockOut) } else ui.update { it.clockIn(ui.now) }
            },
            modifier = Modifier.fillMaxWidth().height(54.dp),
            shape = RoundedCornerShape(14.dp),
            colors = ButtonDefaults.buttonColors(
                containerColor = if (stats.onDuty) p.accent else Palette.LIGHT.green,
                contentColor = if (stats.onDuty) p.onAccent else Color.White,
            ),
        ) { Text(if (stats.onDuty) "下班打卡" else "上班打卡", fontSize = 20.sp, fontWeight = FontWeight.Bold) }
        val sub = when {
            stats.lunchUntil != null -> "午休中，${stats.lunchUntil.format(Fmt.HM)} 结束 · 期间不弹提醒，循环倒计时已暂停"
            stats.sessionStart != null -> "${stats.sessionStart.format(Fmt.HM)} 上班 · 已工作 ${Fmt.duration(stats.workedSecs)}"
            stats.workedSecs > 0 -> "今天已工作 ${Fmt.duration(stats.workedSecs)}，可以再次打卡继续"
            else -> "今天还没打卡"
        }
        Text(sub, fontSize = 12.sp, color = if (stats.lunchUntil != null) p.gold else p.heroMuted, modifier = Modifier.fillMaxWidth(), textAlign = androidx.compose.ui.text.style.TextAlign.Center)
        if (stats.onDuty || stats.lunchUntil != null) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center) {
                HeroPill(if (stats.lunchUntil != null) "结束午休" else "开始午休（不打扰）") {
                    ui.update { if (stats.lunchUntil != null) it.endLunch(ui.now) else it.startLunch(ui.now) }
                }
            }
        }
    }
}

@Composable
private fun HeroPill(label: String, onClick: () -> Unit) {
    val p = LocalPalette.current
    Box(
        Modifier.clip(RoundedCornerShape(10.dp)).background(Color.White.copy(alpha = 0.09f)).clickable(onClick = onClick)
            .padding(horizontal = 10.dp, vertical = 6.dp),
    ) { Text(label, fontSize = 12.sp, color = p.heroInk) }
}

private fun countdownText(ui: Ui): String {
    val stats = ui.e.stats(ui.now)
    if (!stats.onDuty) return "未在上班"
    val left = stats.toEndSecs ?: return "计划外上班 ${Fmt.clock(stats.workedSecs.toLong())}"
    return if (left > 0) "距下班 ${Fmt.clock(left)}"
    else "${if (ui.cfg.salary.overtimeMultiplier <= 0.0) "白嫖中" else "加班中"} ${Fmt.clock(-left)}"
}

/** 待处理的提醒：通知被划掉或没看到时，在这里也能处理。 */
@Composable
private fun PendingAlerts(ui: Ui) {
    val p = LocalPalette.current
    val alerts = ui.e.alerts()
    if (alerts.isEmpty()) return
    for (a in alerts) {
        Column(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).background(p.dangerSoft).padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text(a.title, fontWeight = FontWeight.Bold, fontSize = 16.sp, color = p.ink)
            if (a.body.isNotEmpty()) Muted(a.body)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                fun ack(action: AckAction) = ui.update { it.ack(a.kind, action, ui.now) }
                when (a.kind) {
                    is AlertKind.Loop -> {
                        Button(onClick = { ack(AckAction.Done) }) { Text("搞定了") }
                        OutlinedButton(onClick = { ack(AckAction.Snooze) }) { Text("稍后") }
                        TextButton(onClick = { ack(AckAction.Skip) }) { Text("跳过") }
                    }
                    is AlertKind.Med -> {
                        Button(onClick = { ack(AckAction.Done) }) { Text("吃了") }
                        OutlinedButton(onClick = { ack(AckAction.Snooze) }) { Text("稍后") }
                    }
                    AlertKind.WorkEnd -> {
                        Button(onClick = { ack(AckAction.Done) }) { Text("知道了") }
                        OutlinedButton(onClick = { ack(AckAction.Snooze) }) { Text("再卷半小时") }
                    }
                    AlertKind.Sick -> Button(onClick = { ack(AckAction.Done) }) { Text("知道了") }
                }
            }
        }
        Spacer(Modifier.height(8.dp))
    }
}

@Composable
fun LoopProgress(ui: Ui, l: ActiveLoop) {
    val p = LocalPalette.current
    val total = (l.intervalMin * 60L).coerceAtLeast(1)
    val (right, frac, color) = when {
        l.pendingSince != null -> Triple("到点了", 1f, p.danger)
        l.pausedLeft != null -> {
            val left = l.pausedLeft!!
            Triple("${if (ui.e.isLunchFrozen(l.id)) "午休暂停" else "已暂停"} · 剩 ${Fmt.clock(left)}", 1f - left.toFloat() / total, p.muted)
        }
        else -> {
            val left = Duration.between(ui.now, l.nextFire).seconds.coerceAtLeast(0)
            Triple("还剩 ${Fmt.clock(left)}", 1f - left.toFloat() / total, p.accent)
        }
    }
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row {
            Text(l.title, fontSize = 14.sp, color = p.ink, modifier = Modifier.weight(1f), maxLines = 1)
            Num(right, 13.sp, color)
        }
        Bar(frac, 5, p.sunken, color)
    }
}

@Composable
private fun MonthEditDialog(ui: Ui, current: Double, onClose: () -> Unit) {
    var text by remember { mutableStateOf("%.2f".format(current)) }
    val value = text.toDoubleOrNull()
    val daily = ui.cfg.salary.monthly / ui.cfg.salary.paidDays.coerceAtLeast(1.0)
    AlertDialog(
        onDismissRequest = onClose,
        title = { Text("修改本月已赚") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("工资不是从装软件那天算起的？填上这个月到现在实际赚了多少。之后照常累加，下个月自动从零开始。", fontSize = 13.sp)
                OutlinedTextField(
                    value = text, onValueChange = { text = it }, singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    prefix = { Text(ui.cfg.salary.currency) },
                )
                if (value != null && daily > 0) Muted("约等于 ${"%.1f".format(value / daily)} 天日薪", 12.sp)
            }
        },
        confirmButton = {
            TextButton(enabled = value != null && value >= 0, onClick = { ui.update { it.setMonthEarned(ui.now, value!!) }; onClose() }) { Text("保存") }
        },
        dismissButton = {
            Row {
                if (Math.abs(ui.e.monthAdjust(ui.now.year, ui.now.monthValue)) >= 0.005) {
                    TextButton(onClick = { ui.update { it.clearMonthAdjust(ui.now.year, ui.now.monthValue) }; onClose() }) { Text("清除补记") }
                }
                TextButton(onClick = onClose) { Text("取消") }
            }
        },
    )
}
