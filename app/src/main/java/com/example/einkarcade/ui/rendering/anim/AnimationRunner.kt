package com.example.einkarcade.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Rect
import java.util.ArrayDeque

/**
 * Owns animation sequencing and timing policy.
 *
 * It relies on injected callbacks for invalidation and scheduling.
 */
internal class AnimationRunner(
    private val invalidateRect: (Rect) -> Unit,
    private val postDelayed: (Runnable, Long) -> Unit
) {

    private val queue = ArrayDeque<Animation>()
    private var active: Animation? = null
    private val requirements: AnimationRequirements by lazy { buildRequirements() }

    fun requirements(): AnimationRequirements = requirements

    /** Enqueue an animation and start immediately if idle. */
    fun enqueue(animation: Animation) {
        queue.addLast(animation)
        if (active == null) {
            startNext()
        }
    }

    /** Called by the View during onDraw. */
    fun draw(canvas: Canvas) {
        active?.draw(canvas)
    }

    private fun startNext() {
        val previous = active
        val next: Animation? = queue.pollFirst()

        active = null

        // Clean up previous animation region
        previous?.dirtyRect()?.let { invalidateRect(it) }

        if (next == null) return

        active = next

        // Invalidate initial region if needed
        next.dirtyRect()?.let { invalidateRect(it) }

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
        active?.dirtyRect()?.let { invalidateRect(it) }
        scheduleNextStep()
    }

    private fun buildRequirements(): AnimationRequirements {
        val boxScales = BoxVanishAnimation.phaseScales().toSet()
        return AnimationRequirements(boxScaleFactors = boxScales)
    }
}
