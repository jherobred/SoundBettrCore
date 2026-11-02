package com.jhems.soundbettrcore

import android.annotation.SuppressLint
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothSocket
import android.content.Context
import android.media.AudioAttributes
import android.media.MediaPlayer
import android.media.audiofx.DynamicsProcessing
import android.media.audiofx.Virtualizer
import android.util.Log
import org.json.JSONObject
import java.util.UUID

/** What the Rust core asks Android to do: system EQ, playing the announcement, talking to the earbuds. */
object AudioEngine {
    private const val TAG = "SoundBettrCore"
    private val BANDS = floatArrayOf(31f, 62f, 125f, 250f, 500f, 1000f, 2000f, 4000f, 8000f, 16000f)
    private val UUIDS = listOf(UUID.fromString("0cf12d31-fac3-4553-bd80-d6832e700000"), UUID.fromString("00001101-0000-1000-8000-00805f9b34fb"))

    @Volatile var context: Context? = null
    private var dp: DynamicsProcessing? = null
    private var virtualizer: Virtualizer? = null
    private var player: MediaPlayer? = null
    private val sockets = HashMap<String, BluetoothSocket>()

    /** Applies a profile to all media on the phone (audio session 0). Returns an error message, or null. */
    @JvmStatic
    @Synchronized
    fun apply(json: String): String? {
        val p = JSONObject(json)
        val engine = dp ?: try {
            val config = DynamicsProcessing.Config.Builder(
                DynamicsProcessing.VARIANT_FAVOR_FREQUENCY_RESOLUTION, 2,
                true, BANDS.size, false, 0, false, 0, true,
            ).build()
            DynamicsProcessing(0, 0, config).also { dp = it }
        } catch (t: Throwable) {
            Log.w(TAG, "session 0 refused", t)
            return "This phone does not let apps change the sound of other apps. The earbuds' own EQ still works."
        }
        return try {
            val gains = p.getJSONArray("gains")
            BANDS.forEachIndexed { i, f ->
                // Each band's cutoff sits half an octave above its center
                engine.setPreEqBandAllChannelsTo(i, DynamicsProcessing.EqBand(true, f * 1.414f, gains.getDouble(i).toFloat()))
            }
            val preamp = p.getDouble("preamp").toFloat()
            val bal = p.getDouble("balance").toFloat()
            engine.setInputGainbyChannel(0, preamp + if (bal > 0) -bal else 0f)
            engine.setInputGainbyChannel(1, preamp + if (bal < 0) bal else 0f)
            engine.setLimiterAllChannelsTo(DynamicsProcessing.Limiter(true, true, 0, 1f, 60f, 10f, -0.3f, 0f))
            engine.enabled = p.getBoolean("enabled")
            surround(p.getBoolean("surround") && p.getBoolean("enabled"))
        } catch (t: Throwable) {
            Log.w(TAG, "apply failed", t)
            t.message ?: "Could not apply the sound"
        }
    }

    private fun surround(on: Boolean): String? {
        if (!on) {
            virtualizer?.enabled = false
            return null
        }
        return try {
            val v = virtualizer ?: Virtualizer(0, 0).also { virtualizer = it }
            if (v.strengthSupported) v.setStrength(1000)
            v.enabled = true
            null
        } catch (t: Throwable) {
            "This phone does not let apps add surround. Try Spatial audio in the phone's sound settings."
        }
    }

    /** Plays the announcement through the current output. */
    @JvmStatic
    fun play(path: String) {
        try {
            player?.release()
            player = MediaPlayer().apply {
                setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_SPEECH).build())
                setDataSource(path)
                setOnCompletionListener { it.release(); if (player === it) player = null }
                prepare()
                start()
            }
        } catch (t: Throwable) {
            Log.w(TAG, "play failed", t)
        }
    }

    /** Sends an A3959 packet and returns whatever the earbuds answer within 900 ms (the core picks out the reply). */
    @JvmStatic
    @SuppressLint("MissingPermission")
    fun exchange(address: String, packet: ByteArray): ByteArray? {
        for (attempt in 0..1) {
            val s = socket(address) ?: return null
            try {
                s.outputStream.write(packet)
                s.outputStream.flush()
                val input = s.inputStream
                val out = java.io.ByteArrayOutputStream()
                val until = System.currentTimeMillis() + 900
                val buf = ByteArray(512)
                while (System.currentTimeMillis() < until) {
                    if (input.available() > 0) out.write(buf, 0, input.read(buf)) else Thread.sleep(15)
                }
                return out.toByteArray()
            } catch (t: Throwable) {
                // The link drops when the earbuds go back in the case; reconnect once
                close(address)
            }
        }
        return null
    }

    @SuppressLint("MissingPermission")
    @Synchronized
    private fun socket(address: String): BluetoothSocket? {
        sockets[address]?.let { if (it.isConnected) return it }
        val ctx = context ?: return null
        val adapter = (ctx.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager).adapter ?: return null
        return try {
            val device = adapter.getRemoteDevice(address)
            UUIDS.firstNotNullOfOrNull { uuid ->
                try {
                    device.createRfcommSocketToServiceRecord(uuid).also { it.connect() }
                } catch (_: Throwable) {
                    null
                }
            }?.also { sockets[address] = it }
        } catch (_: SecurityException) {
            null
        }
    }

    @Synchronized
    private fun close(address: String) {
        try {
            sockets.remove(address)?.close()
        } catch (_: Throwable) {
        }
    }
}
