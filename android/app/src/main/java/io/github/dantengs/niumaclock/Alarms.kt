package io.github.dantengs.niumaclock

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import java.time.LocalDateTime
import java.time.ZoneId

/**
 * 只排一个精确闹钟：引擎给出的下一个唤醒时刻。到点后 AlarmReceiver 调 Repo.tick()，再排下一个。
 * 用 setExactAndAllowWhileIdle，手机休眠（Doze）时也能准点；vivo 等系统还需要用户允许后台运行，见 BackgroundGuide。
 */
object Alarms {
    private const val TICK = 1
    private const val WIDGET = 2
    const val ACTION_TICK = "io.github.dantengs.niumaclock.TICK"
    const val ACTION_WIDGET = "io.github.dantengs.niumaclock.WIDGET_REFRESH"

    private fun intent(context: Context, action: String, code: Int) = PendingIntent.getBroadcast(
        context, code,
        Intent(context, AlarmReceiver::class.java).setAction(action),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )

    private fun millis(at: LocalDateTime) = at.atZone(ZoneId.systemDefault()).toInstant().toEpochMilli()

    fun canExact(context: Context): Boolean {
        val am = context.getSystemService(AlarmManager::class.java)
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.S || am.canScheduleExactAlarms()
    }

    fun schedule(context: Context, at: LocalDateTime?) {
        val am = context.getSystemService(AlarmManager::class.java)
        val pi = intent(context, ACTION_TICK, TICK)
        if (at == null) {
            am.cancel(pi)
            return
        }
        if (canExact(context)) {
            am.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, millis(at), pi)
        } else {
            am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, millis(at), pi)
        }
    }

    /** 上班期间每 10 分钟左右刷新一次桌面小部件上的金额（不精确，省电）。 */
    fun scheduleWidgetRefresh(context: Context, enabled: Boolean) {
        val am = context.getSystemService(AlarmManager::class.java)
        val pi = intent(context, ACTION_WIDGET, WIDGET)
        if (enabled) {
            am.setAndAllowWhileIdle(AlarmManager.RTC, System.currentTimeMillis() + 10 * 60_000L, pi)
        } else {
            am.cancel(pi)
        }
    }
}
