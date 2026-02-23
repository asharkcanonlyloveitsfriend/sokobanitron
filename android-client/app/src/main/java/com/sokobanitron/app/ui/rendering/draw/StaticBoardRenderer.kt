package com.sokobanitron.app.ui.rendering.draw

import android.graphics.Bitmap
import android.graphics.Canvas
import androidx.core.graphics.createBitmap
import com.sokobanitron.app.sokoban.TileMap
import com.sokobanitron.app.ui.rendering.geom.BoardViewport

internal class StaticBoardRenderer(
    private val context: android.content.Context,
    private val tileDrawer: TileDrawer,
) {
    private var staticFrameBitmap: Bitmap? = null

    fun rebuildStaticLayout(
        viewWidth: Int,
        viewHeight: Int,
        viewport: BoardViewport,
        tileMap: TileMap,
    ) {
        val bitmap = createBitmap(viewWidth, viewHeight)
        val canvas = Canvas(bitmap)

        val background = BackgroundBitmapCache.get(context, viewWidth, viewHeight)
        canvas.drawBitmap(background, 0f, 0f, null)
        tileDrawer.drawTiles(canvas, viewport, tileMap)

        staticFrameBitmap = bitmap
    }

    fun getStaticFrameBitmap(): Bitmap = staticFrameBitmap ?: error("Static frame bitmap not initialized")
}
