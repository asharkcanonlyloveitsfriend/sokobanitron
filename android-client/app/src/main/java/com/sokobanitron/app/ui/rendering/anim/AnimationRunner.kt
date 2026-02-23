package com.sokobanitron.app.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Rect
import java.util.ArrayDeque

/**
 * Owns animation sequencing and timing policy.
 *
 * It relies on injected callbacks for invalidation and scheduling.
 */
internal class AnimationRunner(
    private val invalidateRects: (Array<Rect?>) -> Unit,
    private val postDelayed: (Runnable, Long) -> Unit,
) {
    private val queue = ArrayDeque<Animation>()
    private var active: Animation? = null

    fun enqueue(animation: Animation) {
        queue.addLast(animation)
        if (active == null) {
            startNext()
        }
    }

    fun drawUnderEntities(canvas: Canvas) {
        active?.drawUnderEntities(canvas)
    }

    fun drawOverEntities(canvas: Canvas) {
        active?.drawOverEntities(canvas)
    }

    fun hidesPlayer(): Boolean = active?.hidesPlayer() == true

    private fun startNext() {
        val previous = active
        val next: Animation? = queue.pollFirst()

        active = null

        // Clean up previous animation region
        previous?.let { invalidateRects(it.dirtyRects()) }

        if (next == null) return

        active = next

        // Invalidate initial region if needed
        invalidateRects(next.dirtyRects())

        scheduleNextStep()
    }

    private fun scheduleNextStep() {
        val animation = active ?: return
        val ticks = animation.ticksUntilNextStep()

        if (ticks == null) {
            startNext()
        } else {
            val delayMs = ticks * ANIMATION_TICK_MS
            postDelayed(Runnable { advance() }, delayMs)
        }
    }

    private fun advance() {
        active?.let { invalidateRects(it.dirtyRects()) }
        scheduleNextStep()
    }
}
