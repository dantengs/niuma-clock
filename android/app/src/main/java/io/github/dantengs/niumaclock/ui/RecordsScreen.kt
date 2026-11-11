package io.github.dantengs.niumaclock.ui

import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.dantengs.niumaclock.Fmt
import java.time.Duration
import java.time.YearMonth

@Composable
fun RecordsScreen(ui: Ui) {
    val p = LocalPalette.current
    val context = LocalContext.current
    var month by rememberSaveable { mutableStateOf(YearMonth.from(ui.now).toString()) }
    val ym = YearMonth.parse(month)
    var deleting by remember { mutableStateOf<Int?>(null) }
    val export = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("text/csv")) { uri ->
        if (uri != null) {
            context.contentResolver.openOutputStream(uri)?.use { it.write((Char(0xFEFF) + ui.e.toCsv()).toByteArray()) }
            Toast.makeText(context, "已导出打卡记录 CSV，Excel 可以直接打开", Toast.LENGTH_SHORT).show()
        }
    }

    val m = ui.e.monthSummary(ym.year, ym.monthValue)
    NCard {
        Row(verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = { month = ym.minusMonths(1).toString() }) { Text("‹") }
            Text("${ym.year} 年 ${ym.monthValue} 月", fontWeight = FontWeight.Bold, color = p.ink)
            TextButton(onClick = { month = ym.plusMonths(1).toString() }, enabled = ym < YearMonth.from(ui.now)) { Text("›") }
            Spacer(Modifier.weight(1f))
            TextButton(onClick = { export.launch("牛马打卡记录.csv") }) { Text("导出 CSV") }
        }
        Row(Modifier.fillMaxWidth()) {
            Cell("出勤", "${m.days} 天", Modifier.weight(1f))
            Cell("工时", Fmt.hours(m.workedSecs), Modifier.weight(1f))
            Cell("加班", Fmt.hours(m.overtimeSecs), Modifier.weight(1f))
        }
        Cell("收入", ui.money(m.income) + if (Math.abs(m.adjust) >= 0.005 && !ui.cfg.hideMoney) "（含补记 ${Fmt.signedMoney(ui.cfg, m.adjust)}）" else "", Modifier)
        DailyChart(ui, ym)
    }
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("打卡记录", "点时间可以修改")
        val indexed = ui.e.data.sessions.withIndex()
            .filter { it.value.start.year == ym.year && it.value.start.monthValue == ym.monthValue }
            .reversed()
        if (indexed.isEmpty()) Muted("这个月没有打卡记录。")
        for ((n, iv) in indexed.withIndex()) {
            val (index, s) = iv
            if (n > 0) HorizontalDivider(color = p.line)
            Column {
                Text(Fmt.dateLabel(s.start.toLocalDate()), fontWeight = FontWeight.Bold, color = p.ink, fontSize = 14.sp)
                val end = s.end
                if (end == null) {
                    Muted("${s.start.format(Fmt.HM)} 上班 · 进行中")
                } else {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        TextButton(onClick = { pickDateTime(context, s.start) { t -> ui.update { it.editSession(index, t, end) } } }) { Text(s.start.format(Fmt.HM)) }
                        Muted("→")
                        TextButton(onClick = { pickDateTime(context, end) { t -> ui.update { it.editSession(index, s.start, t) } } }) {
                            Text(if (end.toLocalDate() == s.start.toLocalDate()) end.format(Fmt.HM) else end.format(Fmt.MD_HM))
                        }
                        Spacer(Modifier.weight(1f))
                        TextButton(onClick = { deleting = index }) { Text("删除", color = p.danger, fontSize = 12.sp) }
                    }
                    Muted("工作 ${Fmt.duration(Duration.between(s.start, end).seconds.toDouble())} · 收入 ${ui.money(s.income)}" +
                        if (s.overtimeSecs > 0) " · 加班 ${Fmt.duration(s.overtimeSecs)}" else "", 12.sp)
                }
            }
        }
    }

    deleting?.let { index ->
        AlertDialog(
            onDismissRequest = { deleting = null },
            title = { Text("删除这条打卡记录？") },
            text = { Text("删除后当天收入会重新计算，不能撤销。") },
            confirmButton = { TextButton(onClick = { ui.update { it.deleteSession(index) }; deleting = null }) { Text("删除", color = p.danger) } },
            dismissButton = { TextButton(onClick = { deleting = null }) { Text("取消") } },
        )
    }
}

@Composable
private fun Cell(label: String, value: String, modifier: Modifier) {
    val p = LocalPalette.current
    Column(modifier) {
        Muted(label, 12.sp)
        Text(value, fontSize = 15.sp, fontWeight = FontWeight.Bold, color = p.ink)
    }
}

/** 每天工时柱状图，标准工时处画一条虚线。 */
@Composable
private fun DailyChart(ui: Ui, ym: YearMonth) {
    val p = LocalPalette.current
    val worked = ui.e.dailyWorked(ym.year, ym.monthValue)
    if (worked.isEmpty()) return
    val days = ym.lengthOfMonth()
    val standard = io.github.dantengs.niumaclock.core.Salary.standardDailySecs(ui.cfg.salary)
    val top = maxOf(worked.values.maxOrNull() ?: 0.0, standard, 3600.0)
    Canvas(Modifier.fillMaxWidth().height(110.dp)) {
        val slot = size.width / days
        val barW = slot * 0.62f
        for (d in 1..days) {
            val secs = worked[ym.atDay(d)] ?: continue
            val h = (secs / top * size.height).toFloat()
            val color = if (secs > standard + 60) p.accent else p.green
            drawRoundRect(color, Offset(slot * (d - 1) + (slot - barW) / 2, size.height - h), Size(barW, h), CornerRadius(3f, 3f))
        }
        if (standard > 0) {
            val y = (size.height - standard / top * size.height).toFloat()
            var x = 0f
            while (x < size.width) {
                drawLine(p.muted, Offset(x, y), Offset(minOf(x + 8f, size.width), y), strokeWidth = 2f)
                x += 16f
            }
        }
    }
    Muted("绿色为正常，橙色为加班；虚线是标准工时 ${Fmt.hours(standard)}", 11.sp)
}
