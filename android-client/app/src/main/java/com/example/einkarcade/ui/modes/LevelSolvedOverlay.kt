package com.example.einkarcade.ui.modes

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.drawable.Drawable
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.View
import androidx.core.content.ContextCompat
import com.example.einkarcade.R
import kotlin.math.max

class LevelSolvedOverlay
    @JvmOverloads
    constructor(
        context: Context,
        attrs: AttributeSet? = null,
    ) : View(context, attrs) {
        var onAdvance: (() -> Unit)? = null
        var onThumbUp: (() -> Unit)? = null
        var onThumbDown: (() -> Unit)? = null

        // Hit regions for icons
        private val leftIconHitRect = RectF()
        private val rightIconHitRect = RectF()
        private val boxRect = RectF()

        private var rating: Int = 0 // -1 = down, 0 = neutral, 1 = up

        fun setRating(rating: Int) {
            if (this.rating == rating) return
            this.rating = rating
            invalidate()
        }

        private val borderPaint =
            Paint().apply {
                color = Color.BLACK
                style = Paint.Style.STROKE
                strokeWidth = 2f * resources.displayMetrics.density
                isAntiAlias = false
            }

        private val fillPaint =
            Paint().apply {
                color = Color.WHITE
                style = Paint.Style.FILL
                isAntiAlias = false
            }

        private val textPaint =
            Paint().apply {
                color = Color.BLACK
                textAlign = Paint.Align.CENTER
                val density = resources.displayMetrics.density
                val fontScale = resources.configuration.fontScale
                textSize = 32f * density * fontScale
                isAntiAlias = true
            }

        private val leftIconOutline: Drawable =
            requireNotNull(ContextCompat.getDrawable(context, R.drawable.ic_trash))
        private val leftIconFilled: Drawable =
            requireNotNull(ContextCompat.getDrawable(context, R.drawable.ic_trash_filled))
        private val rightIconOutline: Drawable =
            requireNotNull(ContextCompat.getDrawable(context, R.drawable.ic_heart))
        private val rightIconFilled: Drawable =
            requireNotNull(ContextCompat.getDrawable(context, R.drawable.ic_heart_filled))

        override fun onDraw(canvas: Canvas) {
            val density = resources.displayMetrics.density

            val text = "You win!"
            val textWidth = textPaint.measureText(text)
            val fontMetrics = textPaint.fontMetrics
            val textHeight = fontMetrics.descent - fontMetrics.ascent

            val horizontalPadding = 24f * density
            val verticalPadding = 16f * density

            val iconSize = 24f * density
            val iconSpacing = 12f * density

            val boxWidth =
                iconSize + iconSpacing + textWidth + iconSpacing + iconSize + horizontalPadding * 2
            val boxHeight = max(textHeight, iconSize) + verticalPadding * 2

            val left = (width - boxWidth) / 2f
            val top = (height - boxHeight) / 2f
            boxRect.set(
                left,
                top,
                left + boxWidth,
                top + boxHeight,
            )
            val box = boxRect

            canvas.drawRect(box, fillPaint)
            canvas.drawRect(box, borderPaint)

            // Draw left icon drawable
            val leftIconLeft = box.left + horizontalPadding
            val leftIconTop = box.centerY() - iconSize / 2f
            val leftIconRight = leftIconLeft + iconSize
            val leftIconBottom = leftIconTop + iconSize

            // Set left icon hit rect - expand to cover larger tap area
            // Compute text start X (left side of text)
            val textStartX = box.left + horizontalPadding + iconSize + iconSpacing
            leftIconHitRect.set(
                box.left,
                box.top,
                textStartX,
                box.bottom,
            )

            val leftIconToDraw = if (rating == -1) leftIconFilled else leftIconOutline
            leftIconToDraw.setBounds(
                leftIconLeft.toInt(),
                leftIconTop.toInt(),
                leftIconRight.toInt(),
                leftIconBottom.toInt(),
            )
            leftIconToDraw.draw(canvas)

            // Draw right icon drawable
            val rightIconRight = box.left + box.width() - horizontalPadding
            val rightIconLeft = rightIconRight - iconSize
            val rightIconTop = box.centerY() - iconSize / 2f
            val rightIconBottom = rightIconTop + iconSize

            // Set right icon hit rect - expand to cover larger tap area
            // Compute text end X (right side of text)
            val textCenterX = textStartX + textWidth / 2f
            val textEndX = textCenterX + textWidth / 2f
            rightIconHitRect.set(
                textEndX,
                box.top,
                box.right,
                box.bottom,
            )

            val rightIconToDraw = if (rating == 1) rightIconFilled else rightIconOutline
            rightIconToDraw.setBounds(
                rightIconLeft.toInt(),
                rightIconTop.toInt(),
                rightIconRight.toInt(),
                rightIconBottom.toInt(),
            )
            rightIconToDraw.draw(canvas)

            // Draw text centered vertically and horizontally between icons
            val textX = textStartX + textWidth / 2f
            val textY = box.centerY() - (fontMetrics.descent + fontMetrics.ascent) / 2f
            canvas.drawText(text, textX, textY, textPaint)
        }

        @SuppressLint("ClickableViewAccessibility")
        override fun onTouchEvent(event: MotionEvent): Boolean {
            if (event.action == MotionEvent.ACTION_DOWN) {
                val x = event.x
                val y = event.y
                when {
                    leftIconHitRect.contains(x, y) -> {
                        onThumbDown?.invoke()
                    }

                    rightIconHitRect.contains(x, y) -> {
                        onThumbUp?.invoke()
                    }

                    x > width / 2f -> {
                        onAdvance?.invoke()
                    }
                }
                return true
            }
            return super.onTouchEvent(event)
        }
    }
