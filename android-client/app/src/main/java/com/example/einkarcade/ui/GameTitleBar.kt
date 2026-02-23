@file:Suppress("ktlint:standard:function-naming")

package com.example.einkarcade.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.einkarcade.R

@Composable
fun GameTitleBar(
    setName: String,
    levelName: String,
    onOpenSetPicker: () -> Unit,
    onOpenLevelPicker: () -> Unit,
    isStarred: Boolean,
    onToggleStar: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier.weight(1f),
            contentAlignment = Alignment.CenterStart,
        ) {
            Text(
                text = levelName,
                fontSize = 16.sp,
                color = Color.LightGray,
                modifier =
                    Modifier
                        .clickable { onOpenLevelPicker() }
                        .padding(horizontal = 6.dp, vertical = 2.dp),
            )
        }

        Box(
            modifier = Modifier.weight(1f),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = setName,
                fontSize = 16.sp,
                color = Color.LightGray,
                modifier =
                    Modifier
                        .clickable { onOpenSetPicker() }
                        .padding(horizontal = 6.dp, vertical = 2.dp),
            )
        }

        Box(
            modifier = Modifier.weight(1f),
            contentAlignment = Alignment.CenterEnd,
        ) {
            GameControlButton(
                onClick = onToggleStar,
                drawableResId =
                    if (isStarred) {
                        R.drawable.ic_star_filled
                    } else {
                        R.drawable.ic_star
                    },
                contentDescription = "Star level",
            )
        }
    }
}
