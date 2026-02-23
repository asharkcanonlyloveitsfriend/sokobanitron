package com.example.einkarcade.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Rect
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.draw.EntityRenderer
import com.example.einkarcade.ui.rendering.geom.BoardViewport

private data class BoxVanishPhase(
    val scale: Float,
    val ticks: Int,
)

private val BOX_VANISH_PHASES =
    listOf(
        BoxVanishPhase(scale = 1.0f, ticks = 4),
        BoxVanishPhase(scale = 0.75f, ticks = 3),
        BoxVanishPhase(scale = 0.5f, ticks = 3),
        BoxVanishPhase(scale = 0.3f, ticks = 2),
        BoxVanishPhase(scale = 0.18f, ticks = 2),
        BoxVanishPhase(scale = 0.14f, ticks = 1),
        BoxVanishPhase(scale = 0.1f, ticks = 1),
    )

internal class BoxVanishAnimation(
    private val renderer: EntityRenderer,
    private val viewport: BoardViewport,
    private val position: Position,
) : Animation {
    override fun dirtyRects(): Array<Rect?> = arrayOf(boxRect)

    override fun drawOverEntities(canvas: Canvas) {
        if (phaseIndex >= BOX_VANISH_PHASES.size) return
        val phase = BOX_VANISH_PHASES[phaseIndex]
        renderer.drawVanishingBox(canvas, viewport, position, phase.scale)
        phaseIndex++
    }

    override fun ticksUntilNextStep(): Int? = if (phaseIndex < BOX_VANISH_PHASES.size) BOX_VANISH_PHASES[phaseIndex].ticks else null

    private val boxRect: Rect by lazy { renderer.computeBoxRect(viewport, position) }
    private var phaseIndex = 0
}
