package com.example.einkarcade.ui.rendering

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.graphics.painter.Painter
import com.example.einkarcade.sokoban.Position

const val CELL_SIZE = 100f
const val GRID_OFFSET_X = 50f
const val GRID_OFFSET_Y = 50f

fun DrawScope.drawGameObject(
    position: Position,
    draw: DrawScope.(Offset) -> Unit
) {
    this.draw(position.toOffset())
}

fun DrawScope.drawWall(position: Position) {
    drawGameObject(position) { offset ->
        drawRect(
            color = Color.Black,
            topLeft = offset,
            size = Size(CELL_SIZE, CELL_SIZE)
        )
    }
}

fun DrawScope.drawFloor(position: Position) {
    drawGameObject(position) { offset ->
        drawRect(
            color = Color.White,
            topLeft = offset,
            size = Size(CELL_SIZE, CELL_SIZE)
        )
    }
}

fun DrawScope.drawGoal(position: Position) {
    drawGameObject(position) { offset ->
        drawRect(
            color = Color.LightGray,
            topLeft = offset,
            size = Size(CELL_SIZE, CELL_SIZE)
        )
    }
}

fun DrawScope.drawBox(position: Position, painter: Painter, selected: Boolean) {
    val offset = position.toOffset()
    val targetSize = CELL_SIZE * 0.90f
    val left = offset.x + (CELL_SIZE - targetSize) / 2
    val top = offset.y + (CELL_SIZE - targetSize) / 2

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

fun DrawScope.drawPlayer(position: Position, painter: Painter) {
    val offset = position.toOffset()
    val targetSize = CELL_SIZE * 0.8f
    val left = offset.x + (CELL_SIZE - targetSize) / 2
    val top = offset.y + (CELL_SIZE - targetSize) / 2

    withTransform({
        translate(left, top)
    }) {
        with(painter) {
            draw(size = Size(targetSize, targetSize))
        }
    }
}

fun Position.toOffset(): Offset {
    return Offset(
        GRID_OFFSET_X + this.col * CELL_SIZE,
        GRID_OFFSET_Y + this.row * CELL_SIZE
    )
}