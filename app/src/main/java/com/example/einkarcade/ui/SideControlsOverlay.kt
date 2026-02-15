@file:Suppress("ktlint:standard:function-naming")

package com.example.einkarcade.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.example.einkarcade.R

@Composable
fun SideControlsOverlay(
    onSkip: () -> Unit,
) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.CenterEnd,
    ) {
        GameControlButton(
            onClick = onSkip,
            drawableResId = R.drawable.ic_forward,
            contentDescription = "Skip level",
            modifier = Modifier.padding(end = 16.dp),
        )
    }
}
