package com.example.einkarcade.ui.rendering

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import androidx.appcompat.content.res.AppCompatResources
import androidx.core.graphics.createBitmap

internal class AndroidGameAssets(
    private val context: Context,
) {
    private val bitmapPaint =
        Paint(Paint.ANTI_ALIAS_FLAG).apply {
            isFilterBitmap = false
        }

    private val cache = mutableMapOf<Int, MutableMap<Int, Bitmap>>()
    private val opaqueBoundsCache = mutableMapOf<Int, MutableMap<Int, Rect>>()

    fun bitmapPaint(): Paint = bitmapPaint

    fun getBitmap(
        resId: Int,
        sizePx: Int,
    ): Bitmap {
        require(sizePx > 0)
        val bySize = cache.getOrPut(resId) { mutableMapOf() }
        return bySize.getOrPut(sizePx) {
            val drawable = AppCompatResources.getDrawable(context, resId)
            requireNotNull(drawable) { "Missing drawable: $resId" }
            val bitmap = createBitmap(sizePx, sizePx)
            val canvas = Canvas(bitmap)
            drawable.setBounds(0, 0, sizePx, sizePx)
            drawable.draw(canvas)
            bitmap
        }
    }

    fun getOpaqueBounds(
        resId: Int,
        sizePx: Int,
    ): Rect {
        require(sizePx > 0)
        val bySize = opaqueBoundsCache.getOrPut(resId) { mutableMapOf() }
        return bySize.getOrPut(sizePx) {
            val bitmap = getBitmap(resId, sizePx)
            val pixels = IntArray(sizePx * sizePx)
            bitmap.getPixels(pixels, 0, sizePx, 0, 0, sizePx, sizePx)
            var minX = sizePx
            var minY = sizePx
            var maxX = -1
            var maxY = -1
            for (y in 0 until sizePx) {
                val rowStart = y * sizePx
                for (x in 0 until sizePx) {
                    val alpha = pixels[rowStart + x] ushr 24
                    if (alpha != 0) {
                        if (x < minX) minX = x
                        if (y < minY) minY = y
                        if (x > maxX) maxX = x
                        if (y > maxY) maxY = y
                    }
                }
            }
            if (maxX < 0 || maxY < 0) {
                Rect(0, 0, 0, 0)
            } else {
                Rect(minX, minY, maxX + 1, maxY + 1)
            }
        }
    }
}
