@file:Suppress("ktlint:standard:function-naming")

package com.sokobanitron.app.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.viewinterop.AndroidView
import com.sokobanitron.app.GameController
import com.sokobanitron.app.catalog.RepositoryLevelCatalog
import com.sokobanitron.app.sokoban.Position
import com.sokobanitron.app.ui.GameHud
import com.sokobanitron.app.ui.GameTitleBar
import com.sokobanitron.app.ui.SideControlsOverlay
import com.sokobanitron.app.sokoban.TileMap
import com.sokobanitron.app.ui.modes.LevelPickerOverlay
import com.sokobanitron.app.ui.modes.LevelSetPickerOverlay
import com.sokobanitron.app.ui.modes.LevelSolvedOverlay
import com.sokobanitron.app.ui.modes.LevelTransitionView
import com.sokobanitron.app.ui.rendering.GameBoardPresenter
import com.sokobanitron.app.ui.rendering.GameBoardView
import com.sokobanitron.app.ui.rendering.geom.computeBoardViewport
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

private fun createGameSurface(context: android.content.Context): GameBoardPresenter = GameBoardView(context)

@Composable
fun GameScreen(
    modifier: Modifier = Modifier,
    gameController: GameController,
) {
    val screenState = requireNotNull(gameController.screenState.value) { "Game screen state is not initialized" }
    val uiMode = gameController.uiMode
    val transitionSnapshot = gameController.transitionSnapshot.value
    val currentTileMap: TileMap = screenState.tileMap
    val surfaceRef = remember { mutableStateOf<GameBoardPresenter?>(null) }
    val context = androidx.compose.ui.platform.LocalContext.current
    val surface =
        remember {
            createGameSurface(context)
        }
    if (surfaceRef.value == null) {
        surfaceRef.value = surface
    }
    val levelCatalog = remember(context) { RepositoryLevelCatalog(context = context) }
    val currentSetName = screenState.setName
    val currentLevelName = screenState.levelName
    val currentPuzzleId = screenState.puzzleId
    val boardWidth = remember { mutableIntStateOf(0) }
    val boardHeight = remember { mutableIntStateOf(0) }
    val loadedPuzzleId = remember { mutableStateOf<Int?>(null) }
    var showLevelPicker by remember { mutableStateOf(false) }
    var showLevelSetPicker by remember { mutableStateOf(false) }
    var pickerRefreshNonce by remember { mutableLongStateOf(0L) }

    DisposableEffect(surfaceRef.value) {
        val surface = surfaceRef.value
        val sink: (GameController.RenderDelta) -> Unit = { delta ->
            surface?.applyDelta(delta)
        }
        gameController.onRenderDelta = sink
        onDispose {
            if (gameController.onRenderDelta === sink) {
                gameController.onRenderDelta = null
            }
        }
    }

    DisposableEffect(surfaceRef.value) {
        val surface = surfaceRef.value
        if (surface is GameBoardView) {
            val view = surface.asView()
            view.addOnLayoutChangeListener { _, _, _, right, bottom, _, _, _, _ ->
                val width = right
                val height = bottom
                if (width > 0 && height > 0) {
                    boardWidth.intValue = width
                    boardHeight.intValue = height
                }
            }
        }
        onDispose { }
    }

    LaunchedEffect(boardWidth.value, boardHeight.value, uiMode, currentPuzzleId) {
        if (uiMode == GameController.UiMode.GAMEPLAY &&
            boardWidth.value > 0 &&
            boardHeight.value > 0 &&
            loadedPuzzleId.value != currentPuzzleId
        ) {
            val frame =
                gameController.buildStaticBoardFrame(
                    context = context,
                    tileMap = currentTileMap,
                    width = boardWidth.value,
                    height = boardHeight.value,
                )

            gameController.emitLevelLoaded(frame)
            loadedPuzzleId.value = currentPuzzleId
        }
    }

    LaunchedEffect(uiMode) {
        if (uiMode == GameController.UiMode.LEVEL_TRANSITION) {
            loadedPuzzleId.value = null
        }
    }

    BackHandler(enabled = true) {
        GameInputHandler.handleBackKeyUp(
            gameController = gameController,
        )
    }

    Box(
        modifier =
            modifier
                .fillMaxSize(),
    ) {
        AndroidView(
            modifier =
                Modifier
                    .fillMaxSize()
                    .testTag("gameCanvas"),
            factory = {
                val selection =
                    object : GameInputHandler.BoxSelection {
                        override fun getSelectedBox(): Position? = surface.getSelectedBox()

                        override fun setSelectedBox(position: Position?) {
                            surface.setSelectedBox(position)
                        }
                    }

                surface.setOnTapCell { pos ->
                    GameInputHandler.handleTap(
                        tappedPosition = pos,
                        gameController = gameController,
                        selection = selection,
                    )
                }

                surface.asView()
            },
        )

        if (uiMode == GameController.UiMode.LEVEL_TRANSITION) {
            AndroidView(
                modifier = Modifier.fillMaxSize(),
                factory = { ctx ->
                    val snapshot = requireNotNull(transitionSnapshot) { "Missing level transition snapshot" }
                    val width = boardWidth.value
                    val height = boardHeight.value

                    check(width > 0 && height > 0) {
                        "LevelTransitionView requires board size before construction"
                    }
                    val oldViewport =
                        computeBoardViewport(
                            surfaceWidth = width.toFloat(),
                            surfaceHeight = height.toFloat(),
                            innerRows = snapshot.oldTileMap.rowCount,
                            innerCols = snapshot.oldTileMap.columnCount,
                        )

                    val newFrame =
                        gameController.buildStaticBoardFrame(
                            context = ctx,
                            tileMap = currentTileMap,
                            width = width,
                            height = height,
                        )

                    LevelTransitionView(ctx).apply {
                        setTransitionData(
                            oldViewport = oldViewport,
                            oldTileMap = snapshot.oldTileMap,
                            newFrame = newFrame,
                        )
                        onDismiss = {
                            gameController.finishLevelTransition()
                            gameController.emitLevelLoaded(newFrame)
                            loadedPuzzleId.value = currentPuzzleId
                        }
                    }
                },
            )
        }

        if (uiMode == GameController.UiMode.LEVEL_SOLVED) {
            AndroidView(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .testTag("levelSolvedView"),
                factory = { ctx ->
                    LevelSolvedOverlay(ctx).apply {
                        setRating(gameController.getCurrentRating())
                        onThumbUp = {
                            gameController.toggleThumbUp()
                            setRating(gameController.getCurrentRating())
                        }
                        onThumbDown = {
                            gameController.toggleThumbDown()
                            setRating(gameController.getCurrentRating())
                        }
                        onAdvance = { gameController.nextLevel() }
                    }
                },
            )
        }

        Column(modifier = Modifier.fillMaxSize()) {
            GameTitleBar(
                setName = currentSetName,
                levelName = currentLevelName,
                onOpenSetPicker = { showLevelSetPicker = true },
                onOpenLevelPicker = { showLevelPicker = true },
                isStarred = screenState.isStarred,
                onToggleStar = { gameController.toggleStar() },
            )

            Box(
                modifier =
                    Modifier
                        .weight(1f)
                        .fillMaxWidth(),
            ) {
            }

            if (uiMode == GameController.UiMode.GAMEPLAY) {
                GameHud(
                    currentRating = screenState.rating,
                    onThumbUp = { gameController.toggleThumbUp() },
                    onThumbDown = { gameController.toggleThumbDown() },
                )
            }
        }

        if (uiMode != GameController.UiMode.LEVEL_TRANSITION) {
            SideControlsOverlay(
                showRestartButton = gameController.showRestartControl.value,
                onRestart = { gameController.restart() },
                onSkip = { gameController.skipLevel() },
            )
        }

        if (showLevelPicker) {
            LevelPickerOverlay(
                levels = gameController.getCurrentLevelSummaries(),
                selectedPuzzleId = screenState.puzzleId,
                onPickLevel = { puzzleId -> gameController.selectLevelByPuzzleId(puzzleId) },
                onToggleLike = { puzzleId ->
                    gameController.toggleLikeByPuzzleId(puzzleId)
                    pickerRefreshNonce++
                },
                onToggleStar = { puzzleId ->
                    gameController.toggleStarByPuzzleId(puzzleId)
                    pickerRefreshNonce++
                },
                onToggleDislike = { puzzleId ->
                    gameController.toggleDislikeByPuzzleId(puzzleId)
                    pickerRefreshNonce++
                },
                refreshNonce = pickerRefreshNonce,
                onDismiss = { showLevelPicker = false },
            )
        }
        if (showLevelSetPicker) {
            LevelSetPickerOverlay(
                catalog = levelCatalog,
                selectedSetId = screenState.setId,
                onPickSet = { setId -> gameController.selectSetById(setId) },
                onRefresh = {
                    try {
                        withContext(Dispatchers.IO) {
                            gameController.syncWithServer()
                        }
                        true
                    } catch (_: Throwable) {
                        false
                    }
                },
                onDismiss = { showLevelSetPicker = false },
            )
        }
    }
}
