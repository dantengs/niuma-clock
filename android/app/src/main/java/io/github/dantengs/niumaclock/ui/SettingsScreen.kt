package io.github.dantengs.niumaclock.ui

import android.content.Intent
import android.net.Uri
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.dantengs.niumaclock.Alarms
import io.github.dantengs.niumaclock.BackgroundGuide
import io.github.dantengs.niumaclock.BuildConfig
import io.github.dantengs.niumaclock.Repo
import io.github.dantengs.niumaclock.core.AlertStyle
import io.github.dantengs.niumaclock.core.ColorOverrides
import io.github.dantengs.niumaclock.core.Config
import io.github.dantengs.niumaclock.core.Salary
import io.github.dantengs.niumaclock.core.ThemeMode
import io.github.dantengs.niumaclock.core.TickUnit

/** 改设置：改完标记 cfgDirty，由 Repo 落盘并重排提醒。 */
private fun Ui.set(block: (Config) -> Unit) = update { block(it.cfg); it.cfgDirty = true }

@Composable
fun SettingsScreen(ui: Ui) {
    val p = LocalPalette.current
    val s = ui.cfg.salary

    BackgroundCard()
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("赚钱表", "数据只存在本机")
        NumberField("月薪", s.monthly, prefix = s.currency) { v -> ui.set { it.salary.monthly = v } }
        NumberField("月计薪天数（国家标准 21.75）", s.paidDays, range = 1.0..31.0) { v -> ui.set { it.salary.paidDays = v } }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("上班", color = p.ink); TimeButton(s.workStart) { t -> ui.set { it.salary.workStart = t } }
            Spacer(Modifier.width(12.dp))
            Text("下班", color = p.ink); TimeButton(s.workEnd) { t -> ui.set { it.salary.workEnd = t } }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("午休", color = p.ink); TimeButton(s.lunchStart) { t -> ui.set { it.salary.lunchStart = t } }
            Text("到", color = p.ink); TimeButton(s.lunchEnd) { t -> ui.set { it.salary.lunchEnd = t } }
        }
        SwitchRow("午休不计薪", s.lunchUnpaid) { v -> ui.set { it.salary.lunchUnpaid = v } }
        SwitchRow("午休模式：午休时不弹提醒，循环倒计时暂停", ui.cfg.lunchQuiet, "上班时到点自动进入，吃药提醒等午休结束再响。别的时间吃饭可以在“今日”页手动开始。") { v ->
            ui.set { it.lunchQuiet = v }
        }
        Text("跳字频率", color = p.ink)
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            for (u in TickUnit.entries) Chip(u.label, s.tickUnit == u) { ui.set { it.salary.tickUnit = u } }
        }
        NumberField("加班倍率（0 = 白嫖模式，表停只记时长）", s.overtimeMultiplier, range = 0.0..10.0) { v -> ui.set { it.salary.overtimeMultiplier = v } }
        NumberField("发薪日（每月几号）", s.payday.toDouble(), range = 1.0..31.0, integer = true) { v -> ui.set { it.salary.payday = v.toInt() } }
        val daily = Salary.standardDailySecs(s)
        Muted(
            if (daily <= 0) "上下班时间扣除午休后没有计薪时长，请检查设置。"
            else "标准工时每天 ${"%.1f".format(daily / 3600)} 小时，时薪约 ${ui.money(Salary.ratePerSec(s) * 3600)}",
            12.sp, if (daily <= 0) p.danger else p.muted,
        )
    }
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("提醒")
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            for (style in listOf(AlertStyle.Card, AlertStyle.Toast)) {
                val selected = (ui.cfg.alertStyle == AlertStyle.Toast) == (style == AlertStyle.Toast)
                Chip(style.label, selected) { ui.set { it.alertStyle = style } }
            }
        }
        Muted("“等我确认”：通知带“搞定了 / 稍后 / 跳过”按钮，确认后才进入下一轮，划掉通知算“稍后”。", 12.sp)
        NumberField("稍后提醒（分钟）", ui.cfg.snoozeMinutes.toDouble(), range = 1.0..120.0, integer = true) { v -> ui.set { it.snoozeMinutes = v.toInt() } }
        SwitchRow("提示音", ui.cfg.sound) { v -> ui.set { it.sound = v } }
        SwitchRow("到下班时间提醒我打卡", ui.cfg.workEndReminder) { v -> ui.set { it.workEndReminder = v } }
    }
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("下班特效")
        SwitchRow("下班打卡时播放全屏特效", ui.cfg.effectEnabled) { v -> ui.set { it.effectEnabled = v } }
        Text("时长 ${"%.0f".format(ui.cfg.effectSeconds)} 秒", color = p.ink)
        Slider(ui.cfg.effectSeconds, { v -> ui.set { it.effectSeconds = Math.round(v).toFloat() } }, valueRange = 3f..15f, steps = 11)
    }
    Spacer(Modifier.height(8.dp))

    AppearanceCard(ui)
    Spacer(Modifier.height(8.dp))

    DataCard(ui)
    Spacer(Modifier.height(8.dp))

    NCard {
        SectionTitle("关于")
        Muted("牛马上下班 安卓版 ${BuildConfig.VERSION_NAME} · MIT 许可证 · 完全离线")
        val context = LocalContext.current
        Text(
            "github.com/dantengs/niuma-clock", color = p.accent, fontSize = 13.sp,
            modifier = Modifier.clickable { context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/dantengs/niuma-clock"))) },
        )
    }
}

@Composable
private fun SwitchRow(label: String, value: Boolean, hint: String = "", onChange: (Boolean) -> Unit) {
    val p = LocalPalette.current
    Row(verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(label, color = p.ink, fontSize = 14.sp)
            if (hint.isNotEmpty()) Muted(hint, 12.sp)
        }
        Switch(value, onChange)
    }
}

/** 数字输入：输入合法才提交，失焦不丢内容。 */
@Composable
private fun NumberField(
    label: String, value: Double, prefix: String = "", range: ClosedFloatingPointRange<Double> = 0.0..100_000_000.0,
    integer: Boolean = false, onCommit: (Double) -> Unit,
) {
    fun show(v: Double) = if (integer || v == Math.floor(v)) v.toLong().toString() else v.toString()
    var text by remember(value) { mutableStateOf(show(value)) }
    val parsed = text.toDoubleOrNull()?.takeIf { it in range && (!integer || it == Math.floor(it)) }
    OutlinedTextField(
        value = text,
        onValueChange = {
            text = it
            it.toDoubleOrNull()?.takeIf { v -> v in range && (!integer || v == Math.floor(v)) }?.let(onCommit)
        },
        label = { Text(label) }, singleLine = true, isError = parsed == null,
        prefix = if (prefix.isNotEmpty()) ({ Text(prefix) }) else null,
        keyboardOptions = KeyboardOptions(keyboardType = if (integer) KeyboardType.Number else KeyboardType.Decimal),
        modifier = Modifier,
    )
}

/** 后台运行设置：vivo / OriginOS 上提醒能否准时响的关键。 */
@Composable
fun BackgroundCard() {
    val p = LocalPalette.current
    val context = LocalContext.current
    // 版本号变化（回到应用时会 tick）就重新检查一次状态。
    @Suppress("UNUSED_VARIABLE") val version by Repo.version.collectAsState()
    val notify = BackgroundGuide.notificationsAllowed(context)
    val battery = BackgroundGuide.ignoringBatteryOptimizations(context)
    val exact = Alarms.canExact(context)
    NCard {
        SectionTitle("让提醒准时响")
        Muted("手机系统会限制后台应用，不放行的话锁屏后提醒可能不响或延迟。下面几项都打开最稳。", 12.sp)
        StatusRow("允许通知", notify, "去打开") { BackgroundGuide.openNotificationSettings(context) }
        StatusRow("精确闹钟", exact, "去打开") { BackgroundGuide.openAppDetails(context) }
        StatusRow("忽略电池优化", battery, "去设置") { BackgroundGuide.requestIgnoreBatteryOptimizations(context) }
        if (BackgroundGuide.isVivo()) {
            HorizontalLine()
            Text("vivo / OriginOS 还需要两项（系统不提供状态查询，请手动确认）：", fontSize = 13.sp, color = p.ink)
            ManualRow("自启动：允许牛马上下班", "去设置") { BackgroundGuide.openAutoStart(context) }
            ManualRow("后台耗电管理：选“允许后台高耗电”", "去设置") { BackgroundGuide.openBatterySettings(context) }
            Muted("找不到的话：设置 → 电池 → 后台耗电管理 → 牛马上下班；设置 → 应用与权限 → 权限管理 → 自启动。另外在最近任务里把本应用下拉“锁定”。", 12.sp)
        }
    }
}

@Composable
private fun HorizontalLine() = androidx.compose.material3.HorizontalDivider(color = LocalPalette.current.line)

@Composable
private fun StatusRow(label: String, ok: Boolean, action: String, onClick: () -> Unit) {
    val p = LocalPalette.current
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(if (ok) "✅" else "⚠️", fontSize = 14.sp)
        Spacer(Modifier.width(8.dp))
        Text(label, color = p.ink, fontSize = 14.sp, modifier = Modifier.weight(1f))
        if (!ok) TextButton(onClick = onClick) { Text(action) } else Muted("已开启", 12.sp)
    }
}

@Composable
private fun ManualRow(label: String, action: String, onClick: () -> Unit) {
    val p = LocalPalette.current
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text("👉", fontSize = 14.sp)
        Spacer(Modifier.width(8.dp))
        Text(label, color = p.ink, fontSize = 14.sp, modifier = Modifier.weight(1f))
        TextButton(onClick = onClick) { Text(action) }
    }
}

/** 第一次打开时提示去做后台设置，只提示一次。 */
@Composable
fun FirstRunGuide(ui: Ui) {
    if (ui.cfg.backgroundGuideShown) return
    AlertDialog(
        onDismissRequest = {},
        title = { Text("先做一下后台设置") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("手机锁屏后，系统会限制后台应用。想让喝水、吃药提醒准时响，请在“设置 → 让提醒准时响”里把几项都打开。", fontSize = 14.sp)
                if (BackgroundGuide.isVivo()) Text("vivo / OriginOS 需要额外允许“自启动”和“后台高耗电”。", fontSize = 14.sp)
            }
        },
        confirmButton = { TextButton(onClick = { ui.set { it.backgroundGuideShown = true } }) { Text("知道了") } },
    )
}

@Composable
private fun AppearanceCard(ui: Ui) {
    val p = LocalPalette.current
    val dark = p.dark
    val overrides = if (dark) ui.cfg.colorsDark else ui.cfg.colorsLight
    fun setOverrides(o: ColorOverrides) = ui.set { if (dark) it.colorsDark = o else it.colorsLight = o }
    var slotEdit by remember { mutableStateOf<String?>(null) }

    NCard {
        SectionTitle("外观", if (dark) "正在改深色主题的配色" else "正在改浅色主题的配色")
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            for (t in ThemeMode.entries) Chip(t.label, ui.cfg.theme == t) { ui.set { it.theme = t } }
        }
        Text("一键配色", color = p.ink)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            for ((name, light, darkSet) in Palette.PRESETS) {
                val target = if (dark) darkSet else light
                Chip(name, overrides == target) { ui.set { it.colorsLight = light.copy(); it.colorsDark = darkSet.copy() } }
            }
        }
        Text("单独改颜色", color = p.ink)
        val slots = listOf(
            "accent" to ("主色" to p.accent), "gold" to ("金额色" to p.gold), "hero" to ("赚钱卡片底色" to p.hero),
            "bg" to ("背景" to p.bg), "card" to ("卡片底色" to p.card),
        )
        FlowRow(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((key, pair) in slots) {
                val (label, color) = pair
                Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.clickable { slotEdit = key }) {
                    Box(Modifier.size(34.dp).clip(CircleShape).background(color).border(1.dp, p.line, CircleShape))
                    Muted(label, 11.sp)
                }
            }
        }
        if (p.bgCardClash()) Muted("背景和卡片一深一浅，部分文字可能看不清。", 12.sp, p.danger)
        TextButton(onClick = { setOverrides(ColorOverrides()) }) { Text("恢复默认配色") }
    }

    slotEdit?.let { key ->
        val current = when (key) { "accent" -> p.accent; "gold" -> p.gold; "hero" -> p.hero; "bg" -> p.bg; else -> p.card }
        ColorDialog(current, onDismiss = { slotEdit = null }) { rgb ->
            val o = overrides.copy()
            when (key) {
                "accent" -> o.accent = rgb
                "gold" -> o.gold = rgb
                "hero" -> o.hero = rgb
                "bg" -> o.bg = rgb
                else -> o.card = rgb
            }
            setOverrides(o)
            slotEdit = null
        }
    }
}

private val SWATCHES = listOf(
    0xC95420, 0xE87535, 0xF4BC3A, 0x2F7D55, 0x2F8F6B, 0x2F6FC9, 0x5B9BF0, 0x7A4FC2, 0xC84B7A, 0xB9382E,
    0x24201B, 0x162236, 0x162B24, 0x351A26, 0x444B55, 0xF5EFE3, 0xFFFCF6, 0xEBF1F8, 0x181512, 0x25211C,
)

@Composable
private fun ColorDialog(current: Color, onDismiss: () -> Unit, onPick: (List<Int>?) -> Unit) {
    var hex by remember { mutableStateOf(Palette.hex(current)) }
    val parsed = Palette.parseHex(hex)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("选颜色") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    for (c in SWATCHES) {
                        val color = Color(0xFF000000 or c.toLong())
                        Box(Modifier.size(32.dp).clip(CircleShape).background(color).border(1.dp, Color.Gray, CircleShape).clickable { hex = Palette.hex(color) })
                    }
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(Modifier.size(32.dp).clip(CircleShape).background(parsed?.let { Palette.rgb(it) } ?: Color.Transparent))
                    Spacer(Modifier.width(8.dp))
                    OutlinedTextField(hex, { hex = it }, label = { Text("十六进制，如 #C95420") }, singleLine = true, isError = parsed == null)
                }
            }
        },
        confirmButton = { TextButton(enabled = parsed != null, onClick = { onPick(parsed) }) { Text("用这个") } },
        dismissButton = {
            Row {
                TextButton(onClick = { onPick(null) }) { Text("用默认") }
                TextButton(onClick = onDismiss) { Text("取消") }
            }
        },
    )
}

@Composable
private fun DataCard(ui: Ui) {
    val context = LocalContext.current
    var confirm by remember { mutableStateOf<List<Pair<String, String>>?>(null) }
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        val files = uris.mapNotNull { uri ->
            runCatching { context.contentResolver.openInputStream(uri)?.use { it.readBytes().toString(Charsets.UTF_8).removePrefix(Char(0xFEFF).toString()) } }.getOrNull()
                ?.let { (uri.lastPathSegment ?: "文件") to it }
        }
        if (files.isNotEmpty()) confirm = files
    }
    NCard {
        SectionTitle("数据")
        Muted("数据只保存在这台手机上，卸载应用会一起删除。打卡记录和记账可以在对应页面导出 CSV。", 12.sp)
        OutlinedButton(onClick = { pick.launch(arrayOf("application/json", "text/plain", "*/*")) }) { Text("导入电脑版数据") }
        Muted("把电脑上 %APPDATA%\\NiuMaClock 里的 config.json 和 data.json 传到手机，在这里一起选中。导入前会先备份手机上的数据。", 12.sp)
    }
    confirm?.let { files ->
        val cfgText = files.firstOrNull { it.second.contains("\"salary\"") }?.second
        val dataText = files.firstOrNull { it.second.contains("\"sessions\"") }?.second
        AlertDialog(
            onDismissRequest = { confirm = null },
            title = { Text("用电脑版数据替换？") },
            text = {
                Text(
                    listOfNotNull(cfgText?.let { "· 设置（config.json）" }, dataText?.let { "· 打卡记录、记账（data.json）" })
                        .ifEmpty { listOf("选中的文件不像电脑版的 config.json 或 data.json。") }
                        .joinToString("\n") + "\n\n手机上对应的数据会被替换，原数据先备份。",
                )
            },
            confirmButton = {
                TextButton(enabled = cfgText != null || dataText != null, onClick = {
                    val msg = Repo.importDesktop(context, cfgText, dataText)
                    Toast.makeText(context, msg, Toast.LENGTH_LONG).show()
                    confirm = null
                }) { Text("导入") }
            },
            dismissButton = { TextButton(onClick = { confirm = null }) { Text("取消") } },
        )
    }
}
