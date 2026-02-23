package com.example.einkarcade.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Rect
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.draw.EntityRenderer
import com.example.einkarcade.ui.rendering.geom.BoardViewport

/**
 * Simple blink animation:
 * - Step 0: wait (no drawing)
 * - Step 1: draw blink (eyes)
 * - Step 2: cleanup (invalidate eyes, draw nothing), then finish
 */
internal class BlinkAnimation(
    renderer: EntityRenderer,
    viewport: BoardViewport,
    playerPos: Position,
    private val waitTicks: Int = 8, // e.g. 8 * 50ms = 400ms
    private val blinkTicks: Int = 6, // e.g. 6 * 50ms = 300ms
) : Animation {
    private val eyesRect: Rect by lazy { renderer.computePlayerEyesRect(viewport, playerPos) }
    private val eyesBitmap by lazy { renderer.getPlayerEyesBlinkBitmap() }
    private val spriteRect: Rect by lazy { renderer.computePlayerRect(viewport, playerPos) }

    private enum class Phase {
        WAITING,
        BLINKING,
        CLEANUP,
    }

    private var phase: Phase = Phase.WAITING

    override fun dirtyRects(): Array<Rect?> =
        arrayOf(
            when (phase) {
                Phase.BLINKING,
                Phase.CLEANUP,
                -> eyesRect

                else -> null
            },
        )

    override fun drawOverEntities(canvas: Canvas) {
        when (phase) {
            Phase.WAITING -> {
                // Do nothing, then advance to BLINKING
                phase = Phase.BLINKING
            }

            Phase.BLINKING -> {
                canvas.drawBitmap(eyesBitmap, null, spriteRect, null)
                phase = Phase.CLEANUP
            }

            Phase.CLEANUP -> {}
        }
    }

    override fun ticksUntilNextStep(): Int? =
        when (phase) {
            Phase.WAITING -> waitTicks
            Phase.BLINKING -> blinkTicks
            Phase.CLEANUP -> null
        }
}
