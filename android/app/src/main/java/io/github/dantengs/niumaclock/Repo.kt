package io.github.dantengs.niumaclock

import android.content.Context
import io.github.dantengs.niumaclock.core.Config
import io.github.dantengs.niumaclock.core.Data
import io.github.dantengs.niumaclock.core.Engine
import io.github.dantengs.niumaclock.core.Store
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.io.File
import java.time.LocalDateTime

/**
 * 进程内唯一的引擎实例。界面、闹钟、通知按钮、小部件都通过 update() 改它：
 * 改完自动落盘、处理到点的提醒、重排闹钟、刷新小部件，并通知界面重绘。
 */
object Repo {
    private var engine: Engine? = null
    private lateinit var appContext: Context
    private val _version = MutableStateFlow(0)

    /** 每次数据变化 +1，界面据此重组。 */
    val version: StateFlow<Int> = _version

    fun now(): LocalDateTime = LocalDateTime.now()

    private fun dir(context: Context) = File(context.filesDir, "NiuMaClock")
    private fun configFile(context: Context) = File(dir(context), "config.json")
    private fun dataFile(context: Context) = File(dir(context), "data.json")

    @Synchronized
    fun engine(context: Context): Engine {
        engine?.let { return it }
        appContext = context.applicationContext
        val cfg = Store.load(configFile(appContext), Store::decodeConfig) ?: Config()
        val data = Store.load(dataFile(appContext), Store::decodeData) ?: Data()
        return Engine(cfg, data).also {
            engine = it
            it.cfgDirty = !configFile(appContext).exists()
            it.dataDirty = !dataFile(appContext).exists()
        }
    }

    /** 只读访问。 */
    fun <T> read(context: Context, block: (Engine) -> T): T = synchronized(this) { block(engine(context)) }

    /** 修改引擎。改完会 tick 一次，把到点的提醒发出去，再重排闹钟。 */
    fun <T> update(context: Context, block: (Engine) -> T): T {
        val (result, notices) = synchronized(this) {
            val e = engine(context)
            val r = block(e)
            val n = e.tick(now())
            persist(e)
            r to n
        }
        Notifier.show(context, notices)
        afterChange(context)
        return result
    }

    /** 闹钟到点、开机、改时间时调用。 */
    fun tick(context: Context) = update(context) { }

    private fun persist(e: Engine) {
        if (e.cfgDirty) {
            Store.save(configFile(appContext), Store.encode(e.cfg))
            e.cfgDirty = false
        }
        if (e.dataDirty) {
            Store.save(dataFile(appContext), Store.encode(e.data))
            e.dataDirty = false
        }
    }

    private fun afterChange(context: Context) {
        val (wake, onDuty, alerts) = read(context) { e ->
            Triple(e.nextWake(now()), e.onDuty(), e.alerts().map { it.kind.key }.toSet())
        }
        Alarms.schedule(context, wake)
        Alarms.scheduleWidgetRefresh(context, onDuty && NiumaWidget.hasWidgets(context))
        Notifier.cancelStale(context, alerts)
        NiumaWidget.updateAll(context)
        _version.value += 1
    }

    /** 用电脑版导出的 config.json / data.json 替换手机上的数据。先把手机原有数据备份一份。 */
    fun importDesktop(context: Context, configText: String?, dataText: String?): String {
        val cfg = configText?.let { runCatching { Store.decodeConfig(it) }.getOrElse { e -> return "config.json 读不了：${e.message}" } }
        val data = dataText?.let { runCatching { Store.decodeData(it) }.getOrElse { e -> return "data.json 读不了：${e.message}" } }
        if (cfg == null && data == null) return "没有选文件"
        synchronized(this) {
            val e = engine(context)
            val stamp = System.currentTimeMillis()
            Store.save(File(dir(context), "config.before-import-$stamp.json"), Store.encode(e.cfg))
            Store.save(File(dir(context), "data.before-import-$stamp.json"), Store.encode(e.data))
            val fresh = Engine(cfg ?: e.cfg, data ?: e.data)
            fresh.cfgDirty = true
            fresh.dataDirty = true
            engine = fresh
            persist(fresh)
        }
        Notifier.cancelAll(context)
        tick(context)
        val what = listOfNotNull(cfg?.let { "设置" }, data?.let { "打卡记录" }).joinToString("和")
        return "已导入$what。原来手机上的数据已备份。"
    }
}
