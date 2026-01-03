package com.example.einkarcade.ui.screens

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import com.example.einkarcade.sokoban.Position
import kotlinx.coroutines.delay
import kotlin.math.min

internal class BoxPathAnimationState(
    val path: MutableState<List<Position>>,
    val shrink: MutableState<Float>,
    val isActive: MutableState<Boolean>,
    private val trigger: MutableState<Int>,
    private val pendingPlayerPosition: MutableState<Position?>,
    private val holdPlayerPosition: MutableState<Boolean>,
    private val displayedPlayerPosition: MutableState<Position>
) {
    fun start(path: List<Position>, pendingPlayer: Position) {
        require(path.size >= 2) { "Box path requires at least two points." }
        holdPlayerPosition.value = true
        pendingPlayerPosition.value = pendingPlayer
        this.path.value = path
        trigger.value += 1
    }

    fun displayedPlayerPosition(currentPlayer: Position): Position {
        if (!holdPlayerPosition.value) {
            displayedPlayerPosition.value = currentPlayer
        }
        return displayedPlayerPosition.value
    }
}

@Composable
internal fun rememberBoxPathAnimationState(): BoxPathAnimationState {
    val path = remember { mutableStateOf<List<Position>>(emptyList()) }
    val shrink = remember { mutableStateOf(0f) }
    val isActive = remember { mutableStateOf(false) }
    val trigger = remember { mutableStateOf(0) }
    val pendingPlayerPosition = remember { mutableStateOf<Position?>(null) }
    val holdPlayerPosition = remember { mutableStateOf(false) }
    val displayedPlayerPosition = remember { mutableStateOf(Position(0, 0)) }

    LaunchedEffect(trigger.value) {
        if (trigger.value == 0) return@LaunchedEffect
        val durationMs = 100L
        val stepMs = 10L
        val steps = (durationMs / stepMs).coerceAtLeast(1)
        isActive.value = true
        shrink.value = 0f
        for (i in 1..steps) {
            delay(stepMs)
            shrink.value = min(1f, i.toFloat() / steps.toFloat())
        }
        isActive.value = false
        val pending = requireNotNull(pendingPlayerPosition.value) {
            "Box path animation finished without a pending player position."
        }
        displayedPlayerPosition.value = pending
        pendingPlayerPosition.value = null
        holdPlayerPosition.value = false
    }

    return BoxPathAnimationState(
        path = path,
        shrink = shrink,
        isActive = isActive,
        trigger = trigger,
        pendingPlayerPosition = pendingPlayerPosition,
        holdPlayerPosition = holdPlayerPosition,
        displayedPlayerPosition = displayedPlayerPosition
    )
}
