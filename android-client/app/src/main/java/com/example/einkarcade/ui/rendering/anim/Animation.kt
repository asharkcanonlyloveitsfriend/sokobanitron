package com.example.einkarcade.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Rect

const val ANIMATION_TICK_MS: Long = 50L

interface Animation {
    /** Return true when the animation should hide the player sprite. */
    fun hidesPlayer(): Boolean = false

    /** Regions affected at the current state. Null entries are ignored. */
    fun dirtyRects(): Array<Rect?>

    /** Draw elements that should appear below entities. */
    fun drawUnderEntities(canvas: Canvas) {}

    /** Draw elements that should appear above entities. */
    fun drawOverEntities(canvas: Canvas) {}

    /**
     * Number of animation ticks until the next state change.
     * Return null when the animation is complete.
     */
    fun ticksUntilNextStep(): Int?
}
