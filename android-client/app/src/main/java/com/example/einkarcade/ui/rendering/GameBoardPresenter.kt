package com.example.einkarcade.ui.rendering

import android.view.View
import com.example.einkarcade.GameController
import com.example.einkarcade.sokoban.Position

interface GameBoardPresenter {
    /** Apply a render delta produced by the GameController. */
    fun applyDelta(delta: GameController.RenderDelta)

    /** Register a callback for cell taps (board coordinates). */
    fun setOnTapCell(handler: (Position) -> Unit)

    /** Selection remains surface-owned for now. */
    fun getSelectedBox(): Position?

    fun setSelectedBox(position: Position?)

    /** Expose the underlying View for AndroidView embedding. */
    fun asView(): View
}
