package com.example.einkarcade.ui.screens

import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import com.example.einkarcade.sokoban.Position

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
        val delay = when (step) {
            0 -> VANISH_BASE_DELAY_MS
            1 -> (VANISH_BASE_DELAY_MS * 0.75f).toLong()
            2 -> (VANISH_BASE_DELAY_MS * 0.5f).toLong()
            3 -> (VANISH_BASE_DELAY_MS * 0.36f).toLong()
            4 -> (VANISH_BASE_DELAY_MS * 0.2f).toLong()
            5 -> INVISIBLE_DELAY_MS
            else -> 0L
        }

        handler.postDelayed({
            val nextStep = step + 1
            if (nextStep >= TOTAL_STEPS) {
                state.value = null
                return@postDelayed
            }
            state.value = VanishState(position, nextStep)
            advance(nextStep, position)
        }, delay)
    }

    private companion object {
        const val VANISH_BASE_DELAY_MS = 170L
        const val INVISIBLE_DELAY_MS = 100L
        const val TOTAL_STEPS = 6
    }
}

@Composable
internal fun rememberVanishAnimationState(): VanishAnimationState {
    val state = remember { mutableStateOf<VanishState?>(null) }
    val handler = remember { Handler(Looper.getMainLooper()) }
    return VanishAnimationState(state, handler)
}
