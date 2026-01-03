package com.example.einkarcade.ui.screens

import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.vanish.VanishSpec

internal data class VanishState(val position: Position, val step: Int)

internal class VanishAnimator {
    var state: VanishState? = null
        private set

    private var startTimeMs: Long = 0L
    private var position: Position? = null

    fun start(position: Position, nowMs: Long) {
        this.position = position
        startTimeMs = nowMs
        state = VanishState(position, 0)
    }

    fun update(nowMs: Long): Boolean {
        val currentPosition = position ?: return false
        val elapsed = nowMs - startTimeMs
        var cumulative = 0L

        for (step in 0..VanishSpec.LAST_STEP) {
            val delay = VanishSpec.delayMs(step)
            if (elapsed < cumulative + delay) {
                val nextState = VanishState(currentPosition, step)
                if (state != nextState) {
                    state = nextState
                    return true
                }
                return false
            }
            cumulative += delay
        }

        if (state != null) {
            state = null
            position = null
            return true
        }

        return false
    }
}
