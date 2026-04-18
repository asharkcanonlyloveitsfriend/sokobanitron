package com.sokobanitron.app.dev

import android.content.Context
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView

class RustSurfaceView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : SurfaceView(context, attrs), SurfaceHolder.Callback {
    private var nativeHandle: Long = 0L
    private var presentRetryPending = false

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

        NativeBridge.setSurface(nativeHandle, holder.surface)
        render()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        presentRetryPending = false
        val handle = nativeHandle
        if (handle != 0L) {
            NativeBridge.setSurface(handle, null)
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val handle = nativeHandle
        if (handle == 0L) return super.onTouchEvent(event)

        val shouldRender =
            when (event.actionMasked) {
            MotionEvent.ACTION_DOWN,
            MotionEvent.ACTION_POINTER_DOWN,
            -> dispatchPointerEvent(handle, event, event.actionIndex, PHASE_STARTED)

            MotionEvent.ACTION_MOVE -> {
                var shouldRender = false
                for (index in 0 until event.pointerCount) {
                    shouldRender =
                        dispatchPointerEvent(handle, event, index, PHASE_MOVED) || shouldRender
                }
                shouldRender
            }

            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_POINTER_UP,
            -> {
                val shouldRender =
                    dispatchPointerEvent(handle, event, event.actionIndex, PHASE_ENDED)
                if (event.actionMasked == MotionEvent.ACTION_UP) {
                    performClick()
                }
                shouldRender
            }

            MotionEvent.ACTION_CANCEL -> {
                var shouldRender = false
                for (index in 0 until event.pointerCount) {
                    shouldRender =
                        dispatchPointerEvent(handle, event, index, PHASE_CANCELLED) || shouldRender
                }
                shouldRender
            }

            else -> return super.onTouchEvent(event)
        }

        if (shouldRender) {
            render()
        }
        return true
    }

    override fun performClick(): Boolean {
        super.performClick()
        return true
    }

    fun release() {
        presentRetryPending = false
        if (nativeHandle != 0L) {
            NativeBridge.setSurface(nativeHandle, null)
            NativeBridge.destroy(nativeHandle)
            nativeHandle = 0L
        }
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
    ): Boolean =
        NativeBridge.onPointerEvent(
            handle,
            event.getPointerId(index).toLong(),
            phase,
            event.getX(index),
            event.getY(index),
        )

    private fun render() {
        render(allowRetry = true)
    }

    private fun render(allowRetry: Boolean) {
        val handle = nativeHandle
        if (handle == 0L || !holder.surface.isValid) {
            presentRetryPending = false
            return
        }

        if (NativeBridge.presentFrame(handle)) {
            presentRetryPending = false
            if (NativeBridge.hasPendingGameplayPresentation(handle) && holder.surface.isValid) {
                postOnAnimation { render() }
            }
            return
        }
        if (!allowRetry || presentRetryPending || !holder.surface.isValid) return

        presentRetryPending = true
        post {
            presentRetryPending = false
            render(allowRetry = false)
        }
    }

    private companion object {
        const val PHASE_STARTED = 0
        const val PHASE_MOVED = 1
        const val PHASE_ENDED = 2
        const val PHASE_CANCELLED = 3
    }
}
