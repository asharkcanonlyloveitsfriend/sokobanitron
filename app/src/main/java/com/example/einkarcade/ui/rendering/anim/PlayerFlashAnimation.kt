package com.example.einkarcade.ui.rendering.anim

import android.graphics.Rect
import android.os.SystemClock
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.RenderTimings

class PlayerFlashAnimation(
    private val flashPosition: Position,
    private val dirtyRect: Rect,
    private val flashStartTick: Long,
    private val renderPlayerFlashDirty: (Rect, Position, Long, Long, Boolean) -> Unit
) : Animation {

    private var tick: Long = 0L

    override fun tick(): Boolean {
        val nowMs = SystemClock.elapsedRealtime()
        when {
            tick < RenderTimings.FLASH_DURATION_TICKS -> {
                renderPlayerFlashDirty(dirtyRect, flashPosition, flashStartTick, nowMs, true)
            }

            tick == RenderTimings.FLASH_DURATION_TICKS -> {
                renderPlayerFlashDirty(dirtyRect, flashPosition, flashStartTick, nowMs, false)
                return false
            }
        }

        tick++
        return true
    }
}
