package com.sokobanitron.app.ui.rendering.draw

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import androidx.core.graphics.createBitmap
import com.sokobanitron.app.R
import kotlin.math.roundToInt

internal class BackgroundDrawer(
    context: Context,
) {
    private val resources = context.resources
    private var backgroundBitmap: Bitmap? = null
    private var cachedScreenBitmap: Bitmap? = null
    private var cachedScreenW: Int = 0
    private var cachedScreenH: Int = 0
    private val backgroundPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { isFilterBitmap = true }
    private val backgroundSrcRect = Rect()
    private val backgroundDstRect = Rect()

    fun draw(
        canvas: Canvas,
        viewW: Int,
        viewH: Int,
    ) {
        require(viewW > 0 && viewH > 0)

        val cached = cachedScreenBitmap
        if (cached != null && !cached.isRecycled && cachedScreenW == viewW && cachedScreenH == viewH) {
            canvas.drawBitmap(cached, 0f, 0f, null)
            return
        }

        val bitmap = requireBackgroundBitmap()
        val bmpW = bitmap.width
        val bmpH = bitmap.height
        require(bmpW > 0 && bmpH > 0)

        val viewAspect = viewW.toFloat() / viewH.toFloat()
        val bmpAspect = bmpW.toFloat() / bmpH.toFloat()

        if (bmpAspect > viewAspect) {
            // Bitmap is wider than the view; crop left/right.
            val srcW = (bmpH * viewAspect).roundToInt().coerceAtMost(bmpW)
            val left = ((bmpW - srcW) / 2f).roundToInt().coerceAtLeast(0)
            backgroundSrcRect.set(left, 0, left + srcW, bmpH)
        } else {
            // Bitmap is taller than the view; crop top/bottom.
            val srcH = (bmpW / viewAspect).roundToInt().coerceAtMost(bmpH)
            val top = ((bmpH - srcH) / 2f).roundToInt().coerceAtLeast(0)
            backgroundSrcRect.set(0, top, bmpW, top + srcH)
        }

        // Render the fitted background into a cached screen-sized bitmap
        val screenBitmap = createBitmap(viewW, viewH)
        val screenCanvas = Canvas(screenBitmap)

        backgroundDstRect.set(0, 0, viewW, viewH)
        screenCanvas.drawBitmap(bitmap, backgroundSrcRect, backgroundDstRect, backgroundPaint)

        cachedScreenBitmap?.recycle()
        cachedScreenBitmap = screenBitmap
        cachedScreenW = viewW
        cachedScreenH = viewH

        canvas.drawBitmap(screenBitmap, 0f, 0f, null)
    }

    private fun requireBackgroundBitmap(): Bitmap {
        val existing = backgroundBitmap
        if (existing != null && !existing.isRecycled) return existing

        val decoded = BitmapFactory.decodeResource(resources, R.drawable.bg_space)
        require(!decoded.isRecycled)
        backgroundBitmap = decoded
        return decoded
    }
}
