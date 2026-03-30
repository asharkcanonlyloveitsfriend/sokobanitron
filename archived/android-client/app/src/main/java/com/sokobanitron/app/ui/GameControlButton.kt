@file:Suppress("ktlint:standard:function-naming")

package com.sokobanitron.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp

@Composable
fun GameControlButton(
    onClick: () -> Unit,
    drawableResId: Int,
    contentDescription: String,
    modifier: Modifier = Modifier,
    backgroundColor: Color = Color.Transparent,
    pressedBackgroundColor: Color = Color.Transparent,
    tintColor: Color = Color.LightGray,
    pressedTintAlpha: Float = 0.6f,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val isPressed = interactionSource.collectIsPressedAsState()
    val currentTint =
        if (isPressed.value) {
            tintColor.copy(alpha = tintColor.alpha * pressedTintAlpha)
        } else {
            tintColor
        }
    Box(
        modifier =
            modifier
                .height(48.dp)
                .background(if (isPressed.value) pressedBackgroundColor else backgroundColor)
                .clickable(
                    interactionSource = interactionSource,
                    indication = null,
                    onClick = onClick,
                ).padding(horizontal = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            painter = painterResource(drawableResId),
            contentDescription = contentDescription,
            tint = currentTint,
        )
    }
}
