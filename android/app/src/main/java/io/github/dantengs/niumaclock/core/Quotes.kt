package io.github.dantengs.niumaclock.core

import java.time.LocalDate

/** 牛马语录。按日期和心情挑一句，同一天内不跳来跳去。与电脑版 quotes.rs 相同。 */
object Quotes {
    private val COMMON = listOf(
        "工资是老板的，身体是自己的。",
        "再忙也要喝水，膀胱不会替你写周报。",
        "今天的努力，都是老板明年的新车。",
        "能坐着绝不站着，能摸鱼绝不内卷。",
        "颈椎：你开心就好，我先走一步。",
        "别的牛马都在吃草，你也歇一口。",
        "需求会变，腰椎不会自己长回来。",
        "人生不是打卡，但打卡能换饭钱。",
        "站起来走两步，也算今日运动达标。",
        "工位是租的，肝是原装的。",
    )
    private val HAPPY = listOf(
        "心情不错，但别让老板看出来，他会加活。",
        "快乐的牛马，草都吃得更香。",
        "保持住，这份好心情比绩效值钱。",
    )
    private val SLACKING = listOf(
        "摸鱼不是偷懒，是在给下午蓄力。",
        "带薪上厕所，是打工人最后的浪漫。",
        "鱼摸得好，下班才能早。",
    )
    private val TIRED = listOf(
        "累了就闭眼三分钟，世界不会塌。",
        "牛马也要充电，不是只有手机才需要。",
        "今天先这样，明天的活留给明天的你。",
    )
    private val ANNOYED = listOf(
        "深呼吸。这个会议迟早会结束的。",
        "别生气，气坏身子没有工伤认定。",
        "烦的时候喝口水，至少膀胱能带你离开工位。",
    )
    private val BROKEN = listOf(
        "撑不住就先停一下，真的没有什么比你重要。",
        "崩溃不丢人，去走走，找个信任的人聊聊。",
        "今天能到这里已经很好了，别再为难自己。",
    )
    private val OFF_WORK = listOf(
        "下班了！现在开始，你的时间只属于你。",
        "牛马收工，回圈吃草。",
        "今天的班到此为止，手机静音，消息免打扰。",
        "溜了溜了，明天的事明天再说。",
        "这一天辛苦了，好好吃饭，早点睡。",
    )

    /** 与 chrono 的 num_days_from_ce 一致（公元 1 年 1 月 1 日为第 1 天），两端同一天挑到同一句。 */
    private fun daysFromCe(date: LocalDate): Long = date.toEpochDay() + 719163

    private fun pick(list: List<String>, date: LocalDate) = list[(daysFromCe(date) % list.size).toInt()]

    fun daily(date: LocalDate, mood: Mood): String = when (mood) {
        Mood.Happy -> pick(HAPPY, date)
        Mood.Calm -> pick(COMMON, date)
        Mood.Slacking -> pick(SLACKING, date)
        Mood.Tired -> pick(TIRED, date)
        Mood.Annoyed -> pick(ANNOYED, date)
        Mood.Broken -> pick(BROKEN, date)
    }

    fun offWork(date: LocalDate): String = pick(OFF_WORK, date)
}
