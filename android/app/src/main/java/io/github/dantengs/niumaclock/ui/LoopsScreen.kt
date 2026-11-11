package io.github.dantengs.niumaclock.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.dantengs.niumaclock.core.ReminderTemplate

@Composable
fun LoopsScreen(ui: Ui) {
    val p = LocalPalette.current
    var editing by remember { mutableStateOf<ReminderTemplate?>(null) }
    var adding by remember { mutableStateOf(false) }
    var addingTemp by remember { mutableStateOf(false) }

    NCard {
        SectionTitle("运行中") {
            if (ui.e.onDuty()) TextButton(onClick = { addingTemp = true }) { Text("+ 今日临时事项") }
        }
        if (ui.e.data.loops.isEmpty()) {
            Muted(if (ui.e.onDuty()) "没有运行中的循环。可以加一个今日临时事项，或者在下面开始常用事项。" else "上班打卡后，勾了“上班自动开始”的常用事项会自动计时。")
        }
        for (l in ui.e.data.loops) {
            LoopProgress(ui, l)
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                val frozen = ui.e.isLunchFrozen(l.id)
                if (!frozen && l.pendingSince == null) {
                    TextButton(onClick = { ui.update { it.togglePause(l.id, ui.now) } }) { Text(if (l.pausedLeft != null) "继续" else "暂停") }
                }
                TextButton(onClick = { ui.update { it.restartLoop(l.id, ui.now) } }) { Text("重新计时") }
                TextButton(onClick = { ui.update { it.removeLoop(l.id) } }) { Text("移除", color = p.danger) }
                Spacer(Modifier.weight(1f))
                Muted("完成 ${l.done} · 跳过 ${l.skipped}", 12.sp)
            }
        }
    }
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("常用事项", "存下来，每天都能用") { TextButton(onClick = { adding = true }) { Text("+ 添加") } }
        if (ui.cfg.reminders.isEmpty()) Muted("还没有常用事项。")
        for ((i, t) in ui.cfg.reminders.withIndex()) {
            if (i > 0) HorizontalDivider(color = p.line)
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(t.title, fontWeight = FontWeight.Bold, color = p.ink)
                Muted("每 ${t.intervalMin} 分钟${if (t.autoStart) " · 上班自动开始" else ""}${if (t.note.isNotEmpty()) " · ${t.note}" else ""}", 12.sp)
                Row {
                    val running = ui.e.data.loops.any { it.templateId == t.id }
                    if (!running && ui.e.onDuty()) TextButton(onClick = { ui.update { it.startTemplate(t.id, ui.now) } }) { Text("今天开始") }
                    TextButton(onClick = { editing = t }) { Text("编辑") }
                    TextButton(onClick = { ui.update { it.removeTemplate(t.id) } }) { Text("删除", color = p.danger) }
                }
            }
        }
    }

    if (adding) {
        LoopDialog("添加常用事项", ReminderTemplate(0, "", "", 45, true), showAuto = true, onDismiss = { adding = false }) { t ->
            ui.update { it.addTemplate(t.title, t.note, t.intervalMin, t.autoStart) }
            adding = false
        }
    }
    if (addingTemp) {
        LoopDialog("今日临时事项", ReminderTemplate(0, "", "", 30, false), showAuto = false, onDismiss = { addingTemp = false }) { t ->
            ui.update { it.addTempLoop(t.title, t.note, t.intervalMin, ui.now) }
            addingTemp = false
        }
    }
    editing?.let { tpl ->
        LoopDialog("编辑常用事项", tpl, showAuto = true, onDismiss = { editing = null }) { t ->
            ui.update { it.updateTemplate(t, ui.now) }
            editing = null
        }
    }
}

@Composable
private fun LoopDialog(title: String, initial: ReminderTemplate, showAuto: Boolean, onDismiss: () -> Unit, onSave: (ReminderTemplate) -> Unit) {
    var name by remember { mutableStateOf(initial.title) }
    var note by remember { mutableStateOf(initial.note) }
    var minutes by remember { mutableStateOf(initial.intervalMin.toString()) }
    var auto by remember { mutableStateOf(initial.autoStart) }
    val interval = minutes.toIntOrNull()
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(name, { name = it }, label = { Text("事项，比如“喝口水”") }, singleLine = true)
                OutlinedTextField(note, { note = it }, label = { Text("提示语（可不填）") })
                OutlinedTextField(
                    minutes, { minutes = it.filter(Char::isDigit) }, label = { Text("每隔几分钟") }, singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                )
                if (showAuto) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(auto, { auto = it })
                        Spacer(Modifier.width(4.dp))
                        Text("上班打卡时自动开始")
                    }
                }
            }
        },
        confirmButton = {
            Button(enabled = name.isNotBlank() && interval != null && interval in 1..1440, onClick = {
                onSave(initial.copy(title = name.trim(), note = note.trim(), intervalMin = interval!!, autoStart = auto))
            }) { Text("保存") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("取消") } },
    )
}
