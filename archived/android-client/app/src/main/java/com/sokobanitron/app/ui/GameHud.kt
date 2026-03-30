@file:Suppress("ktlint:standard:function-naming")

package com.sokobanitron.app.ui

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.sokobanitron.app.R

@Composable
fun GameHud(
    currentRating: Int,
    onThumbUp: () -> Unit,
    onThumbDown: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        GameControlButton(
            onClick = onThumbDown,
            drawableResId =
                if (currentRating == -1) {
                    R.drawable.ic_trash_filled
                } else {
                    R.drawable.ic_trash
                },
            contentDescription = "Dislike level",
        )

        Spacer(modifier = Modifier.weight(1f))

        GameControlButton(
            onClick = onThumbUp,
            drawableResId =
                if (currentRating == 1) {
                    R.drawable.ic_heart_filled
                } else {
                    R.drawable.ic_heart
                },
            contentDescription = "Like level",
        )
    }
}
