package io.github.dantengs.niumaclock.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.dantengs.niumaclock.Fmt
import io.github.dantengs.niumaclock.Repo
import io.github.dantengs.niumaclock.core.DaySummary
import io.github.dantengs.niumaclock.core.Engine
import io.github.dantengs.niumaclock.core.ThemeMode
import kotlinx.coroutines.delay
import java.time.LocalDateTime

enum class Tab(val label: String, val emoji: String) {
    Today("今日", "💰"), Loops("循环", "🔁"), Health("身心", "❤️"), Ledger("记账", "🧾"), Records("记录", "📅"),
}

/** 页面共用的上下文：当前时刻、引擎和修改入口。 */
class Ui(val e: Engine, val now: LocalDateTime, val update: ((Engine) -> Unit) -> Unit) {
    val cfg get() = e.cfg
    fun money(v: Double) = Fmt.money(e.cfg, v)
}

@Composable
fun NiumaApp() {
    val context = LocalContext.current
    val version by Repo.version.collectAsState()
    var now by remember { mutableStateOf(LocalDateTime.now()) }
    LaunchedEffect(Unit) {
        while (true) {
            now = LocalDateTime.now()
            delay(1000L - System.currentTimeMillis() % 1000L)
        }
    }
    val e = remember(version) { Repo.engine(context) }
    val ui = Ui(e, now) { block -> Repo.update(context, block) }

    val systemDark = isSystemInDarkTheme()
    val dark = when (e.cfg.theme) {
        ThemeMode.System -> systemDark
        ThemeMode.Light -> false
        ThemeMode.Dark -> true
    }
    val palette = (if (dark) Palette.DARK else Palette.LIGHT).withOverrides(if (dark) e.cfg.colorsDark else e.cfg.colorsLight)

    var tab by rememberSaveable { mutableStateOf(Tab.Today) }
    var settings by rememberSaveable { mutableStateOf(false) }
    var effect by remember { mutableStateOf<DaySummary?>(null) }
    BackHandler(enabled = settings) { settings = false }

    NiumaTheme(palette) {
        val p = LocalPalette.current
        Box(Modifier.fillMaxSize().background(p.bg)) {
            Scaffold(
                containerColor = p.bg,
                bottomBar = {
                    if (!settings) {
                        NavigationBar(containerColor = p.card) {
                            for (t in Tab.entries) {
                                val badge = if (t == Tab.Today && e.alerts().isNotEmpty()) " ·${e.alerts().size}" else ""
                                NavigationBarItem(
                                    selected = tab == t,
                                    onClick = { tab = t },
                                    icon = { Text(t.emoji, fontSize = 18.sp) },
                                    label = { Text(t.label + badge, fontSize = 12.sp) },
                                    colors = NavigationBarItemDefaults.colors(indicatorColor = p.sunken, selectedTextColor = p.accent),
                                )
                            }
                        }
                    }
                },
            ) { pad ->
                Column(Modifier.fillMaxSize().padding(bottom = pad.calculateBottomPadding()).statusBarsPadding()) {
                    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                        if (settings) {
                            TextButton(onClick = { settings = false }) { Text("‹ 返回") }
                            Text("设置", fontSize = 20.sp, fontWeight = FontWeight.Bold, color = p.ink)
                        } else {
                            Text(if (tab == Tab.Today) "牛马上下班" else tab.label, fontSize = 20.sp, fontWeight = FontWeight.Bold, color = p.ink)
                            Spacer(Modifier.weight(1f))
                            TextButton(onClick = { settings = true }) { Text("⚙ 设置") }
                        }
                    }
                    Column(
                        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(PaddingValues(start = 14.dp, end = 14.dp, bottom = 20.dp)),
                    ) {
                        if (settings) {
                            SettingsScreen(ui)
                        } else {
                            when (tab) {
                                Tab.Today -> TodayScreen(ui, onClockOut = { if (e.cfg.effectEnabled) effect = it }, openSettings = { settings = true }, openHealth = { tab = Tab.Health })
                                Tab.Loops -> LoopsScreen(ui)
                                Tab.Health -> HealthScreen(ui)
                                Tab.Ledger -> LedgerScreen(ui)
                                Tab.Records -> RecordsScreen(ui)
                            }
                        }
                        Spacer(Modifier.height(12.dp))
                    }
                }
            }
            effect?.let { summary -> OffWorkEffect(ui, summary, onClose = { effect = null }) }
            ForgottenDialog(ui)
            FirstRunGuide(ui)
        }
    }
}

/** 跨天没打下班卡时询问。 */
@Composable
private fun ForgottenDialog(ui: Ui) {
    val start = ui.e.forgotten(ui.now) ?: return
    val end = io.github.dantengs.niumaclock.core.Salary.workEndAt(start.toLocalDate(), ui.cfg.salary)
    val context = LocalContext.current
    AlertDialog(
        onDismissRequest = {},
        title = { Text("昨天忘打下班卡了？") },
        text = { Text("${start.format(Fmt.MD_HM)} 上班后一直没下班。按下班时间 ${end.format(Fmt.MD_HM)} 补卡吗？") },
        confirmButton = { TextButton(onClick = { ui.update { it.clockOut(maxOf(end, start)) } }) { Text("按下班时间补卡") } },
        dismissButton = {
            Row {
                TextButton(onClick = { pickDateTime(context, end) { t -> ui.update { it.clockOut(maxOf(t, start)) } } }) { Text("自定义时间") }
                TextButton(onClick = { ui.update { it.ackForgotten(start) } }) { Text("真的还在上班") }
            }
        },
    )
}
