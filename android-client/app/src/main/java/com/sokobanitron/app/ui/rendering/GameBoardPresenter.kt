package com.sokobanitron.app.ui.rendering

import android.view.View
import com.sokobanitron.app.GameController
import com.sokobanitron.app.sokoban.Position

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
