package io.github.dantengs.niumaclock.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.dantengs.niumaclock.Fmt
import io.github.dantengs.niumaclock.core.Health
import io.github.dantengs.niumaclock.core.MedSchedule
import io.github.dantengs.niumaclock.core.Mood
import io.github.dantengs.niumaclock.core.Salary

@Composable
fun HealthScreen(ui: Ui) {
    val p = LocalPalette.current
    var adding by remember { mutableStateOf(false) }

    NCard {
        SectionTitle("身体")
        FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            for (h in Health.entries) {
                val color = when (h) { Health.Healthy -> p.green; Health.SubHealthy -> p.gold; Health.Unwell -> p.danger }
                Chip(h.label, ui.e.data.health == h, color) { ui.update { it.setHealth(h, ui.now) } }
            }
        }
        Muted("设为“不健康”后按吃药方案提醒，下班也照常提醒，恢复健康后停止。", 12.sp)
        SectionTitle("心情")
        FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            for (m in Mood.entries) Chip(m.label, ui.e.data.mood == m) { ui.update { it.setMood(m, ui.now) } }
        }
    }
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("吃药提醒") { TextButton(onClick = { adding = true }) { Text("+ 添加") } }
        if (ui.cfg.meds.isEmpty()) Muted("还没有吃药方案。添加后，只在身体设为“不健康”时提醒。")
        for ((i, m) in ui.cfg.meds.withIndex()) {
            if (i > 0) HorizontalDivider(color = p.line)
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(m.name, fontWeight = FontWeight.Bold, color = p.ink)
                val schedule = m.schedule.interval?.let { "每 ${it.minutes} 分钟" }
                    ?: "每天 " + m.schedule.times?.times.orEmpty().joinToString("、") { it.format(Fmt.HM) }
                Muted(listOf(m.dose, schedule, m.note).filter { it.isNotEmpty() }.joinToString(" · "), 12.sp)
                ui.e.data.meds.firstOrNull { it.planId == m.id }?.let { run ->
                    Muted(if (run.pendingSince != null) "等你确认吃了没" else "下次 ${run.nextFire.format(Fmt.MD_HM)}", 12.sp, p.accent)
                }
                Row { TextButton(onClick = { ui.update { it.removeMed(m.id) } }) { Text("删除", color = p.danger) } }
            }
        }
    }
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("最近的状态变化")
        val log = ui.e.data.statusLog.takeLast(12).reversed()
        if (log.isEmpty()) Muted("还没有记录。")
        for (entry in log) {
            val what = entry.change.health?.let { "身体 → ${it.label}" } ?: entry.change.mood?.let { "心情 → ${it.label}" } ?: ""
            Row { Muted(entry.at.format(Fmt.MD_HM) + "   ", 12.sp); Text(what, fontSize = 13.sp, color = p.ink) }
        }
    }

    if (adding) MedDialog(ui) { adding = false }
}

@Composable
private fun MedDialog(ui: Ui, onClose: () -> Unit) {
    var name by remember { mutableStateOf("") }
    var dose by remember { mutableStateOf("") }
    var note by remember { mutableStateOf("") }
    var byTimes by remember { mutableStateOf(false) }
    var minutes by remember { mutableStateOf("240") }
    var times by remember { mutableStateOf("08:00, 12:30, 20:00") }
    val schedule = if (byTimes) Salary.parseTimes(times)?.let { MedSchedule.at(it) }
    else minutes.toIntOrNull()?.takeIf { it in 1..1440 }?.let { MedSchedule.every(it) }
    AlertDialog(
        onDismissRequest = onClose,
        title = { Text("添加吃药方案") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(name, { name = it }, label = { Text("药名") }, singleLine = true)
                OutlinedTextField(dose, { dose = it }, label = { Text("用量，比如“1 粒”") }, singleLine = true)
                OutlinedTextField(note, { note = it }, label = { Text("备注，比如“饭后”") }, singleLine = true)
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    Chip("固定间隔", !byTimes) { byTimes = false }
                    Chip("固定时间", byTimes) { byTimes = true }
                }
                if (byTimes) {
                    OutlinedTextField(times, { times = it }, label = { Text("时间，逗号隔开") }, singleLine = true, isError = schedule == null)
                } else {
                    OutlinedTextField(
                        minutes, { minutes = it.filter(Char::isDigit) }, label = { Text("每隔几分钟") }, singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number), isError = schedule == null,
                    )
                }
            }
        },
        confirmButton = {
            Button(enabled = name.isNotBlank() && schedule != null, onClick = {
                ui.update { it.addMed(name.trim(), dose.trim(), note.trim(), schedule!!, ui.now) }
                onClose()
            }) { Text("保存") }
        },
        dismissButton = { TextButton(onClick = onClose) { Text("取消") } },
    )
}
