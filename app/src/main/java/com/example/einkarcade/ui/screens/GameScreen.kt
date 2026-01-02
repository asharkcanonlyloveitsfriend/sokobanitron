package com.example.einkarcade.ui.screens

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowForward
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material.icons.outlined.FavoriteBorder
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.vectorResource
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.einkarcade.GameController
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.drawBox
import com.example.einkarcade.ui.rendering.drawFloor
import com.example.einkarcade.ui.rendering.drawGoal
import com.example.einkarcade.ui.rendering.drawPlayer
import kotlinx.coroutines.delay
import kotlin.math.min
import kotlin.math.roundToInt


@Composable
fun GameScreen(
    modifier: Modifier = Modifier,
    gameController: GameController,
    selectedBoxPosition: MutableState<Position?>
) {
    gameController.revision.value
    val playerPosition = gameController.playerPosition
    val displayedPlayerPosition = remember { mutableStateOf(playerPosition) }
    val pendingPlayerPosition = remember { mutableStateOf<Position?>(null) }
    val holdPlayerPosition = remember { mutableStateOf(false) }
    val syncError = remember { mutableStateOf<String?>(null) }
    val syncSuccess = remember { mutableStateOf(false) }
    val backDownTime = remember { mutableStateOf<Long?>(null) }
    val boxPathShrink = remember { mutableStateOf(0f) }
    val boxPathActive = remember { mutableStateOf(false) }
    val boxPathTrigger = remember { mutableStateOf(0) }
    val boxPathPositions = remember { mutableStateOf<List<Position>>(emptyList()) }
    val boxPainter = painterResource(id = R.drawable.box)
    val selectedBoxPainter = painterResource(id = R.drawable.box_selected)
    val playerPainter = painterResource(id = R.drawable.player_slime)
    val focusRequester = remember { FocusRequester() }
    data class VanishState(val position: Position, val step: Int)
    val vanishingTile = remember { mutableStateOf<VanishState?>(null) }

    BackHandler(enabled = true) {
        // handled manually via key events below
    }

    LaunchedEffect(Unit) {
        focusRequester.requestFocus()
    }

    LaunchedEffect(boxPathTrigger.value) {
        val durationMs = 100L
        val stepMs = 10L
        val steps = (durationMs / stepMs).coerceAtLeast(1)
        boxPathActive.value = true
        boxPathShrink.value = 0f
        for (i in 1..steps) {
            delay(stepMs)
            boxPathShrink.value = min(1f, i.toFloat() / steps.toFloat())
        }
        boxPathActive.value = false
        pendingPlayerPosition.value?.let { pending ->
            displayedPlayerPosition.value = pending
            pendingPlayerPosition.value = null
        }
        holdPlayerPosition.value = false
    }

    LaunchedEffect(playerPosition, holdPlayerPosition.value) {
        if (!holdPlayerPosition.value) {
            displayedPlayerPosition.value = playerPosition
        }
    }

    Box(
        modifier = modifier
            .fillMaxSize()
            .focusRequester(focusRequester)
            .focusable()
            .onKeyEvent { event ->
                if (event.key == Key.Back) {
                    when (event.type) {
                        KeyEventType.KeyDown -> {
                            if (backDownTime.value == null) {
                                backDownTime.value = SystemClock.elapsedRealtime()
                            }
                            true
                        }
                        KeyEventType.KeyUp -> {
                            val downTime = backDownTime.value
                            backDownTime.value = null
                            if (downTime != null) {
                                val duration = SystemClock.elapsedRealtime() - downTime
                                if (duration >= 500) {
                                    // Long press → Restart
                                    gameController.restart()
                                } else {
                                    // Short press → Undo
                                    gameController.undo()
                                }
                            }
                            true
                        }
                        else -> false
                    }
                } else {
                    false
                }
            }
    ) {
        Image(
            painter = painterResource(id = R.drawable.bg_space),
            contentDescription = null,
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Crop
        )
        val VANISH_BASE_DELAY_MS = 170L
        val WHITE_FLASH_DELAY_MS = 100L
        val INVISIBLE_DELAY_MS = 100L

        fun advanceVanish(step: Int, position: Position) {
            val delay = when (step) {
                0 -> VANISH_BASE_DELAY_MS
                1 -> (VANISH_BASE_DELAY_MS * 0.75f).toLong()
                2 -> (VANISH_BASE_DELAY_MS * 0.5f).toLong()
                3 -> (VANISH_BASE_DELAY_MS * 0.36f).toLong()
                4 -> (VANISH_BASE_DELAY_MS * 0.2f).toLong()
                5 -> INVISIBLE_DELAY_MS
                else -> WHITE_FLASH_DELAY_MS
            }

            Handler(Looper.getMainLooper()).postDelayed({
                val nextStep = step + 1
                if (nextStep >= 7) {
                    vanishingTile.value = null
                    return@postDelayed
                }
                vanishingTile.value = VanishState(position, nextStep)
                advanceVanish(nextStep, position)
            }, delay)
        }

        fun handleTap(tappedPosition: Position) {
            val tile = gameController.tiles[tappedPosition.row][tappedPosition.col]
            if (tile == Tile.WALL) {
                vanishingTile.value = VanishState(tappedPosition, 0)
                advanceVanish(0, tappedPosition)
                return
            }
            val selectedBox = selectedBoxPosition.value

            if (gameController.boxPositions.contains(tappedPosition)) {
                if (selectedBox == tappedPosition) {
                    selectedBoxPosition.value = null
                } else {
                    selectedBoxPosition.value = tappedPosition
                }
            } else if (selectedBox != null) {
                selectedBoxPosition.value = null
                holdPlayerPosition.value = true
                val boxPath = gameController.moveBoxTo(selectedBox, tappedPosition)
                if (boxPath != null) {
                    pendingPlayerPosition.value = gameController.playerPosition
                    boxPathPositions.value = boxPath
                    boxPathTrigger.value += 1
                } else {
                    holdPlayerPosition.value = false
                }
            } else {
                gameController.movePlayerTo(tappedPosition)
            }
        }

        fun buildTestPath(): List<Position> {
            val lineColIndex = 3
            val maxRowIndex = gameController.tiles.size - 1
            val colCount = gameController.tiles.firstOrNull()?.size ?: 0
            if (lineColIndex !in 0 until colCount || maxRowIndex < 0) {
                return emptyList()
            }
            val endRow = min(6, maxRowIndex)
            return (0..endRow).map { Position(it, lineColIndex) }
        }

        Column(modifier = Modifier.fillMaxSize()) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // --- Set (top-left) ---
                val setExpanded = remember { mutableStateOf(false) }
                val setOptions = gameController.availableSetOptions

                Box(
                    modifier = Modifier
                        .clickable { setExpanded.value = true }
                ) {
                    Text(
                        text = gameController.currentSetName,
                        fontSize = 16.sp,
                        color = Color.LightGray,
                        modifier = Modifier
                            .background(
                                Color.Black,
                                shape = androidx.compose.foundation.shape.RoundedCornerShape(6.dp)
                            )
                            .padding(horizontal = 6.dp, vertical = 2.dp)
                    )

                    DropdownMenu(
                        expanded = setExpanded.value,
                        onDismissRequest = { setExpanded.value = false }
                    ) {
                        Column(
                            modifier = Modifier.heightIn(max = 800.dp)
                        ) {
                            setOptions.forEach { (id, name) ->
                                val isSelected = name == gameController.currentSetName
                                DropdownMenuItem(
                                    text = { Text(name) },
                                    onClick = {
                                        gameController.selectSetById(id)
                                        setExpanded.value = false
                                    },
                                    contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .background(
                                            if (isSelected) Color.LightGray else Color.Transparent
                                        )
                                )
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.weight(1f))

                // --- Level (top-right, right-aligned) ---
                val levelExpanded = remember { mutableStateOf(false) }
                val levels = gameController.levels()
                val currentLevelName = gameController.levelName
                val selectedLevelIndex = levels.indexOfFirst { it.name == currentLevelName }
                val levelScrollState = rememberScrollState()
                val density = LocalDensity.current
                val itemHeight: Dp = 40.dp

                Box(
                    modifier = Modifier
                        .clickable { levelExpanded.value = true }
                ) {
                    Text(
                        text = gameController.levelName,
                        fontSize = 16.sp,
                        color = Color.LightGray,
                        modifier = Modifier
                            .background(
                                Color.Black,
                                shape = androidx.compose.foundation.shape.RoundedCornerShape(6.dp)
                            )
                            .padding(horizontal = 6.dp, vertical = 2.dp)
                    )

                    DropdownMenu(
                        expanded = levelExpanded.value,
                        onDismissRequest = { levelExpanded.value = false }
                    ) {
                        LaunchedEffect(levelExpanded.value, selectedLevelIndex) {
                            if (levelExpanded.value && selectedLevelIndex >= 0) {
                                val targetIndex = (selectedLevelIndex - 2).coerceAtLeast(0)
                                val targetOffset =
                                    with(density) { (itemHeight * targetIndex).roundToPx() }
                                levelScrollState.scrollTo(targetOffset)
                            }
                        }

                        Column(
                            modifier = Modifier
                                .heightIn(max = 800.dp)
                                .verticalScroll(levelScrollState)
                        ) {
                            levels.forEach { lvl ->
                                val completedMark = if (lvl.isCompleted) " ✓" else ""
                                val ratingBadge =
                                    when (lvl.rating) { 1 -> " 👍"; -1 -> " 👎"; else -> "" }
                                val isSelected = lvl.name == currentLevelName

                                DropdownMenuItem(
                                    text = { Text(lvl.name + completedMark + ratingBadge) },
                                    onClick = {
                                        gameController.selectLevel(lvl.name)
                                        levelExpanded.value = false
                                    },
                                    contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .background(
                                            if (isSelected) Color.LightGray else Color.Transparent
                                        )
                                )
                            }
                        }
                    }
                }
            }

            Canvas(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .testTag("gameCanvas")
                    .pointerInput(Unit) {
                        detectTapGestures { offset ->
                            val rows = gameController.tiles.size + 2
                            val cols = (gameController.tiles.firstOrNull()?.size ?: 0) + 2
                            if (rows == 0 || cols == 0) return@detectTapGestures

                            val tileSizeByWidth = size.width / cols
                            val tileSizeByHeight = size.height / rows
                            val cellSize = min(tileSizeByWidth, tileSizeByHeight)

                            val renderedWidth = cellSize * cols
                            val renderedHeight = cellSize * rows

                            val offsetX = (size.width - renderedWidth) / 2f
                            val offsetY = (size.height - renderedHeight) / 2f

                            val col = ((offset.x - offsetX) / cellSize).toInt()
                            val row = ((offset.y - offsetY) / cellSize).toInt()
                            val innerRow = row - 1
                            val innerCol = col - 1
                            if (!gameController.isGameWon &&
                                innerRow in gameController.tiles.indices &&
                                innerCol in gameController.tiles[0].indices
                            ) {
                                handleTap(Position(innerRow, innerCol))
                            }
                        }
                    }
            ) {
                val rows = gameController.tiles.size + 2
                val cols = (gameController.tiles.firstOrNull()?.size ?: 0) + 2

                if (rows == 0 || cols == 0) return@Canvas

                val tileSizeByWidth = size.width / cols
                val tileSizeByHeight = size.height / rows
                val cellSize = min(tileSizeByWidth, tileSizeByHeight)

                val renderedWidth = cellSize * cols
                val renderedHeight = cellSize * rows

                val offsetX = (size.width - renderedWidth) / 2f
                val offsetY = (size.height - renderedHeight) / 2f

                for ((rowIndex, row) in gameController.tiles.withIndex()) {
                    for ((colIndex, tile) in row.withIndex()) {
                        val paddedRow = rowIndex + 1
                        val paddedCol = colIndex + 1
                        when (tile) {
                            Tile.GOAL -> drawGoal(Position(paddedRow, paddedCol), cellSize, offsetX, offsetY)
                            Tile.FLOOR -> drawFloor(Position(paddedRow, paddedCol), cellSize, offsetX, offsetY)
                            Tile.WALL -> {
                                val vanish = vanishingTile.value
                                if (vanish != null && vanish.position == Position(rowIndex, colIndex)) {
                                    when (vanish.step) {
                                        0, 1, 2, 3, 4 -> {
                                            val tileLeft = offsetX + paddedCol * cellSize
                                            val tileTop = offsetY + paddedRow * cellSize
                                            val baseSize =
                                                (cellSize * 0.90f * 0.72f).roundToInt().toFloat()
                                            val baseLeft = (tileLeft + (cellSize - baseSize) / 2).roundToInt().toFloat()
                                            val baseTop = (tileTop + (cellSize - baseSize) / 2).roundToInt().toFloat()
                                            val scale = when (vanish.step) {
                                                0 -> 1.0f
                                                1 -> 0.75f
                                                2 -> 0.5f
                                                3 -> 0.3f
                                                else -> 0.18f
                                            }
                                            val size = baseSize * scale
                                            val left = baseLeft + (baseSize - size) / 2
                                            val top = baseTop + (baseSize - size) / 2
                                            val baseRadius = baseSize * (14f / 72f)
                                            val innerRadius = size * (14f / 72f)

                                            val shade = when (vanish.step) {
                                                0 -> Color(0xFF6B7280)
                                                1 -> Color(0xFF646C79)
                                                2 -> Color(0xFF5E6672)
                                                else -> Color(0xFF58616C)
                                            }

                                            if (vanish.step == 0) {
                                                drawBox(
                                                    Position(paddedRow, paddedCol),
                                                    boxPainter,
                                                    selectedBoxPainter,
                                                    false,
                                                    cellSize,
                                                    offsetX,
                                                    offsetY
                                                )
                                            } else {
                                                drawRoundRect(
                                                    color = shade,
                                                    topLeft = androidx.compose.ui.geometry.Offset(
                                                        left,
                                                        top
                                                    ),
                                                    size = androidx.compose.ui.geometry.Size(size, size),
                                                    cornerRadius = CornerRadius(innerRadius, innerRadius)
                                                )
                                            }
                                        }
                                        5 -> {
                                            // Invisible pause before flash.
                                        }
                                        6 -> {
                                            // full-tile white flash (not the box)
                                            drawRect(
                                                color = Color.White,
                                                topLeft = androidx.compose.ui.geometry.Offset(
                                                    offsetX + paddedCol * cellSize,
                                                    offsetY + paddedRow * cellSize
                                                ),
                                                size = androidx.compose.ui.geometry.Size(cellSize, cellSize)
                                            )
                                        }
                                    }
                                } else {
                                    // no-op: wall is invisible, background shows through
                                }
                            }
                        }
                    }
                }

                drawBoxPathLine(
                    isActive = boxPathActive.value,
                    shrink = boxPathShrink.value,
                    path = boxPathPositions.value,
                    cellSize = cellSize,
                    offsetX = offsetX,
                    offsetY = offsetY
                )

                for (position in gameController.boxPositions) {
                    drawBox(
                        Position(position.row + 1, position.col + 1),
                        boxPainter,
                        selectedBoxPainter,
                        position == selectedBoxPosition.value,
                        cellSize,
                        offsetX,
                        offsetY
                    )
                }

                val drawnPlayerPosition = displayedPlayerPosition.value
                drawPlayer(
                    Position(drawnPlayerPosition.row + 1, drawnPlayerPosition.col + 1),
                    playerPainter,
                    cellSize,
                    offsetX,
                    offsetY
                )
            }

            @Composable
            fun BottomIconButton(
                onClick: () -> Unit,
                icon: ImageVector,
                contentDescription: String
            ) {
                val interactionSource = remember { MutableInteractionSource() }
                val isPressed = interactionSource.collectIsPressedAsState()

                Box(
                    modifier = Modifier
                        .height(48.dp)
                        .background(if (isPressed.value) Color.DarkGray else Color.Black)
                        .clickable(
                            interactionSource = interactionSource,
                            indication = null,
                            onClick = onClick
                        )
                        .padding(horizontal = 12.dp)
                        .focusProperties { canFocus = false },
                    contentAlignment = Alignment.Center
                ) {
                    Icon(
                        imageVector = icon,
                        contentDescription = contentDescription,
                        tint = Color.LightGray
                    )
                }
            }

            Row(
                modifier = Modifier.padding(16.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                BottomIconButton(
                    onClick = { gameController.previousLevel() },
                    icon = Icons.Filled.ArrowBack,
                    contentDescription = "Previous level"
                )

                BottomIconButton(
                    onClick = { gameController.nextLevel() },
                    icon = Icons.Filled.ArrowForward,
                    contentDescription = "Next level"
                )
                val currentRating = gameController.getCurrentRating()

                // --- X (dislike) ---
                BottomIconButton(
                    onClick = {
                        syncSuccess.value = false
                        syncError.value = null
                        gameController.toggleThumbDown()
                    },
                    icon = ImageVector.vectorResource(
                        if (currentRating == -1) R.drawable.ic_dislike_filled else R.drawable.ic_dislike_outline
                    ),
                    contentDescription = "Dislike level"
                )

                // --- Heart (like) ---
                BottomIconButton(
                    onClick = {
                        syncSuccess.value = false
                        syncError.value = null
                        gameController.toggleThumbUp()
                    },
                    icon = if (currentRating == 1) Icons.Filled.Favorite else Icons.Outlined.FavoriteBorder,
                    contentDescription = "Like level"
                )


                Spacer(modifier = Modifier.weight(1f))

                BottomIconButton(
                    onClick = {
                        val path = buildTestPath()
                        if (path.isNotEmpty()) {
                            boxPathPositions.value = path
                            boxPathTrigger.value += 1
                        }
                    },
                    icon = Icons.Filled.Info,
                    contentDescription = "Test animation"
                )

                BottomIconButton(
                    onClick = {
                        syncError.value = null
                        syncSuccess.value = false
                        val handler = Handler(Looper.getMainLooper())
                        Thread {
                            try {
                                gameController.syncWithServer()
                                handler.post {
                                    syncSuccess.value = true
                                }
                            } catch (t: Throwable) {
                                handler.post {
                                    syncError.value = "Sync failed."
                                    syncSuccess.value = false
                                }
                            }
                        }.start()
                    },
                    icon = when {
                        syncSuccess.value -> Icons.Filled.Check
                        syncError.value != null -> Icons.Filled.Warning
                        else -> Icons.Filled.Refresh
                    },
                    contentDescription = "Sync"
                )
            }
        }

        if (gameController.isGameWon) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null
                    ) {
                        gameController.nextLevel()
                    },
                contentAlignment = Alignment.Center
            ) {
                Box(
                    modifier = Modifier
                        .padding(16.dp)
                        .background(Color.White)
                        .border(width = 2.dp, color = Color.Black)
                        .padding(16.dp)
                ) {
                    Text(
                        text = "You win!",
                        color = Color.Black,
                        fontSize = 32.sp
                    )
                }
            }
        }
    }
}

private fun DrawScope.drawBoxPathLine(
    isActive: Boolean,
    shrink: Float,
    path: List<Position>,
    cellSize: Float,
    offsetX: Float,
    offsetY: Float
) {
    if (!isActive) return
    if (path.size < 2) return

    val points = path.map { position ->
        Offset(
            offsetX + (position.col + 1) * cellSize + cellSize / 2,
            offsetY + (position.row + 1) * cellSize + cellSize / 2
        )
    }

    val totalSegments = points.size - 1
    val endT = totalSegments.toFloat()
    val startT = endT * shrink.coerceIn(0f, 1f)
    val startSegment = startT.toInt().coerceIn(0, totalSegments - 1)
    val endSegment = endT.toInt().coerceIn(0, totalSegments)
    val startFraction = startT - startSegment
    val endFraction = endT - endSegment

    fun interpolateOffset(start: Offset, end: Offset, t: Float): Offset {
        return Offset(
            start.x + (end.x - start.x) * t,
            start.y + (end.y - start.y) * t
        )
    }

    val startPoint = interpolateOffset(points[startSegment], points[startSegment + 1], startFraction)
    val endPoint = if (endSegment >= totalSegments) {
        points.last()
    } else {
        interpolateOffset(points[endSegment], points[endSegment + 1], endFraction)
    }

    val strokeWidth = cellSize * 0.2f
    val drawPoints = mutableListOf(startPoint)
    for (index in (startSegment + 1)..endSegment) {
        if (index in points.indices) {
            drawPoints.add(points[index])
        }
    }
    drawPoints.add(endPoint)

    if (drawPoints.size >= 2) {
        for (index in 0 until drawPoints.size - 1) {
            drawLine(
                color = Color(0xFFD3D3D3),
                start = drawPoints[index],
                end = drawPoints[index + 1],
                strokeWidth = strokeWidth,
                cap = androidx.compose.ui.graphics.StrokeCap.Round
            )
        }
    } else {
        drawCircle(
            color = Color(0xFFD3D3D3),
            radius = strokeWidth / 2,
            center = drawPoints.first()
        )
    }
}
