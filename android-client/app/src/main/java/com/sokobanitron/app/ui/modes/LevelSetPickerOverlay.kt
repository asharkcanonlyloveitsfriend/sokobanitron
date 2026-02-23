@file:Suppress("ktlint:standard:function-naming")

package com.sokobanitron.app.ui.modes

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.sokobanitron.app.R
import com.sokobanitron.app.catalog.LevelCatalog
import kotlinx.coroutines.launch

private enum class SyncStatus {
    SUCCESS,
    FAILURE,
}

@Composable
fun LevelSetPickerOverlay(
    catalog: LevelCatalog,
    selectedSetId: Int,
    onPickSet: (setId: Int) -> Unit,
    onRefresh: suspend () -> Boolean,
    onDismiss: () -> Unit,
) {
    BackHandler { onDismiss() }

    val setOptions = catalog.getSetSummaries()
    val coroutineScope = rememberCoroutineScope()
    var syncStatus by remember { mutableStateOf<SyncStatus?>(null) }

    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .background(Color.Black)
                .clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                    onClick = {},
                ),
    ) {
        Column(
            modifier =
                Modifier
                    .matchParentSize()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Image(
                    painter = painterResource(R.drawable.ic_back),
                    contentDescription = "Back",
                    colorFilter = ColorFilter.tint(Color.LightGray),
                    modifier =
                        Modifier
                            .width(48.dp)
                            .clickable { onDismiss() }
                            .padding(horizontal = 12.dp, vertical = 8.dp),
                )

                Box(modifier = Modifier.weight(1f), contentAlignment = Alignment.Center) {
                    Text(
                        text = "Select Set",
                        fontSize = 20.sp,
                        color = Color.LightGray,
                    )
                }

                Image(
                    painter =
                        painterResource(
                            when (syncStatus) {
                                SyncStatus.SUCCESS -> R.drawable.ic_sync_success
                                SyncStatus.FAILURE -> R.drawable.ic_sync_error
                                null -> R.drawable.ic_sync
                            },
                        ),
                    contentDescription = "Sync",
                    colorFilter = ColorFilter.tint(Color.LightGray),
                    modifier =
                        Modifier
                            .size(48.dp)
                            .clickable {
                                coroutineScope.launch {
                                    syncStatus =
                                        if (onRefresh()) {
                                            SyncStatus.SUCCESS
                                        } else {
                                            SyncStatus.FAILURE
                                        }
                                }
                            }
                            .padding(horizontal = 12.dp, vertical = 8.dp),
                )
            }

            LazyColumn(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .padding(top = 12.dp),
            ) {
                items(setOptions, key = { it.id }) { set ->
                    val isSelected = set.id == selectedSetId
                    LevelSetCard(
                        setName = set.name,
                        completedCount = set.completedCount,
                        levelCount = set.levelCount,
                        isSelected = isSelected,
                        onClick = {
                            onPickSet(set.id)
                            onDismiss()
                        },
                    )
                }
            }
        }
    }
}

@Composable
private fun LevelSetCard(
    setName: String,
    completedCount: Int,
    levelCount: Int,
    isSelected: Boolean,
    onClick: () -> Unit,
) {
    val borderColor = if (isSelected) Color.LightGray else Color(0xFF7A7A7A)
    val borderWidth = if (isSelected) 3.dp else 2.dp
    val cardShape = RoundedCornerShape(8.dp)

    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(80.dp)
                .border(width = borderWidth, color = borderColor, shape = cardShape)
                .clickable(onClick = onClick)
                .padding(horizontal = 12.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = setName,
            fontSize = 18.sp,
            color = Color.LightGray,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        Text(
            text = "$completedCount/$levelCount",
            fontSize = 14.sp,
            color = Color.LightGray,
        )
    }
}
