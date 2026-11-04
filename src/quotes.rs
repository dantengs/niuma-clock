//! 牛马语录。按日期和心情挑一句，同一天内不跳来跳去。
use chrono::{Datelike, NaiveDate};

use crate::model::Mood;

const COMMON: &[&str] = &[
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
];

const HAPPY: &[&str] = &[
    "心情不错，但别让老板看出来，他会加活。",
    "快乐的牛马，草都吃得更香。",
    "保持住，这份好心情比绩效值钱。",
];

const SLACKING: &[&str] = &[
    "摸鱼不是偷懒，是在给下午蓄力。",
    "带薪上厕所，是打工人最后的浪漫。",
    "鱼摸得好，下班才能早。",
];

const TIRED: &[&str] = &[
    "累了就闭眼三分钟，世界不会塌。",
    "牛马也要充电，不是只有手机才需要。",
    "今天先这样，明天的活留给明天的你。",
];

const ANNOYED: &[&str] = &[
    "深呼吸。这个会议迟早会结束的。",
    "别生气，气坏身子没有工伤认定。",
    "烦的时候喝口水，至少膀胱能带你离开工位。",
];

const BROKEN: &[&str] = &[
    "撑不住就先停一下，真的没有什么比你重要。",
    "崩溃不丢人，去走走，找个信任的人聊聊。",
    "今天能到这里已经很好了，别再为难自己。",
];

const OFF_WORK: &[&str] = &[
    "下班了！现在开始，你的时间只属于你。",
    "牛马收工，回圈吃草。",
    "今天的班到此为止，电脑合上，消息免打扰。",
    "溜了溜了，明天的事明天再说。",
    "这一天辛苦了，好好吃饭，早点睡。",
];

fn pick(list: &[&'static str], date: NaiveDate) -> &'static str {
    list[date.num_days_from_ce() as usize % list.len()]
}

pub fn daily(date: NaiveDate, mood: Mood) -> &'static str {
    match mood {
        Mood::Happy => pick(HAPPY, date),
        Mood::Calm => pick(COMMON, date),
        Mood::Slacking => pick(SLACKING, date),
        Mood::Tired => pick(TIRED, date),
        Mood::Annoyed => pick(ANNOYED, date),
        Mood::Broken => pick(BROKEN, date),
    }
}

pub fn off_work(date: NaiveDate) -> &'static str {
    pick(OFF_WORK, date)
}
