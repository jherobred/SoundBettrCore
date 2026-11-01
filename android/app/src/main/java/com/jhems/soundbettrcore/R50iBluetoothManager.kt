package com.jhems.soundbettrcore

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.content.Context
import java.io.InputStream
import java.io.OutputStream
import java.util.UUID

object R50iBluetoothManager {
    val SOUNDCORE_VENDOR_UUID: UUID = UUID.fromString("0cf12d31-fac3-4553-bd80-d6832e700000")
    val STANDARD_SPP_UUID: UUID = UUID.fromString("00001101-0000-1000-8000-00805f9b34fb")

    fun isR50iDeviceName(name: String?): Boolean {
        val lower = name?.lowercase() ?: return false
        return lower.contains("r50i") ||
            lower.contains("p30i") ||
            lower.contains("a3959") ||
            lower.contains("soundcore")
    }

    @SuppressLint("MissingPermission")
    fun findPairedR50i(context: Context): BluetoothDevice? {
        return try {
            val mgr = context.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager
            val adapter: BluetoothAdapter? = mgr?.adapter
            val bonded = adapter?.bondedDevices ?: emptySet()
            bonded.firstOrNull { isR50iDeviceName(it.name) }
        } catch (_: SecurityException) {
            null
        } catch (_: Throwable) {
            null
        }
    }

    @SuppressLint("MissingPermission")
    fun exchangePacket(
        context: Context,
        outbound: ByteArray,
        expectReply: Boolean = false,
    ): ByteArray? {
        val device = findPairedR50i(context) ?: return null
        for (uuid in listOf(SOUNDCORE_VENDOR_UUID, STANDARD_SPP_UUID)) {
            var socket: android.bluetooth.BluetoothSocket? = null
            try {
                socket = device.createRfcommSocketToServiceRecord(uuid)
                socket.connect()
                val out: OutputStream = socket.outputStream
                out.write(outbound)
                out.flush()
                if (!expectReply) {
                    socket.close()
                    return ByteArray(0)
                }
                val input: InputStream = socket.inputStream
                val buf = ByteArray(256)
                val read = input.read(buf)
                socket.close()
                if (read > 0) {
                    return buf.copyOfRange(0, read)
                }
            } catch (_: Throwable) {
                try {
                    socket?.close()
                } catch (_: Throwable) {
                }
            }
        }
        return null
    }

    @SuppressLint("MissingPermission")
    fun queryOrFallbackState(context: Context, current: A3959State): A3959State {
        val device = findPairedR50i(context)
        val req = A3959Protocol.buildStateRequestPacket()
        val reply = exchangePacket(context, req, expectReply = true)
        if (reply != null && reply.size >= 89) {
            return try {
                val parsed = A3959Protocol.parseStatePacket(reply)
                parsed.copy(
                    deviceName = device?.name ?: "Soundcore R50i NC (A3959)",
                    macAddress = device?.address ?: "RFCOMM Connected",
                    connected = true,
                    simulated = false,
                )
            } catch (_: Throwable) {
                current
            }
        }
        return if (device != null) {
            current.copy(
                deviceName = device.name ?: "Soundcore R50i NC (A3959)",
                macAddress = device.address ?: current.macAddress,
            )
        } else {
            current
        }
    }
}
