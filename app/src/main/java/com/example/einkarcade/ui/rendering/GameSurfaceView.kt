package com.example.einkarcade.ui.rendering

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile

@SuppressLint("ClickableViewAccessibility")
internal class GameSurfaceView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {
    private var scene: GameScene? = null
    private var isGameWon: Boolean = false
    private var onTapCell: ((Position) -> Unit)? = null
    private var lastViewport: BoardViewport? = null
    private val wallPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.DKGRAY }
    private val floorPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.GRAY }
    private val goalPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.LTGRAY }
    private val boxPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.LTGRAY }
    private val playerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.WHITE }
    private val pathPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.LTGRAY
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
    }

    init {
        holder.addCallback(this)
        setOnTouchListener { _, event ->
            if (event.action == MotionEvent.ACTION_UP) {
                if (isGameWon) return@setOnTouchListener true
                val viewport = requireNotNull(lastViewport) {
                    "SurfaceView tap received before viewport was initialized."
                }
                val position = viewport.screenToInnerCell(event.x, event.y)
                if (position != null) {
                    onTapCell?.invoke(position)
                }
                return@setOnTouchListener true
            }
            true
        }
    }

    fun setContent(scene: GameScene, isGameWon: Boolean, onTapCell: (Position) -> Unit) {
        this.scene = scene
        this.isGameWon = isGameWon
        this.onTapCell = onTapCell
        render()
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        if (scene != null) {
            render()
        }
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (scene != null) {
            render()
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) = Unit

    private fun render() {
        if (width <= 0 || height <= 0) return
        val scene = scene ?: return
        if (scene.tiles.isEmpty()) return
        if (scene.tiles.first().isEmpty()) return

        val innerRows = scene.tiles.size
        val innerCols = scene.tiles.first().size
        val viewport = computeBoardViewport(width.toFloat(), height.toFloat(), innerRows, innerCols)
        lastViewport = viewport

        if (!holder.surface.isValid) return

        val canvas = holder.lockCanvas() ?: return
        try {
            canvas.drawColor(Color.BLACK)
            pathPaint.strokeWidth = viewport.cellSize * 0.2f

            val cellSize = viewport.cellSize
            val offsetX = viewport.offsetX
            val offsetY = viewport.offsetY

            for ((rowIndex, row) in scene.tiles.withIndex()) {
                for ((colIndex, tile) in row.withIndex()) {
                    val point = Position(rowIndex + 1, colIndex + 1)
                        .toRenderPoint(cellSize, offsetX, offsetY)
                    val left = point.x
                    val top = point.y
                    val right = left + cellSize
                    val bottom = top + cellSize
                    when (tile) {
                        Tile.WALL -> canvas.drawRect(left, top, right, bottom, wallPaint)
                        Tile.FLOOR -> canvas.drawRect(left, top, right, bottom, floorPaint)
                        Tile.GOAL -> {
                            canvas.drawRect(left, top, right, bottom, floorPaint)
                            val centerX = left + cellSize / 2f
                            val centerY = top + cellSize / 2f
                            canvas.drawCircle(centerX, centerY, cellSize * 0.2f, goalPaint)
                        }
                    }
                }
            }

            for (position in scene.boxPositions) {
                val point = Position(position.row + 1, position.col + 1)
                    .toRenderPoint(cellSize, offsetX, offsetY)
                val size = cellSize * 0.9f
                val left = point.x + (cellSize - size) / 2f
                val top = point.y + (cellSize - size) / 2f
                canvas.drawRect(left, top, left + size, top + size, boxPaint)
            }

            val playerPoint = Position(scene.playerPosition.row + 1, scene.playerPosition.col + 1)
                .toRenderPoint(cellSize, offsetX, offsetY)
            val playerCenterX = playerPoint.x + cellSize / 2f
            val playerCenterY = playerPoint.y + cellSize / 2f
            canvas.drawCircle(playerCenterX, playerCenterY, cellSize * 0.35f, playerPaint)

            if (scene.boxPathActive && scene.boxPath.size >= 2) {
                val points = scene.boxPath.map { position ->
                    val centerX = offsetX + (position.col + 1) * cellSize + cellSize / 2f
                    val centerY = offsetY + (position.row + 1) * cellSize + cellSize / 2f
                    Pair(centerX, centerY)
                }
                for (i in 0 until points.size - 1) {
                    val start = points[i]
                    val end = points[i + 1]
                    canvas.drawLine(start.first, start.second, end.first, end.second, pathPaint)
                }
            }
        } finally {
            holder.unlockCanvasAndPost(canvas)
        }
    }
}
