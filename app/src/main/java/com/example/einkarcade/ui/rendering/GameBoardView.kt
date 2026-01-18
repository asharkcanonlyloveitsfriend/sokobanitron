package com.example.einkarcade.ui.rendering

import android.graphics.Bitmap
import android.graphics.Canvas

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Rect
import android.view.MotionEvent
import android.view.View
import com.example.einkarcade.GameController
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.anim.AnimationRunner
import com.example.einkarcade.ui.rendering.anim.BlinkAnimation
import com.example.einkarcade.ui.rendering.anim.BoxPathAnimation
import com.example.einkarcade.ui.rendering.anim.BoxVanishAnimation
import com.example.einkarcade.ui.rendering.anim.EntityFlashAnimation
import com.example.einkarcade.ui.rendering.anim.LevelTransitionAnimation
import com.example.einkarcade.ui.rendering.draw.BackgroundDrawer
import com.example.einkarcade.ui.rendering.draw.EntityDrawer
import com.example.einkarcade.ui.rendering.draw.GameRenderer
import com.example.einkarcade.ui.rendering.draw.TileDrawer
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import com.example.einkarcade.ui.rendering.geom.computeBoardViewport
import com.example.einkarcade.ui.rendering.geom.screenToInnerCell
import androidx.core.graphics.createBitmap

@SuppressLint("ClickableViewAccessibility")
internal class GameBoardView(
    context: Context
) : View(context), GameBoardPresenter {

    private val renderer = GameRenderer(
        assets = AndroidGameAssets(context),
        backgroundDrawer = BackgroundDrawer(context),
        tileDrawer = TileDrawer(),
        entityDrawer = EntityDrawer(AndroidGameAssets(context))
    )

    private var tiles: List<List<Tile>> = emptyList()
    private var boxPositions: Set<Position> = emptySet()
    private var playerPosition: Position? = null

    private var onTapCell: ((Position) -> Unit)? = null
    private var selectedBox: Position? = null

    private var lastViewport: BoardViewport? = null

    private val animationRunner = AnimationRunner(
        invalidateRects = { rects -> invalidateRects(*rects) },
        postDelayed = { runnable, delayMs -> postDelayed(runnable, delayMs) }
    )

    init {
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

    override fun asView(): View = this

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        drawInternal(canvas)
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        rebuildStaticLayout()
        invalidate()
    }

    override fun setOnTapCell(handler: (Position) -> Unit) {
        onTapCell = handler
    }

    override fun getSelectedBox(): Position? = selectedBox

    override fun setSelectedBox(position: Position?) {
        val previous = selectedBox
        selectedBox = position

        val viewport = lastViewport!!

        invalidateRects(
            previous?.let { renderer.computeBoxRect(viewport, it) },
            position?.let { renderer.computeBoxRect(viewport, it) }
        )
    }

    override fun applyDelta(delta: GameController.RenderDelta) {
        when (delta) {
            is GameController.RenderDelta.LevelLoaded -> {
                onLevelLoaded(
                    tiles = delta.tiles,
                    boxPositions = delta.boxPositions,
                    playerPosition = delta.playerPosition
                )
            }
            is GameController.RenderDelta.PlayerMoved -> onPlayerMoved(to = delta.to)
            is GameController.RenderDelta.BoxMoved -> onBoxMoved(path = delta.path)
            is GameController.RenderDelta.MoveRejected -> onMoveRejected()
            is GameController.RenderDelta.GameWon -> Unit
        }
    }

    private fun drawInternal(canvas: Canvas) {
        val playerPos = playerPosition ?: return
        val viewport = lastViewport ?: return

        if (animationRunner.hidesBoard()) {
            animationRunner.drawOverEntities(canvas)
            return
        }

        renderer.drawStaticFrame(canvas)

        animationRunner.drawUnderEntities(canvas)

        renderer.drawBoxes(
            canvas = canvas,
            viewport = viewport,
            boxPositions = boxPositions,
            selectedBox = selectedBox
        )

        if (!animationRunner.hidesPlayer()) {
            renderer.drawPlayer(
                canvas = canvas,
                viewport = viewport,
                playerPosition = playerPos
            )
        }

        animationRunner.drawOverEntities(canvas)
    }

    private fun onLevelLoaded(
        tiles: List<List<Tile>>,
        boxPositions: Set<Position>,
        playerPosition: Position
    ) {
        val previousTiles = this.tiles
        val previousViewport = lastViewport

        this.tiles = tiles
        this.boxPositions = boxPositions
        this.playerPosition = playerPosition
        selectedBox = null

        if (width <= 0 || height <= 0 || previousViewport == null) {
            return
        }

        if (previousTiles == tiles) {
            invalidate()
            return
        }

        val backgroundBitmap = createBitmap(width, height).also {
            renderer.drawBackground(Canvas(it), width, height)
        }

        // Rebuild layout for new tiles
        rebuildStaticLayout()

        val newViewport = lastViewport ?: run {
            invalidate()
            return
        }

        val newBitmap = createBitmap(width, height).also {
            renderer.drawStaticFrame(Canvas(it))
        }

        animationRunner.enqueue(
            LevelTransitionAnimation(
                backgroundBitmap = backgroundBitmap,
                newBitmap = newBitmap,
                oldViewport = previousViewport,
                newViewport = newViewport,
                oldTiles = previousTiles,
                newTiles = tiles
            )
        )
    }

    private fun onPlayerMoved(to: Position) {
        val viewport = lastViewport!!

        val previous = playerPosition!!
        playerPosition = to

        invalidateRects(
            renderer.computePlayerRect(viewport, previous),
            renderer.computePlayerRect(viewport, to)
        )

        animationRunner.enqueue(
            EntityFlashAnimation(
                renderer = renderer,
                viewport = viewport,
                playerPosition = previous,
                boxPosition = null
            )
        )
    }

    private fun onBoxMoved(path: List<Position>) {
        val viewport = lastViewport!!
        val previousPlayer = playerPosition!!

        val boxFrom = path.first()
        val boxTo = path.last()
        val newPlayer = path[path.size - 2]
        val isWall = tiles[boxTo.row][boxTo.col] == Tile.WALL
        val isLongMove = path.size > 2

        boxPositions = if (isWall) {
            boxPositions - boxFrom
        } else {
            boxPositions - boxFrom + boxTo
        }
        playerPosition = newPlayer

        invalidateRects(
            renderer.computeBoxRect(viewport, boxFrom),
            renderer.computeBoxRect(viewport, boxTo),
            renderer.computePlayerRect(viewport, previousPlayer),
            renderer.computePlayerRect(viewport, newPlayer)
        )

        if (isWall) {
            animationRunner.enqueue(BoxVanishAnimation(renderer, viewport, boxTo))
            animationRunner.enqueue(BlinkAnimation(renderer, viewport, newPlayer))
        } else if (isLongMove) {
            animationRunner.enqueue(
                EntityFlashAnimation(
                    renderer = renderer,
                    viewport = viewport,
                    playerPosition = previousPlayer,
                    boxPosition = boxFrom,
                    hidePlayer = true
                )
            )
            animationRunner.enqueue(
                BoxPathAnimation(
                    viewport = viewport,
                    path = path
                )
            )
        }
    }

    private fun onMoveRejected() {
        val viewport = lastViewport ?: return
        val playerPos = playerPosition ?: return

        animationRunner.enqueue(BlinkAnimation(renderer, viewport, playerPos))
    }

    private fun rebuildStaticLayout() {
        if (width <= 0 || height <= 0) return
        if (tiles.isEmpty()) return

        val viewport = computeBoardViewport(
            surfaceWidth = width.toFloat(),
            surfaceHeight = height.toFloat(),
            innerRows = tiles.size,
            innerCols = tiles[0].size
        )
        lastViewport = viewport

        renderer.rebuildStaticLayout(
            viewWidth = width,
            viewHeight = height,
            viewport = viewport,
            tiles = tiles
        )
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
