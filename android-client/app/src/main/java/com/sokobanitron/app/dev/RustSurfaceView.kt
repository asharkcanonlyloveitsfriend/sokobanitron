package com.sokobanitron.app.dev

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView

class RustSurfaceView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : SurfaceView(context, attrs), SurfaceHolder.Callback {
    private var nativeHandle: Long = 0L
    private var frameBitmap: Bitmap? = null

    init {
        holder.addCallback(this)
        isFocusable = true
        isClickable = true
    }

    override fun surfaceCreated(holder: SurfaceHolder) = Unit

    override fun surfaceChanged(
        holder: SurfaceHolder,
        format: Int,
        width: Int,
        height: Int,
    ) {
        if (width <= 0 || height <= 0) return

        val levelSetsRoot = SeedLevelSets.prepare(context)
        if (nativeHandle == 0L) {
            nativeHandle = NativeBridge.create(levelSetsRoot.absolutePath, width, height)
        } else {
            NativeBridge.resize(nativeHandle, width, height)
        }

        frameBitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        render()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        frameBitmap?.recycle()
        frameBitmap = null
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val handle = nativeHandle
        if (handle == 0L) return super.onTouchEvent(event)

        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN,
            MotionEvent.ACTION_POINTER_DOWN,
            -> dispatchPointerEvent(handle, event, event.actionIndex, PHASE_STARTED)

            MotionEvent.ACTION_MOVE -> {
                for (index in 0 until event.pointerCount) {
                    dispatchPointerEvent(handle, event, index, PHASE_MOVED)
                }
            }

            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_POINTER_UP,
            -> {
                dispatchPointerEvent(handle, event, event.actionIndex, PHASE_ENDED)
                if (event.actionMasked == MotionEvent.ACTION_UP) {
                    performClick()
                }
            }

            MotionEvent.ACTION_CANCEL -> {
                for (index in 0 until event.pointerCount) {
                    dispatchPointerEvent(handle, event, index, PHASE_CANCELLED)
                }
            }

            else -> return super.onTouchEvent(event)
        }

        render()
        return true
    }

    override fun performClick(): Boolean {
        super.performClick()
        return true
    }

    fun release() {
        if (nativeHandle != 0L) {
            NativeBridge.destroy(nativeHandle)
            nativeHandle = 0L
        }
        frameBitmap?.recycle()
        frameBitmap = null
    }

    override fun onDetachedFromWindow() {
        release()
        super.onDetachedFromWindow()
    }

    private fun dispatchPointerEvent(
        handle: Long,
        event: MotionEvent,
        index: Int,
        phase: Int,
    ) {
        NativeBridge.onPointerEvent(
            handle,
            event.getPointerId(index).toLong(),
            phase,
            event.getX(index),
            event.getY(index),
        )
    }

    private fun render() {
        val handle = nativeHandle
        val bitmap = frameBitmap
        if (handle == 0L || bitmap == null || !holder.surface.isValid) return

        val pixels = NativeBridge.renderFrame(handle)
        if (pixels.size != bitmap.width * bitmap.height) return

        bitmap.setPixels(pixels, 0, bitmap.width, 0, 0, bitmap.width, bitmap.height)

        val canvas = holder.lockCanvas() ?: return
        try {
            drawFrame(canvas, bitmap)
        } finally {
            holder.unlockCanvasAndPost(canvas)
        }
    }

    private fun drawFrame(
        canvas: Canvas,
        bitmap: Bitmap,
    ) {
        canvas.drawBitmap(bitmap, 0f, 0f, null)
    }

    private companion object {
        const val PHASE_STARTED = 0
        const val PHASE_MOVED = 1
        const val PHASE_ENDED = 2
        const val PHASE_CANCELLED = 3
    }
}
