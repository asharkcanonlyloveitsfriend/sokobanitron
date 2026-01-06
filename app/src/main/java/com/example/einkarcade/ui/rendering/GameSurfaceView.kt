package com.example.einkarcade.ui.rendering

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.RectF
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.os.SystemClock
import com.example.einkarcade.GameController
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.vanish.VanishSpec
import com.example.einkarcade.ui.vanish.VanishVisualSpec
import kotlin.math.roundToInt
import androidx.core.graphics.withTranslation

internal data class LevelInit(
    val tiles: List<List<Tile>>,
    val playerPosition: Position,
    val boxPositions: Set<Position>
)

@SuppressLint("ClickableViewAccessibility")
internal class GameSurfaceView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {
    private var tiles: List<List<Tile>> = emptyList()
    private var boxPositions: Set<Position> = emptySet()
    private var playerPosition: Position? = null
    private var displayedPlayerPosition: Position? = null
    private var pendingPlayerPosition: Position? = null
    private var selectedBox: Position? = null
    private var isFacingLeft: Boolean = false
    private var pendingFacingLeft: Boolean? = null
    private var isInitialized: Boolean = false
    private var onTapCell: ((Position) -> Unit)? = null
    private var lastViewport: BoardViewport? = null
    private var boxPath: List<Position> = emptyList()
    private var boxPathActive: Boolean = false
    private var boxPathShrink: Float = 0f
    private var boxPathStartMs: Long = 0L
    private val boxPathDurationMs: Long = 100L
    private var blinkStartMs: Long = 0L
    private var blinkEndMs: Long = 0L
    private var lastBlinkActive: Boolean = false
    private var vanishPosition: Position? = null
    private var vanishStartMs: Long = 0L
    private var vanishStep: Int? = null
    private val assets = AndroidGameAssets(context)
    private var backgroundBitmap: Bitmap? = null
    private var staticFrameBitmap: Bitmap? = null
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
    private val boxPathPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFD3D3D3.toInt()
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
    }
    private val boxPathDotPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFD3D3D3.toInt()
        style = Paint.Style.FILL
    }
    private val vanishRectPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }
    private val vanishRect = RectF()
    private val animationFrameRunnable = object : Runnable {
        override fun run() {
            val now = SystemClock.elapsedRealtime()
            val changed = updateBoxPathAnimation(now)
            val vanishChanged = updateVanishAnimation(now)
            val blinkActive = isBlinking(now)
            val pendingBlink = !blinkActive && blinkStartMs > now
            if (blinkActive != lastBlinkActive) {
                lastBlinkActive = blinkActive
                if (!changed && !vanishChanged) {
                    render()
                }
            }
            if (changed) {
                render()
            }
            if (vanishChanged && !changed) {
                render()
            }
            val vanishActive = vanishPosition != null
            if (boxPathActive || blinkActive || vanishActive) {
                postOnAnimation(this)
            } else if (pendingBlink) {
                val delay = (blinkStartMs - now).coerceAtLeast(0L)
                postDelayed(this, delay)
            }
        }
    }
    init {
        holder.addCallback(this)
        setOnTouchListener { _, event ->
            if (event.action == MotionEvent.ACTION_UP) {
                val viewport = lastViewport ?: return@setOnTouchListener true
                val position = viewport.screenToInnerCell(event.x, event.y)
                if (position != null) {
                    onTapCell?.invoke(position)
                }
                return@setOnTouchListener true
            }
            true
        }
    }

    fun setOnTapCell(onTapCell: (Position) -> Unit) {
        this.onTapCell = onTapCell
    }

    fun applyDelta(delta: GameController.RenderDelta) {
        when (delta) {
            is GameController.RenderDelta.LevelLoaded -> {
                loadLevel(
                    LevelInit(
                        tiles = delta.tiles,
                        playerPosition = delta.playerPosition,
                        boxPositions = delta.boxPositions
                    )
                )
            }
            is GameController.RenderDelta.PlayerMoved -> onPlayerMoved(to = delta.to)
            is GameController.RenderDelta.BoxMoved -> onBoxMoved(path = delta.path)
            is GameController.RenderDelta.MoveRejected -> onMoveRejected()
            is GameController.RenderDelta.GameWon -> onGameWon(isClean = delta.isClean)
        }
    }

    fun loadLevel(init: LevelInit) {
        tiles = init.tiles.map { it.toList() }
        boxPositions = init.boxPositions.toSet()
        playerPosition = init.playerPosition
        displayedPlayerPosition = init.playerPosition
        pendingPlayerPosition = null
        selectedBox = null
        resetFacing()
        boxPath = emptyList()
        boxPathActive = false
        boxPathShrink = 0f
        blinkStartMs = 0L
        blinkEndMs = 0L
        lastBlinkActive = false
        vanishPosition = null
        vanishStep = null
        vanishStartMs = 0L
        isInitialized = true
        rebuildStaticFrameIfPossible()
        render()
    }

    fun setSelectedBox(selected: Position?) {
        if (!isInitialized) return
        selectedBox = selected
        render()
    }

    fun getSelectedBox(): Position? = selectedBox

    fun onPlayerMoved(to: Position) {
        if (!isInitialized) return

        val from = playerPosition
        resetFacing()
        playerPosition = to
        displayedPlayerPosition = to

        // Phase 1: player teleports -> redraw only old+new player sprite regions.
        // If we don't have a cached static frame / viewport (or animations are active), fall back.
        if (from != null && !boxPathActive) {
            val viewport = lastViewport
            val staticFrame = staticFrameBitmap
            if (viewport != null && staticFrame != null && !staticFrame.isRecycled) {
                val fromParams = spriteDrawParams(viewport, from, 0.80f)
                val toParams = spriteDrawParams(viewport, to, 0.80f)
                val dirty = Rect(fromParams.dirtyRect)
                dirty.union(toParams.dirtyRect)
                renderDirty(dirty)
                return
            }
        }

        render()
    }

    fun onMoveRejected() {
        if (!isInitialized) return
        triggerBlink()
    }

    fun onGameWon(isClean: Boolean) {
        if (!isInitialized) return
        if (!isClean) {
            triggerBlink()
        }
    }

    fun onBoxMoved(path: List<Position>) {
        if (!isInitialized) return
        if (path.size < 2) return
        val from = path.first()
        val to = path.last()
        if (tiles[to.row][to.col] == Tile.WALL) {
            boxPositions = boxPositions - from
            startVanishBoxAnimation(at = to)
            triggerBlink()
        } else {
            boxPositions = (boxPositions - from) + to
        }
        for (i in path.size - 1 downTo 1) {
            val prev = path[i - 1]
            val curr = path[i]
            if (curr.col != prev.col) {
                pendingFacingLeft = curr.col < prev.col
                break
            }
        }
        displayedPlayerPosition = playerPosition
        playerPosition = path[path.size - 2]
        startBoxPathAnimation(path, playerPosition ?: path[path.size - 2])
        render()
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        if (isInitialized) {
            rebuildStaticFrameIfPossible()
            render()
        }
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (isInitialized) {
            rebuildStaticFrameIfPossible()
            render()
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        removeCallbacks(animationFrameRunnable)
        staticFrameBitmap?.recycle()
        staticFrameBitmap = null
        backgroundBitmap?.recycle()
        backgroundBitmap = null
    }

    private fun render() {
        if (width <= 0 || height <= 0) return
        if (!isInitialized) return
        val playerPosition = if (boxPathActive) {
            displayedPlayerPosition ?: playerPosition!!
        } else {
            playerPosition!!
        }
        val innerRows = tiles.size
        val innerCols = tiles.first().size
        val viewport = computeBoardViewport(width.toFloat(), height.toFloat(), innerRows, innerCols)
        lastViewport = viewport

        if (!holder.surface.isValid) return
        val canvas = holder.lockCanvas() ?: return
        try {
            drawScene(
                canvas = canvas,
                viewport = viewport,
                tiles = tiles,
                boxPositions = boxPositions,
                playerPosition = playerPosition,
                selectedBox = selectedBox,
                isFacingLeft = isFacingLeft
            )
        } finally {
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private data class SpriteDrawParams(
        val left: Float,
        val top: Float,
        val sizePx: Int,
        val dirtyRect: Rect
    )

    private fun spriteDrawParams(
        viewport: BoardViewport,
        position: Position,
        sizeFactor: Float
    ): SpriteDrawParams {
        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY

        val origin = Position(position.row + 1, position.col + 1)
            .toRenderPoint(cellSize, offsetX, offsetY)

        val targetSize = snapToWholePixel(cellSize * sizeFactor)
        val sizePx = targetSize.toInt().coerceAtLeast(1)
        val left = snapToWholePixel(origin.x + (cellSize - targetSize) / 2f)
        val top = snapToWholePixel(origin.y + (cellSize - targetSize) / 2f)

        // Padding accounts for bitmap filtering, anti-aliasing, and pixel rounding.
        val paddingPx = 6
        val dirtyRect = Rect(
            left.toInt() - paddingPx,
            top.toInt() - paddingPx,
            (left + sizePx).toInt() + paddingPx,
            (top + sizePx).toInt() + paddingPx
        )

        return SpriteDrawParams(left = left, top = top, sizePx = sizePx, dirtyRect = dirtyRect)
    }

    private fun rebuildStaticFrameIfPossible() {
        if (width <= 0 || height <= 0) return
        if (!isInitialized) return
        if (tiles.isEmpty()) return
        val firstRow = tiles.firstOrNull() ?: return
        if (firstRow.isEmpty()) return

        val existing = staticFrameBitmap
        if (existing != null && !existing.isRecycled && existing.width == width && existing.height == height) {
            return
        }

        existing?.recycle()

        val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        val bitmapCanvas = Canvas(bitmap)

        val innerRows = tiles.size
        val innerCols = tiles.first().size
        val viewport = computeBoardViewport(width.toFloat(), height.toFloat(), innerRows, innerCols)
        // Keep this consistent with full render so touch mapping stays correct.
        lastViewport = viewport

        drawBackground(bitmapCanvas)

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
                drawTileCell(bitmapCanvas, tile, tileLeft, tileTop, tileRight, tileBottom, halfStroke)
            }
        }

        staticFrameBitmap = bitmap
    }

    private fun renderDirty(requestedDirtyRect: Rect) {
        if (width <= 0 || height <= 0) return
        if (!isInitialized) return

        // Phase 1 scope: don't attempt partial redraws during path/vanish animations.
        if (boxPathActive || vanishPosition != null) {
            render()
            return
        }

        // Ensure we have a valid static frame to restore from.
        if (staticFrameBitmap == null || staticFrameBitmap?.isRecycled == true) {
            rebuildStaticFrameIfPossible()
        }
        val staticFrame = staticFrameBitmap
        val viewport = lastViewport
        if (staticFrame == null || staticFrame.isRecycled || viewport == null) {
            render()
            return
        }
        if (staticFrame.width != width || staticFrame.height != height) {
            rebuildStaticFrameIfPossible()
        }
        val frame = staticFrameBitmap
        if (frame == null || frame.isRecycled) {
            render()
            return
        }

        val dirtyRect = Rect(requestedDirtyRect)
        if (!dirtyRect.intersect(0, 0, width, height)) return
        if (!holder.surface.isValid) return

        val playerPos = playerPosition ?: return

        val canvas = holder.lockCanvas(dirtyRect) ?: return
        try {
            canvas.save()
            canvas.clipRect(dirtyRect)

            // Restore static pixels (background + tiles) for this region.
            val src = Rect(dirtyRect)
            val dst = Rect(dirtyRect)
            canvas.drawBitmap(frame, src, dst, null)

            // Draw dynamic overlays that intersect the dirty region.
            drawDynamicScene(
                canvas = canvas,
                viewport = viewport,
                dirtyRect = dirtyRect,
                playerPosition = playerPos
            )
        } finally {
            canvas.restore()
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun drawDynamicScene(
        canvas: Canvas,
        viewport: BoardViewport,
        dirtyRect: Rect,
        playerPosition: Position
    ) {
        val bitmapPaint = assets.bitmapPaint()

        // Boxes
        for (position in boxPositions) {
            val params = spriteDrawParams(viewport, position, 0.90f)
            if (!Rect.intersects(dirtyRect, params.dirtyRect)) continue
            val resId = if (selectedBox == position) R.drawable.box_selected else R.drawable.box
            val bitmap = assets.getBitmap(resId, params.sizePx)
            canvas.drawBitmap(bitmap, params.left, params.top, bitmapPaint)
        }

        // Player
        val playerParams = spriteDrawParams(viewport, playerPosition, 0.80f)
        if (Rect.intersects(dirtyRect, playerParams.dirtyRect)) {
            val body = assets.getBitmap(R.drawable.player_slime, playerParams.sizePx)
            val eyesRes = if (isBlinking(SystemClock.elapsedRealtime())) {
                R.drawable.player_eyes_blink
            } else {
                R.drawable.player_eyes_open
            }
            val eyes = assets.getBitmap(eyesRes, playerParams.sizePx)
            drawSprite(canvas, body, playerParams.left, playerParams.top, playerParams.sizePx, isFacingLeft, bitmapPaint)
            drawSprite(canvas, eyes, playerParams.left, playerParams.top, playerParams.sizePx, isFacingLeft, bitmapPaint)
        }
    }

    private fun resetFacing() {
        isFacingLeft = false
        pendingFacingLeft = null
    }
    private fun requireBackgroundBitmap(): Bitmap {
        val existing = backgroundBitmap
        if (existing != null && !existing.isRecycled) return existing

        val decoded = BitmapFactory.decodeResource(resources, R.drawable.bg_space)
        require(!decoded.isRecycled)
        backgroundBitmap = decoded
        return decoded
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


    private fun drawSprite(
        canvas: Canvas,
        bitmap: Bitmap,
        left: Float,
        top: Float,
        sizePx: Int,
        flipX: Boolean,
        paint: Paint
    ) {
        canvas.withTranslation(left, top) {
            if (flipX) {
                scale(-1f, 1f, sizePx / 2f, sizePx / 2f)
            }
            drawBitmap(bitmap, 0f, 0f, paint)
        }
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

    private fun drawScene(
        canvas: Canvas,
        viewport: BoardViewport,
        tiles: List<List<Tile>>,
        boxPositions: Set<Position>,
        playerPosition: Position,
        selectedBox: Position?,
        isFacingLeft: Boolean
    ) {
        drawBackground(canvas)
        val bitmapPaint = assets.bitmapPaint()
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
                if (tile == Tile.WALL) {
                    drawVanishingBox(
                        canvas = canvas,
                        viewport = viewport,
                        gridPosition = Position(rowIndex, colIndex),
                        paddedPosition = Position(rowIndex + 1, colIndex + 1)
                    )
                }
            }
        }

        if (boxPathActive) {
            drawBoxPathLine(
                canvas = canvas,
                viewport = viewport,
                path = boxPath,
                shrink = boxPathShrink
            )
        }

        for (position in boxPositions) {
            val origin = Position(position.row + 1, position.col + 1)
                .toRenderPoint(cellSize, offsetX, offsetY)
            val targetSize = snapToWholePixel(cellSize * 0.90f)
            val sizePx = targetSize.toInt()
            require(sizePx > 0)
            val left = snapToWholePixel(origin.x + (cellSize - targetSize) / 2f)
            val top = snapToWholePixel(origin.y + (cellSize - targetSize) / 2f)
            val resId =
                if (selectedBox == position) R.drawable.box_selected else R.drawable.box
            val bitmap = assets.getBitmap(resId, sizePx)
            canvas.drawBitmap(bitmap, left, top, bitmapPaint)
        }

        val origin = Position(playerPosition.row + 1, playerPosition.col + 1)
            .toRenderPoint(cellSize, offsetX, offsetY)
        val targetSize = snapToWholePixel(cellSize * 0.80f)
        val sizePx = targetSize.toInt()
        require(sizePx > 0)
        val left = snapToWholePixel(origin.x + (cellSize - targetSize) / 2f)
        val top = snapToWholePixel(origin.y + (cellSize - targetSize) / 2f)
        val body = assets.getBitmap(R.drawable.player_slime, sizePx)
        val eyesRes = if (isBlinking(SystemClock.elapsedRealtime())) {
            R.drawable.player_eyes_blink
        } else {
            R.drawable.player_eyes_open
        }
        val eyes = assets.getBitmap(eyesRes, sizePx)
        drawSprite(canvas, body, left, top, sizePx, isFacingLeft, bitmapPaint)
        drawSprite(canvas, eyes, left, top, sizePx, isFacingLeft, bitmapPaint)

    }

    private fun startVanishBoxAnimation(at: Position) {
        vanishPosition = at
        vanishStartMs = SystemClock.elapsedRealtime()
        vanishStep = 0
        removeCallbacks(animationFrameRunnable)
        postOnAnimation(animationFrameRunnable)
    }

    private fun startBoxPathAnimation(path: List<Position>, pendingPlayer: Position) {
        require(path.size >= 2) { "Box path requires at least two points." }
        boxPath = path
        pendingPlayerPosition = pendingPlayer
        boxPathStartMs = SystemClock.elapsedRealtime()
        boxPathShrink = 0f
        boxPathActive = true
        removeCallbacks(animationFrameRunnable)
        postOnAnimation(animationFrameRunnable)
    }

    private fun updateBoxPathAnimation(nowMs: Long): Boolean {
        if (!boxPathActive) return false
        val elapsed = nowMs - boxPathStartMs
        val progress = (elapsed.toFloat() / boxPathDurationMs.toFloat()).coerceAtMost(1f)
        var changed = false
        if (progress != boxPathShrink) {
            boxPathShrink = progress
            changed = true
        }
        if (elapsed >= boxPathDurationMs) {
            boxPathActive = false
            val pending = pendingPlayerPosition
            if (pending != null) {
                displayedPlayerPosition = pending
                pendingPlayerPosition = null
            }
            pendingFacingLeft?.let { facing ->
                isFacingLeft = facing
            }
            pendingFacingLeft = null
            changed = true
        }
        return changed
    }

    private fun updateVanishAnimation(nowMs: Long): Boolean {
        if (vanishPosition == null) return false
        val elapsed = nowMs - vanishStartMs
        var cumulative = 0L

        for (step in 0..VanishSpec.LAST_STEP) {
            val delay = VanishSpec.delayMs(step)
            if (elapsed < cumulative + delay) {
                if (vanishStep != step) {
                    vanishStep = step
                    return true
                }
                return false
            }
            cumulative += delay
        }

        if (vanishStep != null) {
            vanishStep = null
            vanishPosition = null
            return true
        }

        return false
    }

    private fun isBlinking(nowMs: Long): Boolean {
        return nowMs in blinkStartMs until blinkEndMs
    }

    private fun triggerBlink() {
        val nowMs = SystemClock.elapsedRealtime()
        val start = nowMs + 400L
        blinkStartMs = start
        blinkEndMs = start + 300L
        lastBlinkActive = false
        val delay = (blinkStartMs - nowMs).coerceAtLeast(0L)
        removeCallbacks(animationFrameRunnable)
        if (boxPathActive || delay == 0L) {
            postOnAnimation(animationFrameRunnable)
        } else {
            postDelayed(animationFrameRunnable, delay)
        }
    }

    private fun drawBoxPathLine(
        canvas: Canvas,
        viewport: BoardViewport,
        path: List<Position>,
        shrink: Float
    ) {
        if (path.size < 2) return
        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY
        val strokeWidth = cellSize * 0.2f
        boxPathPaint.strokeWidth = strokeWidth

        val points = path.map { position ->
            val cx = offsetX + (position.col + 1) * cellSize + cellSize / 2f
            val cy = offsetY + (position.row + 1) * cellSize + cellSize / 2f
            android.graphics.PointF(cx, cy)
        }

        val totalSegments = points.size - 1
        val startT = totalSegments.toFloat() * shrink.coerceIn(0f, 1f)
        val startSegment = startT.toInt().coerceIn(0, totalSegments - 1)
        val startFraction = startT - startSegment

        fun interpolate(start: android.graphics.PointF, end: android.graphics.PointF, t: Float): android.graphics.PointF {
            return android.graphics.PointF(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t
            )
        }

        val startPoint = interpolate(points[startSegment], points[startSegment + 1], startFraction)

        var prev = startPoint
        var drewAnySegment = false
        for (index in (startSegment + 1) until points.size) {
            val next = points[index]
            canvas.drawLine(prev.x, prev.y, next.x, next.y, boxPathPaint)
            prev = next
            drewAnySegment = true
        }

        if (!drewAnySegment) {
            canvas.drawCircle(startPoint.x, startPoint.y, strokeWidth / 2f, boxPathDotPaint)
        }
    }

    private fun drawVanishingBox(
        canvas: Canvas,
        viewport: BoardViewport,
        gridPosition: Position,
        paddedPosition: Position
    ) {
        val currentPosition = vanishPosition ?: return
        val step = vanishStep ?: return
        if (currentPosition != gridPosition) return
        require(step in 0..VanishSpec.LAST_STEP) { "Vanish step out of range: $step" }

        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY
        val tileLeft = offsetX + paddedPosition.col * cellSize
        val tileTop = offsetY + paddedPosition.row * cellSize
        val baseSize = (cellSize * VanishVisualSpec.BASE_SIZE_FACTOR).roundToInt().toFloat()
        val baseLeft = (tileLeft + (cellSize - baseSize) / 2f).roundToInt().toFloat()
        val baseTop = (tileTop + (cellSize - baseSize) / 2f).roundToInt().toFloat()
        val scale = VanishVisualSpec.scale(step)
        val size = baseSize * scale
        val left = baseLeft + (baseSize - size) / 2f
        val top = baseTop + (baseSize - size) / 2f
        val innerRadius = size * VanishVisualSpec.CORNER_RADIUS_FACTOR

        if (VanishVisualSpec.isSpriteStep(step)) {
            val origin = paddedPosition.toRenderPoint(cellSize, offsetX, offsetY)
            val targetSize = snapToWholePixel(cellSize * 0.90f)
            val sizePx = targetSize.toInt()
            require(sizePx > 0)
            val leftPx = snapToWholePixel(origin.x + (cellSize - targetSize) / 2f)
            val topPx = snapToWholePixel(origin.y + (cellSize - targetSize) / 2f)
            val bitmap = assets.getBitmap(R.drawable.box, sizePx)
            canvas.drawBitmap(bitmap, leftPx, topPx, assets.bitmapPaint())
        } else {
            vanishRectPaint.color = VanishVisualSpec.colorArgb(step)
            vanishRect.set(left, top, left + size, top + size)
            canvas.drawRoundRect(vanishRect, innerRadius, innerRadius, vanishRectPaint)
        }
    }
}
