package com.jhems.soundbettrcore

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.media.audiofx.DynamicsProcessing
import android.os.Build
import android.os.IBinder
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.ln
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

enum class TestSignalKind(val label: String) {
    ORBIT_71("7.1 Speaker Orbit 360"),
    SUB_BASS_SWEEP("25-160 Hz Sub-Bass Sweep"),
    VOCAL_PULSE("Vocal Clarity Pulse"),
    PINK_NOISE("Pink Noise Reference"),
}

object StudioRepository {
    private val _profile = MutableStateFlow(DspProfile())
    val profile: StateFlow<DspProfile> = _profile

    private val _r50i = MutableStateFlow(A3959State())
    val r50i: StateFlow<A3959State> = _r50i

    private val _lastPacketHex = MutableStateFlow(
        A3959Protocol.formatHex(
            A3959Protocol.buildEqualizerWithDrcPacket(
                A3959Protocol.CUSTOM_EQ_PRESET_ID,
                DspProfile().hwEqTenthsDb,
            )
        )
    )
    val lastPacketHex: StateFlow<String> = _lastPacketHex

    private val _playingTest = MutableStateFlow<TestSignalKind?>(null)
    val playingTest: StateFlow<TestSignalKind?> = _playingTest

    fun updateProfile(next: DspProfile) {
        _profile.value = next
        val drc = A3959Protocol.computeA3959DrcBytes(next.hwEqTenthsDb)
        val drcTenths = IntArray(8) { i -> (drc[i].toInt() and 0xFF) - 120 }
        _r50i.value = _r50i.value.copy(
            eqGainsTenthsDb = next.hwEqTenthsDb.copyOf(),
            drcPreviewTenthsDb = drcTenths,
        )
    }

    fun updateR50i(next: A3959State, packet: ByteArray? = null) {
        _r50i.value = next
        if (packet != null) {
            _lastPacketHex.value = A3959Protocol.formatHex(packet)
        }
    }

    fun setPlayingTest(kind: TestSignalKind?) {
        _playingTest.value = kind
    }
}

class SoundBettrService : Service() {
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private var globalDynamics: DynamicsProcessing? = null
    private var testPlayerJob: Job? = null

    override fun onCreate() {
        super.onCreate()
        startForegroundWithChannel()
        attachGlobalDynamicsProcessing(StudioRepository.profile.value)
        serviceScope.launch {
            StudioRepository.profile.collect { prof ->
                applyProfileToGlobalDynamics(prof)
            }
        }
        serviceScope.launch {
            StudioRepository.playingTest.collect { kind ->
                testPlayerJob?.cancel()
                if (kind != null) {
                    testPlayerJob = launchTestGenerator(kind)
                }
            }
        }
    }

    private fun startForegroundWithChannel() {
        val channelId = "soundbettrcore_dsp_channel"
        val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val ch = NotificationChannel(
                channelId,
                "SoundBettrCore R50i NC Audio Engine",
                NotificationManager.IMPORTANCE_LOW,
            )
            nm.createNotificationChannel(ch)
        }
        val notif = Notification.Builder(this, channelId)
            .setContentTitle("SoundBettrCore R50i NC Active")
            .setContentText("Harman Target EQ, 7.1 Spatializer & Dolby Leveler running")
            .setSmallIcon(R.drawable.ic_launcher_foreground)
            .build()
        try {
            startForeground(1001, notif)
        } catch (_: Throwable) {
        }
    }

    private fun attachGlobalDynamicsProcessing(profile: DspProfile) {
        try {
            val config = DynamicsProcessing.Config.Builder(
                DynamicsProcessing.VARIANT_FAVOR_FREQUENCY_RESOLUTION,
                2,
                true,
                10,
                true,
                3,
                true,
                10,
                true,
            ).build()
            val dp = DynamicsProcessing(0, 0, config)
            dp.enabled = !profile.bypass
            globalDynamics = dp
            applyProfileToGlobalDynamics(profile)
        } catch (_: Throwable) {
            // Device may restrict session 0 DynamicsProcessing; built-in PCM float engine still runs
        }
    }

    private fun applyProfileToGlobalDynamics(profile: DspProfile) {
        val dp = globalDynamics ?: return
        try {
            dp.enabled = !profile.bypass
            profile.bands.forEachIndexed { idx, band ->
                val eqBand = DynamicsProcessing.EqBand(
                    band.enabled,
                    band.freqHz.toFloat() * 1.4f,
                    band.gainDb.toFloat(),
                )
                dp.setPreEqBandAllChannelsTo(idx, eqBand)
            }
            val limiter = DynamicsProcessing.Limiter(
                true,
                true,
                0,
                2.0f,
                40.0f,
                4.0f,
                profile.limiterCeilingDb.toFloat(),
                profile.preampDb.toFloat(),
            )
            dp.setLimiterAllChannelsTo(limiter)
        } catch (_: Throwable) {
        }
    }

    private fun launchTestGenerator(kind: TestSignalKind): Job = serviceScope.launch(Dispatchers.Default) {
        val sampleRate = 48000
        val minBuf = AudioTrack.getMinBufferSize(
            sampleRate,
            AudioFormat.CHANNEL_OUT_STEREO,
            AudioFormat.ENCODING_PCM_FLOAT,
        ).coerceAtLeast(4096)

        val track = AudioTrack.Builder()
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                    .build()
            )
            .setAudioFormat(
                AudioFormat.Builder()
                    .setEncoding(AudioFormat.ENCODING_PCM_FLOAT)
                    .setSampleRate(sampleRate)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                    .build()
            )
            .setBufferSizeInBytes(minBuf * 2)
            .build()

        val pipeline = DspPipeline(sampleRate.toDouble(), StudioRepository.profile.value)
        val chunkFrames = 1024
        val pcm = FloatArray(chunkFrames * 2)
        var frameIdx = 0L
        var rng = -0x5c6a6412

        try {
            track.play()
            while (isActive) {
                pipeline.updateProfile(StudioRepository.profile.value)
                for (i in 0 until chunkFrames) {
                    val t = frameIdx.toDouble() / sampleRate.toDouble()
                    frameIdx++
                    rng = rng * 1664525 + 1013904223
                    val white = ((rng ushr 9).toDouble() / 4194304.0) - 1.0
                    val rawL: Float
                    val rawR: Float
                    when (kind) {
                        TestSignalKind.ORBIT_71 -> {
                            val angle = 2.0 * PI * (t / 5.0)
                            val panL = sqrt((1.0 - sin(angle)) * 0.5)
                            val panR = sqrt((1.0 + sin(angle)) * 0.5)
                            val env = exp(-((t * 2.0) % 1.0) * 5.0)
                            val sig = sin(2.0 * PI * 440.0 * t) * env * 0.28 + white * 0.04
                            rawL = (sig * panL).toFloat()
                            rawR = (sig * panR).toFloat()
                        }
                        TestSignalKind.SUB_BASS_SWEEP -> {
                            val prog = (t % 5.0) / 5.0
                            val fStart = 25.0
                            val fEnd = 160.0
                            val phase = 2.0 * PI * fStart * 5.0 * ((fEnd / fStart).pow(prog) - 1.0) / ln(fEnd / fStart)
                            val v = (sin(phase) * 0.55).toFloat()
                            rawL = v
                            rawR = v
                        }
                        TestSignalKind.VOCAL_PULSE -> {
                            val env = maxOf(0.0, sin(2.0 * PI * 1.5 * t)).pow(2.0)
                            val sig = (sin(2.0 * PI * 190.0 * t) * 0.25 +
                                sin(2.0 * PI * 2400.0 * t) * 0.16 +
                                sin(2.0 * PI * 3200.0 * t) * 0.12) * env
                            rawL = sig.toFloat()
                            rawR = sig.toFloat()
                        }
                        TestSignalKind.PINK_NOISE -> {
                            val v = (white * 0.18).toFloat()
                            rawL = v
                            rawR = v
                        }
                    }
                    val (outL, outR) = pipeline.processFrame(rawL, rawR)
                    pcm[i * 2] = outL
                    pcm[i * 2 + 1] = outR
                }
                track.write(pcm, 0, pcm.size, AudioTrack.WRITE_BLOCKING)
            }
        } finally {
            try {
                track.stop()
                track.release()
            } catch (_: Throwable) {
            }
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int = START_STICKY

    override fun onDestroy() {
        testPlayerJob?.cancel()
        try {
            globalDynamics?.release()
        } catch (_: Throwable) {
        }
        serviceScope.cancel()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
