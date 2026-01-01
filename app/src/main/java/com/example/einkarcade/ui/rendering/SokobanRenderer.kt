package com.example.einkarcade.ui.rendering

import kotlin.math.roundToInt

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.graphics.painter.Painter
import com.example.einkarcade.sokoban.Position

const val CELL_SIZE = 100f
const val GRID_OFFSET_X = 50f
const val GRID_OFFSET_Y = 50f

/**
 * Snap drawing coordinates to whole pixels to avoid subpixel
 * artifacts (especially visible on e-ink displays).
 */
private fun snap(px: Float): Float =
    px.roundToInt().toFloat()

fun DrawScope.drawGameObject(
    position: Position,
    cellSize: Float,
    offsetX: Float,
    offsetY: Float,
    draw: DrawScope.(Offset) -> Unit
) {
    this.draw(position.toOffset(cellSize, offsetX, offsetY))
}

fun DrawScope.drawFloor(position: Position, cellSize: Float, offsetX: Float, offsetY: Float) {
    drawGameObject(position, cellSize, offsetX, offsetY) { offset ->
        drawRect(
            color = Color.White,
            topLeft = offset,
            size = Size(cellSize, cellSize)
        )
        drawRect(
            color = Color(0xFFF0F0F0),
            topLeft = offset,
            size = Size(cellSize, cellSize),
            style = Stroke(width = 2f)
        )
    }
}

fun DrawScope.drawGoal(position: Position, cellSize: Float, offsetX: Float, offsetY: Float) {
    drawGameObject(position, cellSize, offsetX, offsetY) { offset ->
        drawRect(
            color = Color(0xFFE0E0E0),
            topLeft = offset,
            size = Size(cellSize, cellSize)
        )
        drawRect(
            color = Color.White,
            topLeft = offset,
            size = Size(cellSize, cellSize),
            style = Stroke(width = 2f)
        )
    }
}

fun DrawScope.drawBox(
    position: Position,
    painter: Painter,
    selected: Boolean,
    cellSize: Float,
    offsetX: Float,
    offsetY: Float
) {
    val offset = position.toOffset(cellSize, offsetX, offsetY)
    val targetSize = snap(cellSize * 0.90f)
    val left = snap(offset.x + (cellSize - targetSize) / 2)
    val top = snap(offset.y + (cellSize - targetSize) / 2)

    // Draw box SVG
    withTransform({
        translate(left, top)
    }) {
        with(painter) {
            draw(size = Size(targetSize, targetSize))
        }
    }

    if (selected) {
        val bracketLength = targetSize * 0.24f
        val strokeWidth = targetSize * 0.065f
        val inset = targetSize * 0.075f

        val x0 = left + inset
        val y0 = top + inset
        val x1 = left + targetSize - inset - bracketLength
        val y1 = top + targetSize - inset - bracketLength

        val color = Color.Black

        // Top-left
        drawRect(color, Offset(x0, y0), Size(bracketLength, strokeWidth))
        drawRect(color, Offset(x0, y0), Size(strokeWidth, bracketLength))

        // Top-right
        drawRect(color, Offset(x1, y0), Size(bracketLength, strokeWidth))
        drawRect(color, Offset(x1 + bracketLength - strokeWidth, y0), Size(strokeWidth, bracketLength))

        // Bottom-left
        drawRect(color, Offset(x0, y1 + bracketLength - strokeWidth), Size(bracketLength, strokeWidth))
        drawRect(color, Offset(x0, y1), Size(strokeWidth, bracketLength))

        // Bottom-right
        drawRect(
            color,
            Offset(x1, y1 + bracketLength - strokeWidth),
            Size(bracketLength, strokeWidth)
        )
        drawRect(
            color,
            Offset(x1 + bracketLength - strokeWidth, y1),
            Size(strokeWidth, bracketLength)
        )
    }
}

fun DrawScope.drawPlayer(
    position: Position,
    painter: Painter,
    cellSize: Float,
    offsetX: Float,
    offsetY: Float
) {
    val offset = position.toOffset(cellSize, offsetX, offsetY)
    val targetSize = snap(cellSize * 0.8f)
    val left = snap(offset.x + (cellSize - targetSize) / 2)
    val top = snap(offset.y + (cellSize - targetSize) / 2)

    withTransform({
        translate(left, top)
    }) {
        with(painter) {
            draw(size = Size(targetSize, targetSize))
        }
    }
}

fun Position.toOffset(
    cellSize: Float = CELL_SIZE,
    offsetX: Float = GRID_OFFSET_X,
    offsetY: Float = GRID_OFFSET_Y
): Offset {
    return Offset(
        offsetX + this.col * cellSize,
        offsetY + this.row * cellSize
    )
}
