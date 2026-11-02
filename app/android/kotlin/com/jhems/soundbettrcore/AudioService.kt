package com.jhems.soundbettrcore

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import java.util.concurrent.Executors

/** Keeps the EQ on and announces connections while the app's window is closed. */
class AudioService : Service() {
    private lateinit var am: AudioManager
    private val main = Handler(Looper.getMainLooper())
    private val worker = Executors.newSingleThreadExecutor()
    private var last: String? = null

    private val callback = object : AudioDeviceCallback() {
        override fun onAudioDevicesAdded(added: Array<out AudioDeviceInfo>) = check()
        override fun onAudioDevicesRemoved(removed: Array<out AudioDeviceInfo>) = check()
    }

    // ponytail: route changes without a device add or remove (rare) are caught by this 2 s check
    private val tick = object : Runnable {
        override fun run() {
            check()
            main.postDelayed(this, 2000)
        }
    }

    override fun onCreate() {
        super.onCreate()
        NativeBridge.start(this)
        foreground()
        am = getSystemService(AUDIO_SERVICE) as AudioManager
        am.registerAudioDeviceCallback(callback, main)
        main.post(tick)
        safely { NativeBridge.voiceReady(Assets.voice(this).absolutePath) }
    }

    private fun check() {
        val json = Outputs.current(am)?.toString() ?: "null"
        if (json == last) return
        last = json
        safely { NativeBridge.onOutput(json) }
    }

    /** An exception on a worker thread would close the app; log it instead. */
    private fun safely(work: () -> Unit) = worker.execute {
        try {
            work()
        } catch (t: Throwable) {
            android.util.Log.w("SoundBettrCore", "background work failed", t)
        }
    }

    private fun foreground() {
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(NotificationChannel("sound", "Sound for your devices", NotificationManager.IMPORTANCE_MIN))
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        val n = Notification.Builder(this, "sound")
            .setContentTitle("SoundBettrCore is on")
            .setContentText("Your EQ follows whatever you plug in or connect")
            .setSmallIcon(android.R.drawable.stat_sys_headset)
            .setContentIntent(open)
            .setOngoing(true)
            .build()
        if (Build.VERSION.SDK_INT >= 29) startForeground(1, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK) else startForeground(1, n)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int) = START_STICKY

    override fun onDestroy() {
        main.removeCallbacks(tick)
        am.unregisterAudioDeviceCallback(callback)
        worker.shutdown()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
