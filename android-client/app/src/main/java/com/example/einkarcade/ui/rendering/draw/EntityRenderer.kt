package com.example.einkarcade.ui.rendering.draw

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Rect
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.AndroidGameAssets
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import com.example.einkarcade.ui.rendering.geom.ResolvedEntityGeometry
import com.example.einkarcade.ui.rendering.geom.toRenderPoint
import kotlin.math.roundToInt

internal class EntityRenderer(
    private val assets: AndroidGameAssets,
    private val entityDrawer: EntityDrawer,
) {
    private lateinit var geometry: ResolvedEntityGeometry

    fun initGeometry(viewport: BoardViewport) {
        geometry =
            ResolvedEntityGeometry.compute(
                viewport.cellSize,
                assets = assets,
            )
    }

    fun drawBoxes(
        canvas: Canvas,
        viewport: BoardViewport,
        boxPositions: Set<Position>,
        selectedBox: Position?,
    ) {
        entityDrawer.drawBoxes(canvas, viewport, geometry, boxPositions)

        if (selectedBox != null) {
            entityDrawer.drawBox(
                canvas = canvas,
                viewport = viewport,
                geometry = geometry,
                position = selectedBox,
                resId = R.drawable.box_selected,
            )
        }
    }

    fun drawPlayer(
        canvas: Canvas,
        viewport: BoardViewport,
        playerPosition: Position,
    ) {
        entityDrawer.drawPlayer(
            canvas = canvas,
            viewport = viewport,
            geometry = geometry,
            playerPosition = playerPosition,
        )
    }

    fun computeBoxRect(
        viewport: BoardViewport,
        position: Position,
    ): Rect {
        val origin =
            Position(position.row + 1, position.col + 1)
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
        position: Position,
    ): Rect {
        val origin =
            Position(position.row + 1, position.col + 1)
                .toRenderPoint(viewport.cellSize, viewport.offsetX, viewport.offsetY)
        val bounds = geometry.playerBoundsPx

        val left = (origin.x + bounds.left).toInt()
        val top = (origin.y + bounds.top).toInt()
        val right = left + bounds.width()
        val bottom = top + bounds.height()

        return Rect(left, top, right, bottom)
    }

    fun getPlayerBodyBitmap(): Bitmap = assets.getBitmap(R.drawable.player_slime, geometry.playerSizePx)

    fun getBoxBitmap(): Bitmap = assets.getBitmap(R.drawable.box, geometry.boxSizePx)

    fun computePlayerEyesRect(
        viewport: BoardViewport,
        position: Position,
    ): Rect {
        val origin =
            Position(position.row + 1, position.col + 1)
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

    fun getPlayerEyesBlinkBitmap(): Bitmap = assets.getBitmap(R.drawable.player_eyes_blink, geometry.playerSizePx)

    fun drawVanishingBox(
        canvas: Canvas,
        viewport: BoardViewport,
        position: Position,
        scale: Float,
    ) {
        if (scale <= 0f) return
        val origin =
            Position(position.row + 1, position.col + 1)
                .toRenderPoint(viewport.cellSize, viewport.offsetX, viewport.offsetY)
        val bounds = geometry.boxBoundsPx
        val left = origin.x + bounds.left
        val top = origin.y + bounds.top
        val size = geometry.boxSizePx.toFloat()
        val scaledSize = (geometry.boxSizePx * scale).roundToInt().coerceAtLeast(1)
        val scaledSizeF = scaledSize.toFloat()
        val drawLeft = left + (size - scaledSizeF) / 2f
        val drawTop = top + (size - scaledSizeF) / 2f

        val bitmap = assets.getBitmap(R.drawable.box, scaledSize)
        canvas.drawBitmap(bitmap, drawLeft, drawTop, assets.bitmapPaint())
    }
}
