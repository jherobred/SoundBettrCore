package com.jhems.soundbettrcore

import android.Manifest
import android.content.Intent
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlin.math.exp
import kotlin.math.ln
import kotlin.math.pow
import kotlin.math.roundToInt

private val GroundColor = Color(0xFF171719)
private val SurfaceColor = Color(0xFF131217)
private val RaisedColor = Color(0xFF212023)
private val BorderColor = Color(0xFF222224)
private val TextColor = Color(0xFFF2F4F8)
private val MutedColor = Color(0xFF8A8C92)
private val AccentColor = Color(0xFF4A8AD6)
private val PositiveColor = Color(0xFF5FC39C)
private val WarningColor = Color(0xFFD9A441)
private val NegativeColor = Color(0xFFE5566F)

private val DataTerminalScheme = darkColorScheme(
    primary = AccentColor,
    background = GroundColor,
    surface = SurfaceColor,
    onPrimary = TextColor,
    onBackground = TextColor,
    onSurface = TextColor,
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        try {
            val svcIntent = Intent(this, SoundBettrService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                startForegroundService(svcIntent)
            } else {
                startService(svcIntent)
            }
        } catch (_: Throwable) {
        }

        setContent {
            MaterialTheme(colorScheme = DataTerminalScheme) {
                Surface(modifier = Modifier.fillMaxSize(), color = GroundColor) {
                    SoundBettrMobileStudio()
                }
            }
        }
    }
}

@Composable
fun SoundBettrMobileStudio() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val profile by StudioRepository.profile.collectAsState()
    val r50i by StudioRepository.r50i.collectAsState()
    val lastPacketHex by StudioRepository.lastPacketHex.collectAsState()
    val playingTest by StudioRepository.playingTest.collectAsState()
    val presets = remember { DspProfile.builtinPresets() }
    var activeTab by remember { mutableIntStateOf(0) }

    val permLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) {
        scope.launch(Dispatchers.IO) {
            val updated = R50iBluetoothManager.queryOrFallbackState(context, r50i)
            StudioRepository.updateR50i(updated)
        }
    }

    LaunchedEffect(Unit) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            permLauncher.launch(
                arrayOf(
                    Manifest.permission.BLUETOOTH_CONNECT,
                    Manifest.permission.BLUETOOTH_SCAN,
                )
            )
        }
    }

    Column(modifier = Modifier.fillMaxSize().background(GroundColor)) {
        // Top Header
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(SurfaceColor)
                .padding(horizontal = 16.dp, vertical = 12.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column {
                    Text(
                        text = "SoundBettrCore",
                        color = TextColor,
                        fontSize = 20.sp,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Text(
                        text = "Soundcore R50i NC (A3959) Spatial & Harman DSP",
                        color = MutedColor,
                        fontSize = 12.sp,
                    )
                }
                ActionPillButton(
                    label = if (profile.bypass) "DSP Bypassed" else "A/B Active",
                    active = !profile.bypass,
                    danger = profile.bypass,
                    onClick = {
                        StudioRepository.updateProfile(profile.copy(bypass = !profile.bypass))
                    },
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                val connText = if (r50i.connected) "RFCOMM Live" else "A3959 Ready"
                Text(
                    text = "R50i NC ($connText): L ${r50i.leftBatteryPercent}%, R ${r50i.rightBatteryPercent}%",
                    color = if (r50i.connected) PositiveColor else WarningColor,
                    fontSize = 12.sp,
                    fontWeight = FontWeight.Medium,
                )
                ActionPillButton(
                    label = "Sync R50i NC",
                    active = false,
                    onClick = {
                        scope.launch(Dispatchers.IO) {
                            val next = R50iBluetoothManager.queryOrFallbackState(context, r50i)
                            StudioRepository.updateR50i(next)
                        }
                    },
                )
            }

            Spacer(modifier = Modifier.height(10.dp))

            // 1px Accent Line Navigation Tabs
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(20.dp),
            ) {
                listOf("Simple Studio", "Pro EQ Rack", "R50i NC Hardware").forEachIndexed { idx, title ->
                    val selected = activeTab == idx
                    Box(
                        modifier = Modifier
                            .height(48.dp)
                            .clickable { activeTab = idx }
                            .padding(horizontal = 4.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Text(
                                text = title,
                                color = if (selected) TextColor else MutedColor,
                                fontSize = 13.sp,
                                fontWeight = FontWeight.Medium,
                            )
                            Spacer(modifier = Modifier.height(6.dp))
                            Box(
                                modifier = Modifier
                                    .width(72.dp)
                                    .height(1.dp)
                                    .background(if (selected) AccentColor else Color.Transparent)
                            )
                        }
                    }
                }
            }
        }

        // Scrollable Workspace
        Column(
            modifier = Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            when (activeTab) {
                0 -> SimpleStudioTab(
                    profile = profile,
                    presets = presets,
                    r50i = r50i,
                    onSelectPreset = { p ->
                        StudioRepository.updateProfile(p)
                        scope.launch(Dispatchers.IO) {
                            val pkt = A3959Protocol.buildEqualizerWithDrcPacket(
                                A3959Protocol.CUSTOM_EQ_PRESET_ID,
                                p.hwEqTenthsDb,
                            )
                            R50iBluetoothManager.exchangePacket(context, pkt, false)
                            StudioRepository.updateR50i(
                                r50i.copy(eqGainsTenthsDb = p.hwEqTenthsDb),
                                pkt,
                            )
                        }
                    },
                    onUpdateModes = { nextModes ->
                        val pkt = A3959Protocol.buildSoundModesPacket(nextModes)
                        StudioRepository.updateR50i(r50i.copy(soundModes = nextModes), pkt)
                        scope.launch(Dispatchers.IO) {
                            R50iBluetoothManager.exchangePacket(context, pkt, false)
                        }
                    },
                )
                1 -> ProRackTab(
                    profile = profile,
                    r50i = r50i,
                    onFlashHwEq = { gains, bassUp ->
                        val presetId = if (bassUp) A3959Protocol.BASS_UP_PRESET_ID else A3959Protocol.CUSTOM_EQ_PRESET_ID
                        val pkt = A3959Protocol.buildEqualizerWithDrcPacket(presetId, gains)
                        val drc = A3959Protocol.computeA3959DrcBytes(gains)
                        val drcTenths = IntArray(8) { i -> (drc[i].toInt() and 0xFF) - 120 }
                        StudioRepository.updateR50i(
                            r50i.copy(
                                eqPresetId = presetId,
                                eqGainsTenthsDb = gains,
                                drcPreviewTenthsDb = drcTenths,
                                bassUpEnabled = bassUp,
                            ),
                            pkt,
                        )
                        scope.launch(Dispatchers.IO) {
                            R50iBluetoothManager.exchangePacket(context, pkt, false)
                        }
                    },
                )
                else -> HardwareControlTab(
                    r50i = r50i,
                    lastPacketHex = lastPacketHex,
                    onToggleSurround = { enabled ->
                        val pkt = A3959Protocol.buildSurroundSoundPacket(enabled)
                        StudioRepository.updateR50i(r50i.copy(onboardSurround = enabled), pkt)
                        scope.launch(Dispatchers.IO) {
                            R50iBluetoothManager.exchangePacket(context, pkt, false)
                        }
                    },
                    onToggleGaming = { enabled ->
                        val pkt = A3959Protocol.buildGamingModePacket(enabled)
                        StudioRepository.updateR50i(r50i.copy(gamingMode = enabled), pkt)
                        scope.launch(Dispatchers.IO) {
                            R50iBluetoothManager.exchangePacket(context, pkt, false)
                        }
                    },
                    onCycleButtonAction = { side, press, nextAction ->
                        val pkt = A3959Protocol.buildButtonActionPacket(side, press, nextAction)
                        val b = r50i.buttons
                        val nextBtns = when (side to press) {
                            EarbudSide.LEFT to ButtonPressKind.SINGLE -> b.copy(leftSingle = nextAction)
                            EarbudSide.RIGHT to ButtonPressKind.SINGLE -> b.copy(rightSingle = nextAction)
                            EarbudSide.LEFT to ButtonPressKind.DOUBLE -> b.copy(leftDouble = nextAction)
                            EarbudSide.RIGHT to ButtonPressKind.DOUBLE -> b.copy(rightDouble = nextAction)
                            EarbudSide.LEFT to ButtonPressKind.TRIPLE -> b.copy(leftTriple = nextAction)
                            EarbudSide.RIGHT to ButtonPressKind.TRIPLE -> b.copy(rightTriple = nextAction)
                            EarbudSide.LEFT to ButtonPressKind.LONG -> b.copy(leftLong = nextAction)
                            else -> b.copy(rightLong = nextAction)
                        }
                        StudioRepository.updateR50i(r50i.copy(buttons = nextBtns), pkt)
                        scope.launch(Dispatchers.IO) {
                            R50iBluetoothManager.exchangePacket(context, pkt, false)
                        }
                    },
                )
            }
        }

        // Bottom Spatial Test Player Bar
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(SurfaceColor)
                .border(1.dp, BorderColor)
                .padding(horizontal = 16.dp, vertical = 10.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = playingTest?.let { "Playing: ${it.label}" } ?: "Spatial & Sub-Bass Audition Player",
                    color = TextColor,
                    fontSize = 13.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                if (playingTest != null) {
                    ActionPillButton(
                        label = "Stop Audio",
                        active = false,
                        danger = true,
                        onClick = { StudioRepository.setPlayingTest(null) },
                    )
                }
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                TestSignalKind.entries.forEach { kind ->
                    val isPlaying = playingTest == kind
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .height(48.dp)
                            .background(RaisedColor, RoundedCornerShape(6.dp))
                            .border(
                                width = 1.dp,
                                color = if (isPlaying) AccentColor else BorderColor,
                                shape = RoundedCornerShape(6.dp),
                            )
                            .clickable {
                                StudioRepository.setPlayingTest(if (isPlaying) null else kind)
                            }
                            .padding(horizontal = 6.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = when (kind) {
                                TestSignalKind.ORBIT_71 -> "7.1 Orbit"
                                TestSignalKind.SUB_BASS_SWEEP -> "Sub Sweep"
                                TestSignalKind.VOCAL_PULSE -> "Vocal"
                                TestSignalKind.PINK_NOISE -> "Pink Noise"
                            },
                            color = if (isPlaying) TextColor else MutedColor,
                            fontSize = 11.sp,
                            fontWeight = FontWeight.Medium,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun SimpleStudioTab(
    profile: DspProfile,
    presets: List<DspProfile>,
    r50i: A3959State,
    onSelectPreset: (DspProfile) -> Unit,
    onUpdateModes: (A3959SoundModes) -> Unit,
) {
    StudioCard(title = "Acoustic & Spatial Presets", meta = "Active: ${profile.name}") {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            presets.forEach { p ->
                val selected = p.id == profile.id
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .background(GroundColor, RoundedCornerShape(6.dp))
                        .border(
                            width = 1.dp,
                            color = if (selected) AccentColor else BorderColor,
                            shape = RoundedCornerShape(6.dp),
                        )
                        .clickable { onSelectPreset(p) }
                        .padding(12.dp)
                ) {
                    Text(text = p.name, color = TextColor, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
                    Spacer(modifier = Modifier.height(2.dp))
                    Text(text = p.subtitle, color = MutedColor, fontSize = 12.sp)
                }
            }
        }
    }

    StudioCard(title = "Harman Target & Response Curve", meta = "Ceiling -0.3 dBFS") {
        Canvas(
            modifier = Modifier
                .fillMaxWidth()
                .height(140.dp)
                .background(GroundColor, RoundedCornerShape(6.dp))
                .border(1.dp, BorderColor, RoundedCornerShape(6.dp))
        ) {
            val w = size.width
            val h = size.height
            drawLine(BorderColor, Offset(0f, h * 0.5f), Offset(w, h * 0.5f), strokeWidth = 1f)

            val path = Path()
            val steps = 72
            for (i in 0..steps) {
                val norm = i.toFloat() / steps.toFloat()
                val freq = 20.0 * 1000.0.pow(norm.toDouble())
                var db = if (profile.bypass) 0.0 else profile.preampDb
                if (!profile.bypass) {
                    for (b in profile.bands) {
                        val oct = ln(freq / b.freqHz) / ln(2.0)
                        db += b.gainDb * exp(-(oct * oct) * (b.q * 1.4))
                    }
                }
                val x = norm * w
                val y = (h * 0.5f - (db.toFloat() / 15f) * (h * 0.42f)).coerceIn(4f, h - 4f)
                if (i == 0) path.moveTo(x, y) else path.lineTo(x, y)
            }
            drawPath(
                path = path,
                color = if (profile.bypass) MutedColor else AccentColor,
                style = Stroke(width = 3f),
            )
        }
    }

    StudioCard(title = "Macro Audio Enhancers", meta = "64-Bit Float DSP") {
        MacroSliderRow(
            label = "3D 7.1 Binaural Surround",
            value = profile.surroundWidth.toFloat(),
            onValueChange = { StudioRepository.updateProfile(profile.copy(surroundWidth = it.toDouble())) },
        )
        MacroSliderRow(
            label = "Harmonic Sub-Bass Exciter",
            value = profile.subBassExciter.toFloat(),
            onValueChange = { StudioRepository.updateProfile(profile.copy(subBassExciter = it.toDouble())) },
        )
        MacroSliderRow(
            label = "Vocal & Dialog Clarity",
            value = profile.vocalClarity.toFloat(),
            onValueChange = { StudioRepository.updateProfile(profile.copy(vocalClarity = it.toDouble())) },
        )
        MacroSliderRow(
            label = "Room Space & Hall Reverb",
            value = profile.roomSpace.toFloat(),
            onValueChange = { StudioRepository.updateProfile(profile.copy(roomSpace = it.toDouble())) },
        )
        MacroSliderRow(
            label = "Dolby 3-Band Loudness Leveler",
            value = profile.dolbyLevelerAmount.toFloat(),
            onValueChange = { StudioRepository.updateProfile(profile.copy(dolbyLevelerAmount = it.toDouble())) },
        )
    }

    StudioCard(title = "R50i NC Ambient & ANC Control (06 81)", meta = "FW ${r50i.firmwareLeft}") {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            AmbientSoundMode.entries.forEach { mode ->
                ActionPillButton(
                    label = mode.label,
                    active = r50i.soundModes.ambientMode == mode,
                    onClick = { onUpdateModes(r50i.soundModes.copy(ambientMode = mode)) },
                )
            }
        }
        Spacer(modifier = Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            AncMode.entries.forEach { mode ->
                ActionPillButton(
                    label = mode.label,
                    active = r50i.soundModes.ancMode == mode,
                    onClick = { onUpdateModes(r50i.soundModes.copy(ancMode = mode)) },
                )
            }
        }
        Spacer(modifier = Modifier.height(8.dp))
        MacroSliderRow(
            label = "Manual ANC Depth (Level ${r50i.soundModes.manualAncLevel}/5)",
            value = (r50i.soundModes.manualAncLevel - 1) / 4f,
            onValueChange = {
                val lvl = (1 + (it * 4f).roundToInt()).coerceIn(1, 5)
                onUpdateModes(r50i.soundModes.copy(manualAncLevel = lvl))
            },
        )
    }
}

@Composable
private fun ProRackTab(
    profile: DspProfile,
    r50i: A3959State,
    onFlashHwEq: (IntArray, Boolean) -> Unit,
) {
    StudioCard(title = "10-Band Host Parametric EQ", meta = "Preamp ${"%+.1f".format(profile.preampDb)} dB") {
        profile.bands.forEachIndexed { idx, band ->
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(
                    text = if (band.freqHz >= 1000) "${(band.freqHz / 1000).toInt()}k" else "${band.freqHz.toInt()}",
                    color = MutedColor,
                    fontSize = 12.sp,
                    modifier = Modifier.width(42.dp),
                )
                Slider(
                    value = band.gainDb.toFloat(),
                    onValueChange = { nextG ->
                        val nextBands = profile.bands.toMutableList()
                        nextBands[idx] = band.copy(gainDb = nextG.toDouble())
                        StudioRepository.updateProfile(profile.copy(bands = nextBands))
                    },
                    valueRange = -12f..12f,
                    modifier = Modifier.weight(1f),
                    colors = SliderDefaults.colors(
                        thumbColor = AccentColor,
                        activeTrackColor = AccentColor,
                        inactiveTrackColor = RaisedColor,
                    ),
                )
                Text(
                    text = "%+.1f dB".format(band.gainDb),
                    color = TextColor,
                    fontSize = 12.sp,
                    modifier = Modifier.width(56.dp),
                )
            }
        }
    }

    StudioCard(title = "8-Band R50i NC Hardware EEPROM EQ (02 83)", meta = "Includes A3959 DRC Matrix") {
        A3959Protocol.HW_BAND_FREQS.forEachIndexed { idx, freq ->
            val tenths = r50i.eqGainsTenthsDb.getOrElse(idx) { 0 }
            val drcTenths = r50i.drcPreviewTenthsDb.getOrElse(idx) { 0 }
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text(
                    text = if (freq >= 1000) "%.1fk".format(freq / 1000.0) else "$freq",
                    color = MutedColor,
                    fontSize = 12.sp,
                    modifier = Modifier.width(44.dp),
                )
                Slider(
                    value = tenths / 10f,
                    onValueChange = { db ->
                        val copy = r50i.eqGainsTenthsDb.copyOf()
                        copy[idx] = (db * 10f).roundToInt().coerceIn(-120, 120)
                        onFlashHwEq(copy, r50i.bassUpEnabled)
                    },
                    valueRange = -12f..12f,
                    modifier = Modifier.weight(1f),
                    colors = SliderDefaults.colors(
                        thumbColor = AccentColor,
                        activeTrackColor = AccentColor,
                        inactiveTrackColor = RaisedColor,
                    ),
                )
                Text(
                    text = "%+.1f (DRC %+.1f)".format(tenths / 10.0, drcTenths / 10.0),
                    color = TextColor,
                    fontSize = 11.sp,
                    modifier = Modifier.width(96.dp),
                )
            }
        }
    }
}

@Composable
private fun HardwareControlTab(
    r50i: A3959State,
    lastPacketHex: String,
    onToggleSurround: (Boolean) -> Unit,
    onToggleGaming: (Boolean) -> Unit,
    onCycleButtonAction: (EarbudSide, ButtonPressKind, ButtonAction) -> Unit,
) {
    StudioCard(title = "R50i NC Hardware Features", meta = "Serial ${r50i.serialNumber}") {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            ActionPillButton(
                label = if (r50i.onboardSurround) "3D Surround: ON" else "3D Surround: OFF",
                active = r50i.onboardSurround,
                onClick = { onToggleSurround(!r50i.onboardSurround) },
            )
            ActionPillButton(
                label = if (r50i.gamingMode == true) "Gaming Mode: ON" else "Gaming Mode: OFF",
                active = r50i.gamingMode == true,
                onClick = { onToggleGaming(!(r50i.gamingMode ?: false)) },
            )
        }
        Spacer(modifier = Modifier.height(8.dp))
        Text(text = "Last A3959 Wire Packet:", color = MutedColor, fontSize = 12.sp)
        Text(text = lastPacketHex, color = TextColor, fontSize = 11.sp)
    }

    StudioCard(title = "Touch Control Remapper (04 81)", meta = "Tap to cycle action") {
        val slots = listOf(
            Triple("Left · Single Tap", EarbudSide.LEFT to ButtonPressKind.SINGLE, r50i.buttons.leftSingle),
            Triple("Right · Single Tap", EarbudSide.RIGHT to ButtonPressKind.SINGLE, r50i.buttons.rightSingle),
            Triple("Left · Double Tap", EarbudSide.LEFT to ButtonPressKind.DOUBLE, r50i.buttons.leftDouble),
            Triple("Right · Double Tap", EarbudSide.RIGHT to ButtonPressKind.DOUBLE, r50i.buttons.rightDouble),
            Triple("Left · Triple Tap", EarbudSide.LEFT to ButtonPressKind.TRIPLE, r50i.buttons.leftTriple),
            Triple("Right · Triple Tap", EarbudSide.RIGHT to ButtonPressKind.TRIPLE, r50i.buttons.rightTriple),
            Triple("Left · Long Press", EarbudSide.LEFT to ButtonPressKind.LONG, r50i.buttons.leftLong),
            Triple("Right · Long Press", EarbudSide.RIGHT to ButtonPressKind.LONG, r50i.buttons.rightLong),
        )
        slots.forEach { (title, pair, currentAct) ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(48.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(text = title, color = MutedColor, fontSize = 13.sp)
                ActionPillButton(
                    label = currentAct.label,
                    active = currentAct != ButtonAction.DISABLED,
                    onClick = {
                        val all = ButtonAction.entries
                        val next = all[(all.indexOf(currentAct) + 1) % all.size]
                        onCycleButtonAction(pair.first, pair.second, next)
                    },
                )
            }
        }
    }
}

@Composable
private fun StudioCard(
    title: String,
    meta: String,
    content: @Composable () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(SurfaceColor, RoundedCornerShape(6.dp))
            .border(1.dp, BorderColor, RoundedCornerShape(6.dp))
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(text = title, color = TextColor, fontSize = 15.sp, fontWeight = FontWeight.SemiBold)
            Text(text = meta, color = MutedColor, fontSize = 12.sp)
        }
        content()
    }
}

@Composable
private fun MacroSliderRow(
    label: String,
    value: Float,
    onValueChange: (Float) -> Unit,
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(text = label, color = MutedColor, fontSize = 12.sp)
            Text(
                text = "${(value * 100).roundToInt()}%",
                color = TextColor,
                fontSize = 12.sp,
                fontWeight = FontWeight.SemiBold,
            )
        }
        Slider(
            value = value.coerceIn(0f, 1f),
            onValueChange = onValueChange,
            valueRange = 0f..1f,
            colors = SliderDefaults.colors(
                thumbColor = AccentColor,
                activeTrackColor = AccentColor,
                inactiveTrackColor = RaisedColor,
            ),
        )
    }
}

@Composable
private fun ActionPillButton(
    label: String,
    active: Boolean,
    danger: Boolean = false,
    onClick: () -> Unit,
) {
    val borderColor = when {
        danger -> NegativeColor
        active -> AccentColor
        else -> BorderColor
    }
    Box(
        modifier = Modifier
            .height(48.dp)
            .background(RaisedColor, RoundedCornerShape(6.dp))
            .border(1.dp, borderColor, RoundedCornerShape(6.dp))
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            color = if (danger) NegativeColor else TextColor,
            fontSize = 12.sp,
            fontWeight = FontWeight.Medium,
        )
    }
}
