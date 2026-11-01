package com.jhems.soundbettrcore

import kotlin.math.roundToInt

enum class AmbientSoundMode(val id: Int, val label: String) {
    NOISE_CANCELING(0, "ANC On"),
    TRANSPARENCY(1, "Transparency"),
    NORMAL(2, "Normal");

    companion object {
        fun fromId(id: Int): AmbientSoundMode = entries.find { it.id == id } ?: NORMAL
    }
}

enum class AncMode(val id: Int, val label: String) {
    MANUAL(0, "Manual 1-5"),
    ADAPTIVE(1, "Adaptive"),
    MULTI_SCENE(2, "Multi-Scene");

    companion object {
        fun fromId(id: Int): AncMode = entries.find { it.id == id } ?: MULTI_SCENE
    }
}

enum class MultiSceneAnc(val id: Int, val label: String) {
    TRANSPORT(0, "Transport"),
    OUTDOOR(1, "Outdoor"),
    INDOOR(2, "Indoor");

    companion object {
        fun fromId(id: Int): MultiSceneAnc = entries.find { it.id == id } ?: INDOOR
    }
}

data class A3959SoundModes(
    val ambientMode: AmbientSoundMode = AmbientSoundMode.NOISE_CANCELING,
    val manualAncLevel: Int = 5,
    val adaptiveAncLevel: Int = 3,
    val ancMode: AncMode = AncMode.ADAPTIVE,
    val windNoiseSuppression: Boolean = true,
    val adaptiveSensitivity: Int = 5,
    val multiSceneAnc: MultiSceneAnc = MultiSceneAnc.TRANSPORT,
)

enum class EarbudSide(val id: Int) {
    LEFT(0x00),
    RIGHT(0x01),
}

enum class ButtonPressKind(val id: Int) {
    DOUBLE(0x00),
    LONG(0x01),
    SINGLE(0x02),
    TRIPLE(0x05),
}

enum class ButtonAction(val nibble: Int?, val label: String) {
    VOLUME_UP(0x00, "Volume Up"),
    VOLUME_DOWN(0x01, "Volume Down"),
    PREVIOUS_SONG(0x02, "Previous Track"),
    NEXT_SONG(0x03, "Next Track"),
    AMBIENT_SOUND_MODE(0x04, "Cycle ANC / Transparency"),
    VOICE_ASSISTANT(0x05, "Voice Assistant"),
    PLAY_PAUSE(0x06, "Play / Pause"),
    GAME_MODE(0x09, "Toggle Game Mode"),
    DISABLED(null, "Disabled");

    companion object {
        fun fromRawByte(raw: Int, twsConnected: Boolean, isSingleOrTriple: Boolean): ButtonAction {
            val u = raw and 0xFF
            if (u == 0xFF) return DISABLED
            if (isSingleOrTriple && !twsConnected) return DISABLED
            return entries.find { it.nibble == (u and 0x0F) } ?: DISABLED
        }
    }
}

data class A3959ButtonConfig(
    val leftSingle: ButtonAction = ButtonAction.VOLUME_DOWN,
    val rightSingle: ButtonAction = ButtonAction.VOLUME_UP,
    val leftDouble: ButtonAction = ButtonAction.PLAY_PAUSE,
    val rightDouble: ButtonAction = ButtonAction.NEXT_SONG,
    val leftTriple: ButtonAction = ButtonAction.VOICE_ASSISTANT,
    val rightTriple: ButtonAction = ButtonAction.GAME_MODE,
    val leftLong: ButtonAction = ButtonAction.AMBIENT_SOUND_MODE,
    val rightLong: ButtonAction = ButtonAction.AMBIENT_SOUND_MODE,
)

data class A3959State(
    val connected: Boolean = false,
    val simulated: Boolean = true,
    val deviceName: String = "Soundcore R50i NC (A3959)",
    val macAddress: String = "AC:12:2F:39:59:A0",
    val hostDevice: Int = 1,
    val twsConnected: Boolean = true,
    val leftBatteryLevel: Int = 9,
    val rightBatteryLevel: Int = 9,
    val leftBatteryPercent: Int = 90,
    val rightBatteryPercent: Int = 90,
    val leftCharging: Boolean = false,
    val rightCharging: Boolean = false,
    val firmwareLeft: String = "01.64",
    val firmwareRight: String = "01.64",
    val serialNumber: String = "3959685923BF067C",
    val eqPresetId: Int = A3959Protocol.CUSTOM_EQ_PRESET_ID,
    val eqGainsTenthsDb: IntArray = intArrayOf(-25, -18, 5, 18, 32, 24, 10, 15),
    val drcPreviewTenthsDb: IntArray = intArrayOf(-2, -2, 1, 2, 4, 3, 1, 2),
    val bassUpEnabled: Boolean = false,
    val buttons: A3959ButtonConfig = A3959ButtonConfig(),
    val ambientCycleMask: Int = 0x07,
    val soundModes: A3959SoundModes = A3959SoundModes(),
    val touchTone: Boolean = true,
    val dualConnection: Boolean = true,
    val onboardSurround: Boolean = false,
    val autoPowerOffEnabled: Boolean = true,
    val autoPowerOffIndex: Int = 1,
    val lowBatteryPrompt: Boolean = true,
    val gamingMode: Boolean? = false,
)

object A3959Protocol {
    val DIRECTION_HOST_TO_DEVICE = byteArrayOf(0x08, 0xEE.toByte(), 0x00, 0x00, 0x00)
    val DIRECTION_DEVICE_TO_HOST = byteArrayOf(0x09, 0xFF.toByte(), 0x00, 0x00, 0x01)

    val CMD_STATE_REQUEST = byteArrayOf(0x01, 0x01)
    val CMD_SET_TOUCH_TONE = byteArrayOf(0x01, 0x83.toByte())
    val CMD_SET_GAMING_MODE = byteArrayOf(0x01, 0x87.toByte())
    val CMD_SET_EQUALIZER_DRC = byteArrayOf(0x02, 0x83.toByte())
    val CMD_SET_SURROUND_SOUND = byteArrayOf(0x02, 0x86.toByte())
    val CMD_SET_BUTTON_ACTION = byteArrayOf(0x04, 0x81.toByte())
    val CMD_SET_SOUND_MODES = byteArrayOf(0x06, 0x81.toByte())

    const val CUSTOM_EQ_PRESET_ID = 0xFEFE
    const val BASS_UP_PRESET_ID = 0x0002
    val HW_BAND_FREQS = intArrayOf(100, 200, 400, 800, 1600, 3200, 6400, 12800)

    fun calculateChecksum(bytesWithoutChecksum: ByteArray, length: Int = bytesWithoutChecksum.size): Byte {
        var sum = 0
        for (i in 0 until length) {
            sum = (sum + (bytesWithoutChecksum[i].toInt() and 0xFF)) and 0xFF
        }
        return sum.toByte()
    }

    fun encodeHostPacket(command: ByteArray, body: ByteArray): ByteArray {
        val totalLen = 5 + 2 + 2 + body.size + 1
        val pkt = ByteArray(totalLen)
        System.arraycopy(DIRECTION_HOST_TO_DEVICE, 0, pkt, 0, 5)
        pkt[5] = command[0]
        pkt[6] = command[1]
        pkt[7] = (totalLen and 0xFF).toByte()
        pkt[8] = ((totalLen ushr 8) and 0xFF).toByte()
        System.arraycopy(body, 0, pkt, 9, body.size)
        pkt[totalLen - 1] = calculateChecksum(pkt, totalLen - 1)
        return pkt
    }

    fun buildStateRequestPacket(): ByteArray = encodeHostPacket(CMD_STATE_REQUEST, ByteArray(0))

    fun buildSoundModesPacket(modes: A3959SoundModes): ByteArray {
        val manual = modes.manualAncLevel.coerceIn(1, 5)
        val adaptive = modes.adaptiveAncLevel.coerceIn(0, 5)
        val combined = ((adaptive shl 4) or (manual and 0x0F)).toByte()
        val body = byteArrayOf(
            modes.ambientMode.id.toByte(),
            combined,
            modes.ambientMode.id.toByte(),
            modes.ancMode.id.toByte(),
            (if (modes.windNoiseSuppression) 1 else 0).toByte(),
            modes.adaptiveSensitivity.toByte(),
            modes.multiSceneAnc.id.toByte(),
        )
        return encodeHostPacket(CMD_SET_SOUND_MODES, body)
    }

    fun buildSurroundSoundPacket(enabled: Boolean): ByteArray =
        encodeHostPacket(CMD_SET_SURROUND_SOUND, byteArrayOf((if (enabled) 1 else 0).toByte()))

    fun buildGamingModePacket(enabled: Boolean): ByteArray =
        encodeHostPacket(CMD_SET_GAMING_MODE, byteArrayOf((if (enabled) 1 else 0).toByte()))

    fun buildTouchTonePacket(enabled: Boolean): ByteArray =
        encodeHostPacket(CMD_SET_TOUCH_TONE, byteArrayOf((if (enabled) 1 else 0).toByte()))

    fun buildButtonActionPacket(
        side: EarbudSide,
        press: ButtonPressKind,
        action: ButtonAction,
    ): ByteArray {
        val actionByte = action.nibble?.let { 0xF0 or (it and 0x0F) } ?: 0xFF
        val body = byteArrayOf(side.id.toByte(), press.id.toByte(), actionByte.toByte())
        return encodeHostPacket(CMD_SET_BUTTON_ACTION, body)
    }

    fun computeA3959DrcBytes(gainsTenthsDb: IntArray): ByteArray {
        val a = DoubleArray(8) { i -> gainsTenthsDb.getOrElse(i) { 0 }.coerceIn(-120, 120).toDouble() }
        val s = 0.85
        val l = 0.95
        val matrix = arrayOf(
            doubleArrayOf(1.26, -0.71 * s, 0.177, -0.0494, 0.0345, -0.0197, 0.0075, -0.00217),
            doubleArrayOf(-0.71 * s, 1.73 * l, -0.81 * s, 0.204, -0.068, 0.045, -0.0235, 0.0075),
            doubleArrayOf(0.177, -0.81 * s, 1.73 * l, -0.81 * s, 0.208, -0.07, 0.045, -0.0197),
            doubleArrayOf(-0.0494, 0.204, -0.81 * s, 1.73 * l, -0.82 * s, 0.208, -0.068, 0.0345),
            doubleArrayOf(0.0345, -0.068, 0.208, -0.82 * s, 1.73 * l, -0.81 * s, 0.204, -0.0494),
            doubleArrayOf(-0.0197, 0.045, -0.07, 0.208, -0.81 * s, 1.73 * l, -0.81 * s, 0.177),
            doubleArrayOf(0.0075, -0.0235, 0.045, -0.068, 0.204, -0.81 * s, 1.83 * l, -0.71 * s),
            doubleArrayOf(-0.00217, 0.0075, -0.0197, 0.0345, -0.0494, 0.177, -0.71 * s, 1.5),
        )
        return ByteArray(8) { row ->
            var sum = 0.0
            for (col in 0 until 8) {
                sum += matrix[row][col] * a[col]
            }
            val drcTenths = (sum / 10.0).roundToInt()
            (120 + drcTenths).coerceIn(0, 240).toByte()
        }
    }

    fun buildEqualizerWithDrcPacket(presetId: Int, gainsTenthsDb: IntArray): ByteArray {
        val body = ByteArray(18)
        body[0] = (presetId and 0xFF).toByte()
        body[1] = ((presetId ushr 8) and 0xFF).toByte()
        for (i in 0 until 8) {
            val g = gainsTenthsDb.getOrElse(i) { 0 }.coerceIn(-120, 120)
            body[2 + i] = (120 + g).toByte()
        }
        val drc = computeA3959DrcBytes(gainsTenthsDb)
        System.arraycopy(drc, 0, body, 10, 8)
        return encodeHostPacket(CMD_SET_EQUALIZER_DRC, body)
    }

    fun parseStatePacket(packet: ByteArray): A3959State {
        require(packet.size >= 10) { "Packet too short: ${packet.size}" }
        for (i in 0 until 5) {
            require(packet[i] == DIRECTION_DEVICE_TO_HOST[i]) { "Invalid direction header" }
        }
        val declaredLen = (packet[7].toInt() and 0xFF) or ((packet[8].toInt() and 0xFF) shl 8)
        require(declaredLen >= 10 && packet.size >= declaredLen) {
            "Declared length $declaredLen exceeds packet size ${packet.size}"
        }
        val expectedChecksum = calculateChecksum(packet, declaredLen - 1)
        val actualChecksum = packet[declaredLen - 1]
        require(expectedChecksum == actualChecksum) {
            "Checksum mismatch: expected $expectedChecksum, got $actualChecksum"
        }

        val bodyLen = declaredLen - 10
        require(bodyLen >= 79) { "State body too short: $bodyLen" }
        val body = packet.copyOfRange(9, 9 + bodyLen)

        val hostDevice = body[0].toInt() and 0xFF
        val twsConnected = (body[1].toInt() and 0xFF) != 0
        val leftBat = (body[2].toInt() and 0xFF).coerceAtMost(10)
        val rightBat = (body[3].toInt() and 0xFF).coerceAtMost(10)
        val leftCharging = (body[4].toInt() and 0xFF) == 1
        val rightCharging = (body[5].toInt() and 0xFF) == 1

        val fwLeft = String(body, 6, 5, Charsets.US_ASCII).trim()
        val fwRight = String(body, 11, 5, Charsets.US_ASCII).trim()
        val serial = String(body, 16, 16, Charsets.US_ASCII).trim()

        val eqPresetId = (body[32].toInt() and 0xFF) or ((body[33].toInt() and 0xFF) shl 8)
        val eqGains = IntArray(8) { i -> (body[34 + i].toInt() and 0xFF) - 120 }
        val drcBytes = computeA3959DrcBytes(eqGains)
        val drcPreview = IntArray(8) { i -> (drcBytes[i].toInt() and 0xFF) - 120 }

        val buttons = A3959ButtonConfig(
            leftSingle = ButtonAction.fromRawByte(body[56].toInt(), twsConnected, true),
            rightSingle = ButtonAction.fromRawByte(body[57].toInt(), twsConnected, true),
            leftDouble = ButtonAction.fromRawByte(body[58].toInt(), twsConnected, false),
            rightDouble = ButtonAction.fromRawByte(body[59].toInt(), twsConnected, false),
            leftTriple = ButtonAction.fromRawByte(body[60].toInt(), twsConnected, true),
            rightTriple = ButtonAction.fromRawByte(body[61].toInt(), twsConnected, true),
            leftLong = ButtonAction.fromRawByte(body[62].toInt(), twsConnected, false),
            rightLong = ButtonAction.fromRawByte(body[63].toInt(), twsConnected, false),
        )

        val cycleMask = body[64].toInt() and 0xFF
        val ambientMode = AmbientSoundMode.fromId(body[65].toInt() and 0xFF)
        val combinedLevel = body[66].toInt() and 0xFF
        val adaptiveLevel = (combinedLevel ushr 4) and 0x0F
        val manualLevel = (combinedLevel and 0x0F).coerceIn(1, 5)
        val ancMode = AncMode.fromId(body[68].toInt() and 0xFF)
        val windNoise = (body[69].toInt() and 0xFF) != 0
        val adaptiveSens = body[70].toInt() and 0xFF
        val multiScene = MultiSceneAnc.fromId(body[71].toInt() and 0xFF)

        val touchTone = (body[73].toInt() and 0xFF) != 0
        val dualConn = (body[74].toInt() and 0xFF) != 0
        val surround = (body[75].toInt() and 0xFF) != 0
        val autoOffEnabled = (body[76].toInt() and 0xFF) != 0
        val autoOffIdx = body[77].toInt() and 0xFF
        val lowBatPrompt = (body[78].toInt() and 0xFF) != 0

        val fwNumL = fwLeft.filter { it.isDigit() }.toIntOrNull() ?: 0
        val fwNumR = fwRight.filter { it.isDigit() }.toIntOrNull() ?: 0
        val gamingMode = if ((fwNumL >= 160 || fwNumR >= 160) && body.size > 79) {
            (body[79].toInt() and 0xFF) != 0
        } else {
            null
        }

        return A3959State(
            connected = true,
            simulated = false,
            hostDevice = hostDevice,
            twsConnected = twsConnected,
            leftBatteryLevel = leftBat,
            rightBatteryLevel = rightBat,
            leftBatteryPercent = leftBat * 10,
            rightBatteryPercent = rightBat * 10,
            leftCharging = leftCharging,
            rightCharging = rightCharging,
            firmwareLeft = fwLeft,
            firmwareRight = fwRight,
            serialNumber = serial,
            eqPresetId = eqPresetId,
            eqGainsTenthsDb = eqGains,
            drcPreviewTenthsDb = drcPreview,
            bassUpEnabled = eqPresetId == BASS_UP_PRESET_ID,
            buttons = buttons,
            ambientCycleMask = cycleMask,
            soundModes = A3959SoundModes(
                ambientMode = ambientMode,
                manualAncLevel = manualLevel,
                adaptiveAncLevel = adaptiveLevel,
                ancMode = ancMode,
                windNoiseSuppression = windNoise,
                adaptiveSensitivity = adaptiveSens,
                multiSceneAnc = multiScene,
            ),
            touchTone = touchTone,
            dualConnection = dualConn,
            onboardSurround = surround,
            autoPowerOffEnabled = autoOffEnabled,
            autoPowerOffIndex = autoOffIdx,
            lowBatteryPrompt = lowBatPrompt,
            gamingMode = gamingMode,
        )
    }

    fun formatHex(bytes: ByteArray): String =
        bytes.joinToString(" ") { "%02X".format(it.toInt() and 0xFF) }
}
