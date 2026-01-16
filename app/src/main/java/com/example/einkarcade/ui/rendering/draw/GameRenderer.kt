package com.example.einkarcade.ui.rendering.draw

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Rect
import androidx.core.graphics.createBitmap
import androidx.core.graphics.withClip
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.AndroidGameAssets
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import com.example.einkarcade.ui.rendering.geom.ResolvedEntityGeometry
import com.example.einkarcade.ui.rendering.geom.toRenderPoint

internal class GameRenderer(
    private val assets: AndroidGameAssets,
    private val backgroundDrawer: BackgroundDrawer,
    private val tileDrawer: TileDrawer,
    private val entityDrawer: EntityDrawer
) {
    private var staticFrameBitmap: Bitmap? = null
    private lateinit var geometry: ResolvedEntityGeometry

    fun rebuildStaticLayout(
        viewWidth: Int,
        viewHeight: Int,
        viewport: BoardViewport,
        tiles: List<List<Tile>>
    ) {
        geometry = ResolvedEntityGeometry.compute(
            viewport.cellSize,
            assets = assets
        )

        val bitmap = createBitmap(viewWidth, viewHeight)
        val canvas = Canvas(bitmap)

        backgroundDrawer.draw(canvas, viewWidth, viewHeight)
        tileDrawer.drawTiles(canvas, viewport, tiles)

        staticFrameBitmap = bitmap
    }

    fun drawStaticFrame(canvas: Canvas) {
        val bitmap = staticFrameBitmap ?: error("Static frame bitmap not initialized")
        canvas.drawBitmap(bitmap, 0f, 0f, null)
    }

    fun drawEntities(
        canvas: Canvas,
        viewport: BoardViewport,
        boxPositions: Set<Position>,
        playerPosition: Position,
        selectedBox: Position?
    ) {
        entityDrawer.drawBoxes(canvas, viewport, geometry, boxPositions)

        if (selectedBox != null) {
            entityDrawer.drawBox(
                canvas = canvas,
                viewport = viewport,
                geometry = geometry,
                position = selectedBox,
                resId = R.drawable.box_selected
            )
        }

        entityDrawer.drawPlayer(
            canvas = canvas,
            viewport = viewport,
            geometry = geometry,
            playerPosition = playerPosition
        )
    }

    fun computeBoxRect(
        viewport: BoardViewport,
        position: Position
    ): Rect {
        val origin = Position(position.row + 1, position.col + 1)
            .toRenderPoint(viewport.cellSize, viewport.offsetX, viewport.offsetY)
        val bounds = geometry.boxBoundsPx

        val left = (origin.x + bounds.left).toInt()
        val top = (origin.y + bounds.top).toInt()
        val right = left + bounds.width()
        val bottom = top + bounds.height()

        return Rect(left, top, right, bottom)
    }

    fun computePlayerRect(
        viewport: BoardViewport,
        position: Position
    ): Rect {
        val origin = Position(position.row + 1, position.col + 1)
            .toRenderPoint(viewport.cellSize, viewport.offsetX, viewport.offsetY)
        val bounds = geometry.playerBoundsPx

        val left = (origin.x + bounds.left).toInt()
        val top = (origin.y + bounds.top).toInt()
        val right = left + bounds.width()
        val bottom = top + bounds.height()

        return Rect(left, top, right, bottom)
    }

    fun getPlayerBodyBitmap(): Bitmap {
        return assets.getBitmap(R.drawable.player_slime, geometry.playerSizePx)
    }

    fun computePlayerEyesRect(
        viewport: BoardViewport,
        position: Position
    ): Rect {
        val origin = Position(position.row + 1, position.col + 1)
            .toRenderPoint(viewport.cellSize, viewport.offsetX, viewport.offsetY)

        val spriteLeft = origin.x + geometry.playerBoundsPx.left
        val spriteTop = origin.y + geometry.playerBoundsPx.top
        val bounds = geometry.playerEyesOpaqueBoundsPx

        val left = (spriteLeft + bounds.left).toInt()
        val top = (spriteTop + bounds.top).toInt()
        val right = (spriteLeft + bounds.right).toInt()
        val bottom = (spriteTop + bounds.bottom).toInt()
        return Rect(left, top, right, bottom)
    }

    fun getPlayerEyesBlinkBitmap(): Bitmap {
        return assets.getBitmap(R.drawable.player_eyes_blink, geometry.playerSizePx)
    }

    fun drawVanishingBox(
        canvas: Canvas,
        viewport: BoardViewport,
        position: Position,
        scale: Float
    ) {
        val origin = Position(position.row + 1, position.col + 1)
            .toRenderPoint(viewport.cellSize, viewport.offsetX, viewport.offsetY)
        val bounds = geometry.boxBoundsPx
        val left = origin.x + bounds.left
        val top = origin.y + bounds.top
        val size = geometry.boxSizePx.toFloat()
        val clippedSize = size * scale
        val clipLeft = left + (size - clippedSize) / 2f
        val clipTop = top + (size - clippedSize) / 2f

        val bitmap = assets.getBitmap(R.drawable.box, geometry.boxSizePx)
        canvas.withClip(clipLeft, clipTop, clipLeft + clippedSize, clipTop + clippedSize) {
            drawBitmap(bitmap, left, top, assets.bitmapPaint())
        }
    }
}
