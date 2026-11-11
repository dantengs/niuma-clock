package io.github.dantengs.niumaclock.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import io.github.dantengs.niumaclock.core.ColorOverrides

/** 与电脑版 widgets.rs 的 Palette 相同的配色和叠加规则。 */
data class Palette(
    val dark: Boolean,
    val bg: Color,
    val card: Color,
    val sunken: Color,
    val ink: Color,
    val muted: Color,
    val line: Color,
    val accent: Color,
    val onAccent: Color,
    val green: Color,
    val danger: Color,
    val dangerSoft: Color,
    val hero: Color,
    val heroInk: Color,
    val heroMuted: Color,
    val gold: Color,
) {
    fun withOverrides(o: ColorOverrides): Palette {
        var p = this
        o.bg?.let { c ->
            p = p.copy(bg = rgb(c))
            if (o.card == null) p = p.copy(card = lerp(p.bg, Color.White, if (luminance(p.bg) > 140f) 0.6f else 0.06f))
        }
        o.card?.let { p = p.copy(card = rgb(it)) }
        if (o.bg != null || o.card != null) {
            val base = if (luminance(p.card) > 140f) LIGHT else DARK
            p = p.copy(
                ink = base.ink, muted = base.muted,
                sunken = lerp(p.card, base.ink, 0.07f),
                line = lerp(p.card, base.ink, 0.13f),
                dangerSoft = lerp(p.card, base.danger, 0.15f),
            )
        }
        o.accent?.let { p = p.copy(accent = rgb(it), onAccent = onColorFor(rgb(it))) }
        o.gold?.let { p = p.copy(gold = rgb(it)) }
        o.hero?.let {
            val ink = onColorFor(rgb(it))
            p = p.copy(hero = rgb(it), heroInk = ink, heroMuted = lerp(rgb(it), ink, 0.6f))
        }
        return p
    }

    /** 窗口背景和卡片底色一深一浅时，直接写在背景上的标题会看不清。 */
    fun bgCardClash() = (luminance(bg) > 140f) != (luminance(card) > 140f)

    companion object {
        val LIGHT = Palette(
            dark = false,
            bg = Color(0xFFF5EFE3), card = Color(0xFFFFFCF6), sunken = Color(0xFFECE4D4),
            ink = Color(0xFF2B2620), muted = Color(0xFF7D7365), line = Color(0xFFE0D6C4),
            accent = Color(0xFFC95420), onAccent = Color(0xFFFFF8F0), green = Color(0xFF2F7D55),
            danger = Color(0xFFB9382E), dangerSoft = Color(0xFFF8E1DB), hero = Color(0xFF24201B),
            heroInk = Color(0xFFF6EFE1), heroMuted = Color(0xFFA89E8E), gold = Color(0xFFF4BC3A),
        )
        val DARK = Palette(
            dark = true,
            bg = Color(0xFF181512), card = Color(0xFF25211C), sunken = Color(0xFF322C25),
            ink = Color(0xFFF2EBDD), muted = Color(0xFFA39989), line = Color(0xFF3B342C),
            accent = Color(0xFFE87535), onAccent = Color(0xFF1E1208), green = Color(0xFF57B583),
            danger = Color(0xFFE86A5C), dangerSoft = Color(0xFF462520), hero = Color(0xFF0F0D0B),
            heroInk = Color(0xFFF6EFE1), heroMuted = Color(0xFFA89E8E), gold = Color(0xFFF4BC3A),
        )

        fun rgb(c: List<Int>) = Color(c.getOrElse(0) { 0 }, c.getOrElse(1) { 0 }, c.getOrElse(2) { 0 })
        fun luminance(c: Color) = 255f * (0.299f * c.red + 0.587f * c.green + 0.114f * c.blue)
        fun onColorFor(c: Color) = if (luminance(c) > 150f) Color(0xFF1E1208) else Color(0xFFFFF8F0)
        fun toRgb(c: Color) = listOf((c.red * 255).toInt(), (c.green * 255).toInt(), (c.blue * 255).toInt())
        fun hex(c: Color) = "#%02X%02X%02X".format((c.red * 255).toInt(), (c.green * 255).toInt(), (c.blue * 255).toInt())
        fun parseHex(text: String): List<Int>? {
            val t = text.trim().removePrefix("#")
            if (t.length != 6) return null
            val v = t.toIntOrNull(16) ?: return null
            return listOf(v shr 16 and 0xFF, v shr 8 and 0xFF, v and 0xFF)
        }

        /** 一键配色：名字、浅色叠加、深色叠加。与电脑版 presets() 相同。 */
        val PRESETS: List<Triple<String, ColorOverrides, ColorOverrides>> = listOf(
            Triple("牛马橙", ColorOverrides(), ColorOverrides()),
            preset("薄荷绿", 0x2F8F6B, 0x162B24, 0xECF4EE, 0xFAFDFB, 0x4CC295, 0x0A1411, 0x121916, 0x1B2521),
            preset("海盐蓝", 0x2F6FC9, 0x162236, 0xEBF1F8, 0xFAFCFF, 0x5B9BF0, 0x090F1A, 0x11161F, 0x1A212D),
            preset("樱花粉", 0xC84B7A, 0x351A26, 0xF8ECF0, 0xFFFAFC, 0xEE7AA6, 0x14090E, 0x1C1317, 0x281C22),
            preset("葡萄紫", 0x7A4FC2, 0x231A36, 0xF1EEF8, 0xFCFBFF, 0xA280EE, 0x0E0A18, 0x16131F, 0x211C2D),
            preset("极简灰", 0x444B55, 0x1F2226, 0xF0F1F2, 0xFFFFFF, 0xC6CBD2, 0x0B0C0E, 0x161719, 0x212326),
        )

        private fun split(v: Int) = listOf(v shr 16 and 0xFF, v shr 8 and 0xFF, v and 0xFF)
        private fun preset(name: String, la: Int, lh: Int, lb: Int, lc: Int, da: Int, dh: Int, db: Int, dc: Int) = Triple(
            name,
            ColorOverrides(accent = split(la), hero = split(lh), bg = split(lb), card = split(lc)),
            ColorOverrides(accent = split(da), hero = split(dh), bg = split(db), card = split(dc)),
        )
    }
}

val LocalPalette = staticCompositionLocalOf { Palette.LIGHT }

@Composable
fun NiumaTheme(palette: Palette, content: @Composable () -> Unit) {
    val scheme = if (palette.dark) {
        darkColorScheme(
            primary = palette.accent, onPrimary = palette.onAccent, background = palette.bg, onBackground = palette.ink,
            surface = palette.card, onSurface = palette.ink, surfaceVariant = palette.sunken, onSurfaceVariant = palette.muted,
            outline = palette.line, error = palette.danger, secondaryContainer = palette.sunken,
        )
    } else {
        lightColorScheme(
            primary = palette.accent, onPrimary = palette.onAccent, background = palette.bg, onBackground = palette.ink,
            surface = palette.card, onSurface = palette.ink, surfaceVariant = palette.sunken, onSurfaceVariant = palette.muted,
            outline = palette.line, error = palette.danger, secondaryContainer = palette.sunken,
        )
    }
    androidx.compose.runtime.CompositionLocalProvider(LocalPalette provides palette) {
        MaterialTheme(colorScheme = scheme, content = content)
    }
}
