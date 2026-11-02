package com.jhems.soundbettrcore

import android.content.Context
import android.content.Intent
import java.io.File

/** The Rust core. The same library Tauri loads, so the window and the background service share one state. */
object NativeBridge {
    init {
        System.loadLibrary("soundbettrcore_lib")
    }

    @JvmStatic external fun init(dataDir: String, autoeq: String)
    @JvmStatic external fun voiceReady(modelDir: String)
    @JvmStatic external fun onOutput(json: String)

    @Volatile private var started = false

    /** Starts the core. Safe to call from the activity and the service; only the first call does anything. */
    @Synchronized
    fun start(ctx: Context) {
        if (started) return
        started = true
        AudioEngine.context = ctx.applicationContext
        // The APK build unpacks index.json.gz into index.json
        val index = File(ctx.filesDir, "autoeq-index.json")
        try {
            Assets.copyFile(ctx, "autoeq/index.json", index)
        } catch (t: Throwable) {
            android.util.Log.w("SoundBettrCore", "AutoEq index missing from the APK", t)
        }
        init(ctx.filesDir.absolutePath, index.absolutePath)
    }

    fun startService(ctx: Context) {
        ctx.startForegroundService(Intent(ctx, AudioService::class.java))
    }
}
