package com.example.einkarcade.ui.rendering

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Rect
import android.os.SystemClock
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.core.graphics.createBitmap
import com.example.einkarcade.GameController
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.anim.AnimationState
import com.example.einkarcade.ui.rendering.anim.GameAnimator
import com.example.einkarcade.ui.rendering.anim.TickResult
import com.example.einkarcade.ui.rendering.draw.BackgroundDrawer
import com.example.einkarcade.ui.rendering.draw.EffectsDrawer
import com.example.einkarcade.ui.rendering.draw.EntityDrawer
import com.example.einkarcade.ui.rendering.draw.GameRenderer
import com.example.einkarcade.ui.rendering.draw.OverlayState
import com.example.einkarcade.ui.rendering.draw.RenderStateSnapshot
import com.example.einkarcade.ui.rendering.draw.TileDrawer
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import com.example.einkarcade.ui.rendering.geom.computeBoardViewport
import com.example.einkarcade.ui.rendering.geom.screenToInnerCell
import com.example.einkarcade.ui.rendering.geom.spriteDrawParams
import com.example.einkarcade.ui.rendering.model.LevelInit
import com.example.einkarcade.ui.rendering.model.RenderState
import com.example.einkarcade.ui.rendering.model.TransitionState

@SuppressLint("ClickableViewAccessibility")
internal class GameSurfaceView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {
    private val useAnimations = false
    private val useBlinkAnimation = true
    private val useFlashAnimation = true
    private val useBoxFlashAnimation = true
    private val renderState = RenderState()
    private val transitionState = TransitionState()
    private var onTapCell: ((Position) -> Unit)? = null
    private var lastViewport: BoardViewport? = null
    private val assets = AndroidGameAssets(context)
    private val animator = GameAnimator(assets)
    private val animationState: AnimationState
        get() = animator.state
    private var staticFrameBitmap: Bitmap? = null
    private var staticFrameTiles: List<List<Tile>>? = null
    private val backgroundDrawer = BackgroundDrawer(context)
    private val tileDrawer = TileDrawer()
    private val entityDrawer = EntityDrawer(assets)
    private val effectsDrawer = EffectsDrawer(assets)
    private val renderer = GameRenderer(backgroundDrawer, tileDrawer, entityDrawer, effectsDrawer)
    private val animationFrameRunnable = object : Runnable {
        override fun run() {
            if (!useAnimations && !useBlinkAnimation && !useFlashAnimation && !useBoxFlashAnimation) return
            val now = SystemClock.elapsedRealtime()
            val tick = animator.tick(now, lastViewport, renderState)
            val transitionActive = transitionState.transition?.let { !it.isComplete(now) } == true

            renderAnimatorTick(tick, transitionActive, now)

            if (transitionState.transition?.isComplete(now) == true) {
                transitionState.transition = null
                render()
            }

            if (transitionActive) {
                postOnAnimation(this)
                return
            }

            if (tick.needsNextFrame) {
                val delay = tick.nextFrameDelayMs
                if (delay != null) {
                    postDelayed(this, delay)
                } else {
                    postOnAnimation(this)
                }
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
        val nowMs = SystemClock.elapsedRealtime()
        val newTiles = init.tiles.map { it.toList() }
        val isSameLayout = renderState.isInitialized && renderState.tiles == newTiles
        val shouldAnimate = useAnimations && init.tiles.isNotEmpty() && !isSameLayout
        if (shouldAnimate) {
            transitionState.transition = LevelTransition(
                oldTiles = renderState.tiles,
                newTiles = newTiles,
                startMs = nowMs
            )
            removeCallbacks(animationFrameRunnable)
            postOnAnimation(animationFrameRunnable)
        } else {
            transitionState.transition = null
        }
        renderState.tiles = newTiles
        renderState.boxPositions = init.boxPositions.toSet()
        renderState.playerPosition = init.playerPosition
        renderState.displayedPlayerPosition = init.playerPosition
        renderState.pendingPlayerPosition = null
        renderState.selectedBox = null
        resetFacing()
        animator.reset()
        renderState.isInitialized = true
        rebuildStaticFrameIfPossible()
        render()
    }

    fun setSelectedBox(selected: Position?) {
        if (!renderState.isInitialized) return
        val previous = renderState.selectedBox
        if (previous == selected) return

        renderState.selectedBox = selected

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

    fun getSelectedBox(): Position? = renderState.selectedBox

    fun onPlayerMoved(to: Position) {
        if (!renderState.isInitialized) return

        val from = renderState.playerPosition
        resetFacing()
        renderState.playerPosition = to
        renderState.displayedPlayerPosition = to
        if (useAnimations || useFlashAnimation) {
            val nowMs = SystemClock.elapsedRealtime()
            if (useAnimations || useFlashAnimation) {
                animator.startPlayerFlash(from, nowMs)
            }
            removeCallbacks(animationFrameRunnable)
            postOnAnimation(animationFrameRunnable)
        }

        checkNotNull(from) { "Dirty render requested without previous player position." }
        val viewport = checkNotNull(lastViewport) { "Dirty render requested without viewport." }
        val fromParams = spriteDrawParams(viewport, from, 0.80f)
        val toParams = spriteDrawParams(viewport, to, 0.80f)
        val dirty = Rect(fromParams.dirtyRect)
        dirty.union(toParams.dirtyRect)
        renderDirty(dirty)
    }

    fun onMoveRejected() {
        if (!renderState.isInitialized) return
        if (useBlinkAnimation) {
            triggerBlink()
        }
    }

    fun onGameWon(isClean: Boolean) {
        if (!renderState.isInitialized) return
        if (!isClean) return
    }

    fun onBoxMoved(path: List<Position>) {
        if (!renderState.isInitialized) return
        if (path.size < 2) return
        if (!useAnimations) {
            val from = path.first()
            val to = path.last()
            val prevPlayer = renderState.playerPosition
            if (useBoxFlashAnimation) {
                val nowMs = SystemClock.elapsedRealtime()
                animator.startBoxFlash(from, nowMs)
                removeCallbacks(animationFrameRunnable)
                postOnAnimation(animationFrameRunnable)
            }
            if (renderState.tiles[to.row][to.col] == Tile.WALL) {
                renderState.boxPositions -= from
            } else {
                renderState.boxPositions = (renderState.boxPositions - from) + to
            }
            renderState.playerPosition = path[path.size - 2]
            renderState.displayedPlayerPosition = renderState.playerPosition
            renderState.pendingPlayerPosition = null
            renderState.pendingFacingLeft = null

            val viewport = lastViewport
            if (viewport != null && prevPlayer != null) {
                val dirty = Rect(spriteDrawParams(viewport, from, 0.90f).dirtyRect)
                dirty.union(spriteDrawParams(viewport, to, 0.90f).dirtyRect)
                dirty.union(spriteDrawParams(viewport, prevPlayer, 0.80f).dirtyRect)
                dirty.union(spriteDrawParams(viewport, renderState.playerPosition!!, 0.80f).dirtyRect)
                renderDirty(dirty)
            } else {
                render()
            }
            return
        }
        val from = path.first()
        val to = path.last()
        if (renderState.tiles[to.row][to.col] == Tile.WALL) {
            renderState.boxPositions -= from
            animator.startVanish(at = to, nowMs = SystemClock.elapsedRealtime())
            removeCallbacks(animationFrameRunnable)
            postOnAnimation(animationFrameRunnable)
        } else {
            renderState.boxPositions = (renderState.boxPositions - from) + to
        }
        for (i in path.size - 1 downTo 1) {
            val prev = path[i - 1]
            val curr = path[i]
            if (curr.col != prev.col) {
                renderState.pendingFacingLeft = curr.col < prev.col
                break
            }
        }
        val nowMs = SystemClock.elapsedRealtime()
        renderState.displayedPlayerPosition = renderState.playerPosition
        renderState.playerPosition = path[path.size - 2]
        animator.startPlayerSilhouette(renderState.displayedPlayerPosition, nowMs)
        animator.startBoxFlash(from, nowMs)
        val viewport = lastViewport
            ?: run {
                if (width > 0 && height > 0 &&
                    renderState.tiles.isNotEmpty() &&
                    renderState.tiles.first().isNotEmpty()) {
                    computeBoardViewport(
                        width.toFloat(),
                        height.toFloat(),
                        renderState.tiles.size,
                        renderState.tiles.first().size
                    )
                } else {
                    null
                }
            }
        if (viewport != null) {
            lastViewport = viewport
        }
        animator.startBoxPath(
            path = path,
            pendingPlayer = renderState.playerPosition ?: path[path.size - 2],
            displayedPlayer = renderState.displayedPlayerPosition ?: path[path.size - 2],
            viewport = viewport,
            suppressLine = path.size == 2,
            nowMs = nowMs,
            renderState = renderState
        )
        renderBoxPathOrFull(nowMs)
        removeCallbacks(animationFrameRunnable)
        postOnAnimation(animationFrameRunnable)
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        if (renderState.isInitialized) {
            rebuildStaticFrameIfPossible()
            render()
        }
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (renderState.isInitialized) {
            rebuildStaticFrameIfPossible()
            render()
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        removeCallbacks(animationFrameRunnable)
        staticFrameBitmap?.recycle()
        staticFrameBitmap = null
        staticFrameTiles = null
        backgroundDrawer.recycle()
    }

    private fun render() {
        if (width <= 0 || height <= 0) return
        if (!renderState.isInitialized) return
        val playerPosition = if (useAnimations && animationState.boxPathActive) {
            renderState.displayedPlayerPosition ?: renderState.playerPosition!!
        } else {
            renderState.playerPosition!!
        }
        val innerRows = renderState.tiles.size
        val innerCols = renderState.tiles.first().size
        val viewport = computeBoardViewport(width.toFloat(), height.toFloat(), innerRows, innerCols)
        lastViewport = viewport

        if (!holder.surface.isValid) return
        val canvas = holder.lockCanvas() ?: return
        try {
            val nowMs = SystemClock.elapsedRealtime()
            if (useAnimations) {
                val transition = transitionState.transition?.takeUnless { it.isComplete(nowMs) }
                if (transition == null) {
                    transitionState.transition = null
                }
                renderer.drawScene(
                    canvas = canvas,
                    viewWidth = width,
                    viewHeight = height,
                    viewport = viewport,
                    renderState = buildRenderSnapshot(playerPosition),
                    transition = transition,
                    overlay = buildOverlayState(nowMs),
                    nowMs = nowMs,
                    drawPlayer = true
                )
            } else {
                rebuildStaticFrameIfPossible()
                drawStaticFrame(canvas, viewport)
                renderer.drawEntities(
                    canvas = canvas,
                    viewport = viewport,
                    renderState = buildRenderSnapshot(playerPosition),
                    drawPlayer = true,
                    blinkActive = useBlinkAnimation && animator.isBlinking(nowMs)
                )
                if (useFlashAnimation) {
                    val overlay = buildOverlayState(nowMs)
                    effectsDrawer.drawPlayerFlash(
                        canvas = canvas,
                        viewport = viewport,
                        overlay = overlay,
                        nowMs = nowMs,
                        isFacingLeft = renderState.isFacingLeft
                    )
                }
                if (useBoxFlashAnimation) {
                    val overlay = buildOverlayState(nowMs)
                    effectsDrawer.drawBoxFlash(canvas, viewport, overlay, nowMs)
                }
            }
        } finally {
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun rebuildStaticFrameIfPossible() {
        if (width <= 0 || height <= 0) return
        if (!renderState.isInitialized) return
        if (renderState.tiles.isEmpty()) return
        val firstRow = renderState.tiles.firstOrNull() ?: return
        if (firstRow.isEmpty()) return

        val existing = staticFrameBitmap
        if (existing != null &&
            !existing.isRecycled &&
            existing.width == width &&
            existing.height == height &&
            staticFrameTiles == renderState.tiles
        ) {
            return
        }

        existing?.recycle()

        val bitmap = createBitmap(width, height)
        val bitmapCanvas = Canvas(bitmap)

        val innerRows = renderState.tiles.size
        val innerCols = renderState.tiles.first().size
        val viewport = computeBoardViewport(width.toFloat(), height.toFloat(), innerRows, innerCols)
        // Keep this consistent with full render so touch mapping stays correct.
        lastViewport = viewport

        renderer.drawStaticFrame(bitmapCanvas, width, height, viewport, renderState.tiles)

        staticFrameBitmap = bitmap
        staticFrameTiles = renderState.tiles
    }

    private fun renderDirty(
        requestedDirtyRect: Rect,
        blinkActive: Boolean = false,
        flashActive: Boolean = false,
        boxFlashActive: Boolean = false,
        nowMsOverride: Long? = null
    ) {
        if (width <= 0 || height <= 0) return
        if (!renderState.isInitialized) return

        if (useAnimations) {
            check(!animationState.boxPathActive) { "Dirty render requested during box path animation." }
        }

        val viewport = lastViewport
        checkNotNull(viewport) { "Dirty render requested without viewport." }

        val dirtyRect = Rect(requestedDirtyRect)
        if (!dirtyRect.intersect(0, 0, width, height)) return
        if (!holder.surface.isValid) return

        val playerPos = renderState.playerPosition ?: return

        val canvas = holder.lockCanvas(dirtyRect) ?: return
        try {
            canvas.save()
            canvas.clipRect(dirtyRect)

            val nowMs = nowMsOverride ?: SystemClock.elapsedRealtime()
            if (useAnimations) {
                val transition = transitionState.transition?.takeUnless { it.isComplete(nowMs) }
                if (transition == null) {
                    transitionState.transition = null
                }
                val overlay = buildOverlayState(nowMs)
                renderer.drawScene(
                    canvas = canvas,
                    viewWidth = width,
                    viewHeight = height,
                    viewport = viewport,
                    renderState = buildRenderSnapshot(playerPos),
                    transition = transition,
                    overlay = overlay,
                    nowMs = nowMs,
                    drawPlayer = true
                )
                effectsDrawer.drawPlayerFlash(canvas, viewport, overlay, nowMs, renderState.isFacingLeft)
            } else {
                rebuildStaticFrameIfPossible()
                drawStaticFrame(canvas, viewport)
                renderer.drawEntities(
                    canvas = canvas,
                    viewport = viewport,
                    renderState = buildRenderSnapshot(playerPos),
                    drawPlayer = true,
                    blinkActive = blinkActive
                )
                if (useFlashAnimation && flashActive) {
                    val overlay = buildOverlayState(nowMs)
                    effectsDrawer.drawPlayerFlash(
                        canvas = canvas,
                        viewport = viewport,
                        overlay = overlay,
                        nowMs = nowMs,
                        isFacingLeft = renderState.isFacingLeft
                    )
                }
                if (useBoxFlashAnimation && boxFlashActive) {
                    val overlay = buildOverlayState(nowMs)
                    effectsDrawer.drawBoxFlash(canvas, viewport, overlay, nowMs)
                }
            }
        } finally {
            canvas.restore()
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun renderAnimatorTick(tick: TickResult, transitionActive: Boolean, nowMs: Long) {
        if (!useAnimations && !useBlinkAnimation && !useFlashAnimation && !useBoxFlashAnimation) return
        if (tick.forceFullRender) {
            render()
            return
        }

        val dirty = tick.dirtyRect
        if (dirty != null) {
            if (animationState.boxPathActive || animationState.boxPathNeedsFinalClear) {
                renderDirtyForBoxPath(dirty)
            } else {
                renderDirty(
                    requestedDirtyRect = dirty,
                    blinkActive = useBlinkAnimation && animator.isBlinking(nowMs),
                    flashActive = useFlashAnimation,
                    boxFlashActive = useBoxFlashAnimation,
                    nowMsOverride = nowMs
                )
            }
            return
        }

        if (transitionActive && !animationState.boxPathActive) {
            render()
        }
    }

    private fun renderBoxPathOrFull(nowMs: Long) {
        if (!useAnimations) {
            render()
            return
        }
        val viewport = lastViewport ?: run {
            render()
            return
        }
        val dirty = animator.computeBoxPathDirtyUnion(nowMs, viewport)
        if (dirty == null) {
            render()
            return
        }
        renderDirtyForBoxPath(dirty)
    }

    private fun renderDirtyForBoxPath(requestedDirtyRect: Rect) {
        if (!useAnimations) {
            renderDirty(requestedDirtyRect)
            return
        }
        if (width <= 0 || height <= 0) return
        if (!renderState.isInitialized) return

        val viewport = lastViewport
        checkNotNull(viewport) { "Dirty render requested without viewport." }

        val dirtyRect = Rect(requestedDirtyRect)
        if (!dirtyRect.intersect(0, 0, width, height)) return
        if (!holder.surface.isValid) return

        val effectivePlayer = if (animationState.boxPathActive) {
            renderState.displayedPlayerPosition ?: renderState.playerPosition
        } else {
            renderState.playerPosition
        } ?: return

        val canvas = holder.lockCanvas(dirtyRect) ?: return
        try {
            canvas.save()
            canvas.clipRect(dirtyRect)

            val nowMs = SystemClock.elapsedRealtime()
            val transition = transitionState.transition?.takeUnless { it.isComplete(nowMs) }
            if (transition == null) {
                transitionState.transition = null
            }
            val overlay = buildOverlayState(nowMs)
            renderer.drawScene(
                canvas = canvas,
                viewWidth = width,
                viewHeight = height,
                viewport = viewport,
                renderState = buildRenderSnapshot(effectivePlayer),
                transition = transition,
                overlay = overlay,
                nowMs = nowMs,
                drawPlayer = !animationState.boxPathActive
            )
            effectsDrawer.drawBoxFlash(canvas, viewport, overlay, nowMs)
            effectsDrawer.drawPlayerSilhouette(
                canvas = canvas,
                viewport = viewport,
                overlay = overlay,
                nowMs = nowMs,
                isFacingLeft = renderState.isFacingLeft
            )
            effectsDrawer.drawPlayerFlash(
                canvas = canvas,
                viewport = viewport,
                overlay = overlay,
                nowMs = nowMs,
                isFacingLeft = renderState.isFacingLeft
            )
        } finally {
            canvas.restore()
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun buildRenderSnapshot(playerPosition: Position): RenderStateSnapshot {
        return RenderStateSnapshot(
            tiles = renderState.tiles,
            boxPositions = renderState.boxPositions,
            playerPosition = playerPosition,
            selectedBox = renderState.selectedBox,
            isFacingLeft = renderState.isFacingLeft
        )
    }

    private fun buildOverlayState(nowMs: Long): OverlayState {
        return OverlayState(
            boxPathActive = animationState.boxPathActive,
            boxPathSuppressLine = animationState.boxPathSuppressLine,
            boxPath = animationState.boxPath,
            boxPathShrink = animationState.boxPathShrink,
            boxPathStartMs = animationState.boxPathStartMs,
            vanishPosition = animationState.vanishPosition,
            vanishStep = animationState.vanishStep,
            boxFlashPosition = animationState.boxFlashPosition,
            boxFlashStartMs = animationState.boxFlashStartMs,
            playerSilhouettePosition = animationState.playerSilhouettePosition,
            playerSilhouetteStartMs = animationState.playerSilhouetteStartMs,
            playerFlashPosition = animationState.playerFlashPosition,
            playerFlashStartMs = animationState.playerFlashStartMs,
            blinkActive = animator.isBlinking(nowMs)
        )
    }

    private fun drawStaticFrame(canvas: Canvas, viewport: BoardViewport) {
        val bitmap = staticFrameBitmap
        if (bitmap != null && !bitmap.isRecycled) {
            canvas.drawBitmap(bitmap, 0f, 0f, null)
            return
        }
        renderer.drawStaticFrame(canvas, width, height, viewport, renderState.tiles)
    }

    private fun triggerBlink(delayMs: Long = RenderTimings.BLINK_DELAY_MS) {
        val nowMs = SystemClock.elapsedRealtime()
        animator.triggerBlink(nowMs, delayMs)
        removeCallbacks(animationFrameRunnable)
        postOnAnimation(animationFrameRunnable)
    }

    private fun resetFacing() {
        renderState.isFacingLeft = false
        renderState.pendingFacingLeft = null
    }
}
