package io.github.dantengs.niumaclock.ui

import android.app.DatePickerDialog
import android.app.TimePickerDialog
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime

@Composable
fun NCard(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    val p = LocalPalette.current
    Column(
        modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(p.card)
            .border(1.dp, p.line, RoundedCornerShape(16.dp))
            .padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        content = content,
    )
}

@Composable
fun SectionTitle(title: String, hint: String = "", trailing: @Composable RowScope.() -> Unit = {}) {
    val p = LocalPalette.current
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(title, fontWeight = FontWeight.Bold, fontSize = 16.sp, color = p.ink)
        if (hint.isNotEmpty()) Text("  $hint", fontSize = 12.sp, color = p.muted)
        Box(Modifier.weight(1f))
        trailing()
    }
}

@Composable
fun Muted(text: String, size: TextUnit = 13.sp, color: Color = LocalPalette.current.muted) =
    Text(text, fontSize = size, color = color)

@Composable
fun Num(text: String, size: TextUnit, color: Color, modifier: Modifier = Modifier) =
    Text(text, fontSize = size, color = color, fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Bold, modifier = modifier, maxLines = 1)

/** 可选的小胶囊。 */
@Composable
fun Chip(label: String, selected: Boolean, color: Color = LocalPalette.current.accent, onClick: () -> Unit) {
    val p = LocalPalette.current
    Box(
        Modifier
            .clip(RoundedCornerShape(50))
            .background(if (selected) color else p.sunken)
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 7.dp),
    ) {
        Text(label, fontSize = 13.sp, color = if (selected) Palette.onColorFor(color) else p.ink)
    }
}

/** 进度条。 */
@Composable
fun Bar(fraction: Float, height: Int, track: Color, fill: Color) {
    Box(Modifier.fillMaxWidth().height(height.dp).clip(RoundedCornerShape(50)).background(track)) {
        Box(Modifier.fillMaxWidth(fraction.coerceIn(0f, 1f)).height(height.dp).clip(RoundedCornerShape(50)).background(fill))
    }
}

/** 点一下弹出系统时间选择器。 */
@Composable
fun TimeButton(time: LocalTime, onPick: (LocalTime) -> Unit) {
    val context = LocalContext.current
    TextButton(onClick = {
        TimePickerDialog(context, { _, h, m -> onPick(LocalTime.of(h, m)) }, time.hour, time.minute, true).show()
    }) { Text(time.format(io.github.dantengs.niumaclock.Fmt.HM), fontSize = 16.sp) }
}

/** 先选日期再选时间。 */
fun pickDateTime(context: android.content.Context, initial: LocalDateTime, onPick: (LocalDateTime) -> Unit) {
    DatePickerDialog(context, { _, y, mon, d ->
        TimePickerDialog(context, { _, h, m -> onPick(LocalDateTime.of(LocalDate.of(y, mon + 1, d), LocalTime.of(h, m))) },
            initial.hour, initial.minute, true).show()
    }, initial.year, initial.monthValue - 1, initial.dayOfMonth).show()
}
