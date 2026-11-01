package com.jhems.soundbettrcore

import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.ln
import kotlin.math.log10
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt
import kotlin.math.tanh

enum class BiquadFilterKind {
    LOW_SHELF,
    PEAKING,
    HIGH_SHELF,
    LOW_PASS,
    HIGH_PASS,
    BAND_PASS,
}

data class ParametricBand(
    val freqHz: Double,
    val gainDb: Double,
    val q: Double = 1.41,
    val kind: BiquadFilterKind = BiquadFilterKind.PEAKING,
    val enabled: Boolean = true,
)

enum class RoomModelKind(val label: String) {
    STUDIO_BOOTH("Studio Control Booth"),
    CINEMA_THEATER("Dolby Cinema Theater"),
    CONCERT_HALL("Symphony Concert Hall"),
}

data class DspProfile(
    val id: String = "r50i_harman_reference",
    val name: String = "R50i NC Harman Reference",
    val subtitle: String = "AutoEq mid-bass mud cut + upper-mid vocal clarity & subtle 3D stage",
    val bypass: Boolean = false,
    val autoHeadroom: Boolean = true,
    val preampDb: Double = 0.0,
    val bands: List<ParametricBand> = defaultHarmanBands(),
    val surroundWidth: Double = 0.35,
    val crossfeedAmount: Double = 0.22,
    val subBassExciter: Double = 0.38,
    val vocalClarity: Double = 0.42,
    val roomSpace: Double = 0.16,
    val roomModel: RoomModelKind = RoomModelKind.STUDIO_BOOTH,
    val dolbyLevelerAmount: Double = 0.45,
    val limiterCeilingDb: Double = -0.3,
    val hwEqTenthsDb: IntArray = intArrayOf(-25, -18, 5, 18, 32, 24, 10, 15),
) {
    companion object {
        val HOST_FREQS = doubleArrayOf(31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0)
        val HARMAN_GAINS = doubleArrayOf(1.5, -2.0, -4.5, -2.5, 0.0, 1.0, 3.0, 3.5, 1.5, 2.5)

        fun makeBands(gains: DoubleArray): List<ParametricBand> =
            HOST_FREQS.mapIndexed { idx, freq ->
                ParametricBand(
                    freqHz = freq,
                    gainDb = gains.getOrElse(idx) { 0.0 },
                    q = if (idx == 0 || idx == 9) 0.707 else 1.41,
                    kind = when (idx) {
                        0 -> BiquadFilterKind.LOW_SHELF
                        9 -> BiquadFilterKind.HIGH_SHELF
                        else -> BiquadFilterKind.PEAKING
                    },
                    enabled = true,
                )
            }

        fun defaultHarmanBands(): List<ParametricBand> = makeBands(HARMAN_GAINS)

        fun builtinPresets(): List<DspProfile> = listOf(
            DspProfile(),
            DspProfile(
                id = "dolby_cinema_3d",
                name = "Dolby Cinema 3D",
                subtitle = "Wide 7.1 virtual surround, theater room acoustics, dialog lift & loudness leveler",
                preampDb = 0.5,
                bands = makeBands(doubleArrayOf(3.5, 1.0, -3.5, -2.0, 0.5, 2.0, 3.8, 4.0, 2.0, 3.0)),
                surroundWidth = 0.78,
                crossfeedAmount = 0.18,
                subBassExciter = 0.62,
                vocalClarity = 0.68,
                roomSpace = 0.42,
                roomModel = RoomModelKind.CINEMA_THEATER,
                dolbyLevelerAmount = 0.72,
                hwEqTenthsDb = intArrayOf(15, -20, -10, 15, 38, 35, 20, 25),
            ),
            DspProfile(
                id = "dolby_music_studio",
                name = "Dolby Music Studio",
                subtitle = "Mastered Harman target curve, studio crossfeed, warm harmonic sub-bass & natural dynamics",
                bands = makeBands(doubleArrayOf(2.5, -1.0, -4.0, -2.0, 0.5, 1.2, 3.2, 3.5, 2.2, 3.2)),
                surroundWidth = 0.48,
                crossfeedAmount = 0.28,
                subBassExciter = 0.46,
                vocalClarity = 0.45,
                roomSpace = 0.20,
                dolbyLevelerAmount = 0.48,
                hwEqTenthsDb = intArrayOf(-10, -25, 0, 12, 32, 30, 22, 30),
            ),
            DspProfile(
                id = "bassup_ultra_clean",
                name = "BassUp Ultra Clean",
                subtitle = "Deep 31-62 Hz sub-bass slam with 125 Hz anti-mud notch so vocals stay crystal clear",
                bands = makeBands(doubleArrayOf(5.5, 3.2, -4.8, -2.5, 0.0, 1.0, 2.8, 3.2, 2.0, 2.8)),
                surroundWidth = 0.40,
                crossfeedAmount = 0.20,
                subBassExciter = 0.85,
                vocalClarity = 0.40,
                roomSpace = 0.14,
                dolbyLevelerAmount = 0.55,
                hwEqTenthsDb = intArrayOf(35, -30, -15, 10, 28, 30, 20, 25),
            ),
            DspProfile(
                id = "fps_gaming_footsteps",
                name = "FPS Gaming Footsteps",
                subtitle = "Pinpoint 7.1 binaural imaging, 2-6 kHz footstep & reload detail, low-end explosion control",
                preampDb = 0.5,
                bands = makeBands(doubleArrayOf(-2.0, -3.5, -5.0, -1.5, 1.0, 2.5, 4.8, 5.2, 3.5, 2.5)),
                surroundWidth = 0.88,
                crossfeedAmount = 0.10,
                subBassExciter = 0.15,
                vocalClarity = 0.58,
                roomSpace = 0.08,
                dolbyLevelerAmount = 0.80,
                hwEqTenthsDb = intArrayOf(-35, -40, 10, 25, 48, 50, 35, 20),
            ),
            DspProfile(
                id = "podcast_late_night",
                name = "Podcast & Late Night",
                subtitle = "Dialog isolation, sub-bass rumble cut, and strong upward/downward loudness leveling",
                preampDb = 1.0,
                bands = makeBands(doubleArrayOf(-4.0, -4.5, -4.5, -1.0, 1.5, 3.2, 4.2, 3.8, 1.0, 0.5)),
                surroundWidth = 0.20,
                crossfeedAmount = 0.32,
                subBassExciter = 0.05,
                vocalClarity = 0.88,
                roomSpace = 0.10,
                dolbyLevelerAmount = 0.90,
                hwEqTenthsDb = intArrayOf(-45, -35, 15, 32, 42, 38, 10, 5),
            ),
        )
    }
}

class BiquadCoeffs(
    val b0: Double = 1.0,
    val b1: Double = 0.0,
    val b2: Double = 0.0,
    val a1: Double = 0.0,
    val a2: Double = 0.0,
) {
    fun magnitudeDbAt(freqHz: Double, sampleRate: Double): Double {
        val w = 2.0 * PI * freqHz / sampleRate
        val cosW = cos(w)
        val cos2W = cos(2.0 * w)
        val sinW = sin(w)
        val sin2W = sin(2.0 * w)
        val numRe = b0 + b1 * cosW + b2 * cos2W
        val numIm = -b1 * sinW - b2 * sin2W
        val denRe = 1.0 + a1 * cosW + a2 * cos2W
        val denIm = -a1 * sinW - a2 * sin2W
        val numSq = numRe * numRe + numIm * numIm
        val denSq = max(1e-18, denRe * denRe + denIm * denIm)
        return 10.0 * log10(numSq / denSq)
    }

    companion object {
        fun design(
            kind: BiquadFilterKind,
            freqHz: Double,
            sampleRate: Double,
            gainDb: Double,
            q: Double,
        ): BiquadCoeffs {
            val f0 = freqHz.coerceIn(16.0, max(1000.0, sampleRate * 0.49))
            val qSafe = q.coerceIn(0.1, 18.0)
            val w0 = 2.0 * PI * f0 / sampleRate
            val cosW0 = cos(w0)
            val sinW0 = sin(w0)
            val alpha = sinW0 / (2.0 * qSafe)
            val a = 10.0.pow(gainDb / 40.0)

            val b0: Double
            val b1: Double
            val b2: Double
            val a0: Double
            val a1: Double
            val a2: Double

            when (kind) {
                BiquadFilterKind.PEAKING -> {
                    b0 = 1.0 + alpha * a
                    b1 = -2.0 * cosW0
                    b2 = 1.0 - alpha * a
                    a0 = 1.0 + alpha / a
                    a1 = -2.0 * cosW0
                    a2 = 1.0 - alpha / a
                }
                BiquadFilterKind.LOW_SHELF -> {
                    val t = 2.0 * sqrt(a) * alpha
                    b0 = a * ((a + 1.0) - (a - 1.0) * cosW0 + t)
                    b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cosW0)
                    b2 = a * ((a + 1.0) - (a - 1.0) * cosW0 - t)
                    a0 = (a + 1.0) + (a - 1.0) * cosW0 + t
                    a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cosW0)
                    a2 = (a + 1.0) + (a - 1.0) * cosW0 - t
                }
                BiquadFilterKind.HIGH_SHELF -> {
                    val t = 2.0 * sqrt(a) * alpha
                    b0 = a * ((a + 1.0) + (a - 1.0) * cosW0 + t)
                    b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cosW0)
                    b2 = a * ((a + 1.0) + (a - 1.0) * cosW0 - t)
                    a0 = (a + 1.0) - (a - 1.0) * cosW0 + t
                    a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cosW0)
                    a2 = (a + 1.0) - (a - 1.0) * cosW0 - t
                }
                BiquadFilterKind.LOW_PASS -> {
                    b0 = (1.0 - cosW0) * 0.5
                    b1 = 1.0 - cosW0
                    b2 = (1.0 - cosW0) * 0.5
                    a0 = 1.0 + alpha
                    a1 = -2.0 * cosW0
                    a2 = 1.0 - alpha
                }
                BiquadFilterKind.HIGH_PASS -> {
                    b0 = (1.0 + cosW0) * 0.5
                    b1 = -(1.0 + cosW0)
                    b2 = (1.0 + cosW0) * 0.5
                    a0 = 1.0 + alpha
                    a1 = -2.0 * cosW0
                    a2 = 1.0 - alpha
                }
                BiquadFilterKind.BAND_PASS -> {
                    b0 = alpha
                    b1 = 0.0
                    b2 = -alpha
                    a0 = 1.0 + alpha
                    a1 = -2.0 * cosW0
                    a2 = 1.0 - alpha
                }
            }
            return BiquadCoeffs(b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
        }
    }
}

class StereoBiquad(var coeffs: BiquadCoeffs = BiquadCoeffs()) {
    private var z1L = 0.0
    private var z2L = 0.0
    private var z1R = 0.0
    private var z2R = 0.0

    fun processLeft(l: Double): Double {
        val out = coeffs.b0 * l + z1L
        z1L = coeffs.b1 * l - coeffs.a1 * out + z2L
        z2L = coeffs.b2 * l - coeffs.a2 * out
        return out
    }

    fun processRight(r: Double): Double {
        val out = coeffs.b0 * r + z1R
        z1R = coeffs.b1 * r - coeffs.a1 * out + z2R
        z2R = coeffs.b2 * r - coeffs.a2 * out
        return out
    }
}

class DspPipeline(val sampleRate: Double, initialProfile: DspProfile) {
    var profile: DspProfile = initialProfile
        private set

    private var preampLinear = 1.0
    private var eqFilters = emptyList<StereoBiquad>()
    private var subBpFilter = StereoBiquad()
    private var subLpFilter = StereoBiquad()
    private var vocalFilter = StereoBiquad()
    private var pinnaFilter = StereoBiquad()
    private var limiterEnv = 0.0

    init {
        updateProfile(initialProfile)
    }

    fun updateProfile(next: DspProfile) {
        val maxPosGain = next.bands.filter { it.enabled }.maxOfOrNull { it.gainDb }?.coerceAtLeast(0.0) ?: 0.0
        val exciterHeadroom = next.subBassExciter * 2.5 + next.vocalClarity * 1.8
        val autoTrim = if (next.autoHeadroom) -(maxPosGain * 0.55 + exciterHeadroom) else 0.0
        val totalPreampDb = (next.preampDb + autoTrim).coerceIn(-18.0, 12.0)
        preampLinear = 10.0.pow(totalPreampDb / 20.0)

        eqFilters = next.bands.filter { it.enabled && abs(it.gainDb) > 0.01 }.map {
            StereoBiquad(BiquadCoeffs.design(it.kind, it.freqHz, sampleRate, it.gainDb, it.q))
        }
        subBpFilter = StereoBiquad(BiquadCoeffs.design(BiquadFilterKind.BAND_PASS, 72.0, sampleRate, 0.0, 1.1))
        subLpFilter = StereoBiquad(BiquadCoeffs.design(BiquadFilterKind.LOW_PASS, 175.0, sampleRate, 0.0, 0.707))
        vocalFilter = StereoBiquad(BiquadCoeffs.design(BiquadFilterKind.PEAKING, 2750.0, sampleRate, next.vocalClarity * 4.5, 1.15))
        pinnaFilter = StereoBiquad(BiquadCoeffs.design(BiquadFilterKind.HIGH_SHELF, 3400.0, sampleRate, next.surroundWidth * 2.2, 0.707))
        profile = next
    }

    fun processFrame(inL: Float, inR: Float): Pair<Float, Float> {
        if (profile.bypass) return inL to inR

        var l = inL.toDouble() * preampLinear
        var r = inR.toDouble() * preampLinear

        for (f in eqFilters) {
            l = f.processLeft(l)
            r = f.processRight(r)
        }

        if (profile.subBassExciter > 0.005) {
            val drive = 1.0 + profile.subBassExciter * 3.2
            val subL = subBpFilter.processLeft(l)
            val subR = subBpFilter.processRight(r)
            val satL = tanh(subL * drive) + 0.18 * tanh(subL * drive * 2.0)
            val satR = tanh(subR * drive) + 0.18 * tanh(subR * drive * 2.0)
            l += subLpFilter.processLeft(satL) * (profile.subBassExciter * 0.42)
            r += subLpFilter.processRight(satR) * (profile.subBassExciter * 0.42)
        }

        if (profile.vocalClarity > 0.005) {
            val mid = 0.5 * (l + r)
            val side = 0.5 * (l - r)
            val midBoosted = vocalFilter.processLeft(mid)
            val blend = profile.vocalClarity * 0.65
            val newMid = mid * (1.0 - blend) + midBoosted * blend
            l = newMid + side
            r = newMid - side
        }

        if (profile.surroundWidth > 0.005 || profile.crossfeedAmount > 0.005) {
            val mid = 0.5 * (l + r)
            val side = 0.5 * (l - r)
            val pinnaL = pinnaFilter.processLeft(side)
            val pinnaR = pinnaFilter.processRight(-side)
            val w = profile.surroundWidth
            val wideL = mid + side * (1.0 + w * 0.58) + pinnaL * (w * 0.44)
            val wideR = mid - side * (1.0 + w * 0.58) + pinnaR * (w * 0.44)
            val cf = profile.crossfeedAmount.coerceIn(0.0, 0.45)
            l = wideL * (1.0 - cf * 0.35) + wideR * cf
            r = wideR * (1.0 - cf * 0.35) + wideL * cf
        }

        val ceiling = min(0.966, 10.0.pow(profile.limiterCeilingDb.coerceIn(-6.0, -0.1) / 20.0))
        val peak = max(abs(l), abs(r))
        limiterEnv = if (peak > limiterEnv) {
            0.25 * limiterEnv + 0.75 * peak
        } else {
            0.9985 * limiterEnv + 0.0015 * peak
        }
        if (limiterEnv > ceiling) {
            val scale = ceiling / limiterEnv
            l *= scale
            r *= scale
        }
        if (abs(l) > ceiling) l = ceiling * tanh(l / ceiling)
        if (abs(r) > ceiling) r = ceiling * tanh(r / ceiling)

        return l.toFloat() to r.toFloat()
    }

    fun computeResponseCurve(numPoints: Int = 96): List<Pair<Double, Double>> {
        val n = max(16, numPoints)
        val minLog = ln(20.0)
        val maxLog = ln(20000.0)
        return List(n) { i ->
            val t = i.toDouble() / (n - 1).toDouble()
            val freq = exp(minLog + t * (maxLog - minLog))
            var db = profile.preampDb
            for (f in eqFilters) {
                db += f.coeffs.magnitudeDbAt(freq, sampleRate)
            }
            freq to db
        }
    }
}

object EqualizerApoCodec {
    fun renderEqualizerApoTxt(profile: DspProfile): String {
        if (profile.bypass) {
            return "# SoundBettrCore EqualizerAPO Live Chain (BYPASSED)\nPreamp: 0.0 dB\n"
        }
        val sb = StringBuilder()
        sb.append("# SoundBettrCore EqualizerAPO Live Chain\n")
        sb.append("# Profile: ${profile.name} (${profile.id})\n")
        sb.append("Preamp: %.2f dB\n".format(java.util.Locale.US, profile.preampDb))
        profile.bands.forEachIndexed { idx, band ->
            val state = if (band.enabled) "ON" else "OFF"
            val code = when (band.kind) {
                BiquadFilterKind.LOW_SHELF -> "LSC"
                BiquadFilterKind.HIGH_SHELF -> "HSC"
                else -> "PK"
            }
            sb.append(
                "Filter %d: %s %s Fc %.1f Hz Gain %.2f dB Q %.2f\n".format(
                    java.util.Locale.US,
                    idx + 1,
                    state,
                    code,
                    band.freqHz,
                    band.gainDb,
                    band.q,
                )
            )
        }
        return sb.toString()
    }

    fun parseEqualizerApoTxt(content: String, base: DspProfile = DspProfile()): DspProfile {
        var preamp = base.preampDb
        val parsed = mutableListOf<ParametricBand>()
        var sawDirective = false

        content.lineSequence().forEach { raw ->
            val line = raw.trim()
            if (line.isEmpty() || line.startsWith("#")) return@forEach
            if (line.startsWith("Preamp:", ignoreCase = true)) {
                val after = line.substringAfter(":").trim().split(Regex("\\s+")).firstOrNull()
                after?.toDoubleOrNull()?.let {
                    preamp = it.coerceIn(-15.0, 15.0)
                    sawDirective = true
                }
            } else if (line.startsWith("Filter", ignoreCase = true) && line.contains(":")) {
                val tokens = line.substringAfter(":").trim().split(Regex("\\s+"))
                if (tokens.size >= 4) {
                    val enabled = !tokens[0].equals("OFF", ignoreCase = true)
                    val kind = when (tokens[1].uppercase()) {
                        "LS", "LSC" -> BiquadFilterKind.LOW_SHELF
                        "HS", "HSC" -> BiquadFilterKind.HIGH_SHELF
                        else -> BiquadFilterKind.PEAKING
                    }
                    var fc = 1000.0
                    var gain = 0.0
                    var q = 1.41
                    var i = 2
                    while (i < tokens.size) {
                        when {
                            tokens[i].equals("Fc", true) && i + 1 < tokens.size -> {
                                fc = tokens[i + 1].toDoubleOrNull() ?: fc
                                i += 2
                            }
                            tokens[i].equals("Gain", true) && i + 1 < tokens.size -> {
                                gain = tokens[i + 1].toDoubleOrNull() ?: gain
                                i += 2
                            }
                            tokens[i].equals("Q", true) && i + 1 < tokens.size -> {
                                q = tokens[i + 1].toDoubleOrNull() ?: q
                                i += 2
                            }
                            else -> i++
                        }
                    }
                    parsed.add(ParametricBand(fc, gain, q, kind, enabled))
                    sawDirective = true
                }
            }
        }
        require(sawDirective) { "No valid EqualizerAPO directives found" }
        val updatedBands = base.bands.mapIndexed { idx, slot ->
            val match = parsed.find { abs(it.freqHz - slot.freqHz) < slot.freqHz * 0.18 }
                ?: parsed.getOrNull(idx)
            match ?: slot
        }
        return base.copy(
            id = "imported_apo",
            name = "Imported EqualizerAPO",
            preampDb = preamp,
            bands = updatedBands,
        )
    }
}
