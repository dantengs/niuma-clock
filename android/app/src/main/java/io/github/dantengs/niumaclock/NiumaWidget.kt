package io.github.dantengs.niumaclock

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.widget.RemoteViews

/** 桌面小部件：代替电脑版的悬浮赚钱条。显示今日已赚和距下班，带打卡按钮。 */
class NiumaWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) {
        updateAll(context)
    }

    override fun onEnabled(context: Context) {
        Repo.tick(context)
    }

    companion object {
        private fun ids(context: Context): IntArray =
            AppWidgetManager.getInstance(context).getAppWidgetIds(ComponentName(context, NiumaWidget::class.java))

        fun hasWidgets(context: Context) = ids(context).isNotEmpty()

        fun updateAll(context: Context) {
            val ids = ids(context)
            if (ids.isEmpty()) return
            val now = Repo.now()
            val (money, status, onDuty) = Repo.read(context) { e ->
                val s = e.stats(now)
                val status = when {
                    !s.onDuty -> "未在上班"
                    s.lunchUntil != null -> "午休中 · ${s.lunchUntil.format(Fmt.HM)} 结束"
                    s.toEndSecs == null -> "已工作 ${Fmt.duration(s.workedSecs)}"
                    s.toEndSecs > 0 -> "距下班 ${Fmt.duration(s.toEndSecs.toDouble())}"
                    else -> "已超时 ${Fmt.duration(-s.toEndSecs.toDouble())}"
                }
                Triple(Fmt.money(e.cfg, s.earnedExact), "$status · ${now.format(Fmt.HM)} 更新", s.onDuty)
            }
            val open = PendingIntent.getActivity(
                context, 0, Intent(context, MainActivity::class.java),
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            val views = RemoteViews(context.packageName, R.layout.widget).apply {
                setTextViewText(R.id.widget_money, money)
                setTextViewText(R.id.widget_status, status)
                setTextViewText(R.id.widget_button, if (onDuty) "下班打卡" else "上班打卡")
                setOnClickPendingIntent(R.id.widget_button, ActionReceiver.toggleClockIntent(context))
                setOnClickPendingIntent(R.id.widget_root, open)
            }
            AppWidgetManager.getInstance(context).updateAppWidget(ids, views)
        }
    }
}
