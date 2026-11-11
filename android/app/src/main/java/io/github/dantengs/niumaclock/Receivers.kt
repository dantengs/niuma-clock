package io.github.dantengs.niumaclock

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import io.github.dantengs.niumaclock.core.AckAction
import io.github.dantengs.niumaclock.core.AlertKind

/** 精确闹钟到点。 */
class AlarmReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        Repo.tick(context)
    }
}

/** 开机、应用更新、改系统时间或时区后，闹钟会丢，重新排。 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action in RESCHEDULE_ACTIONS) Repo.tick(context)
    }

    private companion object {
        val RESCHEDULE_ACTIONS = setOf(
            Intent.ACTION_BOOT_COMPLETED, Intent.ACTION_MY_PACKAGE_REPLACED, Intent.ACTION_TIME_CHANGED, Intent.ACTION_TIMEZONE_CHANGED,
        )
    }
}

/** 通知按钮和桌面小部件按钮。 */
class ActionReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val now = Repo.now()
        when (intent.action) {
            ACTION_ACK -> {
                val kind = AlertKind.parse(intent.getStringExtra(EXTRA_KIND).orEmpty()) ?: return
                val action = runCatching { AckAction.valueOf(intent.getStringExtra(EXTRA_ACTION).orEmpty()) }.getOrNull() ?: return
                Repo.update(context) { it.ack(kind, action, now) }
                Notifier.cancel(context, kind.key)
            }
            ACTION_CLOCK_OUT -> {
                val summary = Repo.update(context) { e ->
                    if (e.data.workEndPending) e.ack(AlertKind.WorkEnd, AckAction.Done, now)
                    e.clockOut(now)
                }
                Notifier.cancel(context, AlertKind.WorkEnd.key)
                summary?.let { Notifier.showSummary(context, it) }
            }
            ACTION_TOGGLE_CLOCK -> {
                val summary = Repo.update(context) { e -> if (e.onDuty()) e.clockOut(now) else { e.clockIn(now); null } }
                summary?.let { Notifier.showSummary(context, it) }
            }
        }
    }

    companion object {
        const val ACTION_ACK = "io.github.dantengs.niumaclock.ACK"
        const val ACTION_CLOCK_OUT = "io.github.dantengs.niumaclock.CLOCK_OUT"
        const val ACTION_TOGGLE_CLOCK = "io.github.dantengs.niumaclock.TOGGLE_CLOCK"
        private const val EXTRA_KIND = "kind"
        private const val EXTRA_ACTION = "action"

        fun ackIntent(context: Context, kindKey: String, action: String): PendingIntent {
            val intent = Intent(context, ActionReceiver::class.java)
                .setAction(ACTION_ACK)
                .putExtra(EXTRA_KIND, kindKey)
                .putExtra(EXTRA_ACTION, action)
            // 每个提醒、每个按钮各用一个请求码，避免 PendingIntent 互相覆盖。
            val code = (kindKey + action).hashCode()
            return PendingIntent.getBroadcast(context, code, intent, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        }

        fun clockOutIntent(context: Context): PendingIntent = PendingIntent.getBroadcast(
            context, 100, Intent(context, ActionReceiver::class.java).setAction(ACTION_CLOCK_OUT),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        fun toggleClockIntent(context: Context): PendingIntent = PendingIntent.getBroadcast(
            context, 101, Intent(context, ActionReceiver::class.java).setAction(ACTION_TOGGLE_CLOCK),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }
}
