package com.sokobanitron.app.ui.screens

import com.sokobanitron.app.GameController
import com.sokobanitron.app.sokoban.Position

internal object GameInputHandler {
    interface BoxSelection {
        fun getSelectedBox(): Position?

        fun setSelectedBox(position: Position?)
    }

    fun handleBackKeyUp(gameController: GameController) {
        gameController.undo()
    }

    fun handleTap(
        tappedPosition: Position,
        gameController: GameController,
        selection: BoxSelection,
    ) {
        val selectedBox = selection.getSelectedBox()
        if (gameController.tileMap.isVoid(tappedPosition)) {
            if (selectedBox != null) {
                selection.setSelectedBox(null)
                gameController.moveBoxTo(selectedBox, tappedPosition)
            } else {
                selection.setSelectedBox(null)
            }
            return
        }
        if (gameController.boxPositions.contains(tappedPosition)) {
            if (selectedBox == tappedPosition) {
                selection.setSelectedBox(null)
            } else {
                selection.setSelectedBox(tappedPosition)
            }
            return
        }
        if (selectedBox != null) {
            selection.setSelectedBox(null)
            gameController.moveBoxTo(selectedBox, tappedPosition)
        } else {
            gameController.movePlayerTo(tappedPosition)
        }
    }
}
