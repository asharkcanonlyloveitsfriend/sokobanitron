package com.example.einkarcade.ui.rendering

import android.graphics.Bitmap
import com.example.einkarcade.sokoban.TileMap
import com.example.einkarcade.ui.rendering.geom.BoardViewport

data class StaticBoardFrame(
    val bitmap: Bitmap,
    val viewport: BoardViewport,
    val tileMap: TileMap,
    val width: Int,
    val height: Int,
)
