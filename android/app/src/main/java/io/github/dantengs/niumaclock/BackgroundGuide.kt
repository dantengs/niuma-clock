package io.github.dantengs.niumaclock

import android.Manifest
import android.annotation.SuppressLint
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings

/**
 * 后台提醒能不能准时响，取决于系统放不放行。vivo / OriginOS 默认会限制后台应用，
 * 需要用户手动：允许通知、忽略电池优化、允许自启动、允许后台高耗电。这里检查状态并跳到对应设置页。
 */
object BackgroundGuide {
    fun isVivo(): Boolean = Build.MANUFACTURER.equals("vivo", ignoreCase = true) || Build.BRAND.equals("iqoo", ignoreCase = true)

    fun notificationsAllowed(context: Context): Boolean =
        (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) &&
            context.getSystemService(android.app.NotificationManager::class.java).areNotificationsEnabled()

    fun ignoringBatteryOptimizations(context: Context): Boolean =
        context.getSystemService(PowerManager::class.java).isIgnoringBatteryOptimizations(context.packageName)

    fun openNotificationSettings(context: Context) = start(
        context,
        Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName),
    )

    @SuppressLint("BatteryLife")
    fun requestIgnoreBatteryOptimizations(context: Context) = start(
        context,
        Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).setData(Uri.parse("package:${context.packageName}")),
    )

    /** vivo 的自启动管理页。不同系统版本位置不同，挨个试，都不行就打开应用详情页。 */
    fun openAutoStart(context: Context) {
        val candidates = listOf(
            ComponentName("com.vivo.permissionmanager", "com.vivo.permissionmanager.activity.BgStartUpManagerActivity"),
            ComponentName("com.iqoo.secure", "com.iqoo.secure.ui.phoneoptimize.BgStartUpManager"),
            ComponentName("com.iqoo.secure", "com.iqoo.secure.ui.phoneoptimize.AddWhiteListActivity"),
            ComponentName("com.vivo.abe", "com.vivo.applicationbehaviorengine.ui.ExcessivePowerManagerActivity"),
        )
        for (c in candidates) {
            if (start(context, Intent().setComponent(c))) return
        }
        openAppDetails(context)
    }

    /** “后台高耗电”在电池设置里。 */
    fun openBatterySettings(context: Context) {
        if (start(context, Intent().setComponent(ComponentName("com.vivo.abe", "com.vivo.applicationbehaviorengine.ui.ExcessivePowerManagerActivity")))) return
        if (start(context, Intent(Intent.ACTION_POWER_USAGE_SUMMARY))) return
        openAppDetails(context)
    }

    fun openAppDetails(context: Context) = start(
        context,
        Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).setData(Uri.parse("package:${context.packageName}")),
    )

    private fun start(context: Context, intent: Intent): Boolean = try {
        context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        true
    } catch (e: Exception) {
        false
    }
}
