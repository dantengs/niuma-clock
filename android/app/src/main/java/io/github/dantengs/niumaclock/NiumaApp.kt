package io.github.dantengs.niumaclock

import android.app.Application

class NiumaApp : Application() {
    override fun onCreate() {
        super.onCreate()
        Notifier.createChannels(this)
    }
}
