package io.github.dantengs.niumaclock

import io.github.dantengs.niumaclock.core.Config
import java.text.DecimalFormat
import java.time.LocalDate
import java.time.format.DateTimeFormatter

/** 金额、时长等显示格式，与电脑版 widgets.rs 一致。 */
object Fmt {
    private val money = DecimalFormat("#,##0.00")
    val HM: DateTimeFormatter = DateTimeFormatter.ofPattern("HH:mm")
    val MD_HM: DateTimeFormatter = DateTimeFormatter.ofPattern("M月d日 HH:mm")

    fun money(cfg: Config, value: Double): String =
        if (cfg.hideMoney) "${cfg.salary.currency}***" else cfg.salary.currency + money.format(value)

    /** 大数字拆成整数部分和小尾巴（秒级跳字时尾巴单独小字显示）。 */
    fun moneyParts(cfg: Config, value: Double): Pair<String, String> {
        if (cfg.hideMoney) return "${cfg.salary.currency}***" to ""
        val text = money.format(value)
        val dot = text.indexOf('.')
        return cfg.salary.currency + text.substring(0, dot) to text.substring(dot)
    }

    fun signedMoney(cfg: Config, value: Double): String =
        if (cfg.hideMoney) "${cfg.salary.currency}***" else (if (value >= 0) "+" else "-") + money(cfg, Math.abs(value))

    /** 01:02:03 形式。 */
    fun clock(secs: Long): String {
        val s = Math.abs(secs)
        return "%02d:%02d:%02d".format(s / 3600, s % 3600 / 60, s % 60)
    }

    /** 3 小时 05 分 形式。 */
    fun duration(secs: Double): String {
        val total = (secs / 60).toLong()
        return if (total >= 60) "${total / 60} 小时 %02d 分".format(total % 60) else "$total 分钟"
    }

    fun hours(secs: Double): String = "%.1f 小时".format(secs / 3600)

    fun dateLabel(date: LocalDate): String {
        val week = "一二三四五六日"[date.dayOfWeek.value - 1]
        return "${date.monthValue}月${date.dayOfMonth}日 周$week"
    }
}
