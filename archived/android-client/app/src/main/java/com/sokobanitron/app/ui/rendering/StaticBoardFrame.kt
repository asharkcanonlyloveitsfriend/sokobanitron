package com.sokobanitron.app.ui.rendering

import android.graphics.Bitmap
import com.sokobanitron.app.sokoban.TileMap
import com.sokobanitron.app.ui.rendering.geom.BoardViewport

data class StaticBoardFrame(
    val bitmap: Bitmap,
    val viewport: BoardViewport,
    val tileMap: TileMap,
    val width: Int,
    val height: Int,
)
