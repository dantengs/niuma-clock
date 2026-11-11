package io.github.dantengs.niumaclock

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import io.github.dantengs.niumaclock.core.AckAction
import io.github.dantengs.niumaclock.core.AlertKind
import io.github.dantengs.niumaclock.core.DaySummary
import io.github.dantengs.niumaclock.core.Notice
import io.github.dantengs.niumaclock.core.Quotes

/**
 * 系统通知。需要确认的提醒带按钮（搞定了 / 稍后 / 跳过），划掉算“稍后”；
 * 在应用里确认过的提醒，对应通知自动收回。
 */
object Notifier {
    private const val CH_REMIND = "remind"
    private const val CH_QUIET = "remind_quiet"
    private const val CH_INFO = "info"
    private const val ACK_PREFIX = "ack:"

    fun createChannels(context: Context) {
        val nm = context.getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(
            NotificationChannel(CH_REMIND, "提醒（有声音）", NotificationManager.IMPORTANCE_HIGH).apply {
                description = "喝水、活动、吃药、下班等到点提醒"
                enableVibration(true)
            },
        )
        nm.createNotificationChannel(
            NotificationChannel(CH_QUIET, "提醒（静音）", NotificationManager.IMPORTANCE_HIGH).apply {
                description = "设置里关掉提示音时使用"
                setSound(null, null)
                enableVibration(false)
            },
        )
        nm.createNotificationChannel(
            NotificationChannel(CH_INFO, "下班结算", NotificationManager.IMPORTANCE_DEFAULT).apply {
                description = "从通知或小部件下班打卡后的今日收入"
            },
        )
    }

    private fun allowed(context: Context) = Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
        context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED

    private fun openApp(context: Context): PendingIntent = PendingIntent.getActivity(
        context, 0,
        Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )

    private fun ack(context: Context, kind: AlertKind, action: AckAction): PendingIntent =
        ActionReceiver.ackIntent(context, kind.key, action.name)

    fun show(context: Context, notices: List<Notice>) {
        if (notices.isEmpty() || !allowed(context)) return
        val sound = Repo.read(context) { it.cfg.sound }
        val nm = context.getSystemService(NotificationManager::class.java)
        for (n in notices) {
            val builder = Notification.Builder(context, if (sound) CH_REMIND else CH_QUIET)
                .setSmallIcon(R.drawable.ic_notify)
                .setContentTitle(n.title)
                .setContentText(n.body.ifEmpty { "到点了" })
                .setStyle(Notification.BigTextStyle().bigText(n.body.ifEmpty { "到点了" }))
                .setCategory(Notification.CATEGORY_REMINDER)
                .setContentIntent(openApp(context))
                .setAutoCancel(!n.needsAck)
                .setWhen(System.currentTimeMillis())
            if (n.needsAck) {
                when (val kind = n.kind) {
                    is AlertKind.Loop -> builder
                        .addAction(action(context, "搞定了", ack(context, kind, AckAction.Done)))
                        .addAction(action(context, "稍后", ack(context, kind, AckAction.Snooze)))
                        .addAction(action(context, "跳过", ack(context, kind, AckAction.Skip)))
                        .setDeleteIntent(ack(context, kind, AckAction.Snooze))
                    is AlertKind.Med -> builder
                        .addAction(action(context, "吃了", ack(context, kind, AckAction.Done)))
                        .addAction(action(context, "稍后", ack(context, kind, AckAction.Snooze)))
                        .setDeleteIntent(ack(context, kind, AckAction.Snooze))
                    AlertKind.WorkEnd -> builder
                        .addAction(action(context, "下班打卡", ActionReceiver.clockOutIntent(context)))
                        .addAction(action(context, "再卷半小时", ack(context, kind, AckAction.Snooze)))
                        .setDeleteIntent(ack(context, kind, AckAction.Done))
                    AlertKind.Sick -> builder.setDeleteIntent(ack(context, kind, AckAction.Done))
                }
            }
            val tag = if (n.needsAck) ACK_PREFIX + n.kind.key else n.kind.key
            nm.notify(tag, 1, builder.build())
        }
    }

    private fun action(context: Context, title: String, pi: PendingIntent) =
        Notification.Action.Builder(null, title, pi).build()

    /** 收回已经在应用里处理掉的提醒通知。 */
    fun cancelStale(context: Context, pendingKeys: Set<String>) {
        val nm = context.getSystemService(NotificationManager::class.java)
        for (sbn in nm.activeNotifications) {
            val tag = sbn.tag ?: continue
            if (tag.startsWith(ACK_PREFIX) && tag.removePrefix(ACK_PREFIX) !in pendingKeys) nm.cancel(tag, sbn.id)
        }
    }

    fun cancel(context: Context, key: String) {
        val nm = context.getSystemService(NotificationManager::class.java)
        nm.cancel(ACK_PREFIX + key, 1)
        nm.cancel(key, 1)
    }

    fun cancelAll(context: Context) = context.getSystemService(NotificationManager::class.java).cancelAll()

    /** 从通知或小部件下班打卡后，用一条通知报今日收入。 */
    fun showSummary(context: Context, summary: DaySummary) {
        if (!allowed(context)) return
        val (money, text) = Repo.read(context) { e ->
            Fmt.money(e.cfg, summary.income) to "工作 ${Fmt.duration(summary.workedSecs)} · 完成提醒 ${summary.done} 次。${Quotes.offWork(summary.date)}"
        }
        val n = Notification.Builder(context, CH_INFO)
            .setSmallIcon(R.drawable.ic_notify)
            .setContentTitle("下班！今日收入 $money")
            .setContentText(text)
            .setStyle(Notification.BigTextStyle().bigText(text))
            .setContentIntent(openApp(context))
            .setAutoCancel(true)
            .build()
        context.getSystemService(NotificationManager::class.java).notify("summary", 1, n)
    }
}
