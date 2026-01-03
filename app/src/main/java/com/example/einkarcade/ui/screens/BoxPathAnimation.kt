package com.example.einkarcade.ui.screens

import com.example.einkarcade.sokoban.Position

internal class BoxPathAnimator(
    private val durationMs: Long = 100L
) {
    var path: List<Position> = emptyList()
        private set
    var shrink: Float = 0f
        private set
    var isActive: Boolean = false
        private set

    private var startTimeMs: Long = 0L
    private var pendingPlayerPosition: Position? = null
    private var holdPlayerPosition: Boolean = false
    private var displayedPlayerPosition: Position? = null
    private var onArrive: (() -> Unit)? = null

    fun start(
        path: List<Position>,
        pendingPlayer: Position,
        nowMs: Long,
        onArrive: (() -> Unit)? = null
    ) {
        require(path.size >= 2) { "Box path requires at least two points." }
        this.path = path
        this.pendingPlayerPosition = pendingPlayer
        this.onArrive = onArrive
        this.startTimeMs = nowMs
        this.shrink = 0f
        this.isActive = true
        this.holdPlayerPosition = true
    }

    fun update(nowMs: Long): Boolean {
        if (!isActive) return false

        val elapsed = nowMs - startTimeMs
        val progress = (elapsed.toFloat() / durationMs.toFloat()).coerceAtMost(1f)
        var changed = false

        if (progress != shrink) {
            shrink = progress
            changed = true
        }

        if (elapsed >= durationMs) {
            isActive = false
            onArrive?.invoke()
            onArrive = null
            val pending = requireNotNull(pendingPlayerPosition) {
                "Box path animation finished without a pending player position."
            }
            displayedPlayerPosition = pending
            pendingPlayerPosition = null
            holdPlayerPosition = false
            changed = true
        }

        return changed
    }

    fun displayedPlayerPosition(currentPlayer: Position): Position {
        if (!holdPlayerPosition) {
            displayedPlayerPosition = currentPlayer
        }
        return displayedPlayerPosition ?: currentPlayer
    }
}
