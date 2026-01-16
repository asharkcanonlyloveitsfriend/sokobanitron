package com.example.einkarcade.ui.rendering.draw

import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import kotlin.math.roundToInt

internal class TileDrawer {
    private val floorFillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.WHITE }
    private val floorStrokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFF0F0F0.toInt()
        style = Paint.Style.STROKE
        strokeWidth = 2f
    }
    private val goalFillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = 0xFFE0E0E0.toInt() }
    private val goalStrokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        style = Paint.Style.STROKE
        strokeWidth = 2f
    }

    fun drawTiles(canvas: Canvas, viewport: BoardViewport, tiles: List<List<Tile>>) {
        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY
        val halfStroke = floorStrokePaint.strokeWidth / 2f

        for ((rowIndex, row) in tiles.withIndex()) {
            for ((colIndex, tile) in row.withIndex()) {
                val tileLeft = offsetX + (colIndex + 1) * cellSize
                val tileTop = offsetY + (rowIndex + 1) * cellSize
                val tileRight = tileLeft + cellSize
                val tileBottom = tileTop + cellSize
                drawTileCell(canvas, tile, tileLeft, tileTop, tileRight, tileBottom, halfStroke)
            }
        }
    }

    fun drawScaledTile(
        canvas: Canvas,
        tile: Tile,
        rowIndex: Int,
        colIndex: Int,
        scale: Float,
        cellSize: Float,
        offsetX: Float,
        offsetY: Float
    ) {
        if (tile == Tile.WALL) return
        val tileLeft = offsetX + (colIndex + 1) * cellSize
        val tileTop = offsetY + (rowIndex + 1) * cellSize
        val centerX = tileLeft + cellSize / 2f
        val centerY = tileTop + cellSize / 2f
        val size = cellSize * scale
        val left = centerX - size / 2f
        val top = centerY - size / 2f
        val right = centerX + size / 2f
        val bottom = centerY + size / 2f
        val halfStroke = floorStrokePaint.strokeWidth / 2f
        when (tile) {
            Tile.FLOOR -> {
                canvas.drawRect(left, top, right, bottom, floorFillPaint)
                canvas.drawRect(
                    left + halfStroke,
                    top + halfStroke,
                    right - halfStroke,
                    bottom - halfStroke,
                    floorStrokePaint
                )
            }
            Tile.GOAL -> {
                canvas.drawRect(left, top, right, bottom, goalFillPaint)
                canvas.drawRect(
                    left + halfStroke,
                    top + halfStroke,
                    right - halfStroke,
                    bottom - halfStroke,
                    goalStrokePaint
                )
            }
            else -> Unit
        }
    }

    fun drawScaledTileWithAlpha(
        canvas: Canvas,
        tile: Tile,
        rowIndex: Int,
        colIndex: Int,
        scale: Float,
        alpha: Float,
        cellSize: Float,
        offsetX: Float,
        offsetY: Float
    ) {
        val clampedAlpha = alpha.coerceIn(0f, 1f)
        val floorFillAlpha = floorFillPaint.alpha
        val floorStrokeAlpha = floorStrokePaint.alpha
        val goalFillAlpha = goalFillPaint.alpha
        val goalStrokeAlpha = goalStrokePaint.alpha
        val scaledAlpha = (255f * clampedAlpha).roundToInt().coerceIn(0, 255)

        floorFillPaint.alpha = scaledAlpha
        floorStrokePaint.alpha = scaledAlpha
        goalFillPaint.alpha = scaledAlpha
        goalStrokePaint.alpha = scaledAlpha

        try {
            drawScaledTile(canvas, tile, rowIndex, colIndex, scale, cellSize, offsetX, offsetY)
        } finally {
            floorFillPaint.alpha = floorFillAlpha
            floorStrokePaint.alpha = floorStrokeAlpha
            goalFillPaint.alpha = goalFillAlpha
            goalStrokePaint.alpha = goalStrokeAlpha
        }
    }

    private fun drawTileCell(
        canvas: Canvas,
        tile: Tile,
        left: Float,
        top: Float,
        right: Float,
        bottom: Float,
        halfStroke: Float
    ) {
        when (tile) {
            Tile.WALL -> Unit
            Tile.FLOOR -> {
                canvas.drawRect(left, top, right, bottom, floorFillPaint)
                canvas.drawRect(
                    left + halfStroke,
                    top + halfStroke,
                    right - halfStroke,
                    bottom - halfStroke,
                    floorStrokePaint
                )
            }
            Tile.GOAL -> {
                canvas.drawRect(left, top, right, bottom, goalFillPaint)
                canvas.drawRect(
                    left + halfStroke,
                    top + halfStroke,
                    right - halfStroke,
                    bottom - halfStroke,
                    goalStrokePaint
                )
            }
        }
    }
}
