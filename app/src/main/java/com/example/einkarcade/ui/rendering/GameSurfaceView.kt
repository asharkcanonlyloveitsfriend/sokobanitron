package com.example.einkarcade.ui.rendering

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.BitmapFactory
import android.graphics.Rect
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.screens.VanishState
import com.example.einkarcade.ui.vanish.VanishSpec
import com.example.einkarcade.ui.vanish.VanishVisualSpec
import kotlin.math.ceil
import kotlin.math.floor
import kotlin.math.roundToInt

@SuppressLint("ClickableViewAccessibility")
internal class GameSurfaceView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {
    private var scene: GameScene? = null
    private var isGameWon: Boolean = false
    private var onTapCell: ((Position) -> Unit)? = null
    private var lastViewport: BoardViewport? = null
    private val assets = AndroidGameAssets(context)
    private var backgroundBitmap: Bitmap? = null
    private val backgroundPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { isFilterBitmap = true }
    private val backgroundSrcRect = Rect()
    private val backgroundDstRect = Rect()
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
    private val pathPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFD3D3D3.toInt()
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
    }

    private var pathXs = FloatArray(0)
    private var pathYs = FloatArray(0)
    private var lastSnap: GameSceneSnapshot? = null
    private var backBufferBitmap: Bitmap? = null
    private var backBufferCanvas: Canvas? = null
    private val blitRect = Rect()

    internal data class DirtyRect(val left: Float, val top: Float, val right: Float, val bottom: Float)
    private data class FRect(val l: Float, val t: Float, val r: Float, val b: Float)

    private data class GameSceneSnapshot(
        val player: Position,
        val boxes: Set<Position>,
        val selectedBox: Position?,
        val isBlinking: Boolean,
        val isFacingLeft: Boolean,
        val vanish: VanishState?,
        val boxPathActive: Boolean,
        val boxPath: List<Position>,
        val boxPathShrink: Float,
        val innerRows: Int,
        val innerCols: Int
    )

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

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        backBufferBitmap?.recycle()
        backBufferBitmap = null
        backBufferCanvas = null
        lastSnap = null
        backgroundBitmap?.recycle()
        backgroundBitmap = null
    }

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
        ensureBackBuffer()
        val bufferBitmap = requireNotNull(backBufferBitmap)
        val bufferCanvas = requireNotNull(backBufferCanvas)

        val curSnap = GameSceneSnapshot(
            player = scene.playerPosition,
            boxes = scene.boxPositions.toSet(),
            selectedBox = scene.selectedBox,
            isBlinking = scene.isBlinking,
            isFacingLeft = scene.isFacingLeft,
            vanish = scene.vanish,
            boxPathActive = scene.boxPathActive,
            boxPath = scene.boxPath,
            boxPathShrink = scene.boxPathShrink,
            innerRows = innerRows,
            innerCols = innerCols
        )
        val prevSnap = lastSnap
        if (prevSnap != null && (prevSnap.innerRows != innerRows || prevSnap.innerCols != innerCols)) {
            lastSnap = null
        }
        val dirtyRects = computeDirtyRects(lastSnap, curSnap, viewport)

        if (lastSnap == null) {
            drawScene(bufferCanvas, viewport, scene)
            val canvas = holder.lockCanvas() ?: return
            try {
                canvas.drawBitmap(bufferBitmap, 0f, 0f, null)
            } finally {
                holder.unlockCanvasAndPost(canvas)
            }
            lastSnap = curSnap
            return
        }

        if (dirtyRects.isEmpty()) {
            lastSnap = curSnap
            return
        }

        for (dirtyRect in dirtyRects) {
            bufferCanvas.save()
            bufferCanvas.clipRect(dirtyRect)
            drawScene(bufferCanvas, viewport, scene)
            bufferCanvas.restore()

            val canvas = holder.lockCanvas(dirtyRect) ?: continue
            try {
                blitRect.set(dirtyRect)
                canvas.drawBitmap(bufferBitmap, blitRect, blitRect, null)
            } finally {
                holder.unlockCanvasAndPost(canvas)
            }
        }
        lastSnap = curSnap
    }
    private fun requireBackgroundBitmap(): Bitmap {
        val existing = backgroundBitmap
        if (existing != null && !existing.isRecycled) return existing

        val decoded = BitmapFactory.decodeResource(resources, R.drawable.bg_space)
        require(!decoded.isRecycled)
        backgroundBitmap = decoded
        return decoded
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

    private fun drawBackground(canvas: Canvas) {
        val viewW = width
        val viewH = height
        require(viewW > 0 && viewH > 0)

        val bitmap = requireBackgroundBitmap()
        val bmpW = bitmap.width
        val bmpH = bitmap.height
        require(bmpW > 0 && bmpH > 0)

        val viewAspect = viewW.toFloat() / viewH.toFloat()
        val bmpAspect = bmpW.toFloat() / bmpH.toFloat()

        if (bmpAspect > viewAspect) {
            // Bitmap is wider than the view; crop left/right.
            val srcW = (bmpH * viewAspect).roundToInt().coerceAtMost(bmpW)
            val left = ((bmpW - srcW) / 2f).roundToInt().coerceAtLeast(0)
            backgroundSrcRect.set(left, 0, left + srcW, bmpH)
        } else {
            // Bitmap is taller than the view; crop top/bottom.
            val srcH = (bmpW / viewAspect).roundToInt().coerceAtMost(bmpH)
            val top = ((bmpH - srcH) / 2f).roundToInt().coerceAtLeast(0)
            backgroundSrcRect.set(0, top, bmpW, top + srcH)
        }

        backgroundDstRect.set(0, 0, viewW, viewH)
        canvas.drawBitmap(bitmap, backgroundSrcRect, backgroundDstRect, backgroundPaint)
    }

    private fun drawScene(canvas: Canvas, viewport: BoardViewport, scene: GameScene) {
        drawBackground(canvas)
        val bitmapPaint = assets.bitmapPaint()
        pathPaint.strokeWidth = viewport.cellSize * 0.2f

        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY
        val vanish = scene.vanish
        val halfStroke = floorStrokePaint.strokeWidth / 2f

        for ((rowIndex, row) in scene.tiles.withIndex()) {
            for ((colIndex, tile) in row.withIndex()) {
                val tileLeft = offsetX + (colIndex + 1) * cellSize
                val tileTop = offsetY + (rowIndex + 1) * cellSize
                val tileRight = tileLeft + cellSize
                val tileBottom = tileTop + cellSize
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
                    val spriteRect = vanishRectForStep(0, tileLeft, tileTop, cellSize)
                    val targetSize = spriteRect.r - spriteRect.l
                    val sizePx = targetSize.toInt()
                    require(sizePx > 0)
                    val bitmap = assets.getBitmap(R.drawable.box, sizePx)
                    canvas.drawBitmap(bitmap, spriteRect.l, spriteRect.t, bitmapPaint)

                    if (vanish.step >= 1) {
                        var outer = spriteRect
                        for (s in 1..vanish.step) {
                            val inner = vanishRectForStep(s, tileLeft, tileTop, cellSize)
                            eraseRing(
                                canvas = canvas,
                                viewport = viewport,
                                scene = scene,
                                rowIndex = rowIndex,
                                colIndex = colIndex,
                                outer = outer,
                                inner = inner
                            )
                            outer = inner
                        }
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
    }

    private fun computeDirtyRects(
        prev: GameSceneSnapshot?,
        cur: GameSceneSnapshot,
        viewport: BoardViewport
    ): List<Rect> {
        if (prev == null) return emptyList()

        // If the board dimensions changed, we cannot rely on incremental updates.
        if (prev.innerRows != cur.innerRows || prev.innerCols != cur.innerCols) return emptyList()

        val dirty = mutableListOf<DirtyRect>()

        // Player dirty region.
        if (prev.player != cur.player) {
            dirty.add(cellDirtyRect(prev.player, viewport))
            dirty.add(cellDirtyRect(cur.player, viewport))
        } else {
            if (prev.isBlinking != cur.isBlinking || prev.isFacingLeft != cur.isFacingLeft) {
                dirty.add(cellDirtyRect(cur.player, viewport))
            }
        }

        // Boxes: only redraw boxes that changed position (symmetric difference).
        val removedBoxes = prev.boxes - cur.boxes
        val addedBoxes = cur.boxes - prev.boxes
        val changedBoxes = removedBoxes.union(addedBoxes)
        for (pos in changedBoxes) {
            dirty.add(cellDirtyRect(pos, viewport))
        }

        // Selection affects which bitmap is drawn for a box.
        if (prev.selectedBox != cur.selectedBox) {
            prev.selectedBox?.let { dirty.add(cellDirtyRect(it, viewport)) }
            cur.selectedBox?.let { dirty.add(cellDirtyRect(it, viewport)) }
        }

        // Vanish: update the affected cell(s) when vanish state changes.
        if (prev.vanish != cur.vanish) {
            prev.vanish?.let { dirty.add(cellDirtyRect(it.position, viewport)) }
            cur.vanish?.let { dirty.add(cellDirtyRect(it.position, viewport)) }
        }

        // Box path: if anything about the path changes, redraw a single bounding rect for the path.
        val pathChanged = prev.boxPathActive != cur.boxPathActive ||
            prev.boxPath != cur.boxPath ||
            prev.boxPathShrink != cur.boxPathShrink

        if (pathChanged && (prev.boxPathActive || cur.boxPathActive)) {
            val stroke = viewport.cellSize * 0.2f
            val expand = stroke / 2f + 2f

            var minX = Float.POSITIVE_INFINITY
            var minY = Float.POSITIVE_INFINITY
            var maxX = Float.NEGATIVE_INFINITY
            var maxY = Float.NEGATIVE_INFINITY

            fun accumulate(points: List<Position>) {
                for (position in points) {
                    val centerX =
                        viewport.offsetX + (position.col + 1) * viewport.cellSize +
                            viewport.cellSize / 2f
                    val centerY =
                        viewport.offsetY + (position.row + 1) * viewport.cellSize +
                            viewport.cellSize / 2f
                    minX = minX.coerceAtMost(centerX)
                    minY = minY.coerceAtMost(centerY)
                    maxX = maxX.coerceAtLeast(centerX)
                    maxY = maxY.coerceAtLeast(centerY)
                }
            }

            if (prev.boxPathActive) accumulate(prev.boxPath)
            if (cur.boxPathActive) accumulate(cur.boxPath)

            if (minX != Float.POSITIVE_INFINITY) {
                dirty.add(
                    DirtyRect(
                        left = minX - expand,
                        top = minY - expand,
                        right = maxX + expand,
                        bottom = maxY + expand
                    )
                )
            }
        }

        return dirty.mapNotNull { rect ->
            toIntRect(rect)
        }
    }

    private fun cellDirtyRect(position: Position, viewport: BoardViewport): DirtyRect {
        val paddedCol = position.col + 1
        val paddedRow = position.row + 1
        val tileLeft = viewport.offsetX + paddedCol * viewport.cellSize
        val tileTop = viewport.offsetY + paddedRow * viewport.cellSize
        val tileRight = tileLeft + viewport.cellSize
        val tileBottom = tileTop + viewport.cellSize
        return DirtyRect(
            left = tileLeft - 2f,
            top = tileTop - 2f,
            right = tileRight + 2f,
            bottom = tileBottom + 2f
        )
    }

    private fun toIntRect(dirty: DirtyRect): Rect? {
        val left = floor(dirty.left).toInt().coerceIn(0, width)
        val top = floor(dirty.top).toInt().coerceIn(0, height)
        val right = ceil(dirty.right).toInt().coerceIn(0, width)
        val bottom = ceil(dirty.bottom).toInt().coerceIn(0, height)
        if (right <= left || bottom <= top) return null
        return Rect(left, top, right, bottom)
    }

    private fun vanishRectForStep(step: Int, tileLeft: Float, tileTop: Float, cellSize: Float): FRect {
        require(step in 0..VanishSpec.LAST_STEP) { "Vanish step out of range: $step" }
        return if (step == 0) {
            val targetSize = snapToWholePixel(cellSize * 0.90f)
            val left = snapToWholePixel(tileLeft + (cellSize - targetSize) / 2f)
            val top = snapToWholePixel(tileTop + (cellSize - targetSize) / 2f)
            FRect(left, top, left + targetSize, top + targetSize)
        } else {
            val baseSize = (cellSize * VanishVisualSpec.BASE_SIZE_FACTOR).roundToInt().toFloat()
            val baseLeft = (tileLeft + (cellSize - baseSize) / 2f).roundToInt().toFloat()
            val baseTop = (tileTop + (cellSize - baseSize) / 2f).roundToInt().toFloat()
            val scale = VanishVisualSpec.scale(step)
            val size = baseSize * scale
            val left = baseLeft + (baseSize - size) / 2f
            val top = baseTop + (baseSize - size) / 2f
            FRect(left, top, left + size, top + size)
        }
    }


    private fun eraseRing(
        canvas: Canvas,
        viewport: BoardViewport,
        scene: GameScene,
        rowIndex: Int,
        colIndex: Int,
        outer: FRect,
        inner: FRect
    ) {
        val bands = arrayOf(
            FRect(outer.l, outer.t, outer.r, inner.t),
            FRect(outer.l, inner.b, outer.r, outer.b),
            FRect(outer.l, inner.t, inner.l, inner.b),
            FRect(inner.r, inner.t, outer.r, inner.b)
        )
        val tile = scene.tiles[rowIndex][colIndex]
        val cellSize = viewport.cellSize
        val tileLeft = viewport.offsetX + (colIndex + 1) * cellSize
        val tileTop = viewport.offsetY + (rowIndex + 1) * cellSize
        val tileRight = tileLeft + cellSize
        val tileBottom = tileTop + cellSize
        val halfStroke = floorStrokePaint.strokeWidth / 2f

        for (band in bands) {
            if (band.r <= band.l || band.b <= band.t) continue
            canvas.save()
            canvas.clipRect(band.l, band.t, band.r, band.b)
            drawBackground(canvas)
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
            canvas.restore()
        }
    }

    private fun ensureBackBuffer() {
        require(width > 0 && height > 0)
        val existing = backBufferBitmap
        if (existing != null && existing.width == width && existing.height == height) return

        existing?.recycle()

        val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        backBufferBitmap = bitmap
        backBufferCanvas = Canvas(bitmap)
        lastSnap = null
    }
}
