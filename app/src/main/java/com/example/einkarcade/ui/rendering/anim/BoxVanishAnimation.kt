package com.example.einkarcade.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Rect
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.draw.GameRenderer
import com.example.einkarcade.ui.rendering.geom.BoardViewport

internal class BoxVanishAnimation(
    private val renderer: GameRenderer,
    private val viewport: BoardViewport,
    private val position: Position
) : Animation {
    override fun dirtyRect(): Rect {
        return boxRect
    }

    override fun draw(canvas: Canvas) {
        val phase = PHASES[phaseIndex]
        renderer.drawVanishingBox(canvas, viewport, position, phase.scale)
        phaseIndex++
    }

    override fun ticksUntilNextStep(): Int? {
        return if (phaseIndex < PHASES.size) PHASES[phaseIndex].ticks else null
    }

    private val boxRect: Rect by lazy { renderer.computeBoxRect(viewport, position) }
    private var phaseIndex = 0

    private data class Phase(val scale: Float, val ticks: Int)

    private companion object {
        // Edit this list to change vanish phase scales/timing in one place.
        val PHASES = listOf(
            Phase(scale = 1.0f, ticks = 4),
            Phase(scale = 0.75f, ticks = 4),
            Phase(scale = 0.5f, ticks = 3),
            Phase(scale = 0.3f, ticks = 2),
            Phase(scale = 0.18f, ticks = 2),
            Phase(scale = 0.14f, ticks = 1),
            Phase(scale = 0.1f, ticks = 1)
        )
    }
}
