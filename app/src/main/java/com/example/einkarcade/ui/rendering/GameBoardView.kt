package com.example.einkarcade.ui.rendering

import android.annotation.SuppressLint
import android.content.Context
import android.view.MotionEvent
import android.view.View
import com.example.einkarcade.GameController
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.anim.AnimationRunner
import com.example.einkarcade.ui.rendering.anim.BlinkAnimation
import com.example.einkarcade.ui.rendering.anim.BoxVanishAnimation
import com.example.einkarcade.ui.rendering.anim.PlayerFlashAnimation
import com.example.einkarcade.ui.rendering.draw.BackgroundDrawer
import com.example.einkarcade.ui.rendering.draw.EntityDrawer
import com.example.einkarcade.ui.rendering.draw.GameRenderer
import com.example.einkarcade.ui.rendering.draw.TileDrawer
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import com.example.einkarcade.ui.rendering.geom.computeBoardViewport
import com.example.einkarcade.ui.rendering.geom.screenToInnerCell

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
        invalidateRect = { rect -> invalidateRectOnAnimation(rect) },
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

    override fun onDraw(canvas: android.graphics.Canvas) {
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

        previous?.let {
            val r = renderer.computeBoxRect(viewport, it)
            invalidateRectOnAnimation(r)
        }

        position?.let {
            val r = renderer.computeBoxRect(viewport, it)
            invalidateRectOnAnimation(r)
        }
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

    private fun drawInternal(canvas: android.graphics.Canvas) {
        val playerPos = playerPosition ?: return
        val viewport = lastViewport ?: return

        renderer.drawStaticFrame(canvas)

        renderer.drawEntities(
            canvas = canvas,
            viewport = viewport,
            boxPositions = boxPositions,
            playerPosition = playerPos,
            selectedBox = selectedBox
        )

        animationRunner.draw(canvas)
    }

    private fun onLevelLoaded(
        tiles: List<List<Tile>>,
        boxPositions: Set<Position>,
        playerPosition: Position
    ) {
        val tilesChanged = this.tiles != tiles

        this.tiles = tiles
        this.boxPositions = boxPositions
        this.playerPosition = playerPosition
        selectedBox = null

        if (tilesChanged) {
            rebuildStaticLayout()
        }

        invalidate()
    }

    private fun onPlayerMoved(to: Position) {
        val viewport = lastViewport!!

        val previous = playerPosition!!
        playerPosition = to

        var rect = renderer.computePlayerRect(viewport, previous)
        invalidateRectOnAnimation(rect)

        rect = renderer.computePlayerRect(viewport, to)
        invalidateRectOnAnimation(rect)

        animationRunner.enqueue(PlayerFlashAnimation(renderer, viewport, previous))
    }

    private fun onBoxMoved(path: List<Position>) {
        val viewport = lastViewport!!
        val previousPlayer = playerPosition!!

        val boxFrom = path.first()
        val boxTo = path.last()
        val newPlayer = path[path.size - 2]
        val isWall = tiles[boxTo.row][boxTo.col] == Tile.WALL

        boxPositions = if (isWall) {
            boxPositions - boxFrom
        } else {
            boxPositions - boxFrom + boxTo
        }
        playerPosition = newPlayer

        var rect = renderer.computeBoxRect(viewport, boxFrom)
        invalidateRectOnAnimation(rect)

        rect = renderer.computeBoxRect(viewport, boxTo)
        invalidateRectOnAnimation(rect)

        rect = renderer.computePlayerRect(viewport, previousPlayer)
        invalidateRectOnAnimation(rect)

        rect = renderer.computePlayerRect(viewport, newPlayer)
        invalidateRectOnAnimation(rect)

        if (isWall) {
            animationRunner.enqueue(BoxVanishAnimation(renderer, viewport, boxTo))
            animationRunner.enqueue(BlinkAnimation(renderer, viewport, newPlayer))
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
            tiles = tiles,
            animationRequirements = animationRunner.requirements()
        )
    }

    private fun invalidateRectOnAnimation(rect: android.graphics.Rect) {
        postInvalidateOnAnimation(rect.left, rect.top, rect.right, rect.bottom)
    }
}
