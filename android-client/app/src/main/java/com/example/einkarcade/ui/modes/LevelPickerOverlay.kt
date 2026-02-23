@file:Suppress("ktlint:standard:function-naming")

package com.example.einkarcade.ui.modes

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.einkarcade.R
import com.example.einkarcade.catalog.LevelBoardGeometry
import com.example.einkarcade.catalog.LevelBoardTile
import com.example.einkarcade.catalog.LevelSummary
import com.example.einkarcade.ui.rendering.AndroidGameAssets
import kotlinx.coroutines.launch
import kotlin.math.min
import kotlin.math.roundToInt

@Composable
fun LevelPickerOverlay(
    levels: List<LevelSummary>,
    selectedPuzzleId: Int,
    onPickLevel: (puzzleId: Int) -> Unit,
    onToggleLike: (puzzleId: Int) -> Unit,
    onToggleStar: (puzzleId: Int) -> Unit,
    onToggleDislike: (puzzleId: Int) -> Unit,
    refreshNonce: Long,
    onDismiss: () -> Unit,
) {
    BackHandler { onDismiss() }

    var gridReady by remember { mutableIntStateOf(0) } // 0 = not ready, 1 = ready

    val gridState = rememberLazyGridState()
    val scope = rememberCoroutineScope()
    val isAtTop by
        remember(gridState) {
            derivedStateOf {
                gridState.firstVisibleItemIndex == 0 &&
                    gridState.firstVisibleItemScrollOffset == 0
            }
        }
    val isAtBottom by
        remember(gridState) {
            derivedStateOf {
                val layoutInfo = gridState.layoutInfo
                val totalItems = layoutInfo.totalItemsCount
                if (totalItems == 0) {
                    true
                } else {
                    val lastVisible = layoutInfo.visibleItemsInfo.lastOrNull() ?: return@derivedStateOf false
                    val isLastItemVisible = lastVisible.index == totalItems - 1
                    val lastItemBottom = lastVisible.offset.y + lastVisible.size.height
                    isLastItemVisible && lastItemBottom <= layoutInfo.viewportEndOffset
                }
            }
        }
    val selectedIndex = levels.indexOfFirst { it.puzzleId == selectedPuzzleId }
    val density = LocalDensity.current
    val configuration = LocalConfiguration.current
    // Compose's Modifier.aspectRatio() expects width / height.
    // Using screenWidth/screenHeight yields portrait cards on a portrait screen.
    val cardAspect =
        configuration.screenWidthDp.toFloat() / configuration.screenHeightDp.toFloat()

    LaunchedEffect(selectedIndex, refreshNonce) {
        if (selectedIndex < 0) return@LaunchedEffect

        // Wait until the grid has measured so we know spanCount and viewport height.
        repeat(20) {
            val viewportWidthPx = gridState.layoutInfo.viewportSize.width
            val viewportHeightPx = gridState.layoutInfo.viewportSize.height
            if (viewportWidthPx > 0 && viewportHeightPx > 0) {
                val spacingPx = with(density) { 10.dp.roundToPx() }

                // We now use fixed columns: GridCells.Fixed(3).
                val spanCount = 3

                // Compute the actual item height from the measured viewport width and the card aspect ratio.
                val cellWidthPx =
                    ((viewportWidthPx - (spacingPx * (spanCount - 1))) / spanCount)
                        .coerceAtLeast(1)
                val cellHeightPx = (cellWidthPx / cardAspect).roundToInt().coerceAtLeast(1)

                // One row's vertical step is the card height plus the vertical spacing.
                val itemStepPx = cellHeightPx + spacingPx
                val rowsPerScreen = (viewportHeightPx / itemStepPx).coerceAtLeast(1)
                val selectedRow = selectedIndex / spanCount
                val targetTopRow = (selectedRow - (rowsPerScreen / 2)).coerceAtLeast(0)
                val targetIndex = (targetTopRow * spanCount).coerceAtLeast(0)
                gridState.scrollToItem(targetIndex)
                gridReady = 1
                return@LaunchedEffect
            }
        }

        // Fallback if layout info didn't become available quickly.
        gridState.scrollToItem(selectedIndex)
        gridReady = 1
    }

    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                    onClick = {},
                ),
    ) {
        Image(
            painter = painterResource(R.drawable.bg_space),
            contentDescription = null,
            contentScale = ContentScale.Crop,
            modifier = Modifier.fillMaxSize(),
        )
        LazyVerticalGrid(
            state = gridState,
            columns = GridCells.Fixed(3),
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(horizontal = 16.dp, vertical = 8.dp)
                    .alpha(if (gridReady == 1) 1f else 0f),
            verticalArrangement = Arrangement.spacedBy(10.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            items(levels, key = { it.puzzleId }) { level ->
                val isSelected = level.puzzleId == selectedPuzzleId
                LevelCard(
                    level = level,
                    isSelected = isSelected,
                    aspectRatio = cardAspect,
                    onToggleDislike = {
                        onToggleDislike(level.puzzleId)
                    },
                    onToggleLike = {
                        onToggleLike(level.puzzleId)
                    },
                    onToggleStar = {
                        onToggleStar(level.puzzleId)
                    },
                    onClick = {
                        if (!isSelected) {
                            onPickLevel(level.puzzleId)
                        }
                        onDismiss()
                    },
                )
            }
        }

        OverlayCircleIconButton(
            iconRes = R.drawable.ic_back,
            contentDescription = "Back",
            onClick = onDismiss,
            modifier =
                Modifier
                    .align(Alignment.TopStart)
                    .padding(start = 18.dp, top = 12.dp),
        )

        if (!isAtTop) {
            OverlayCircleIconButton(
                iconRes = R.drawable.ic_up,
                contentDescription = "Scroll to top",
                onClick = {
                    scope.launch {
                        gridState.scrollToItem(0)
                    }
                },
                modifier =
                    Modifier
                        .align(Alignment.TopCenter)
                        .padding(top = 12.dp),
            )
        }

        if (!isAtBottom) {
            OverlayCircleIconButton(
                iconRes = R.drawable.ic_down,
                contentDescription = "Scroll to bottom",
                onClick = {
                    if (levels.isNotEmpty()) {
                        scope.launch {
                            gridState.scrollToItem(levels.lastIndex)
                        }
                    }
                },
                modifier =
                    Modifier
                        .align(Alignment.BottomCenter)
                        .padding(bottom = 12.dp),
            )
        }
    }
}

@Composable
private fun OverlayCircleIconButton(
    iconRes: Int,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier =
            modifier
                .size(52.dp)
                .clip(CircleShape)
                .clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                    onClick = onClick,
                ),
        contentAlignment = Alignment.Center,
    ) {
        Canvas(modifier = Modifier.matchParentSize()) {
            drawCircle(
                brush =
                    androidx.compose.ui.graphics.Brush.radialGradient(
                        colors =
                            listOf(
                                Color.Black.copy(alpha = 0.55f),
                                Color.Black.copy(alpha = 0.25f),
                                Color.Black.copy(alpha = 0.01f),
                                Color.Transparent,
                            ),
                        center = center,
                        radius = size.maxDimension * 0.75f,
                    ),
            )
        }
        Image(
            painter = painterResource(iconRes),
            contentDescription = contentDescription,
            colorFilter = ColorFilter.tint(Color.LightGray),
            modifier = Modifier.size(32.dp),
        )
    }
}

@Composable
private fun LevelCard(
    level: LevelSummary,
    isSelected: Boolean,
    aspectRatio: Float,
    onToggleDislike: () -> Unit,
    onToggleLike: () -> Unit,
    onToggleStar: () -> Unit,
    onClick: () -> Unit,
) {
    val context = LocalContext.current
    val gameAssets = remember(context) { AndroidGameAssets(context) }

    val borderColor = Color.LightGray
    val borderWidth = if (isSelected) 3.dp else 0.dp
    val cardShape = RoundedCornerShape(8.dp)

    val density = LocalDensity.current
    val selectionStrokePx = remember(density) { with(density) { 6.dp.toPx() } }
    val selectionCornerRadiusPx = remember(density) { with(density) { 8.dp.toPx() } }

    val trashIcon =
        if (level.rating == -1) {
            R.drawable.ic_trash_filled
        } else {
            R.drawable.ic_trash
        }

    val heartIcon =
        if (level.rating == 1) {
            R.drawable.ic_heart_filled
        } else {
            R.drawable.ic_heart
        }
    val starIcon =
        if (level.isStarred) {
            R.drawable.ic_star_filled
        } else {
            R.drawable.ic_star
        }

    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .aspectRatio(aspectRatio)
                .clip(cardShape)
                .clickable(onClick = onClick),
    ) {
        Canvas(
            modifier = Modifier.matchParentSize(),
        ) {
            // Selected cards lighten more; others lighten subtly.
            val alpha = if (isSelected) 0.40f else 0.16f
            drawRect(
                color = Color.White.copy(alpha = alpha),
                blendMode = androidx.compose.ui.graphics.BlendMode.Screen,
            )
        }
        // Soft halo that will only show through VOID areas.
        if (level.isCompleted) {
            Canvas(
                modifier = Modifier.matchParentSize(),
            ) {
                val radius = size.minDimension * 0.70f
                drawCircle(
                    brush =
                        androidx.compose.ui.graphics.Brush.radialGradient(
                            colors =
                                listOf(
                                    Color(0xFFD0D0D0).copy(alpha = 0.60f),
                                    Color(0xFFD0D0D0).copy(alpha = 0.22f),
                                    Color(0xFFD0D0D0).copy(alpha = 0.06f),
                                    Color.Transparent,
                                ),
                            center = center,
                            radius = radius,
                        ),
                )
            }
        }

        LevelMapPreview(
            board = level.boardGeometry,
            assets = gameAssets,
            isSelected = isSelected,
            modifier = Modifier.fillMaxSize(),
        )

        // Title in the top-left corner
        Box(
            modifier =
                Modifier
                    .align(Alignment.TopStart)
                    .padding(horizontal = 8.dp, vertical = 6.dp)
                    .clip(RoundedCornerShape(6.dp))
                    .background(Color.Black.copy(alpha = 0.40f)),
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.Start,
                modifier =
                    Modifier
                        .padding(horizontal = 6.dp, vertical = 3.dp),
            ) {
                Text(
                    text = level.name,
                    fontSize = 16.sp,
                    color = Color.LightGray,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    textAlign = TextAlign.Start,
                    modifier = Modifier,
                )
            }
        }

        // Large completion check drawn over the map.
        if (level.isCompleted) {
            Image(
                painter = painterResource(R.drawable.ic_check),
                contentDescription = "Completed",
                colorFilter = ColorFilter.tint(Color.Black.copy(alpha = 0.16f)),
                modifier =
                    Modifier
                        .align(Alignment.Center)
                        .fillMaxSize(0.5f),
            )
        }

        // Trash in the bottom-left corner
        Box(
            modifier =
                Modifier
                    .align(Alignment.BottomStart)
                    .padding(horizontal = 10.dp, vertical = 8.dp)
                    .clip(RoundedCornerShape(6.dp)),
        ) {
            Canvas(
                modifier = Modifier.matchParentSize(),
            ) {
                drawRoundRect(
                    brush =
                        androidx.compose.ui.graphics.Brush.radialGradient(
                            colors =
                                listOf(
                                    Color.Black.copy(alpha = 0.35f),
                                    Color.Black.copy(alpha = 0.25f),
                                    Color.Black.copy(alpha = 0.01f),
                                    Color.Transparent,
                                ),
                            center = center,
                            radius = size.maxDimension * 1.15f,
                        ),
                )
            }
            Image(
                painter = painterResource(trashIcon),
                contentDescription = "Trash",
                colorFilter = ColorFilter.tint(Color.LightGray),
                modifier =
                    Modifier
                        .padding(horizontal = 6.dp, vertical = 4.dp)
                        .size(24.dp)
                        .clickable(
                            interactionSource = remember { MutableInteractionSource() },
                            indication = null,
                            onClick = onToggleDislike,
                        ),
            )
        }

        // Star in the top-right corner
        Box(
            modifier =
                Modifier
                    .align(Alignment.TopEnd)
                    .padding(horizontal = 10.dp, vertical = 8.dp)
                    .clip(RoundedCornerShape(6.dp)),
        ) {
            Canvas(
                modifier = Modifier.matchParentSize(),
            ) {
                drawRoundRect(
                    brush =
                        androidx.compose.ui.graphics.Brush.radialGradient(
                            colors =
                                listOf(
                                    Color.Black.copy(alpha = 0.35f),
                                    Color.Black.copy(alpha = 0.25f),
                                    Color.Black.copy(alpha = 0.01f),
                                    Color.Transparent,
                                ),
                            center = center,
                            radius = size.maxDimension * 1.15f,
                        ),
                )
            }
            Image(
                painter = painterResource(starIcon),
                contentDescription = "Star",
                colorFilter = ColorFilter.tint(Color.LightGray),
                modifier =
                    Modifier
                        .padding(horizontal = 6.dp, vertical = 4.dp)
                        .size(24.dp)
                        .clickable(
                            interactionSource = remember { MutableInteractionSource() },
                            indication = null,
                            onClick = onToggleStar,
                        ),
            )
        }

        // Heart in the bottom-right corner
        Box(
            modifier =
                Modifier
                    .align(Alignment.BottomEnd)
                    .padding(horizontal = 10.dp, vertical = 8.dp)
                    .clip(RoundedCornerShape(6.dp)),
        ) {
            Canvas(
                modifier = Modifier.matchParentSize(),
            ) {
                drawRoundRect(
                    brush =
                        androidx.compose.ui.graphics.Brush.radialGradient(
                            colors =
                                listOf(
                                    Color.Black.copy(alpha = 0.35f),
                                    Color.Black.copy(alpha = 0.25f),
                                    Color.Black.copy(alpha = 0.01f),
                                    Color.Transparent,
                                ),
                            center = center,
                            radius = size.maxDimension * 1.15f,
                        ),
                )
            }
            Image(
                painter = painterResource(heartIcon),
                contentDescription = "Heart",
                colorFilter = ColorFilter.tint(Color.LightGray),
                modifier =
                    Modifier
                        .padding(horizontal = 6.dp, vertical = 4.dp)
                        .size(24.dp)
                        .clickable(
                            interactionSource = remember { MutableInteractionSource() },
                            indication = null,
                            onClick = onToggleLike,
                        ),
            )
        }
    }
}

@Composable
private fun LevelMapPreview(
    board: LevelBoardGeometry,
    assets: AndroidGameAssets,
    isSelected: Boolean,
    modifier: Modifier = Modifier,
) {
    val density = LocalDensity.current
    val selectionStrokePx = remember(density) { with(density) { 6.dp.toPx() } }
    val selectionCornerRadiusPx = remember(density) { with(density) { 8.dp.toPx() } }
    Canvas(
        modifier = modifier,
    ) {
        if (board.rowCount == 0 || board.columnCount == 0) return@Canvas

        // Add one cell of padding on each side.
        val paddedCols = board.columnCount + 2
        val paddedRows = board.rowCount + 2

        val cellSize =
            min(size.width / paddedCols.toFloat(), size.height / paddedRows.toFloat())

        val boardWidth = cellSize * board.columnCount
        val boardHeight = cellSize * board.rowCount

        // Board starts after one cell of padding.
        val startX = cellSize
        val startY = cellSize

        val floorFillColor = Color.White
        val goalFillColor = Color(0xFFE4E4E4)
        val gridLineColor = Color(0xFFCDCDCD)

        val colEdges =
            IntArray(board.columnCount + 1) { col -> (startX + (col * cellSize)).roundToInt() }
        val rowEdges =
            IntArray(board.rowCount + 1) { row -> (startY + (row * cellSize)).roundToInt() }

        for (row in 0 until board.rowCount) {
            for (col in 0 until board.columnCount) {
                val tile = board.tileAt(row, col)
                if (tile == LevelBoardTile.VOID) continue

                val left = colEdges[col].toFloat()
                val right = colEdges[col + 1].toFloat()
                val top = rowEdges[row].toFloat()
                val bottom = rowEdges[row + 1].toFloat()
                if (right <= left || bottom <= top) continue

                drawRect(
                    color = if (tile == LevelBoardTile.GOAL) goalFillColor else floorFillColor,
                    topLeft =
                        androidx.compose.ui.geometry
                            .Offset(left, top),
                    size =
                        androidx.compose.ui.geometry
                            .Size(right - left, bottom - top),
                )
            }
        }

        // Draw 1px grid lines exactly once per shared tile edge.
        for (colBoundary in 1 until board.columnCount) {
            val x = colEdges[colBoundary].toFloat()
            var runStartRow = -1
            for (row in 0 until board.rowCount) {
                val hasBoundary =
                    board.tileAt(row, colBoundary - 1) != LevelBoardTile.VOID &&
                        board.tileAt(row, colBoundary) != LevelBoardTile.VOID
                if (hasBoundary && runStartRow == -1) {
                    runStartRow = row
                } else if (!hasBoundary && runStartRow != -1) {
                    drawLine(
                        color = gridLineColor,
                        start =
                            androidx.compose.ui.geometry.Offset(
                                x,
                                rowEdges[runStartRow].toFloat(),
                            ),
                        end =
                            androidx.compose.ui.geometry
                                .Offset(x, rowEdges[row].toFloat()),
                        strokeWidth = 1f,
                    )
                    runStartRow = -1
                }
            }
            if (runStartRow != -1) {
                drawLine(
                    color = gridLineColor,
                    start =
                        androidx.compose.ui.geometry
                            .Offset(x, rowEdges[runStartRow].toFloat()),
                    end =
                        androidx.compose.ui.geometry.Offset(
                            x,
                            rowEdges[board.rowCount].toFloat(),
                        ),
                    strokeWidth = 1f,
                )
            }
        }

        for (rowBoundary in 1 until board.rowCount) {
            val y = rowEdges[rowBoundary].toFloat()
            var runStartCol = -1
            for (col in 0 until board.columnCount) {
                val hasBoundary =
                    board.tileAt(rowBoundary - 1, col) != LevelBoardTile.VOID &&
                        board.tileAt(rowBoundary, col) != LevelBoardTile.VOID
                if (hasBoundary && runStartCol == -1) {
                    runStartCol = col
                } else if (!hasBoundary && runStartCol != -1) {
                    drawLine(
                        color = gridLineColor,
                        start =
                            androidx.compose.ui.geometry.Offset(
                                colEdges[runStartCol].toFloat(),
                                y,
                            ),
                        end =
                            androidx.compose.ui.geometry
                                .Offset(colEdges[col].toFloat(), y),
                        strokeWidth = 1f,
                    )
                    runStartCol = -1
                }
            }
            if (runStartCol != -1) {
                drawLine(
                    color = gridLineColor,
                    start =
                        androidx.compose.ui.geometry
                            .Offset(colEdges[runStartCol].toFloat(), y),
                    end =
                        androidx.compose.ui.geometry.Offset(
                            colEdges[board.columnCount].toFloat(),
                            y,
                        ),
                    strokeWidth = 1f,
                )
            }
        }

        // Draw selection brackets only in the rounded corners.
        if (isSelected) {
            val selectionColor = Color.LightGray
            val strokeWidth = selectionStrokePx
            val radius = selectionCornerRadiusPx

            val left = 0f
            val top = 0f
            val right = size.width
            val bottom = size.height

            val diameter = radius * 2f
            val extension = diameter * 0.75f // shorter bracket arms

            // Top-left
            drawArc(
                color = selectionColor,
                startAngle = 180f,
                sweepAngle = 90f,
                useCenter = false,
                topLeft =
                    androidx.compose.ui.geometry
                        .Offset(left, top),
                size =
                    androidx.compose.ui.geometry
                        .Size(diameter, diameter),
                style =
                    androidx.compose.ui.graphics.drawscope
                        .Stroke(width = strokeWidth),
            )
            // vertical extension
            drawLine(
                color = selectionColor,
                start =
                    androidx.compose.ui.geometry
                        .Offset(left, top + radius),
                end =
                    androidx.compose.ui.geometry
                        .Offset(left, top + radius + extension),
                strokeWidth = strokeWidth,
            )
            // horizontal extension
            drawLine(
                color = selectionColor,
                start =
                    androidx.compose.ui.geometry
                        .Offset(left + radius, top),
                end =
                    androidx.compose.ui.geometry
                        .Offset(left + radius + extension, top),
                strokeWidth = strokeWidth,
            )

            // Top-right
            drawArc(
                color = selectionColor,
                startAngle = 270f,
                sweepAngle = 90f,
                useCenter = false,
                topLeft =
                    androidx.compose.ui.geometry
                        .Offset(right - diameter, top),
                size =
                    androidx.compose.ui.geometry
                        .Size(diameter, diameter),
                style =
                    androidx.compose.ui.graphics.drawscope
                        .Stroke(width = strokeWidth),
            )
            drawLine(
                color = selectionColor,
                start =
                    androidx.compose.ui.geometry
                        .Offset(right, top + radius),
                end =
                    androidx.compose.ui.geometry
                        .Offset(right, top + radius + extension),
                strokeWidth = strokeWidth,
            )
            drawLine(
                color = selectionColor,
                start =
                    androidx.compose.ui.geometry
                        .Offset(right - radius, top),
                end =
                    androidx.compose.ui.geometry
                        .Offset(right - radius - extension, top),
                strokeWidth = strokeWidth,
            )

            // Bottom-right
            drawArc(
                color = selectionColor,
                startAngle = 0f,
                sweepAngle = 90f,
                useCenter = false,
                topLeft =
                    androidx.compose.ui.geometry
                        .Offset(right - diameter, bottom - diameter),
                size =
                    androidx.compose.ui.geometry
                        .Size(diameter, diameter),
                style =
                    androidx.compose.ui.graphics.drawscope
                        .Stroke(width = strokeWidth),
            )
            drawLine(
                color = selectionColor,
                start =
                    androidx.compose.ui.geometry
                        .Offset(right, bottom - radius),
                end =
                    androidx.compose.ui.geometry
                        .Offset(right, bottom - radius - extension),
                strokeWidth = strokeWidth,
            )
            drawLine(
                color = selectionColor,
                start =
                    androidx.compose.ui.geometry
                        .Offset(right - radius, bottom),
                end =
                    androidx.compose.ui.geometry
                        .Offset(right - radius - extension, bottom),
                strokeWidth = strokeWidth,
            )

            // Bottom-left
            drawArc(
                color = selectionColor,
                startAngle = 90f,
                sweepAngle = 90f,
                useCenter = false,
                topLeft =
                    androidx.compose.ui.geometry
                        .Offset(left, bottom - diameter),
                size =
                    androidx.compose.ui.geometry
                        .Size(diameter, diameter),
                style =
                    androidx.compose.ui.graphics.drawscope
                        .Stroke(width = strokeWidth),
            )
            drawLine(
                color = selectionColor,
                start =
                    androidx.compose.ui.geometry
                        .Offset(left, bottom - radius),
                end =
                    androidx.compose.ui.geometry
                        .Offset(left, bottom - radius - extension),
                strokeWidth = strokeWidth,
            )
            drawLine(
                color = selectionColor,
                start =
                    androidx.compose.ui.geometry
                        .Offset(left + radius, bottom),
                end =
                    androidx.compose.ui.geometry
                        .Offset(left + radius + extension, bottom),
                strokeWidth = strokeWidth,
            )
        }

        val bitmapPaint = assets.bitmapPaint()

        drawIntoCanvas { canvas ->
            val native = canvas.nativeCanvas

            for (box in board.boxes) {
                val tileLeft = colEdges[box.col].toFloat()
                val tileRight = colEdges[box.col + 1].toFloat()
                val tileTop = rowEdges[box.row].toFloat()
                val tileBottom = rowEdges[box.row + 1].toFloat()
                val tileWidth = tileRight - tileLeft
                val tileHeight = tileBottom - tileTop
                val baseSize = min(tileWidth, tileHeight)
                val boxSizePx = (baseSize * 0.90f).roundToInt().coerceAtLeast(1)
                val boxBitmap = assets.getBitmap(R.drawable.box, boxSizePx)
                val x = tileLeft + ((tileWidth - boxSizePx) / 2f).roundToInt()
                val y = tileTop + ((tileHeight - boxSizePx) / 2f).roundToInt()
                native.drawBitmap(boxBitmap, x, y, bitmapPaint)
            }

            val playerTileLeft = colEdges[board.player.col].toFloat()
            val playerTileRight = colEdges[board.player.col + 1].toFloat()
            val playerTileTop = rowEdges[board.player.row].toFloat()
            val playerTileBottom = rowEdges[board.player.row + 1].toFloat()
            val playerTileWidth = playerTileRight - playerTileLeft
            val playerTileHeight = playerTileBottom - playerTileTop
            val playerBaseSize = min(playerTileWidth, playerTileHeight)
            val playerSizePx = (playerBaseSize * 0.80f).roundToInt().coerceAtLeast(1)
            val playerBitmap = assets.getBitmap(R.drawable.player_slime, playerSizePx)
            val playerX = playerTileLeft + ((playerTileWidth - playerSizePx) / 2f).roundToInt()
            val playerY = playerTileTop + ((playerTileHeight - playerSizePx) / 2f).roundToInt()
            native.drawBitmap(playerBitmap, playerX, playerY, bitmapPaint)
        }
    }
}
