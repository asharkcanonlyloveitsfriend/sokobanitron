package com.example.einkarcade.ui.rendering

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.vanish.VanishSpec
import com.example.einkarcade.ui.vanish.VanishVisualSpec
import kotlin.math.roundToInt

@SuppressLint("ClickableViewAccessibility")
internal class GameSurfaceView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {
    private var scene: GameScene? = null
    private var isGameWon: Boolean = false
    private var onTapCell: ((Position) -> Unit)? = null
    private var lastViewport: BoardViewport? = null
    private val assets = AndroidGameAssets(context)
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
    private val vanishPaint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val pathPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFD3D3D3.toInt()
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
    }

    private var pathXs = FloatArray(0)
    private var pathYs = FloatArray(0)

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
            val bitmapPaint = assets.bitmapPaint()
            pathPaint.strokeWidth = viewport.cellSize * 0.2f

            val cellSize = viewport.cellSize
            val offsetX = viewport.offsetX
            val offsetY = viewport.offsetY
            val vanish = scene.vanish

            for ((rowIndex, row) in scene.tiles.withIndex()) {
                for ((colIndex, tile) in row.withIndex()) {
                    val tileLeft = offsetX + (colIndex + 1) * cellSize
                    val tileTop = offsetY + (rowIndex + 1) * cellSize
                    val tileRight = tileLeft + cellSize
                    val tileBottom = tileTop + cellSize
                    val halfStroke = floorStrokePaint.strokeWidth / 2f
                    when (tile) {
                        Tile.WALL -> Unit
                        Tile.FLOOR -> {
                            canvas.drawRect(tileLeft, tileTop, tileRight, tileBottom, floorFillPaint)
                            canvas.drawRect(
                                tileLeft + halfStroke,
                                tileTop + halfStroke,
                                tileRight - halfStroke,
                                tileBottom - halfStroke,
                                floorStrokePaint
                            )
                        }
                        Tile.GOAL -> {
                            canvas.drawRect(tileLeft, tileTop, tileRight, tileBottom, goalFillPaint)
                            canvas.drawRect(
                                tileLeft + halfStroke,
                                tileTop + halfStroke,
                                tileRight - halfStroke,
                                tileBottom - halfStroke,
                                goalStrokePaint
                            )
                        }
                    }

                    if (vanish != null &&
                        vanish.position.row == rowIndex &&
                        vanish.position.col == colIndex
                    ) {
                        if (vanish.step !in 0..VanishSpec.LAST_STEP) {
                            continue
                        }
                        if (VanishVisualSpec.isSpriteStep(vanish.step)) {
                                val targetSize = snapToWholePixel(cellSize * 0.90f)
                                val sizePx = targetSize.toInt()
                                require(sizePx > 0)
                                val left =
                                    snapToWholePixel(tileLeft + (cellSize - targetSize) / 2f)
                                val top =
                                    snapToWholePixel(tileTop + (cellSize - targetSize) / 2f)
                                val bitmap = assets.getBitmap(R.drawable.box, sizePx)
                                canvas.drawBitmap(bitmap, left, top, bitmapPaint)
                        } else {
                            val baseSize =
                                (cellSize * VanishVisualSpec.BASE_SIZE_FACTOR).roundToInt().toFloat()
                            val baseLeft =
                                (tileLeft + (cellSize - baseSize) / 2f).roundToInt().toFloat()
                            val baseTop =
                                (tileTop + (cellSize - baseSize) / 2f).roundToInt().toFloat()
                            val scale = VanishVisualSpec.scale(vanish.step)
                            val size = baseSize * scale
                            val left = baseLeft + (baseSize - size) / 2f
                            val top = baseTop + (baseSize - size) / 2f
                            val cornerRadius = size * VanishVisualSpec.CORNER_RADIUS_FACTOR
                            val color = VanishVisualSpec.colorArgb(vanish.step)
                            vanishPaint.color = color
                            canvas.drawRoundRect(
                                left,
                                top,
                                left + size,
                                top + size,
                                cornerRadius,
                                cornerRadius,
                                vanishPaint
                            )
                        }
                    }
                }
            }

            for (position in scene.boxPositions) {
                val origin = Position(position.row + 1, position.col + 1)
                    .toRenderPoint(cellSize, offsetX, offsetY)
                val targetSize = snapToWholePixel(cellSize * 0.90f)
                val sizePx = targetSize.toInt()
                require(sizePx > 0)
                val left = snapToWholePixel(origin.x + (cellSize - targetSize) / 2f)
                val top = snapToWholePixel(origin.y + (cellSize - targetSize) / 2f)
                val resId =
                    if (scene.selectedBox == position) R.drawable.box_selected else R.drawable.box
                val bitmap = assets.getBitmap(resId, sizePx)
                canvas.drawBitmap(bitmap, left, top, bitmapPaint)
            }

            val origin = Position(scene.playerPosition.row + 1, scene.playerPosition.col + 1)
                .toRenderPoint(cellSize, offsetX, offsetY)
            val targetSize = snapToWholePixel(cellSize * 0.80f)
            val sizePx = targetSize.toInt()
            require(sizePx > 0)
            val left = snapToWholePixel(origin.x + (cellSize - targetSize) / 2f)
            val top = snapToWholePixel(origin.y + (cellSize - targetSize) / 2f)
            val body = assets.getBitmap(R.drawable.player_slime, sizePx)
            val eyesRes =
                if (scene.isBlinking) R.drawable.player_eyes_blink else R.drawable.player_eyes_open
            val eyes = assets.getBitmap(eyesRes, sizePx)
            drawSprite(canvas, body, left, top, sizePx, scene.isFacingLeft, bitmapPaint)
            drawSprite(canvas, eyes, left, top, sizePx, scene.isFacingLeft, bitmapPaint)

            if (scene.boxPathActive && scene.boxPath.size >= 2) {
                val n = scene.boxPath.size
                if (pathXs.size < n) {
                    pathXs = FloatArray(n)
                    pathYs = FloatArray(n)
                }

                for (i in 0 until n) {
                    val position = scene.boxPath[i]
                    pathXs[i] = offsetX + (position.col + 1) * cellSize + cellSize / 2f
                    pathYs[i] = offsetY + (position.row + 1) * cellSize + cellSize / 2f
                }

                val totalSegments = n - 1
                val clampedShrink = scene.boxPathShrink.coerceIn(0f, 1f)
                val startT = totalSegments.toFloat() * clampedShrink
                val startSegment = startT.toInt().coerceIn(0, totalSegments - 1)
                val startFraction = startT - startSegment

                val startX = pathXs[startSegment]
                val startY = pathYs[startSegment]
                val endX = pathXs[startSegment + 1]
                val endY = pathYs[startSegment + 1]
                val startPointX = startX + (endX - startX) * startFraction
                val startPointY = startY + (endY - startY) * startFraction

                var prevX = startPointX
                var prevY = startPointY
                var drewAnySegment = false
                for (i in (startSegment + 1) until n) {
                    val x = pathXs[i]
                    val y = pathYs[i]
                    canvas.drawLine(prevX, prevY, x, y, pathPaint)
                    prevX = x
                    prevY = y
                    drewAnySegment = true
                }

                if (!drewAnySegment) {
                    val radius = (cellSize * 0.2f) / 2f
                    canvas.drawCircle(startPointX, startPointY, radius, pathPaint)
                }
            }
        } finally {
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun drawSprite(
        canvas: Canvas,
        bitmap: Bitmap,
        left: Float,
        top: Float,
        sizePx: Int,
        flipX: Boolean,
        paint: Paint
    ) {
        canvas.save()
        canvas.translate(left, top)
        if (flipX) {
            canvas.scale(-1f, 1f, sizePx / 2f, sizePx / 2f)
        }
        canvas.drawBitmap(bitmap, 0f, 0f, paint)
        canvas.restore()
    }
}
