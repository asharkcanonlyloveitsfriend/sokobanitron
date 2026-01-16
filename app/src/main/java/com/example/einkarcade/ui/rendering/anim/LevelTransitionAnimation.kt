package com.example.einkarcade.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Rect
import android.os.SystemClock
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.VanishSpec
import com.example.einkarcade.ui.rendering.draw.GameRenderer
import com.example.einkarcade.ui.rendering.geom.BoardViewport

internal class LevelTransitionAnimation(
    private val renderer: GameRenderer,
    private val oldViewport: BoardViewport,
    private val newViewport: BoardViewport,
    private val oldTiles: List<List<Tile>>,
    private val newTiles: List<List<Tile>>,
    private val oldBoxPositions: Set<Position>,
    private val oldPlayerPosition: Position,
    private val newBoxPositions: Set<Position>,
    private val newPlayerPosition: Position,
    private val viewWidth: Int,
    private val viewHeight: Int
) : Animation {

    private val viewRect = Rect(0, 0, viewWidth, viewHeight)
    private val startTick = nowTick(SystemClock.elapsedRealtime())
    private val oldRows: Int = oldTiles.size
    private val oldCols: Int = oldTiles.firstOrNull()?.size ?: 0
    private val newRows: Int = newTiles.size
    private val newCols: Int = newTiles.firstOrNull()?.size ?: 0
    private val oldMaxIndex: Int =
        if (oldRows == 0 || oldCols == 0) 0 else (oldRows - 1) + (oldCols - 1)
    private val newMaxIndex: Int =
        if (newRows == 0 || newCols == 0) 0 else (newRows - 1) + (newCols - 1)
    private val oldTotalDurationTicks: Long =
        oldMaxIndex * PER_TILE_DELAY_TICKS + totalDurationTicks()
    private val newStartTick: Long = startTick + oldTotalDurationTicks / 2L

    private val flashBlackPaint = Paint().apply {
        color = Color.BLACK
        style = Paint.Style.FILL
        isAntiAlias = false
    }
    private val flashWhitePaint = Paint().apply {
        color = Color.WHITE
        style = Paint.Style.FILL
        isAntiAlias = false
    }

    override fun dirtyRect(): Rect? = viewRect

    override fun drawOverEntities(canvas: Canvas) {
        val nowMs = SystemClock.elapsedRealtime()
        renderer.drawBackground(canvas, viewWidth, viewHeight)
        drawOldTiles(canvas, nowMs)
        drawTransitionTiles(canvas, nowMs)
        drawTransitionFlashOverlay(canvas, nowMs)
        drawTransitionEntities(canvas, nowMs)
    }

    override fun ticksUntilNextStep(): Int? {
        return if (isComplete(SystemClock.elapsedRealtime())) null else 1
    }

    override fun hidesBoard(): Boolean = true

    private fun drawOldTiles(canvas: Canvas, nowMs: Long) {
        for (rowIndex in 0 until oldRows) {
            for (colIndex in 0 until oldCols) {
                val oldTile = tileAt(oldTiles, rowIndex, colIndex)
                if (oldTile == Tile.WALL) continue
                val shrinkScale = shrinkScale(nowMs, rowIndex, colIndex) ?: continue
                val alpha = fadeOutAlpha(nowMs, rowIndex, colIndex) ?: continue
                renderer.drawScaledTileWithAlpha(
                    canvas = canvas,
                    viewport = oldViewport,
                    tile = oldTile,
                    rowIndex = rowIndex,
                    colIndex = colIndex,
                    scale = shrinkScale,
                    alpha = alpha
                )
            }
        }
    }

    private fun drawTransitionTiles(canvas: Canvas, nowMs: Long) {
        for (rowIndex in 0 until newRows) {
            for (colIndex in 0 until newCols) {
                val newTile = tileAt(newTiles, rowIndex, colIndex)
                val growScale = if (newTile != Tile.WALL) {
                    growScale(nowMs, rowIndex, colIndex)
                } else {
                    null
                }
                if (growScale != null) {
                    renderer.drawScaledTile(
                        canvas = canvas,
                        viewport = newViewport,
                        tile = newTile,
                        rowIndex = rowIndex,
                        colIndex = colIndex,
                        scale = growScale
                    )
                }
            }
        }
    }

    private fun drawTransitionFlashOverlay(canvas: Canvas, nowMs: Long) {
        val cellSize = newViewport.cellSize
        val offsetX = newViewport.offsetX
        val offsetY = newViewport.offsetY
        for (rowIndex in 0 until newRows) {
            for (colIndex in 0 until newCols) {
                val newTile = tileAt(newTiles, rowIndex, colIndex)
                if (newTile == Tile.WALL) continue

                val phase = flashPhase(nowMs, rowIndex, colIndex) ?: continue
                val paint = when (phase) {
                    FlashPhase.BLACK -> flashBlackPaint
                    FlashPhase.WHITE -> flashWhitePaint
                }

                val left = offsetX + (colIndex + 1) * cellSize
                val top = offsetY + (rowIndex + 1) * cellSize
                val right = left + cellSize
                val bottom = top + cellSize
                canvas.drawRect(left, top, right, bottom, paint)
            }
        }
    }

    private fun drawTransitionEntities(canvas: Canvas, nowMs: Long) {
        val newBoxes = newBoxPositions.filter {
            isCellReady(nowMs, it.row, it.col)
        }.toSet()
        if (newBoxes.isNotEmpty()) {
            renderer.drawBoxes(canvas, newViewport, newBoxes, selectedBox = null)
        }

        if (isCellReady(nowMs, newPlayerPosition.row, newPlayerPosition.col)) {
            renderer.drawPlayer(canvas, newViewport, newPlayerPosition)
        }
    }

    private fun isComplete(nowMs: Long): Boolean {
        val nowTick = nowTick(nowMs)
        val oldEndTick = startTick + oldTotalDurationTicks
        val newEndTick = newStartTick + newMaxIndex * PER_TILE_DELAY_TICKS +
            totalDurationTicks() + FLASH_TOTAL_TICKS
        val endTick = maxOf(oldEndTick, newEndTick)
        return nowTick >= endTick
    }

    private fun growScale(nowMs: Long, row: Int, col: Int): Float? {
        val local = localElapsedTicks(nowMs, row, col, newStartTick, newRows)
        if (local < 0) return null
        val total = totalDurationTicks()
        if (local >= total) return 1.0f
        val stepCount = GROW_SCALES.size
        if (stepCount <= 1) return 1.0f
        val stepDuration = total.toFloat() / (stepCount - 1).toFloat()
        val stepIndex = (local.toFloat() / stepDuration).toInt().coerceIn(0, stepCount - 2)
        return GROW_SCALES[stepIndex]
    }

    private fun shrinkScale(nowMs: Long, row: Int, col: Int): Float? {
        val local = localElapsedTicks(nowMs, row, col, startTick, oldRows)
        if (local < 0) return 1.0f
        val total = totalDurationTicks()
        if (local >= total) return null
        val stepCount = SHRINK_SCALES.size
        if (stepCount <= 1) return 1.0f
        val stepDuration = total.toFloat() / (stepCount - 1).toFloat()
        val stepIndex = (local.toFloat() / stepDuration).toInt().coerceIn(0, stepCount - 2)
        return SHRINK_SCALES[stepIndex]
    }

    private fun fadeOutAlpha(nowMs: Long, row: Int, col: Int): Float? {
        val local = localElapsedTicks(nowMs, row, col, startTick, oldRows)
        if (local < 0) return 1.0f
        val total = totalDurationTicks()
        if (local >= total) return null
        val progress = (local.toFloat() / total.toFloat()).coerceIn(0f, 1f)
        return 1.0f - progress
    }

    private fun flashPhase(nowMs: Long, row: Int, col: Int): FlashPhase? {
        val local = localElapsedTicks(nowMs, row, col, newStartTick, newRows)
        if (local < 0) return null
        val growDone = totalDurationTicks()
        if (local < growDone) return null
        val flashElapsed = local - growDone
        if (flashElapsed >= FLASH_TOTAL_TICKS) return null
        return if (flashElapsed < FLASH_BLACK_TICKS) {
            FlashPhase.BLACK
        } else {
            FlashPhase.WHITE
        }
    }

    private fun localElapsedTicks(
        nowMs: Long,
        row: Int,
        col: Int,
        baseTick: Long,
        rowCount: Int
    ): Long {
        val nowTick = nowTick(nowMs)
        val index = (rowCount - 1 - row) + col
        return nowTick - (baseTick + index * PER_TILE_DELAY_TICKS)
    }

    private fun totalDurationTicks(): Long {
        return kotlin.math.ceil(
            VanishSpec.totalDurationTicks().toDouble() * TRANSITION_DURATION_MULT
        ).toLong()
    }

    private fun tileAt(tiles: List<List<Tile>>, row: Int, col: Int): Tile {
        if (row < 0 || col < 0) return Tile.WALL
        val rowList = tiles.getOrNull(row) ?: return Tile.WALL
        return rowList.getOrNull(col) ?: Tile.WALL
    }

    private fun isCellReady(nowMs: Long, row: Int, col: Int): Boolean {
        if (row < 0 || col < 0 || row >= newRows || col >= newCols) return false
        val local = localElapsedTicks(nowMs, row, col, newStartTick, newRows)
        return local >= totalDurationTicks() + FLASH_TOTAL_TICKS
    }

    private fun nowTick(nowMs: Long): Long = nowMs / ANIMATION_TICK_MS

    private enum class FlashPhase { BLACK, WHITE }

    private companion object {
        const val TRANSITION_DURATION_MULT: Float = 0.08f
        const val PER_TILE_DELAY_TICKS: Long = 2L
        const val FLASH_TOTAL_TICKS: Long = 2L
        const val FLASH_BLACK_TICKS: Long = 1L
        val GROW_SCALES = floatArrayOf(0.18f, 0.32f, 0.50f, 0.70f, 1.00f)
        val SHRINK_SCALES = floatArrayOf(1.00f, 0.70f, 0.50f, 0.32f, 0.18f)
    }
}
