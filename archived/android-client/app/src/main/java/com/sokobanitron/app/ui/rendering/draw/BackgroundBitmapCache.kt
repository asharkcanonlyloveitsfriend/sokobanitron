package com.sokobanitron.app.ui.rendering.draw

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import androidx.core.graphics.createBitmap

sealed interface BackgroundId {
    data object Default : BackgroundId
}

object BackgroundBitmapCache {
    private data class Key(
        val backgroundId: BackgroundId,
        val width: Int,
        val height: Int,
    )

    private val cache = mutableMapOf<Key, Bitmap>()

    fun get(
        context: Context,
        width: Int,
        height: Int,
        backgroundId: BackgroundId = BackgroundId.Default,
    ): Bitmap {
        val key = Key(backgroundId, width, height)
        val cached = cache[key]
        if (cached != null) return cached

        val bitmap = createBitmap(width, height)
        val canvas = Canvas(bitmap)
        BackgroundDrawer(context).draw(canvas, width, height)
        cache[key] = bitmap
        return bitmap
    }
}
