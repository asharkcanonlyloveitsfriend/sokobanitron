package com.example.einkarcade.ui.rendering

import com.example.einkarcade.sokoban.Position
import kotlin.math.min

internal data class BoardViewport(
    val rows: Int,
    val cols: Int,
    val cellSize: Float,
    val offsetX: Float,
    val offsetY: Float
)

internal fun computeBoardViewport(
    surfaceWidth: Float,
    surfaceHeight: Float,
    innerRows: Int,
    innerCols: Int
): BoardViewport {
    require(innerRows > 0 && innerCols > 0)
    require(surfaceWidth > 0f && surfaceHeight > 0f)

    val rows = innerRows + 2
    val cols = innerCols + 2
    val cellSize = min(surfaceWidth / cols, surfaceHeight / rows)

    val renderedWidth = cellSize * cols
    val renderedHeight = cellSize * rows

    val offsetX = (surfaceWidth - renderedWidth) / 2f
    val offsetY = (surfaceHeight - renderedHeight) / 2f

    return BoardViewport(
        rows = rows,
        cols = cols,
        cellSize = cellSize,
        offsetX = offsetX,
        offsetY = offsetY
    )
}

internal fun BoardViewport.screenToInnerCell(x: Float, y: Float): Position? {
    require(rows > 0 && cols > 0)

    val col = ((x - offsetX) / cellSize).toInt()
    val row = ((y - offsetY) / cellSize).toInt()
    val innerRow = row - 1
    val innerCol = col - 1

    val innerRows = rows - 2
    val innerCols = cols - 2

    if (innerRow !in 0 until innerRows || innerCol !in 0 until innerCols) {
        return null
    }

    return Position(innerRow, innerCol)
}
