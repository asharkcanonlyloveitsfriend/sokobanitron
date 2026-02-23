package com.sokobanitron.app.ui.rendering

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Rect
import android.view.MotionEvent
import android.view.View
import com.sokobanitron.app.GameController
import com.sokobanitron.app.sokoban.Position
import com.sokobanitron.app.ui.rendering.anim.AnimationRunner
import com.sokobanitron.app.ui.rendering.anim.BlinkAnimation
import com.sokobanitron.app.ui.rendering.anim.BoxPathAnimation
import com.sokobanitron.app.ui.rendering.anim.BoxVanishAnimation
import com.sokobanitron.app.ui.rendering.anim.EntityFlashAnimation
import com.sokobanitron.app.ui.rendering.draw.EntityDrawer
import com.sokobanitron.app.ui.rendering.draw.EntityRenderer
import com.sokobanitron.app.ui.rendering.geom.screenToInnerCell

@SuppressLint("ClickableViewAccessibility")
internal class GameBoardView(
    context: Context,
) : View(context),
    GameBoardPresenter {
    private val assets = AndroidGameAssets(context)
    private val entityRenderer =
        EntityRenderer(
            assets = assets,
            entityDrawer = EntityDrawer(assets),
        )

    private var staticFrame: StaticBoardFrame? = null
    private var boxPositions: Set<Position> = emptySet()
    private var playerPosition: Position? = null

    private var onTapCell: ((Position) -> Unit)? = null
    private var selectedBox: Position? = null

    private val animationRunner =
        AnimationRunner(
            invalidateRects = { rects -> invalidateRects(*rects) },
            postDelayed = { runnable, delayMs -> postDelayed(runnable, delayMs) },
        )

    init {
        setOnTouchListener { _, event ->
            if (event.actionMasked == MotionEvent.ACTION_UP) {
                val viewport = staticFrame?.viewport ?: return@setOnTouchListener true
                val position =
                    viewport.screenToInnerCell(event.x, event.y)
                        ?: return@setOnTouchListener true
                onTapCell?.invoke(position)
            }
            true
        }
    }

    override fun asView(): View = this

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        drawInternal(canvas)
    }

    override fun setOnTapCell(handler: (Position) -> Unit) {
        onTapCell = handler
    }

    override fun getSelectedBox(): Position? = selectedBox

    override fun setSelectedBox(position: Position?) {
        val previous = selectedBox
        selectedBox = position

        val viewport = staticFrame!!.viewport

        invalidateRects(
            previous?.let { entityRenderer.computeBoxRect(viewport, it) },
            position?.let { entityRenderer.computeBoxRect(viewport, it) },
        )
    }

    override fun applyDelta(delta: GameController.RenderDelta) {
        when (delta) {
            is GameController.RenderDelta.LevelLoaded -> {
                onLevelLoaded(
                    staticFrame = delta.staticFrame,
                    boxPositions = delta.boxPositions,
                    playerPosition = delta.playerPosition,
                )
            }

            is GameController.RenderDelta.StateChanged -> {
                onStateChanged(
                    playerPosition = delta.playerPosition,
                    boxPositions = delta.boxPositions,
                    annotation = delta.annotation,
                )
            }

            is GameController.RenderDelta.MoveRejected -> {
                onMoveRejected()
            }

            is GameController.RenderDelta.LevelSolved -> {
                onLevelSolved(isClean = delta.isClean)
            }
        }
    }

    private fun drawInternal(canvas: Canvas) {
        val playerPos = playerPosition ?: return
        val frame = staticFrame ?: return
        val viewport = frame.viewport

        canvas.drawBitmap(frame.bitmap, 0f, 0f, null)

        animationRunner.drawUnderEntities(canvas)

        entityRenderer.drawBoxes(
            canvas = canvas,
            viewport = viewport,
            boxPositions = boxPositions,
            selectedBox = selectedBox,
        )

        if (!animationRunner.hidesPlayer()) {
            entityRenderer.drawPlayer(
                canvas = canvas,
                viewport = viewport,
                playerPosition = playerPos,
            )
        }

        animationRunner.drawOverEntities(canvas)
    }

    private fun onLevelLoaded(
        staticFrame: StaticBoardFrame,
        boxPositions: Set<Position>,
        playerPosition: Position,
    ) {
        this.staticFrame = staticFrame
        entityRenderer.initGeometry(staticFrame.viewport)
        this.boxPositions = boxPositions
        this.playerPosition = playerPosition
        selectedBox = null
        invalidate()
    }

    private fun onStateChanged(
        playerPosition: Position,
        boxPositions: Set<Position>,
        annotation: GameController.RenderDelta.StateChangeAnnotation?,
    ) {
        val viewport = staticFrame!!.viewport
        val previousPlayer = this.playerPosition!!
        val previousBoxes = this.boxPositions
        val movedBoxes = previousBoxes - boxPositions
        val playerChanged = previousPlayer != playerPosition

        this.playerPosition = playerPosition
        this.boxPositions = boxPositions
        selectedBox = null

        val addedBoxes = boxPositions - previousBoxes
        invalidateRects(
            entityRenderer.computePlayerRect(viewport, playerPosition),
            *addedBoxes.map { entityRenderer.computeBoxRect(viewport, it) }.toTypedArray(),
        )

        if (movedBoxes.isNotEmpty() || playerChanged) {
            animationRunner.enqueue(
                EntityFlashAnimation(
                    renderer = entityRenderer,
                    viewport = viewport,
                    playerPosition = previousPlayer,
                    boxPositions = movedBoxes.toList(),
                ),
            )
        }

        when (annotation) {
            is GameController.RenderDelta.StateChangeAnnotation.BoxMoved -> {
                onBoxMoved(annotation.path)
            }

            is GameController.RenderDelta.StateChangeAnnotation.BoxRemoved -> {
                onBoxRemoved(annotation.position)
            }

            else -> {}
        }
    }

    private fun onBoxMoved(path: List<Position>) {
        if (path.size > 2) {
            val viewport = staticFrame!!.viewport
            animationRunner.enqueue(BoxPathAnimation(viewport, path))
        }
    }

    private fun onBoxRemoved(removedPosition: Position) {
        val viewport = staticFrame!!.viewport
        animationRunner.enqueue(BoxVanishAnimation(entityRenderer, viewport, removedPosition))
        animationRunner.enqueue(BlinkAnimation(entityRenderer, viewport, this.playerPosition!!))
    }

    private fun onMoveRejected() {
        val viewport = staticFrame!!.viewport
        val playerPos = playerPosition!!

        animationRunner.enqueue(BlinkAnimation(entityRenderer, viewport, playerPos))
    }

    private fun onLevelSolved(isClean: Boolean) {
        if (isClean) return
        val viewport = staticFrame!!.viewport
        val playerPos = playerPosition!!

        animationRunner.enqueue(BlinkAnimation(entityRenderer, viewport, playerPos))
    }

    private fun invalidateRects(vararg rects: Rect?) {
        val nonNull = rects.filterNotNull()
        if (nonNull.isEmpty()) return

        val dirty = Rect(nonNull[0])
        for (i in 1 until nonNull.size) {
            dirty.union(nonNull[i])
        }
        invalidateRectOnAnimation(dirty)
    }

    private fun invalidateRectOnAnimation(rect: Rect) {
        postInvalidateOnAnimation(rect.left, rect.top, rect.right, rect.bottom)
    }
}
