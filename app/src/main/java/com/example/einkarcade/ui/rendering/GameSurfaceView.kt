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
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.anim.AnimationState
import com.example.einkarcade.ui.rendering.anim.GameAnimator
import com.example.einkarcade.ui.rendering.anim.TickResult
import com.example.einkarcade.ui.rendering.anim.QueuedAnimator
import com.example.einkarcade.ui.rendering.anim.BlinkAnimation
import com.example.einkarcade.ui.rendering.anim.PlayerFlashAnimation
import com.example.einkarcade.ui.rendering.anim.BoxVanishAnimation
import com.example.einkarcade.ui.rendering.draw.BackgroundDrawer
import com.example.einkarcade.ui.rendering.draw.EffectsDrawer
import com.example.einkarcade.ui.rendering.draw.EntityDrawer
import com.example.einkarcade.ui.rendering.draw.GameRenderer
import com.example.einkarcade.ui.rendering.draw.OverlayState
import com.example.einkarcade.ui.rendering.draw.RenderStateSnapshot
import com.example.einkarcade.ui.rendering.draw.TileDrawer
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import com.example.einkarcade.ui.rendering.geom.computeBoardViewport
import com.example.einkarcade.ui.rendering.geom.computeVanishDirtyRect
import com.example.einkarcade.ui.rendering.geom.screenToInnerCell
import com.example.einkarcade.ui.rendering.geom.spriteDrawParams
import com.example.einkarcade.ui.rendering.model.LevelInit
import com.example.einkarcade.ui.rendering.model.RenderState
import com.example.einkarcade.ui.rendering.model.TransitionState

@SuppressLint("ClickableViewAccessibility")
internal class GameSurfaceView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {
    private val renderState = RenderState()
    private val transitionState = TransitionState()
    private var onTapCell: ((Position) -> Unit)? = null
    private var lastViewport: BoardViewport? = null
    private val assets = AndroidGameAssets(context)
    private val animator = GameAnimator(assets)
    private val queuedAnimator = QueuedAnimator(
        tickDelayMs = RenderTimings.TICK_MS,
        postTick = { runnable, delayMs -> postDelayed(runnable, delayMs) }
    )
    private val animationState: AnimationState
        get() = animator.state
    private var staticFrameBitmap: Bitmap? = null
    private var staticFrameTiles: List<List<Tile>>? = null
    private val backgroundDrawer = BackgroundDrawer(context)
    private val tileDrawer = TileDrawer()
    private val entityDrawer = EntityDrawer(assets)
    private val effectsDrawer = EffectsDrawer(assets)
    private val renderer = GameRenderer(backgroundDrawer, tileDrawer, entityDrawer)
    private fun clampDelayMs(delayMs: Long): Long {
        return if (delayMs <= 0L) RenderTimings.TICK_MS else delayMs
    }

    private fun nextTickDelayMs(nowMs: Long): Long {
        return clampDelayMs(RenderTimings.msUntilNextTick(nowMs))
    }
    private val animationFrameRunnable = object : Runnable {
        override fun run() {
            val now = SystemClock.elapsedRealtime()
            val tick = animator.tick(now, lastViewport, renderState)
            val transition = transitionState.transition
            val transitionComplete = transition?.isComplete(now) == true
            val transitionActive = transition != null && !transitionComplete

            renderAnimatorTick(tick, transitionActive, now)

            if (transitionComplete) {
                transitionState.transition = null
                render()
            }

            if (transitionActive) {
                postDelayed(this, nextTickDelayMs(now))
                return
            }

            if (tick.needsNextFrame) {
                val delayMs = tick.nextFrameDelayMs ?: nextTickDelayMs(now)
                postDelayed(this, clampDelayMs(delayMs))
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
        val nowTick = RenderTimings.nowTick(nowMs)
        val newTiles = init.tiles.map { it.toList() }
        val isSameLayout = renderState.isReady && renderState.tiles == newTiles
        val shouldAnimate = init.tiles.isNotEmpty() && !isSameLayout
        if (shouldAnimate && !useQueuedAnimator) {
            transitionState.transition = LevelTransition(
                oldTiles = renderState.tiles,
                newTiles = newTiles,
                startTick = nowTick
            )
            removeCallbacks(animationFrameRunnable)
            postDelayed(animationFrameRunnable, nextTickDelayMs(nowMs))
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
        rebuildStaticFrameIfPossible()
        render()
    }

    fun setSelectedBox(selected: Position?) {
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
        renderDirtyStateful(dirty)
    }

    fun getSelectedBox(): Position? = renderState.selectedBox

    fun onPlayerMoved(to: Position) {
        val from = checkNotNull(renderState.playerPosition) {
            "Dirty render requested without previous player position."
        }
        resetFacing()
        renderState.playerPosition = to
        renderState.displayedPlayerPosition = to
        val nowMs = SystemClock.elapsedRealtime()
        if (useQueuedAnimator) {
            enqueuePlayerFlash(from)
        } else {
            animator.startPlayerFlash(from, nowMs)
            removeCallbacks(animationFrameRunnable)
            postDelayed(animationFrameRunnable, nextTickDelayMs(nowMs))
        }

        val viewport = checkNotNull(lastViewport) { "Dirty render requested without viewport." }
        val fromParams = spriteDrawParams(viewport, from, 0.80f)
        val toParams = spriteDrawParams(viewport, to, 0.80f)
        val dirty = Rect(fromParams.dirtyRect)
        dirty.union(toParams.dirtyRect)
        renderDirtyStateful(dirty)
    }

    fun onMoveRejected() {
        if (useQueuedAnimator) {
            enqueueBlink()
        } else {
            triggerBlink()
        }
    }

    fun onGameWon(isClean: Boolean) {
        if (!isClean) {
            if (useQueuedAnimator) {
                enqueueBlink()
            } else {
                triggerBlink()
            }
        }
    }

    fun onBoxMoved(path: List<Position>) {
        if (path.size < 2) return
        val from = path.first()
        val to = path.last()
        val prevPlayer = renderState.playerPosition
        val isWall = renderState.tiles[to.row][to.col] == Tile.WALL
        val nowMs = SystemClock.elapsedRealtime()
        if (!useQueuedAnimator) {
            if (path.size > 2) {
                animator.startBoxFlash(from, nowMs)
            }
            animator.startPlayerFlash(prevPlayer, nowMs)
            if (isWall) {
                animator.startVanish(at = to, nowMs = nowMs)
            }
            removeCallbacks(animationFrameRunnable)
            postDelayed(animationFrameRunnable, nextTickDelayMs(nowMs))
        }
        if (isWall) {
            renderState.boxPositions -= from
        } else {
            renderState.boxPositions = (renderState.boxPositions - from) + to
        }
        var pendingFacing: Boolean? = null
        for (i in path.size - 1 downTo 1) {
            val prev = path[i - 1]
            val curr = path[i]
            if (curr.col != prev.col) {
                pendingFacing = curr.col < prev.col
                break
            }
        }
        if (isWall) {
            pendingFacing?.let { renderState.isFacingLeft = it }
            renderState.pendingFacingLeft = null
        } else {
            renderState.pendingFacingLeft = pendingFacing
        }
        renderState.playerPosition = path[path.size - 2]
        renderState.displayedPlayerPosition = renderState.playerPosition
        renderState.pendingPlayerPosition = null

        val viewport = checkNotNull(lastViewport) {
            "BoxMoved received without viewport"
        }

        if (useQueuedAnimator && isWall) {
            enqueueBoxVanish(to, viewport)
        }

        if (!isWall && !useQueuedAnimator) {
            animator.startBoxPath(
                path = path,
                pendingPlayer = renderState.playerPosition ?: path[path.size - 2],
                displayedPlayer = renderState.displayedPlayerPosition ?: path[path.size - 2],
                viewport = viewport,
                nowMs = SystemClock.elapsedRealtime(),
                renderState = renderState
            )
            if (path.size > 2) {
                animator.startPlayerSilhouette(from, nowMs)
            }
        }

        if (prevPlayer != null) {
            val dirty = Rect(spriteDrawParams(viewport, from, 0.90f).dirtyRect)
            dirty.union(spriteDrawParams(viewport, to, 0.90f).dirtyRect)
            dirty.union(spriteDrawParams(viewport, prevPlayer, 0.80f).dirtyRect)
            dirty.union(spriteDrawParams(viewport, renderState.playerPosition!!, 0.80f).dirtyRect)
            if (isWall) {
                renderDirtyStateful(dirty)
            } else {
                renderDirtyForBoxPath(dirty)
            }
        } else {
            render()
        }
        return
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        if (renderState.isReady) {
            rebuildStaticFrameIfPossible()
            render()
        }
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (renderState.isReady) {
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
        val playerPosition = renderState.playerPosition!!
        val innerRows = renderState.tiles.size
        val innerCols = renderState.tiles.first().size
        val viewport = computeBoardViewport(width.toFloat(), height.toFloat(), innerRows, innerCols)
        lastViewport = viewport

        if (!holder.surface.isValid) return
        val canvas = holder.lockCanvas() ?: return
        try {
            val nowMs = SystemClock.elapsedRealtime()
            val transition = transitionState.transition?.takeUnless { it.isComplete(nowMs) }
            if (transition == null) {
                transitionState.transition = null
            }
            if (transition != null) {
                renderer.drawTransitionFrame(
                    canvas = canvas,
                    viewWidth = width,
                    viewHeight = height,
                    viewport = viewport,
                    transition = transition,
                    renderState = buildRenderSnapshot(playerPosition),
                    overlay = buildOverlayState(nowMs),
                    nowMs = nowMs,
                    drawPlayer = true
                )
                return
            }

            rebuildStaticFrameIfPossible()
            drawStaticFrame(canvas, viewport)
            val overlay = buildOverlayState(nowMs)
            if (animationState.boxPathActive || animationState.boxPathNeedsFinalClear) {
                effectsDrawer.drawBoxPathLine(canvas, viewport, overlay, nowMs)
                effectsDrawer.drawPlayerSilhouette(
                    canvas = canvas,
                    viewport = viewport,
                    overlay = overlay,
                    nowMs = nowMs
                )
            }
            if (overlay.vanishPosition != null) {
                effectsDrawer.drawVanishingBox(canvas, viewport, overlay)
            }
            val hidePlayer =
                animationState.boxPathActive && animationState.boxPath.size > 2
            renderer.drawEntities(
                canvas = canvas,
                viewport = viewport,
                renderState = buildRenderSnapshot(playerPosition),
                drawPlayer = !hidePlayer,
                blinkActive = false
            )
            effectsDrawer.drawPlayerFlash(
                canvas = canvas,
                viewport = viewport,
                overlay = overlay,
                nowMs = nowMs,
                isFacingLeft = renderState.isFacingLeft
            )
            effectsDrawer.drawBoxFlash(canvas, viewport, overlay, nowMs)
        } finally {
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun rebuildStaticFrameIfPossible() {
        if (width <= 0 || height <= 0) return
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

    private fun renderDirtyStateful(
        requestedDirtyRect: Rect,
        nowMsOverride: Long? = null
    ) {
        if (width <= 0 || height <= 0) return

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
            val transition = transitionState.transition?.takeUnless { it.isComplete(nowMs) }
            if (transition == null) {
                transitionState.transition = null
            }
            if (transition != null) {
                renderer.drawTransitionFrame(
                    canvas = canvas,
                    viewWidth = width,
                    viewHeight = height,
                    viewport = viewport,
                    transition = transition,
                    renderState = buildRenderSnapshot(playerPos),
                    overlay = buildOverlayState(nowMs),
                    nowMs = nowMs,
                    drawPlayer = true
                )
                return
            }

            rebuildStaticFrameIfPossible()
            drawStaticFrame(canvas, viewport)
            val overlay = buildOverlayState(nowMs)
            if (overlay.vanishPosition != null) {
                effectsDrawer.drawVanishingBox(canvas, viewport, overlay)
            }
            renderer.drawEntities(
                canvas = canvas,
                viewport = viewport,
                renderState = buildRenderSnapshot(playerPos),
                drawPlayer = true,
                blinkActive = false
            )
            effectsDrawer.drawPlayerFlash(
                canvas = canvas,
                viewport = viewport,
                overlay = overlay,
                nowMs = nowMs,
                isFacingLeft = renderState.isFacingLeft
            )
            effectsDrawer.drawBoxFlash(canvas, viewport, overlay, nowMs)
        } finally {
            canvas.restore()
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun renderBlinkDirty(rect: Rect, blinkActive: Boolean) {
        if (width <= 0 || height <= 0) return

        val viewport = lastViewport
        checkNotNull(viewport) { "Dirty render requested without viewport." }

        val dirtyRect = Rect(rect)
        if (!dirtyRect.intersect(0, 0, width, height)) return
        if (!holder.surface.isValid) return

        val playerPos = renderState.playerPosition ?: return
        val canvas = holder.lockCanvas(dirtyRect) ?: return
        try {
            canvas.save()
            canvas.clipRect(dirtyRect)

            rebuildStaticFrameIfPossible()
            drawStaticFrame(canvas, viewport)
            renderer.drawEntities(
                canvas = canvas,
                viewport = viewport,
                renderState = buildRenderSnapshot(playerPos),
                drawPlayer = true,
                blinkActive = blinkActive
            )
        } finally {
            canvas.restore()
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun renderPlayerFlashDirty(
        requestedDirtyRect: Rect,
        flashPosition: Position,
        flashStartTick: Long,
        nowMs: Long,
        drawFlash: Boolean
    ) {
        if (width <= 0 || height <= 0) return

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

            rebuildStaticFrameIfPossible()
            drawStaticFrame(canvas, viewport)
            renderer.drawEntities(
                canvas = canvas,
                viewport = viewport,
                renderState = buildRenderSnapshot(playerPos),
                drawPlayer = true,
                blinkActive = false
            )
            if (drawFlash) {
                val overlay = OverlayState(
                    boxPathActive = false,
                    boxPath = emptyList(),
                    boxPathShrink = 0f,
                    boxPathStartTick = 0L,
                    vanishPosition = null,
                    vanishStep = null,
                    boxFlashPosition = null,
                    boxFlashStartTick = 0L,
                    playerSilhouettePosition = null,
                    playerSilhouetteStartTick = 0L,
                    playerFlashPosition = flashPosition,
                    playerFlashStartTick = flashStartTick,
                    blinkActive = false
                )
                effectsDrawer.drawPlayerFlash(
                    canvas = canvas,
                    viewport = viewport,
                    overlay = overlay,
                    nowMs = nowMs,
                    isFacingLeft = renderState.isFacingLeft
                )
            }
        } finally {
            canvas.restore()
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun renderVanishDirty(
        requestedDirtyRect: Rect,
        vanishPosition: Position,
        vanishStep: Int,
        drawVanish: Boolean
    ) {
        if (width <= 0 || height <= 0) return

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

            rebuildStaticFrameIfPossible()
            drawStaticFrame(canvas, viewport)
            if (drawVanish) {
                val overlay = OverlayState(
                    boxPathActive = false,
                    boxPath = emptyList(),
                    boxPathShrink = 0f,
                    boxPathStartTick = 0L,
                    vanishPosition = vanishPosition,
                    vanishStep = vanishStep,
                    boxFlashPosition = null,
                    boxFlashStartTick = 0L,
                    playerSilhouettePosition = null,
                    playerSilhouetteStartTick = 0L,
                    playerFlashPosition = null,
                    playerFlashStartTick = 0L,
                    blinkActive = false
                )
                effectsDrawer.drawVanishingBox(canvas, viewport, overlay)
            }
            renderer.drawEntities(
                canvas = canvas,
                viewport = viewport,
                renderState = buildRenderSnapshot(playerPos),
                drawPlayer = true,
                blinkActive = false
            )
        } finally {
            canvas.restore()
            holder.unlockCanvasAndPost(canvas)
        }
    }


    private fun renderAnimatorTick(tick: TickResult, transitionActive: Boolean, nowMs: Long) {
        if (tick.forceFullRender) {
            render()
            return
        }

        val dirty = tick.dirtyRect
        if (dirty != null) {
            if (animationState.boxPathActive || animationState.boxPathNeedsFinalClear) {
                renderDirtyForBoxPath(dirty)
            } else {
                renderDirtyStateful(requestedDirtyRect = dirty, nowMsOverride = nowMs)
            }
            return
        }

        if (transitionActive && !animationState.boxPathActive) {
            render()
        }
    }

    private fun renderDirtyForBoxPath(requestedDirtyRect: Rect) {
        if (width <= 0 || height <= 0) return

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

            val nowMs = SystemClock.elapsedRealtime()
            rebuildStaticFrameIfPossible()
            drawStaticFrame(canvas, viewport)

            val overlay = buildOverlayState(nowMs)
            effectsDrawer.drawBoxPathLine(canvas, viewport, overlay, nowMs)
            effectsDrawer.drawPlayerSilhouette(
                canvas = canvas,
                viewport = viewport,
                overlay = overlay,
                nowMs = nowMs
            )
            if (overlay.vanishPosition != null) {
                effectsDrawer.drawVanishingBox(canvas, viewport, overlay)
            }

            val hidePlayer =
                animationState.boxPathActive && animationState.boxPath.size > 2
            renderer.drawEntities(
                canvas = canvas,
                viewport = viewport,
                renderState = buildRenderSnapshot(playerPos),
                drawPlayer = !hidePlayer,
                blinkActive = false
            )
            effectsDrawer.drawPlayerFlash(
                canvas = canvas,
                viewport = viewport,
                overlay = overlay,
                nowMs = nowMs,
                isFacingLeft = renderState.isFacingLeft
            )
            effectsDrawer.drawBoxFlash(canvas, viewport, overlay, nowMs)
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

    private fun computeBlinkDirtyRect(viewport: BoardViewport, playerPos: Position): Rect {
        val params = spriteDrawParams(viewport, playerPos, 0.80f)
        val bounds = Rect(assets.getOpaqueBounds(R.drawable.player_eyes_open, params.sizePx))
        if (bounds.isEmpty) {
            error("Dirty render requested with empty blink bounds.")
        }
        val paddingPx = 2
        val left = params.left.toInt() + bounds.left - paddingPx
        val top = params.top.toInt() + bounds.top - paddingPx
        val right = params.left.toInt() + bounds.right + paddingPx
        val bottom = params.top.toInt() + bounds.bottom + paddingPx
        return Rect(left, top, right, bottom)
    }

    private fun buildOverlayState(nowMs: Long): OverlayState {
        return OverlayState(
            boxPathActive = animationState.boxPathActive,
            boxPath = animationState.boxPath,
            boxPathShrink = animationState.boxPathShrink,
            boxPathStartTick = animationState.boxPathStartTick,
            vanishPosition = animationState.vanishPosition,
            vanishStep = animationState.vanishStep,
            boxFlashPosition = animationState.boxFlashPosition,
            boxFlashStartTick = animationState.boxFlashStartTick,
            playerSilhouettePosition = animationState.playerSilhouettePosition,
            playerSilhouetteStartTick = animationState.playerSilhouetteStartTick,
            playerFlashPosition = animationState.playerFlashPosition,
            playerFlashStartTick = animationState.playerFlashStartTick,
            blinkActive = false
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

    private fun triggerBlink() {
        if (!useQueuedAnimator) {
            val nowMs = SystemClock.elapsedRealtime()
            removeCallbacks(animationFrameRunnable)
            postDelayed(animationFrameRunnable, nextTickDelayMs(nowMs))
        }
    }

    private fun enqueueBlink() {
        val viewport = lastViewport ?: return
        val playerPos = renderState.playerPosition ?: return
        val blinkDirtyRect = computeBlinkDirtyRect(viewport, playerPos)

        queuedAnimator.enqueue(
            BlinkAnimation(
                delayTicks = RenderTimings.BLINK_DELAY_TICKS,
                blinkTicks = RenderTimings.BLINK_DURATION_TICKS,
                dirtyRect = blinkDirtyRect,
                renderBlinkDirty = ::renderBlinkDirty
            )
        )
    }

    private fun enqueuePlayerFlash(position: Position) {
        val viewport = lastViewport ?: return
        val dirtyRect = spriteDrawParams(viewport, position, 0.80f).dirtyRect
        val nowMs = SystemClock.elapsedRealtime()
        val startTick = RenderTimings.nowTick(nowMs)

        queuedAnimator.enqueue(
            PlayerFlashAnimation(
                flashPosition = position,
                dirtyRect = Rect(dirtyRect),
                flashStartTick = startTick,
                renderPlayerFlashDirty = ::renderPlayerFlashDirty
            )
        )
    }

    private fun enqueueBoxVanish(position: Position, viewport: BoardViewport) {
        val dirtyRect = computeVanishDirtyRect(viewport, position)

        queuedAnimator.enqueue(
            BoxVanishAnimation(
                vanishPosition = position,
                dirtyRect = Rect(dirtyRect),
                renderVanishDirty = ::renderVanishDirty
            )
        )
    }

    private fun resetFacing() {
        renderState.isFacingLeft = false
        renderState.pendingFacingLeft = null
    }
}
