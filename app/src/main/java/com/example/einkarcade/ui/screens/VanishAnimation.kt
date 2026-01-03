package com.example.einkarcade.ui.screens

import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.vanish.VanishSpec

internal data class VanishState(val position: Position, val step: Int)

internal class VanishAnimationState(
    val state: MutableState<VanishState?>,
    private val handler: Handler
) {
    fun start(position: Position) {
        state.value = VanishState(position, 0)
        advance(0, position)
    }

    private fun advance(step: Int, position: Position) {
        val delay = VanishSpec.delayMs(step)

        handler.postDelayed({
            val nextStep = step + 1
            if (nextStep >= VanishSpec.TOTAL_STEPS) {
                state.value = null
                return@postDelayed
            }
            state.value = VanishState(position, nextStep)
            advance(nextStep, position)
        }, delay)
    }
}

@Composable
internal fun rememberVanishAnimationState(): VanishAnimationState {
    val state = remember { mutableStateOf<VanishState?>(null) }
    val handler = remember { Handler(Looper.getMainLooper()) }
    return VanishAnimationState(state, handler)
}
