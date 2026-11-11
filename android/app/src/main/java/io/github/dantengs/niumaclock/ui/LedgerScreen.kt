package io.github.dantengs.niumaclock.ui

import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.dantengs.niumaclock.Fmt
import io.github.dantengs.niumaclock.core.EXPENSE_CATEGORIES
import io.github.dantengs.niumaclock.core.Expense
import io.github.dantengs.niumaclock.core.Salary
import java.time.LocalDateTime
import java.time.YearMonth

@Composable
fun LedgerScreen(ui: Ui) {
    val p = LocalPalette.current
    val context = LocalContext.current
    var month by rememberSaveable { mutableStateOf(YearMonth.from(ui.now).toString()) }
    val ym = YearMonth.parse(month)
    var undo by remember { mutableStateOf<Expense?>(null) }
    val export = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("text/csv")) { uri ->
        if (uri != null) {
            context.contentResolver.openOutputStream(uri)?.use { it.write((Char(0xFEFF) + ui.e.expensesCsv()).toByteArray()) }
            Toast.makeText(context, "已导出记账 CSV", Toast.LENGTH_SHORT).show()
        }
    }

    ExpenseForm(ui)
    Spacer(Modifier.height(8.dp))

    val summary = ui.e.monthExpenses(ym.year, ym.monthValue)
    // 当前月含正在进行的这段打卡。
    val stats = ui.e.stats(ui.now)
    val earned = if (ym == YearMonth.from(stats.workDay)) stats.monthEarned else ui.e.monthSummary(ym.year, ym.monthValue).income
    NCard {
        Row(verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = { month = ym.minusMonths(1).toString() }) { Text("‹") }
            Text("${ym.year} 年 ${ym.monthValue} 月", fontWeight = FontWeight.Bold, color = p.ink)
            TextButton(onClick = { month = ym.plusMonths(1).toString() }, enabled = ym < YearMonth.from(ui.now)) { Text("›") }
            Spacer(Modifier.weight(1f))
            TextButton(onClick = { export.launch("牛马记账.csv") }) { Text("导出 CSV") }
        }
        Row(Modifier.fillMaxWidth()) {
            Stat("花费", ui.money(summary.total), p.danger, Modifier.weight(1f))
            Stat("已赚", ui.money(earned), p.green, Modifier.weight(1f))
            Stat("结余", Fmt.signedMoney(ui.cfg, earned - summary.total), if (earned >= summary.total) p.ink else p.danger, Modifier.weight(1f))
        }
        if (summary.count == 0) Muted("这个月还没有记账。")
        for ((cat, amount) in summary.byCategory) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(cat, fontSize = 13.sp, color = p.ink, modifier = Modifier.width(44.dp))
                Column(Modifier.weight(1f)) { Bar((amount / summary.total).toFloat(), 8, p.sunken, p.accent) }
                Spacer(Modifier.width(8.dp))
                Num(ui.money(amount), 12.sp, p.muted)
            }
        }
    }
    Spacer(Modifier.height(8.dp))

    undo?.let { removed ->
        NCard {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Muted("已删除 ${removed.category} ${ui.money(removed.amount)}", modifier = Modifier.weight(1f))
                TextButton(onClick = { ui.update { it.restoreExpense(removed) }; undo = null }) { Text("撤销") }
            }
        }
        Spacer(Modifier.height(8.dp))
    }

    NCard {
        SectionTitle("明细", "${summary.count} 笔")
        val list = ui.e.monthExpenseList(ym.year, ym.monthValue)
        for ((i, x) in list.withIndex()) {
            if (i > 0) HorizontalDivider(color = p.line)
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("${x.category}${if (x.note.isNotEmpty()) " · ${x.note}" else ""}", color = p.ink, fontSize = 14.sp)
                    Muted(x.at.format(Fmt.MD_HM), 12.sp)
                }
                Num(ui.money(x.amount), 14.sp, p.ink)
                TextButton(onClick = { ui.update { e -> undo = e.deleteExpense(x.id) } }) { Text("删除", color = p.danger, fontSize = 12.sp) }
            }
        }
    }
}

@Composable
private fun Muted(text: String, modifier: Modifier) = Text(text, fontSize = 13.sp, color = LocalPalette.current.muted, modifier = modifier)

@Composable
private fun Stat(label: String, value: String, color: androidx.compose.ui.graphics.Color, modifier: Modifier) {
    Column(modifier) {
        Muted(label, 12.sp)
        Num(value, 15.sp, color)
    }
}

@Composable
private fun ExpenseForm(ui: Ui) {
    val p = LocalPalette.current
    val context = LocalContext.current
    var amount by remember { mutableStateOf("") }
    var category by remember { mutableStateOf(EXPENSE_CATEGORIES[0]) }
    var note by remember { mutableStateOf("") }
    var at by remember { mutableStateOf<LocalDateTime?>(null) }
    val value = amount.toDoubleOrNull()
    fun submit() {
        if (value == null || value <= 0) return
        ui.update { it.addExpense(at ?: ui.now, value, category, note) }
        amount = ""; note = ""; at = null
    }
    NCard {
        SectionTitle("记一笔")
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(
                amount, { amount = it }, label = { Text("金额") }, singleLine = true, modifier = Modifier.weight(1f),
                prefix = { Text(ui.cfg.salary.currency) },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { submit() }),
            )
            OutlinedTextField(note, { note = it }, label = { Text("备注") }, singleLine = true, modifier = Modifier.weight(1f))
        }
        FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            for (c in EXPENSE_CATEGORIES) Chip(c, c == category) { category = c }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = { pickDateTime(context, at ?: ui.now) { at = it } }) {
                Text(at?.let { "时间：${it.format(Fmt.MD_HM)}" } ?: "时间：现在")
            }
            Spacer(Modifier.weight(1f))
            Button(enabled = value != null && value > 0, onClick = { submit() }) { Text("记下") }
        }
        // 提示行一直占位，免得出现时把按钮挤下去。
        val rate = Salary.ratePerSec(ui.cfg.salary) * 3600
        val hint = if (value != null && value > 0 && rate > 0 && !ui.cfg.hideMoney) "这笔相当于上 ${"%.1f".format(value / rate)} 小时班" else " "
        Muted(hint, 12.sp, p.accent)
    }
}
