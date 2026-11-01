package com.jhems.soundbettrcore

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.sin

class A3959AndDspTest {

    @Test
    fun openscq30EqualizerWithDrcPacketMatchesExactVector() {
        val expected = byteArrayOf(
            0x08, 0xee.toByte(), 0x00, 0x00, 0x00, 0x02, 0x83.toByte(), 0x1c, 0x00, 0xfe.toByte(), 0xfe.toByte(),
            0x3c, 0xb4.toByte(), 0x8f.toByte(), 0xf0.toByte(), 0x8e.toByte(), 0x00, 0x74, 0x88.toByte(),
            0x6d, 0x86.toByte(), 0x70, 0x88.toByte(), 0x7b, 0x66, 0x7e, 0x79, 0x4f
        )
        val gains = intArrayOf(-60, 60, 23, 120, 22, -120, -4, 16)
        val pkt = A3959Protocol.buildEqualizerWithDrcPacket(0xFEFE, gains)
        assertArrayEquals(expected, pkt)
    }

    @Test
    fun parsesIssue149A3959StateUpdatePacket() {
        val raw = intArrayOf(
            0x09, 0xff, 0x00, 0x00, 0x01, 0x01, 0x01, 0x65, 0x00, 0x01, 0x00, 0x06, 0x09, 0xff,
            0xff, 0x30, 0x31, 0x2e, 0x36, 0x34, 0x30, 0x31, 0x2e, 0x36, 0x34, 0x33, 0x39, 0x35,
            0x39, 0x36, 0x38, 0x35, 0x39, 0x32, 0x33, 0x42, 0x46, 0x30, 0x36, 0x37, 0x43, 0xfe,
            0xfe, 0x6a, 0x74, 0x75, 0x77, 0x79, 0x78, 0x77, 0x74, 0x78, 0x78, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8a, 0xf1, 0xf0, 0x66, 0x63, 0x15,
            0x19, 0x64, 0x64, 0x07, 0x00, 0x55, 0x00, 0x01, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x14
        ).map { it.toByte() }.toByteArray()

        val state = A3959Protocol.parseStatePacket(raw)
        assertEquals(6, state.leftBatteryLevel)
        assertEquals(9, state.rightBatteryLevel)
        assertEquals(60, state.leftBatteryPercent)
        assertEquals(90, state.rightBatteryPercent)
        assertFalse(state.leftCharging)
        assertEquals("01.64", state.firmwareLeft)
        assertEquals("3959685923BF067C", state.serialNumber)
        assertArrayEquals(intArrayOf(-14, -4, -3, -1, 1, 0, -1, -4), state.eqGainsTenthsDb)
        assertEquals(AmbientSoundMode.NOISE_CANCELING, state.soundModes.ambientMode)
        assertEquals(AncMode.ADAPTIVE, state.soundModes.ancMode)
        assertEquals(false, state.gamingMode)
    }

    @Test
    fun harmanTargetCuts125HzMudAndLimiterPreventsClipping() {
        val defaultProfile = DspProfile()
        val pipeline = DspPipeline(48000.0, defaultProfile)
        val curve = pipeline.computeResponseCurve(128)
        val mud125 = curve.minByOrNull { abs(it.first - 125.0) }!!.second
        val clarity3500 = curve.minByOrNull { abs(it.first - 3500.0) }!!.second
        assertTrue("125 Hz should be cut < -3 dB", mud125 < -3.0)
        assertTrue("3.5 kHz should be boosted > +2 dB", clarity3500 > 2.0)

        val extreme = defaultProfile.copy(
            autoHeadroom = false,
            preampDb = 12.0,
            subBassExciter = 1.0,
            vocalClarity = 1.0,
            surroundWidth = 1.0,
            bands = defaultProfile.bands.map { it.copy(gainDb = 12.0) },
        )
        pipeline.updateProfile(extreme)
        var maxPeak = 0f
        for (i in 0 until 4800) {
            val t = i / 48000.0
            val s = (sin(2.0 * PI * 80.0 * t) * 0.95).toFloat()
            val (l, r) = pipeline.processFrame(s, s)
            maxPeak = maxOf(maxPeak, abs(l), abs(r))
        }
        assertTrue("Brickwall limiter must keep peak <= 0.967 (-0.3 dBFS), got $maxPeak", maxPeak <= 0.967f)
        assertTrue("Signal must remain audible", maxPeak > 0.4f)
    }

    @Test
    fun equalizerApoTxtRoundTripPreservesPreampAndBands() {
        val original = DspProfile(preampDb = -3.5)
        val txt = EqualizerApoCodec.renderEqualizerApoTxt(original)
        val parsed = EqualizerApoCodec.parseEqualizerApoTxt(txt)
        assertEquals(-3.5, parsed.preampDb, 1e-4)
        assertEquals(-4.5, parsed.bands[2].gainDb, 1e-4)
    }
}
