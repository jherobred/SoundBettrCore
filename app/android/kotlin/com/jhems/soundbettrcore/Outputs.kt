package com.jhems.soundbettrcore

import android.media.AudioAttributes
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build
import org.json.JSONObject

/** Which output media plays through right now, in the shape the Rust core's `Output` expects. */
object Outputs {
    private val BLUETOOTH = setOf(AudioDeviceInfo.TYPE_BLUETOOTH_A2DP, AudioDeviceInfo.TYPE_BLE_HEADSET, AudioDeviceInfo.TYPE_BLE_SPEAKER)
    private val WIRED = setOf(AudioDeviceInfo.TYPE_WIRED_HEADSET, AudioDeviceInfo.TYPE_WIRED_HEADPHONES)
    private val USB = setOf(AudioDeviceInfo.TYPE_USB_HEADSET, AudioDeviceInfo.TYPE_USB_DEVICE, AudioDeviceInfo.TYPE_USB_ACCESSORY)

    fun current(am: AudioManager): JSONObject? {
        val all = am.getDevices(AudioManager.GET_DEVICES_OUTPUTS).toList()
        val devices = if (Build.VERSION.SDK_INT >= 33) {
            try {
                am.getAudioDevicesForAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).build()).ifEmpty { all }
            } catch (_: Throwable) {
                all
            }
        } else {
            all
        }
        // Android routes media to Bluetooth first, then the jack, then USB, then the speaker
        val pick = devices.firstOrNull { it.type in BLUETOOTH }
            ?: devices.firstOrNull { it.type in WIRED }
            ?: devices.firstOrNull { it.type in USB }
            ?: devices.firstOrNull { it.type == AudioDeviceInfo.TYPE_BUILTIN_SPEAKER }
            ?: return null
        return toJson(pick)
    }

    fun toJson(d: AudioDeviceInfo): JSONObject {
        val name = d.productName?.toString()?.takeIf { it.isNotBlank() }
        val o = JSONObject()
        when (d.type) {
            in BLUETOOTH -> {
                o.put("kind", "bluetooth").put("key", d.address).put("name", name ?: "Bluetooth audio").put("address", d.address)
            }
            in WIRED -> o.put("kind", "wired").put("key", "wired").put("name", "Wired headset").put("address", JSONObject.NULL)
            in USB -> o.put("kind", "usb").put("key", "usb:" + (name ?: "audio")).put("name", name ?: "USB audio").put("address", JSONObject.NULL)
            else -> o.put("kind", "builtin").put("key", "builtin").put("name", "Phone speaker").put("address", JSONObject.NULL)
        }
        return o
    }
}
