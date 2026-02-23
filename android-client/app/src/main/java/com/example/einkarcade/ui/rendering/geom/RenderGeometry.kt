package com.example.einkarcade.ui.rendering.geom

import com.example.einkarcade.sokoban.Position
import kotlin.math.roundToInt

internal data class RenderPoint(
    val x: Float,
    val y: Float,
)

// E-ink renders subpixel edges poorly.
internal fun snapToWholePixel(value: Float): Float = value.roundToInt().toFloat()

internal fun Position.toRenderPoint(
    cellSize: Float,
    offsetX: Float,
    offsetY: Float,
): RenderPoint =
    RenderPoint(
        x = offsetX + this.col * cellSize,
        y = offsetY + this.row * cellSize,
    )
