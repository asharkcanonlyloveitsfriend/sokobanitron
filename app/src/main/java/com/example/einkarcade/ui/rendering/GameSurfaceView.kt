package com.example.einkarcade.ui.rendering

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.PorterDuff
import android.graphics.PorterDuffColorFilter
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
    private var boxPathDirtyRect: Rect? = null
    private var boxPathNeedsFinalClear: Boolean = false
    private val boxPathDurationMs: Long = 125L
    private val boxPathDelayMs: Long = 200L
    private var boxPathSuppressLine: Boolean = false
    private var playerSilhouettePosition: Position? = null
    private var playerSilhouetteStartMs: Long = 0L
    private var playerFlashPosition: Position? = null
    private var playerFlashStartMs: Long = 0L
    private var boxFlashPosition: Position? = null
    private var boxFlashStartMs: Long = 0L
    private var blinkStartMs: Long = 0L
    private var blinkEndMs: Long = 0L
    private var lastBlinkActive: Boolean = false
    private var vanishPosition: Position? = null
    private var vanishStartMs: Long = 0L
    private var vanishStep: Int? = null
    private var vanishLastPosition: Position? = null
    private var vanishNeedsFinalClear: Boolean = false
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
        strokeJoin = Paint.Join.ROUND
    }
    private val boxPathTailPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFF2F2F2.toInt()
        style = Paint.Style.FILL
    }
    private val playerSilhouetteDarkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFF8E8E8E.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val playerSilhouetteLightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFFF2F2F2.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val playerFlashDarkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFF8E8E8E.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val playerFlashLightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFFF2F2F2.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val boxFlashDarkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFF8E8E8E.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val boxFlashLightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFFF2F2F2.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val boxPathDotPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFD3D3D3.toInt()
        style = Paint.Style.FILL
    }
    private val vanishRectPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }
    private val vanishRect = RectF()
    private val blinkStartRunnable = Runnable {
        renderBlinkDirty()
        postOnAnimation(animationFrameRunnable)
    }
    private val animationFrameRunnable = object : Runnable {
        override fun run() {
            val now = SystemClock.elapsedRealtime()
            val changed = updateBoxPathAnimation(now)
            val vanishChanged = updateVanishAnimation(now)
            val blinkActive = isBlinking(now)
            val pendingBlink = !blinkActive && blinkStartMs > now
            val playerFlashActive =
                playerFlashPosition != null && (now - playerFlashStartMs) <= 200L
            if (blinkActive != lastBlinkActive) {
                lastBlinkActive = blinkActive
                if (!changed && !vanishChanged) {
                    if (boxPathActive || boxPathNeedsFinalClear) {
                        renderBoxPathOrFull()
                    } else {
                        renderBlinkDirty()
                    }
                }
            }
            if (changed) {
                renderBoxPathOrFull()
            }
            if (vanishChanged && !changed) {
                if (boxPathActive || boxPathNeedsFinalClear) {
                    renderBoxPathOrFull()
                } else {
                    renderVanishDirty()
                }
            }
            if (!changed && !vanishChanged && !blinkActive && playerFlashActive && !boxPathActive) {
                renderPlayerFlashDirty()
            }
            if (!playerFlashActive && playerFlashPosition != null && !boxPathActive) {
                val clearedPos = playerFlashPosition
                playerFlashPosition = null
                playerFlashStartMs = 0L
                if (clearedPos != null) {
                    val viewport = checkNotNull(lastViewport) { "Dirty render requested without viewport." }
                    val rect = spriteDrawParams(viewport, clearedPos, 0.80f).dirtyRect
                    renderDirty(rect)
                }
            }
            if (vanishNeedsFinalClear && vanishPosition == null) {
                renderVanishDirty()
                vanishNeedsFinalClear = false
                vanishLastPosition = null
            }
            // If we just completed a box-path animation, drop the cached dirty rect after the first
            // post-animation draw clears the line.
            if (boxPathNeedsFinalClear && !boxPathActive) {
                boxPathDirtyRect = null
                boxPathNeedsFinalClear = false
            }
            val vanishActive = vanishPosition != null
            if (boxPathActive || blinkActive || vanishActive) {
                postOnAnimation(this)
            } else if (pendingBlink) {
                val delay = (blinkStartMs - now).coerceAtLeast(0L)
                postDelayed(this, delay)
            } else if (playerFlashActive) {
                postOnAnimation(this)
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
        boxPathDirtyRect = null
        boxPathNeedsFinalClear = false
        boxPathSuppressLine = false
        boxFlashPosition = null
        boxFlashStartMs = 0L
        playerSilhouettePosition = null
        playerSilhouetteStartMs = 0L
        playerFlashPosition = null
        playerFlashStartMs = 0L
        blinkStartMs = 0L
        blinkEndMs = 0L
        lastBlinkActive = false
        vanishPosition = null
        vanishStep = null
        vanishStartMs = 0L
        vanishLastPosition = null
        vanishNeedsFinalClear = false
        isInitialized = true
        rebuildStaticFrameIfPossible()
        render()
    }

    fun setSelectedBox(selected: Position?) {
        if (!isInitialized) return
        val previous = selectedBox
        if (previous == selected) return

        selectedBox = selected

        val viewport = lastViewport
        checkNotNull(viewport) { "Dirty render requested without viewport." }
        var dirty: Rect? = null
        if (previous != null) {
            dirty = Rect(spriteDrawParams(viewport, previous, 0.90f).dirtyRect)
        }
        if (selected != null) {
            val rect = spriteDrawParams(viewport, selected, 0.90f).dirtyRect
            if (dirty == null) {
                dirty = Rect(rect)
            } else {
                dirty.union(rect)
            }
        }
        if (dirty == null) return
        renderDirty(dirty)
    }

    fun getSelectedBox(): Position? = selectedBox

    fun onPlayerMoved(to: Position) {
        if (!isInitialized) return

        val from = playerPosition
        resetFacing()
        playerPosition = to
        displayedPlayerPosition = to
        playerFlashPosition = from
        playerFlashStartMs = SystemClock.elapsedRealtime()
        removeCallbacks(animationFrameRunnable)
        postOnAnimation(animationFrameRunnable)

        checkNotNull(from) { "Dirty render requested without previous player position." }
        val viewport = checkNotNull(lastViewport) { "Dirty render requested without viewport." }
        val fromParams = spriteDrawParams(viewport, from, 0.80f)
        val toParams = spriteDrawParams(viewport, to, 0.80f)
        val dirty = Rect(fromParams.dirtyRect)
        dirty.union(toParams.dirtyRect)
        renderDirty(dirty)
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
        playerSilhouettePosition = displayedPlayerPosition
        playerSilhouetteStartMs = SystemClock.elapsedRealtime()
        boxFlashPosition = from
        boxFlashStartMs = playerSilhouetteStartMs
        boxPathSuppressLine = path.size == 2
        startBoxPathAnimation(path, playerPosition ?: path[path.size - 2])
        renderBoxPathOrFull()
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
                isFacingLeft = isFacingLeft,
                drawPlayer = true
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

        check(!boxPathActive) { "Dirty render requested during box path animation." }

        val viewport = lastViewport
        checkNotNull(viewport) { "Dirty render requested without viewport." }

        val dirtyRect = Rect(requestedDirtyRect)
        if (!dirtyRect.intersect(0, 0, width, height)) return
        if (!holder.surface.isValid) return

        val playerPos = playerPosition ?: return

        val canvas = holder.lockCanvas(dirtyRect) ?: return
        try {
            canvas.save()
            canvas.clipRect(dirtyRect)

            drawScene(
                canvas = canvas,
                viewport = viewport,
                tiles = tiles,
                boxPositions = boxPositions,
                playerPosition = playerPos,
                selectedBox = selectedBox,
                isFacingLeft = isFacingLeft,
                drawPlayer = true
            )
            if (!boxPathActive && playerFlashPosition != null) {
                val nowMs = SystemClock.elapsedRealtime()
                val elapsedMs = nowMs - playerFlashStartMs
                if (elapsedMs <= 200L) {
                    val flashPos = playerFlashPosition
                    if (flashPos != null) {
                        val params = spriteDrawParams(viewport, flashPos, 0.80f)
                        val body = assets.getBitmap(R.drawable.player_slime, params.sizePx)
                        val paint = if (elapsedMs <= 100L) playerFlashDarkPaint else playerFlashLightPaint
                        drawSprite(
                            canvas,
                            body,
                            params.left,
                            params.top,
                            params.sizePx,
                            isFacingLeft,
                            paint
                        )
                    }
                } else {
                    playerFlashPosition = null
                    playerFlashStartMs = 0L
                }
            }
        } finally {
            canvas.restore()
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun renderBoxPathOrFull() {
        val viewport = lastViewport
        checkNotNull(viewport) { "Dirty render requested without viewport." }
        val boxDirty = boxPathDirtyRect
        val vanishTarget = if (vanishPosition != null || vanishNeedsFinalClear) {
            vanishPosition ?: vanishLastPosition
        } else {
            null
        }
        val vanishDirty = vanishTarget?.let { computeVanishDirtyRect(viewport, it) }
        var dirty = when {
            boxDirty != null && vanishDirty != null -> Rect(boxDirty).apply { union(vanishDirty) }
            boxDirty != null -> Rect(boxDirty)
            vanishDirty != null -> Rect(vanishDirty)
            else -> null
        }
        val nowMs = SystemClock.elapsedRealtime()
        if (boxFlashPosition != null && nowMs - boxFlashStartMs <= 200L) {
            val flashRect = spriteDrawParams(viewport, boxFlashPosition!!, 0.90f).dirtyRect
            dirty = if (dirty == null) {
                Rect(flashRect)
            } else {
                dirty.apply { union(flashRect) }
            }
        }
        val silhouettePos = playerSilhouettePosition
        if (silhouettePos != null) {
            val silhouetteRect = spriteDrawParams(viewport, silhouettePos, 0.80f).dirtyRect
            dirty = if (dirty == null) {
                Rect(silhouetteRect)
            } else {
                dirty.apply { union(silhouetteRect) }
            }
        }
        val playerFlashPos = playerFlashPosition
        if (playerFlashPos != null && nowMs - playerFlashStartMs <= 200L) {
            val flashRect = spriteDrawParams(viewport, playerFlashPos, 0.80f).dirtyRect
            dirty = if (dirty == null) {
                Rect(flashRect)
            } else {
                dirty.apply { union(flashRect) }
            }
        }
        if (dirty == null) {
            error("Dirty render requested without dirty rect.")
        }
        renderDirtyForBoxPath(dirty)
    }

    private fun renderDirtyForBoxPath(requestedDirtyRect: Rect) {
        if (width <= 0 || height <= 0) return
        if (!isInitialized) return

        val viewport = lastViewport
        checkNotNull(viewport) { "Dirty render requested without viewport." }

        val dirtyRect = Rect(requestedDirtyRect)
        if (!dirtyRect.intersect(0, 0, width, height)) return
        if (!holder.surface.isValid) return

        val effectivePlayer = if (boxPathActive) {
            displayedPlayerPosition ?: playerPosition
        } else {
            playerPosition
        } ?: return

        val canvas = holder.lockCanvas(dirtyRect) ?: return
        try {
            canvas.save()
            canvas.clipRect(dirtyRect)

            drawScene(
                canvas = canvas,
                viewport = viewport,
                tiles = tiles,
                boxPositions = boxPositions,
                playerPosition = effectivePlayer,
                selectedBox = selectedBox,
                isFacingLeft = isFacingLeft,
                drawPlayer = !boxPathActive
            )
            if (boxPathActive && boxFlashPosition != null) {
                val nowMs = SystemClock.elapsedRealtime()
                val elapsedMs = nowMs - boxFlashStartMs
                if (elapsedMs <= 200L) {
                    val flashPos = boxFlashPosition
                    if (flashPos != null) {
                        val params = spriteDrawParams(viewport, flashPos, 0.90f)
                        val bitmap = assets.getBitmap(R.drawable.box, params.sizePx)
                        val paint = if (elapsedMs <= 100L) boxFlashDarkPaint else boxFlashLightPaint
                        canvas.drawBitmap(bitmap, params.left, params.top, paint)
                    }
                } else {
                    boxFlashPosition = null
                    boxFlashStartMs = 0L
                }
            }
            if (boxPathActive && playerSilhouettePosition != null) {
                val nowMs = SystemClock.elapsedRealtime()
                val elapsedMs = nowMs - playerSilhouetteStartMs
                if (elapsedMs <= 200L) {
                    val silhouettePosition = playerSilhouettePosition
                    if (silhouettePosition != null) {
                        val params = spriteDrawParams(viewport, silhouettePosition, 0.80f)
                        val body = assets.getBitmap(R.drawable.player_slime, params.sizePx)
                        val paint = if (elapsedMs <= 100L) {
                            playerSilhouetteDarkPaint
                        } else {
                            playerSilhouetteLightPaint
                        }
                        drawSprite(
                            canvas,
                            body,
                            params.left,
                            params.top,
                            params.sizePx,
                            isFacingLeft,
                            paint
                        )
                    }
                } else {
                    playerSilhouettePosition = null
                    playerSilhouetteStartMs = 0L
                }
            }
            if (!boxPathActive && playerFlashPosition != null) {
                val nowMs = SystemClock.elapsedRealtime()
                val elapsedMs = nowMs - playerFlashStartMs
                if (elapsedMs <= 200L) {
                    val flashPos = playerFlashPosition
                    if (flashPos != null) {
                        val params = spriteDrawParams(viewport, flashPos, 0.80f)
                        val body = assets.getBitmap(R.drawable.player_slime, params.sizePx)
                        val paint = if (elapsedMs <= 100L) playerFlashDarkPaint else playerFlashLightPaint
                        drawSprite(
                            canvas,
                            body,
                            params.left,
                            params.top,
                            params.sizePx,
                            isFacingLeft,
                            paint
                        )
                    }
                } else {
                    playerFlashPosition = null
                    playerFlashStartMs = 0L
                }
            }
        } finally {
            canvas.restore()
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun computeBoxPathDirtyRect(
        viewport: BoardViewport,
        path: List<Position>,
        displayedPlayer: Position,
        pendingPlayer: Position
    ): Rect {
        require(path.size >= 2)

        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY

        val strokeWidth = cellSize * 0.2f
        val tailWidth = snapToWholePixel(cellSize * 0.90f)
        val tailHalfWidth = tailWidth / 2f
        val pad = maxOf(strokeWidth / 2f, tailHalfWidth) + 10f

        var minX = Float.POSITIVE_INFINITY
        var minY = Float.POSITIVE_INFINITY
        var maxX = Float.NEGATIVE_INFINITY
        var maxY = Float.NEGATIVE_INFINITY

        for (position in path) {
            val cx = offsetX + (position.col + 1) * cellSize + cellSize / 2f
            val cy = offsetY + (position.row + 1) * cellSize + cellSize / 2f
            if (cx < minX) minX = cx
            if (cy < minY) minY = cy
            if (cx > maxX) maxX = cx
            if (cy > maxY) maxY = cy
        }

        val rect = Rect(
            (minX - pad).toInt(),
            (minY - pad).toInt(),
            (maxX + pad).toInt(),
            (maxY + pad).toInt()
        )

        // Include moved box sprite bounds (from/to).
        val from = path.first()
        val to = path.last()
        rect.union(spriteDrawParams(viewport, from, 0.90f).dirtyRect)
        rect.union(spriteDrawParams(viewport, to, 0.90f).dirtyRect)

        // Include player sprite bounds (both displayed during animation and pending at end).
        rect.union(spriteDrawParams(viewport, displayedPlayer, 0.80f).dirtyRect)
        rect.union(spriteDrawParams(viewport, pendingPlayer, 0.80f).dirtyRect)

        return rect
    }

    private fun computeVanishDirtyRect(viewport: BoardViewport, position: Position): Rect {
        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY
        val left = offsetX + (position.col + 1) * cellSize
        val top = offsetY + (position.row + 1) * cellSize
        val right = left + cellSize
        val bottom = top + cellSize
        val paddingPx = 4f
        return Rect(
            (left - paddingPx).toInt(),
            (top - paddingPx).toInt(),
            (right + paddingPx).toInt(),
            (bottom + paddingPx).toInt()
        )
    }

    private fun renderVanishDirty() {
        if (!isInitialized) return
        val viewport = checkNotNull(lastViewport) { "Dirty render requested without viewport." }
        val position = vanishPosition ?: vanishLastPosition ?: return
        renderDirty(computeVanishDirtyRect(viewport, position))
    }

    private fun renderPlayerFlashDirty() {
        if (!isInitialized) return
        if (boxPathActive) {
            renderBoxPathOrFull()
            return
        }
        val viewport = checkNotNull(lastViewport) { "Dirty render requested without viewport." }
        val position = playerFlashPosition ?: return
        val rect = spriteDrawParams(viewport, position, 0.80f).dirtyRect
        renderDirty(rect)
    }

    private fun renderBlinkDirty() {
        if (!isInitialized) return
        if (boxPathActive || vanishPosition != null) {
            renderBoxPathOrFull()
            return
        }
        val viewport = checkNotNull(lastViewport) { "Dirty render requested without viewport." }
        val playerPos = playerPosition ?: return
        val params = spriteDrawParams(viewport, playerPos, 0.80f)
        val openBounds = assets.getOpaqueBounds(R.drawable.player_eyes_open, params.sizePx)
        val blinkBounds = assets.getOpaqueBounds(R.drawable.player_eyes_blink, params.sizePx)
        val bounds = Rect(openBounds)
        bounds.union(blinkBounds)
        if (bounds.isEmpty) {
            error("Dirty render requested with empty blink bounds.")
        }
        val paddingPx = 2
        val left = params.left.toInt() + bounds.left - paddingPx
        val top = params.top.toInt() + bounds.top - paddingPx
        val right = params.left.toInt() + bounds.right + paddingPx
        val bottom = params.top.toInt() + bounds.bottom + paddingPx
        renderDirty(Rect(left, top, right, bottom))
    }

    private fun isBlinking(nowMs: Long): Boolean {
        return nowMs in blinkStartMs until blinkEndMs
    }

    private fun triggerBlink(delayMs: Long = 400L) {
        val nowMs = SystemClock.elapsedRealtime()
        val start = nowMs + delayMs
        blinkStartMs = start
        blinkEndMs = start + 300L
        lastBlinkActive = false
        val delay = (blinkStartMs - nowMs).coerceAtLeast(0L)
        removeCallbacks(animationFrameRunnable)
        removeCallbacks(blinkStartRunnable)
        if (delay == 0L) {
            renderBlinkDirty()
            postOnAnimation(animationFrameRunnable)
        } else {
            postDelayed(blinkStartRunnable, delay)
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
        isFacingLeft: Boolean,
        drawPlayer: Boolean
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
            }
        }

        if (boxPathActive && !boxPathSuppressLine) {
            drawBoxPathLine(
                canvas = canvas,
                viewport = viewport,
                path = boxPath,
                shrink = boxPathShrink
            )
        }

        val vanishPos = vanishPosition
        if (vanishPos != null) {
            drawVanishingBox(
                canvas = canvas,
                viewport = viewport,
                gridPosition = vanishPos,
                paddedPosition = Position(vanishPos.row + 1, vanishPos.col + 1)
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

        if (drawPlayer) {
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

    }

    private fun startVanishBoxAnimation(at: Position) {
        vanishPosition = at
        vanishStartMs = SystemClock.elapsedRealtime()
        vanishStep = 0
        vanishLastPosition = at
        vanishNeedsFinalClear = false
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
        boxPathNeedsFinalClear = false

        // Conservative dirty rect: entire path line + moved box + both displayed and pending player sprites.
        val viewport = lastViewport
            ?: run {
                if (width > 0 && height > 0 && tiles.isNotEmpty() && tiles.first().isNotEmpty()) {
                    computeBoardViewport(width.toFloat(), height.toFloat(), tiles.size, tiles.first().size)
                } else {
                    null
                }
            }

        if (viewport != null) {
            // Ensure future partial renders have a viewport.
            lastViewport = viewport
            val displayedPlayer = displayedPlayerPosition ?: pendingPlayer
            boxPathDirtyRect = computeBoxPathDirtyRect(viewport, path, displayedPlayer, pendingPlayer)
        } else {
            boxPathDirtyRect = null
        }

        removeCallbacks(animationFrameRunnable)
        postOnAnimation(animationFrameRunnable)
    }

    private fun updateBoxPathAnimation(nowMs: Long): Boolean {
        if (!boxPathActive) return false
        val elapsed = nowMs - boxPathStartMs
        if (elapsed < boxPathDelayMs) return false
        val progress = if (boxPathSuppressLine) {
            1f
        } else {
            ((elapsed - boxPathDelayMs).toFloat() / boxPathDurationMs.toFloat()).coerceAtMost(1f)
        }
        var changed = false
        if (progress != boxPathShrink) {
            boxPathShrink = progress
            changed = true
        }
        if (elapsed >= boxPathDelayMs + if (boxPathSuppressLine) 0L else boxPathDurationMs) {
            boxPathActive = false
            boxPathNeedsFinalClear = true
            boxPathSuppressLine = false
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
        val currentPosition = vanishPosition ?: return false
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
            vanishLastPosition = currentPosition
            vanishNeedsFinalClear = true
            triggerBlink(delayMs = 0L)
            return true
        }

        return false
    }

    private fun drawBoxPathLine(
        canvas: Canvas,
        viewport: BoardViewport,
        path: List<Position>,
        shrink: Float
    ) {
        if (path.size < 2) return
        if (boxPathSuppressLine) return
        val nowMs = SystemClock.elapsedRealtime()
        if (nowMs - boxPathStartMs < boxPathDelayMs) return
        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY
        val strokeWidth = cellSize * 0.2f
        boxPathPaint.strokeWidth = strokeWidth
        boxPathTailPaint.strokeWidth = strokeWidth

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

        val segmentStart = points[startSegment]
        val segmentEnd = points[startSegment + 1]
        val segDx = segmentEnd.x - segmentStart.x
        val segDy = segmentEnd.y - segmentStart.y
        val tailParams = spriteDrawParams(viewport, path[startSegment], 0.90f)
        val tailOpaque = assets.getOpaqueBounds(R.drawable.box, tailParams.sizePx)
        if (!tailOpaque.isEmpty) {
            val left = tailParams.left + tailOpaque.left
            val top = tailParams.top + tailOpaque.top
            val right = tailParams.left + tailOpaque.right
            val bottom = tailParams.top + tailOpaque.bottom
            canvas.drawRect(left, top, right, bottom, boxPathTailPaint)
        }

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

        val origin = paddedPosition.toRenderPoint(cellSize, offsetX, offsetY)
        val targetSize = snapToWholePixel(cellSize * 0.90f)
        val sizePx = targetSize.toInt()
        require(sizePx > 0)
        val leftPx = snapToWholePixel(origin.x + (cellSize - targetSize) / 2f)
        val topPx = snapToWholePixel(origin.y + (cellSize - targetSize) / 2f)
        val scale = VanishSpec.scale(step)
        val size = targetSize * scale
        if (size <= 0f) return
        val left = leftPx + (targetSize - size) / 2f
        val top = topPx + (targetSize - size) / 2f
        val bitmap = assets.getBitmap(R.drawable.box, sizePx)
        canvas.save()
        canvas.clipRect(left, top, left + size, top + size)
        canvas.drawBitmap(bitmap, leftPx, topPx, assets.bitmapPaint())
        canvas.restore()
    }
}
