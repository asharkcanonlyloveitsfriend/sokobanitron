package com.example.einkarcade.ui.rendering.anim

import android.graphics.Rect
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.VanishSpec

class BoxVanishAnimation(
    private val vanishPosition: Position,
    private val dirtyRect: Rect,
    private val renderVanishDirty: (Rect, Position, Int, Boolean) -> Unit
) : Animation {

    private var elapsedTicks: Long = 0L
    private var lastStep: Int? = null

    override fun tick(): Boolean {
        val step = resolveStep(elapsedTicks)
        return if (step == null) {
            if (lastStep != null) {
                renderVanishDirty(dirtyRect, vanishPosition, lastStep ?: 0, false)
            }
            false
        } else {
            if (step != lastStep) {
                renderVanishDirty(dirtyRect, vanishPosition, step, true)
                lastStep = step
            }
            elapsedTicks++
            true
        }
    }

    private fun resolveStep(elapsedTicks: Long): Int? {
        var cumulativeTicks = 0L
        for (step in 0..VanishSpec.LAST_STEP) {
            val delayTicks = VanishSpec.delayTicks(step)
            if (elapsedTicks < cumulativeTicks + delayTicks) {
                return step
            }
            cumulativeTicks += delayTicks
        }
        return null
    }
}
