package com.example.einkarcade.ui.screens

import com.example.einkarcade.GameController
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile

internal class GameUiState(
    var selectedBox: Position? = null,
    var isFacingLeft: Boolean = false,
    var lastBackTapTimeMs: Long? = null,
    val doubleTapWindowMs: Long = 350L,
    var blinkStartMs: Long = 0L,
    var blinkEndMs: Long = 0L
) {
    fun triggerBlink(nowMs: Long) {
        val start = nowMs + 400L
        blinkStartMs = start
        blinkEndMs = start + 300L
    }

    fun isBlinking(nowMs: Long): Boolean =
        nowMs in blinkStartMs until blinkEndMs
}

internal class GameAnimState(
    val boxPathAnimation: BoxPathAnimator,
    val vanishAnimation: VanishAnimator
)

internal object GameInputHandler {
    fun handleBackKeyUp(
        nowMs: Long,
        gameController: GameController,
        ui: GameUiState,
        resetSelection: () -> Unit
    ) {
        val lastTap = ui.lastBackTapTimeMs
        if (lastTap != null && nowMs - lastTap <= ui.doubleTapWindowMs) {
            ui.lastBackTapTimeMs = null
            resetSelection()
            gameController.restart()
        } else {
            ui.lastBackTapTimeMs = nowMs
            ui.isFacingLeft = false
            gameController.undo()
        }
    }

    fun handleTap(
        tappedPosition: Position,
        nowMs: Long,
        gameController: GameController,
        ui: GameUiState,
        anim: GameAnimState
    ) {
        fun attemptBoxMove(selectedBox: Position) {
            val boxPath = gameController.moveBoxTo(selectedBox, tappedPosition)
            if (boxPath == null) {
                ui.triggerBlink(nowMs)
                return
            }
            val previous = boxPath[boxPath.size - 2]
            val current = boxPath.last()
            val pushLeft = previous.row == current.row && current.col < previous.col
            if (!pushLeft) {
                ui.isFacingLeft = false
            }
            val lastPosition = boxPath.last()
            if (gameController.tiles[lastPosition.row][lastPosition.col] == Tile.WALL) {
                anim.vanishAnimation.start(lastPosition, nowMs)
                ui.triggerBlink(nowMs)
            }
            anim.boxPathAnimation.start(boxPath, gameController.playerPosition, nowMs) {
                ui.isFacingLeft = pushLeft
            }
        }

        val tile = gameController.tiles[tappedPosition.row][tappedPosition.col]
        val selectedBox = ui.selectedBox

        if (tile == Tile.WALL) {
            if (selectedBox != null) {
                ui.selectedBox = null
                attemptBoxMove(selectedBox)
            }
            return
        }

        if (gameController.boxPositions.contains(tappedPosition)) {
            if (selectedBox == tappedPosition) {
                ui.selectedBox = null
            } else {
                ui.selectedBox = tappedPosition
            }
        } else if (selectedBox != null) {
            ui.selectedBox = null
            attemptBoxMove(selectedBox)
        } else {
            gameController.movePlayerTo(tappedPosition)
            ui.isFacingLeft = false
        }
    }
}
