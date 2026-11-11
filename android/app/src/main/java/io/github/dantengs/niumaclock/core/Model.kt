@file:UseSerializers(LocalDateTimeSerializer::class, LocalTimeSerializer::class, LocalDateSerializer::class)

package io.github.dantengs.niumaclock.core

import kotlinx.serialization.KSerializer

import kotlinx.serialization.Serializable
import kotlinx.serialization.UseSerializers
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonDecoder
import kotlinx.serialization.json.JsonEncoder
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.format.DateTimeFormatter

/*
 * 持久化的数据结构。字段名和 JSON 格式与电脑版（Rust）一致，电脑版的 config.json / data.json 可以直接导入。
 * 所有字段都有默认值，旧文件缺字段也能读。
 */

fun hm(h: Int, m: Int): LocalTime = LocalTime.of(h, m)

object LocalDateTimeSerializer : KSerializer<LocalDateTime> {
    override val descriptor = PrimitiveSerialDescriptor("LocalDateTime", PrimitiveKind.STRING)
    override fun serialize(encoder: Encoder, value: LocalDateTime) =
        encoder.encodeString(value.format(DateTimeFormatter.ISO_LOCAL_DATE_TIME))
    override fun deserialize(decoder: Decoder): LocalDateTime = LocalDateTime.parse(decoder.decodeString())
}

object LocalTimeSerializer : KSerializer<LocalTime> {
    override val descriptor = PrimitiveSerialDescriptor("LocalTime", PrimitiveKind.STRING)
    override fun serialize(encoder: Encoder, value: LocalTime) =
        encoder.encodeString(value.format(DateTimeFormatter.ISO_LOCAL_TIME))
    override fun deserialize(decoder: Decoder): LocalTime = LocalTime.parse(decoder.decodeString())
}

object LocalDateSerializer : KSerializer<LocalDate> {
    override val descriptor = PrimitiveSerialDescriptor("LocalDate", PrimitiveKind.STRING)
    override fun serialize(encoder: Encoder, value: LocalDate) = encoder.encodeString(value.toString())
    override fun deserialize(decoder: Decoder): LocalDate = LocalDate.parse(decoder.decodeString())
}

@Serializable
enum class TickUnit(val secs: Double, val label: String) {
    Second(1.0, "每秒"),
    Minute(60.0, "每分钟"),
    Hour(3600.0, "每小时"),
}

@Serializable
data class SalaryConfig(
    var monthly: Double = 0.0,
    /** 月计薪天数，国家标准 21.75。 */
    var paidDays: Double = 21.75,
    var workStart: LocalTime = hm(9, 0),
    var workEnd: LocalTime = hm(18, 0),
    /** 午休不计薪。关闭后午休也算钱。 */
    var lunchUnpaid: Boolean = true,
    var lunchStart: LocalTime = hm(12, 0),
    var lunchEnd: LocalTime = hm(13, 0),
    var tickUnit: TickUnit = TickUnit.Second,
    /** 超过标准工时后的倍率。0 表示白嫖模式：表停，只记白干的时长。 */
    var overtimeMultiplier: Double = 1.0,
    var payday: Int = 15,
    var currency: String = "¥",
)

@Serializable
data class ReminderTemplate(
    val id: Long,
    var title: String,
    var note: String,
    var intervalMin: Int,
    /** 上班打卡时自动开始。 */
    var autoStart: Boolean,
)

/** 与电脑版的 Rust 枚举同形：{"Interval":{"minutes":240}} 或 {"Times":{"times":["08:00:00"]}}。 */
@Serializable(with = MedScheduleSerializer::class)
data class MedSchedule(
    val interval: IntervalBody? = null,
    val times: TimesBody? = null,
) {
    data class IntervalBody(val minutes: Int)

    data class TimesBody(val times: List<LocalTime>)

    companion object {
        fun every(minutes: Int) = MedSchedule(interval = IntervalBody(minutes))
        fun at(times: List<LocalTime>) = MedSchedule(times = TimesBody(times))
    }
}

@Serializable
data class MedPlan(
    val id: Long,
    var name: String,
    var dose: String,
    var note: String,
    var schedule: MedSchedule,
)

@Serializable
enum class AlertStyle(val label: String) {
    /** 通知等你确认，确认前不进入下一轮。 */
    Card("等我确认"),
    /** 发完通知自动进入下一轮。 */
    Toast("只通知，自动下一轮"),
    Both("等我确认"),
}

@Serializable
enum class ThemeMode(val label: String) { System("跟随系统"), Light("浅色"), Dark("深色") }

@Serializable
data class ColorOverrides(
    var accent: List<Int>? = null,
    var gold: List<Int>? = null,
    var hero: List<Int>? = null,
    var bg: List<Int>? = null,
    var card: List<Int>? = null,
)

val EXPENSE_CATEGORIES = listOf("餐饮", "交通", "购物", "娱乐", "居住", "医疗", "人情", "其他")

@Serializable
data class Expense(
    val id: Long,
    val at: LocalDateTime,
    val amount: Double,
    val category: String,
    val note: String = "",
)

fun monthKey(year: Int, month: Int) = "%04d-%02d".format(year, month)

@Serializable
data class Config(
    var salary: SalaryConfig = SalaryConfig(),
    var reminders: MutableList<ReminderTemplate> = mutableListOf(
        ReminderTemplate(1, "喝口水", "牛马也要补水，别等渴了才想起来。", 45, true),
        ReminderTemplate(2, "起来动一动", "站起来走两步，活动一下肩颈和腰。", 60, true),
    ),
    var meds: MutableList<MedPlan> = mutableListOf(),
    var alertStyle: AlertStyle = AlertStyle.Card,
    var sound: Boolean = true,
    var snoozeMinutes: Int = 5,
    var workEndReminder: Boolean = true,
    /** 午休模式：到了午休时间（且在上班）自动进入，期间不弹任何提醒，循环倒计时暂停。 */
    var lunchQuiet: Boolean = true,
    var hideMoney: Boolean = false,
    var effectEnabled: Boolean = true,
    var effectSeconds: Float = 7.0f,
    var theme: ThemeMode = ThemeMode.System,
    var colorsLight: ColorOverrides = ColorOverrides(),
    var colorsDark: ColorOverrides = ColorOverrides(),
    /** 手机版：是否已经看过后台设置引导。 */
    var backgroundGuideShown: Boolean = false,
)

@Serializable
enum class Health(val label: String) { Healthy("健康"), SubHealthy("亚健康"), Unwell("不健康") }

@Serializable
enum class Mood(val label: String) {
    Happy("开心"), Calm("平静"), Slacking("想摸鱼"), Tired("累了"), Annoyed("烦躁"), Broken("崩溃"),
}

@Serializable
data class WorkSession(
    var start: LocalDateTime,
    var end: LocalDateTime? = null,
    /** 以下三项在下班时结算并固化，之后改工资不影响历史。 */
    var paidSecs: Double = 0.0,
    var overtimeSecs: Double = 0.0,
    var income: Double = 0.0,
)

@Serializable
data class DayLog(
    var done: Int = 0,
    var skipped: Int = 0,
    var medsTaken: Int = 0,
    var health: Health? = null,
    var mood: Mood? = null,
)

@Serializable
data class ActiveLoop(
    val id: Long,
    /** null 表示今日临时事项。 */
    val templateId: Long? = null,
    var title: String,
    var note: String,
    var intervalMin: Int,
    var nextFire: LocalDateTime,
    /** 暂停时保存的剩余秒数。 */
    var pausedLeft: Long? = null,
    var pendingSince: LocalDateTime? = null,
    var done: Int = 0,
    var skipped: Int = 0,
)

@Serializable
data class MedRuntime(
    val planId: Long,
    var nextFire: LocalDateTime,
    var pendingSince: LocalDateTime? = null,
)

/** 与 Rust 的 StatusChange 枚举同形：{"Health":"Unwell"} 或 {"Mood":"Tired"}。 */
@Serializable(with = StatusChangeSerializer::class)
data class StatusChange(val health: Health? = null, val mood: Mood? = null)

@Serializable
data class StatusEntry(val at: LocalDateTime, val change: StatusChange)

@Serializable
data class Data(
    var nextId: Long = 0,
    var sessions: MutableList<WorkSession> = mutableListOf(),
    var days: MutableMap<LocalDate, DayLog> = sortedMapOf(),
    var loops: MutableList<ActiveLoop> = mutableListOf(),
    var meds: MutableList<MedRuntime> = mutableListOf(),
    var health: Health = Health.Healthy,
    var mood: Mood = Mood.Calm,
    var statusLog: MutableList<StatusEntry> = mutableListOf(),
    var workEndNotified: LocalDate? = null,
    var workEndPending: Boolean = false,
    var workEndSnoozeUntil: LocalDateTime? = null,
    var sickPromptPending: Boolean = false,
    /** 用户确认“真的还在上班”的那次打卡，不再追问。 */
    var forgotAck: LocalDateTime? = null,
    var lastSeen: LocalDateTime? = null,
    /** 每月已赚的手动调整（键为 "2026-11"）。 */
    var monthAdjust: MutableMap<String, Double> = sortedMapOf(),
    var expenses: MutableList<Expense> = mutableListOf(),
    /** 手动开始的午休，从何时到何时。 */
    var lunchManualUntil: LocalDateTime? = null,
    var lunchManualFrom: LocalDateTime? = null,
    /** 这一天提前结束了自动午休，当天不再自动进入。 */
    var lunchAutoOff: LocalDate? = null,
    /** 手机版：午休开始时被冻结的循环 id，午休结束时恢复。 */
    var lunchFrozen: MutableList<Long> = mutableListOf(),
)

// Rust 外部标签枚举的序列化。命名策略会改写 @SerialName，所以这里手写，键名保持大写开头。

object MedScheduleSerializer : KSerializer<MedSchedule> {
    override val descriptor = JsonObject.serializer().descriptor

    override fun serialize(encoder: Encoder, value: MedSchedule) {
        val out = buildJsonObject {
            val iv = value.interval
            if (iv != null) {
                put("Interval", buildJsonObject { put("minutes", iv.minutes) })
            } else {
                put("Times", buildJsonObject {
                    put("times", JsonArray(value.times?.times.orEmpty().map { JsonPrimitive(it.format(DateTimeFormatter.ISO_LOCAL_TIME)) }))
                })
            }
        }
        (encoder as JsonEncoder).encodeJsonElement(out)
    }

    override fun deserialize(decoder: Decoder): MedSchedule {
        val obj = (decoder as JsonDecoder).decodeJsonElement().jsonObject
        obj["Interval"]?.jsonObject?.let { return MedSchedule.every(it["minutes"]!!.jsonPrimitive.int) }
        val times = obj["Times"]?.jsonObject?.get("times")?.jsonArray.orEmpty().map { LocalTime.parse(it.jsonPrimitive.content) }
        return MedSchedule.at(times)
    }
}

object StatusChangeSerializer : KSerializer<StatusChange> {
    override val descriptor = JsonObject.serializer().descriptor

    override fun serialize(encoder: Encoder, value: StatusChange) {
        val out = buildJsonObject {
            value.health?.let { put("Health", it.name) }
            value.mood?.let { put("Mood", it.name) }
        }
        (encoder as JsonEncoder).encodeJsonElement(out)
    }

    override fun deserialize(decoder: Decoder): StatusChange {
        val obj = (decoder as JsonDecoder).decodeJsonElement().jsonObject
        val health = obj["Health"]?.jsonPrimitive?.content?.let { name -> Health.entries.firstOrNull { it.name == name } }
        val mood = obj["Mood"]?.jsonPrimitive?.content?.let { name -> Mood.entries.firstOrNull { it.name == name } }
        return StatusChange(health, mood)
    }
}
