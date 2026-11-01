package com.jhems.soundbettrcore

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService

class SoundBettrTileService : TileService() {
    override fun onStartListening() {
        super.onStartListening()
        syncTile()
    }

    override fun onClick() {
        super.onClick()
        val current = StudioRepository.profile.value
        StudioRepository.updateProfile(current.copy(bypass = !current.bypass))
        syncTile()
    }

    private fun syncTile() {
        val tile = qsTile ?: return
        val current = StudioRepository.profile.value
        tile.state = if (current.bypass) Tile.STATE_INACTIVE else Tile.STATE_ACTIVE
        tile.label = if (current.bypass) "R50i NC Bypassed" else current.name
        tile.updateTile()
    }
}
