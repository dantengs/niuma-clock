package io.github.dantengs.niumaclock.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameMillis
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.dantengs.niumaclock.Fmt
import io.github.dantengs.niumaclock.core.DaySummary
import io.github.dantengs.niumaclock.core.Quotes
import io.github.dantengs.niumaclock.core.hpRank
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.sin
import kotlin.random.Random

private class Coin(val x: Float, val delay: Float, val speed: Float, val size: Float, val spin: Float, val phase: Float, val sway: Float)

/** 下班打卡特效：全屏撒金币，结算卡片上金额滚动。点一下关闭，到时自动关闭。 */
@Composable
fun OffWorkEffect(ui: Ui, summary: DaySummary, onClose: () -> Unit) {
    val p = LocalPalette.current
    val coins = remember { List(70) { Random(it * 31 + 7).let { r -> Coin(r.nextFloat(), r.nextFloat() * 1.6f, 0.35f + r.nextFloat() * 0.45f, 14f + r.nextFloat() * 18f, r.nextFloat() * 4f + 1f, r.nextFloat() * 6.28f, 8f + r.nextFloat() * 24f) } } }
    var elapsed by remember { mutableLongStateOf(0L) }
    val rolling = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        launch { rolling.animateTo(1f, tween(1600, easing = FastOutSlowInEasing)) }
        val start = withFrameMillis { it }
        val total = (ui.cfg.effectSeconds * 1000).toLong()
        while (elapsed < total) {
            elapsed = withFrameMillis { it } - start
        }
        delay(50)
        onClose()
    }

    Box(
        Modifier.fillMaxSize().background(Color.Black.copy(alpha = 0.55f))
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null, onClick = onClose),
        contentAlignment = Alignment.Center,
    ) {
        Canvas(Modifier.fillMaxSize()) {
            val t = elapsed / 1000f
            for (c in coins) {
                val life = t - c.delay
                if (life < 0) continue
                val y = (life * c.speed * size.height) % (size.height + 60f) - 30f
                val x = c.x * size.width + sin(life * 2f + c.phase) * c.sway
                val squash = abs(sin(life * c.spin + c.phase)).coerceAtLeast(0.15f)
                rotate(degrees = (c.phase * 180f / PI.toFloat()), pivot = Offset(x, y)) {
                    drawOval(Color(0xFFF4BC3A), Offset(x - c.size * squash / 2, y - c.size / 2), Size(c.size * squash, c.size))
                    drawOval(Color(0xFFFFE08A), Offset(x - c.size * squash / 4, y - c.size / 4), Size(c.size * squash / 2, c.size / 2))
                }
            }
        }
        Column(
            Modifier.padding(28.dp).fillMaxWidth().clip(RoundedCornerShape(24.dp)).background(Color(0xF21E1A16)).padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text("下班！", fontSize = 28.sp, fontWeight = FontWeight.Bold, color = Color(0xFFF6EFE1))
            Text("今日收入", fontSize = 13.sp, color = Color(0xFFA89E8E))
            Num(ui.money(summary.income * rolling.value), 38.sp, p.gold)
            if (summary.freeLabor) Text("其中白干了 ${Fmt.duration(summary.overtimeSecs)}", fontSize = 13.sp, color = Color(0xFFE86A5C))
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceEvenly) {
                EffectStat("工时", Fmt.duration(summary.workedSecs))
                EffectStat("完成提醒", "${summary.done} 次")
                EffectStat("牛马血条", "${summary.hp} · ${hpRank(summary.hp)}")
            }
            Text(Quotes.offWork(summary.date), fontSize = 14.sp, color = Color(0xFFF6EFE1), textAlign = TextAlign.Center)
            Text("点任意处关闭", fontSize = 11.sp, color = Color(0xFFA89E8E))
        }
    }
}

@Composable
private fun EffectStat(label: String, value: String) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(label, fontSize = 11.sp, color = Color(0xFFA89E8E))
        Text(value, fontSize = 13.sp, color = Color(0xFFF6EFE1), fontWeight = FontWeight.Bold)
    }
}
