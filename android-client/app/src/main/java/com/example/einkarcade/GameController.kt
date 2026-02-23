package com.example.einkarcade

import android.content.Context
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import com.example.einkarcade.appstate.LastSelectionStore
import com.example.einkarcade.catalog.LevelBoardGeometry
import com.example.einkarcade.catalog.LevelBoardPoint
import com.example.einkarcade.catalog.LevelBoardTile
import com.example.einkarcade.catalog.LevelCatalog
import com.example.einkarcade.catalog.LevelSummary
import com.example.einkarcade.catalog.RepositoryLevelCatalog
import com.example.einkarcade.content.LevelSet
import com.example.einkarcade.data.LevelsRepository
import com.example.einkarcade.selection.DefaultLevelPolicy
import com.example.einkarcade.sokoban.GameEngine
import com.example.einkarcade.sokoban.Level
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.sokoban.TileMap
import com.example.einkarcade.ui.rendering.StaticBoardFrame
import com.example.einkarcade.ui.rendering.draw.StaticBoardRenderer
import com.example.einkarcade.ui.rendering.draw.TileDrawer
import com.example.einkarcade.ui.rendering.geom.computeBoardViewport

class GameController(
    context: Context,
    injectedSets: List<LevelSet>? = null,
    private val lastSelectionStore: LastSelectionStore = LastSelectionStore(context),
    private val levelCatalog: LevelCatalog = RepositoryLevelCatalog(context, injectedSets),
) {
    private val repository = LevelsRepository(context)

    data class GameScreenState(
        val setName: String,
        val setId: Int,
        val levelName: String,
        val puzzleId: Int,
        val rating: Int,
        val isStarred: Boolean,
        val tileMap: TileMap,
    )

    data class LevelTransitionSnapshot(
        val oldTileMap: TileMap,
    )

    private val gameScreenState = mutableStateOf<GameScreenState?>(null)

    private var levelSets: List<LevelSet> = emptyList()
    private var currentSetIndex: Int = 0
    private var currentLevelIndex: Int = 0
    private lateinit var level: Level
    private lateinit var gameEngine: GameEngine

    enum class UiMode {
        GAMEPLAY,
        LEVEL_SOLVED,
        LEVEL_TRANSITION,
    }

    private val uiModeState = mutableLongStateOf(UiMode.GAMEPLAY.ordinal.toLong())

    val uiMode: UiMode
        get() = UiMode.entries[uiModeState.longValue.toInt()]

    private val transitionSnapshotState = mutableStateOf<LevelTransitionSnapshot?>(null)
    private val showRestartControlState = mutableStateOf(false)

    sealed interface RenderDelta {
        data class LevelLoaded(
            val staticFrame: StaticBoardFrame,
            val playerPosition: Position,
            val boxPositions: Set<Position>,
        ) : RenderDelta

        data class StateChanged(
            val playerPosition: Position,
            val boxPositions: Set<Position>,
            val annotation: StateChangeAnnotation? = null,
        ) : RenderDelta

        sealed interface StateChangeAnnotation {
            data object Undo : StateChangeAnnotation

            data object Restart : StateChangeAnnotation

            data object PlayerMoved : StateChangeAnnotation

            data class BoxRemoved(
                val position: Position,
            ) : StateChangeAnnotation

            data class BoxMoved(
                val path: List<Position>,
            ) : StateChangeAnnotation
        }

        data class LevelSolved(
            val isClean: Boolean,
        ) : RenderDelta

        data object MoveRejected : RenderDelta
    }

    val screenState: State<GameScreenState?>
        get() = gameScreenState

    var onRenderDelta: ((RenderDelta) -> Unit)? = null
        set(value) {
            field = value
        }

    val currentSetName: String
        get() = requireGameScreenState().setName

    val currentSetId: Int
        get() = requireGameScreenState().setId

    val playerPosition: Position
        get() = gameEngine.playerPosition

    val boxPositions: Set<Position>
        get() = gameEngine.boxPositions

    val tileMap: TileMap
        get() = requireGameScreenState().tileMap

    val transitionSnapshot: State<LevelTransitionSnapshot?>
        get() = transitionSnapshotState

    val showRestartControl: State<Boolean>
        get() = showRestartControlState

    val levelName: String
        get() = requireGameScreenState().levelName

    val currentPuzzleId: Int
        get() = requireGameScreenState().puzzleId

    init {
        val sets = injectedSets ?: (loadLevelSets() ?: emptyList())
        rebuildState(sets)
    }

    fun selectSetById(setId: Int) {
        val setIdx = levelSets.indexOfFirst { it.id == setId }
        if (setIdx == -1) return

        val levels = levelSets[setIdx].levels
        val levelIdx = DefaultLevelPolicy.pickIndex(levels)
        beginLevelTransition(nextSetIndex = setIdx, nextLevelIndex = levelIdx)
    }

    fun levels(): List<Level> = levelsInCurrentSet

    fun getCurrentLevelSummaries(): List<LevelSummary> =
        levelsInCurrentSet.map { level ->
            LevelSummary(
                puzzleId = level.puzzleId,
                name = level.name,
                isCompleted = level.isCompleted,
                rating = level.rating,
                isStarred = level.isStarred,
                boardGeometry = level.toBoardGeometry(),
            )
        }

    fun getCurrentRating(): Int = requireGameScreenState().rating

    fun toggleThumbUp() {
        toggleLikeByPuzzleId(currentPuzzleId)
    }

    fun toggleThumbDown() {
        toggleDislikeByPuzzleId(currentPuzzleId)
    }

    fun toggleStar() {
        toggleStarByPuzzleId(currentPuzzleId)
    }

    fun toggleLikeByPuzzleId(puzzleId: Int) {
        val target = levelsInCurrentSet.firstOrNull { it.puzzleId == puzzleId } ?: return
        val nextRating = if (target.rating == 1) 0 else 1
        levelCatalog.setRating(puzzleId, nextRating)
        target.setRating(nextRating)
        if (puzzleId == currentPuzzleId) {
            refreshRating(nextRating)
        }
    }

    fun toggleDislikeByPuzzleId(puzzleId: Int) {
        val target = levelsInCurrentSet.firstOrNull { it.puzzleId == puzzleId } ?: return
        val nextRating = if (target.rating == -1) 0 else -1
        levelCatalog.setRating(puzzleId, nextRating)
        target.setRating(nextRating)
        if (puzzleId == currentPuzzleId) {
            refreshRating(nextRating)
        }
    }

    fun toggleStarByPuzzleId(puzzleId: Int) {
        val target = levelsInCurrentSet.firstOrNull { it.puzzleId == puzzleId } ?: return
        val nextStarred = !target.isStarred
        levelCatalog.setStarred(puzzleId, nextStarred)
        target.setStarred(nextStarred)
        if (puzzleId == currentPuzzleId) {
            refreshStarred(nextStarred)
        }
    }

    fun syncWithServer() {
        repository.syncWithServer()
        val sets = loadLevelSets() ?: emptyList()
        rebuildState(sets)
    }

    fun selectLevelByPuzzleId(puzzleId: Int) {
        val index = levelsInCurrentSet.indexOfFirst { it.puzzleId == puzzleId }
        if (index == -1) return
        beginLevelTransition(nextSetIndex = currentSetIndex, nextLevelIndex = index)
    }

    fun restart() {
        gameEngine = GameEngine(level)
        refreshShowRestartControl()
        emitStateChanged(RenderDelta.StateChangeAnnotation.Restart)
        uiModeState.longValue = UiMode.GAMEPLAY.ordinal.toLong()
    }

    private fun beginLevelTransition(
        nextSetIndex: Int,
        nextLevelIndex: Int,
    ) {
        if (levelSets.isEmpty()) return
        val oldTileMap = requireGameScreenState().tileMap
        val oldPuzzleId = currentPuzzleId
        if (!applyLevelSelection(nextSetIndex = nextSetIndex, nextLevelIndex = nextLevelIndex)) return
        if (currentPuzzleId == oldPuzzleId) return
        startTransition(oldTileMap)
    }

    fun finishLevelTransition() {
        transitionSnapshotState.value = null
        uiModeState.longValue = UiMode.GAMEPLAY.ordinal.toLong()
    }

    fun nextLevel() {
        val levels = levelsInCurrentSet
        val nextIndex = (currentLevelIndex + 1) % levels.size
        beginLevelTransition(nextSetIndex = currentSetIndex, nextLevelIndex = nextIndex)
    }

    fun skipLevel() {
        val levels = levelsInCurrentSet
        if (levels.size < 2) return

        val skippedLevel = levels[currentLevelIndex]
        val reorderedLevels =
            buildList(levels.size) {
                levels.forEachIndexed { index, level ->
                    if (index != currentLevelIndex) {
                        add(level)
                    }
                }
                add(skippedLevel)
            }

        val mutableSets = levelSets.toMutableList()
        val currentSet = mutableSets[currentSetIndex]
        mutableSets[currentSetIndex] = currentSet.copy(levels = reorderedLevels)
        levelSets = mutableSets

        val nextLevelIndex =
            if (currentLevelIndex >= reorderedLevels.lastIndex) {
                0
            } else {
                currentLevelIndex
            }
        beginLevelTransition(nextSetIndex = currentSetIndex, nextLevelIndex = nextLevelIndex)
    }

    private fun startTransition(oldTileMap: TileMap) {
        transitionSnapshotState.value = LevelTransitionSnapshot(oldTileMap = oldTileMap)
        uiModeState.longValue = UiMode.LEVEL_TRANSITION.ordinal.toLong()
    }

    private fun applyLevelSelection(
        nextSetIndex: Int,
        nextLevelIndex: Int,
    ): Boolean {
        if (levelSets.isEmpty()) return false
        val resolvedSetIndex = nextSetIndex.coerceIn(0, levelSets.lastIndex)
        val levels = levelSets[resolvedSetIndex].levels
        if (levels.isEmpty()) return false
        val resolvedLevelIndex = nextLevelIndex.coerceIn(0, levels.lastIndex)

        currentSetIndex = resolvedSetIndex
        currentLevelIndex = resolvedLevelIndex
        level = levels[currentLevelIndex]
        gameEngine = GameEngine(level)
        refreshShowRestartControl()
        persistSelection()
        refreshGameScreenState()
        return true
    }

    fun undo(): Boolean {
        if (uiMode != UiMode.GAMEPLAY) return false
        if (gameEngine.undo() == null) return false
        refreshShowRestartControl()
        emitStateChanged(RenderDelta.StateChangeAnnotation.Undo)
        return true
    }

    fun movePlayerTo(position: Position) {
        val changed = gameEngine.movePlayerTo(position)
        if (changed) {
            refreshShowRestartControl()
            emitStateChanged(RenderDelta.StateChangeAnnotation.PlayerMoved)
        }
    }

    fun moveBoxTo(
        boxFrom: Position,
        boxTo: Position,
    ) {
        if (tileMap.isVoid(boxTo)) {
            val removed = gameEngine.pushBoxIntoVoid(boxFrom, boxTo)
            if (!removed) {
                onRenderDelta?.invoke(RenderDelta.MoveRejected)
                return
            }
            refreshShowRestartControl()
            emitStateChanged(
                RenderDelta.StateChangeAnnotation.BoxRemoved(boxTo),
            )
            recordCompletionIfSolved()
            updateUiModeIfSolved()
            return
        }
        val boxPath = gameEngine.moveBoxTo(boxFrom, boxTo)
        if (boxPath == null) {
            onRenderDelta?.invoke(RenderDelta.MoveRejected)
            return
        }
        refreshShowRestartControl()
        emitStateChanged(
            RenderDelta.StateChangeAnnotation.BoxMoved(boxPath),
        )
        recordCompletionIfSolved()
        updateUiModeIfSolved()
    }

    private val levelsInCurrentSet: List<Level>
        get() = levelSets[currentSetIndex].levels

    fun emitLevelLoaded(staticFrame: StaticBoardFrame) {
        onRenderDelta?.invoke(
            RenderDelta.LevelLoaded(
                staticFrame = staticFrame,
                playerPosition = playerPosition,
                boxPositions = boxPositions,
            ),
        )
    }

    private fun emitStateChanged(annotation: RenderDelta.StateChangeAnnotation? = null) {
        onRenderDelta?.invoke(
            RenderDelta.StateChanged(
                playerPosition = gameEngine.playerPosition,
                boxPositions = gameEngine.boxPositions,
                annotation = annotation,
            ),
        )
    }

    private fun loadLevelSets(): List<LevelSet>? = repository.loadSets()

    private fun rebuildState(sets: List<LevelSet>) {
        val nonEmpty = sets.filter { it.levels.isNotEmpty() }
        levelSets = nonEmpty
        currentSetIndex = 0
        currentLevelIndex = 0
        transitionSnapshotState.value = null
        if (levelSets.isEmpty()) {
            showRestartControlState.value = false
            return
        }
        restoreLastSelection()
        gameEngine = GameEngine(level)
        refreshShowRestartControl()
        persistSelection()
        refreshGameScreenState()
    }

    private fun refreshShowRestartControl() {
        showRestartControlState.value = !gameEngine.isAtStart
    }

    private fun restoreLastSelection() {
        level = levelsInCurrentSet[currentLevelIndex]
        val (savedSetId, savedPuzzleId) = lastSelectionStore.load()
        val setIdx = levelSets.indexOfFirst { it.id == savedSetId }
        if (setIdx != -1) {
            currentSetIndex = setIdx
            val levelIdx = levelsInCurrentSet.indexOfFirst { it.puzzleId == savedPuzzleId }
            if (levelIdx != -1) {
                currentLevelIndex = levelIdx
            }
        }
        level = levelsInCurrentSet[currentLevelIndex]
    }

    private fun persistSelection() {
        lastSelectionStore.save(levelSets[currentSetIndex].id, level.puzzleId)
    }

    private fun requireGameScreenState(): GameScreenState =
        requireNotNull(gameScreenState.value) { "Game screen state is not initialized" }

    private fun refreshGameScreenState() {
        gameScreenState.value =
            GameScreenState(
                setName = levelSets[currentSetIndex].name,
                setId = levelSets[currentSetIndex].id,
                levelName = level.name,
                puzzleId = level.puzzleId,
                rating = level.rating,
                isStarred = level.isStarred,
                tileMap = level.tileMap,
            )
    }

    private fun refreshRating(rating: Int) {
        val current = requireGameScreenState()
        gameScreenState.value = current.copy(rating = rating)
    }

    private fun refreshStarred(isStarred: Boolean) {
        val current = requireGameScreenState()
        gameScreenState.value = current.copy(isStarred = isStarred)
    }

    private fun recordCompletionIfSolved() {
        if (gameEngine.isCleanSolution) {
            val timestamp =
                repository.recordCompletion(
                    level,
                    gameEngine.getBoxMoveHistory(),
                )
            level.markCompleted(timestamp)
        }
    }

    private fun updateUiModeIfSolved() {
        if (gameEngine.isLevelSolved) {
            uiModeState.longValue = UiMode.LEVEL_SOLVED.ordinal.toLong()
            onRenderDelta?.invoke(RenderDelta.LevelSolved(isClean = gameEngine.isCleanSolution))
        }
    }

    internal fun buildStaticBoardFrame(
        context: Context,
        tileMap: TileMap,
        width: Int,
        height: Int,
    ): StaticBoardFrame {
        val renderer =
            StaticBoardRenderer(
                context = context,
                tileDrawer = TileDrawer(),
            )

        val viewport =
            computeBoardViewport(
                surfaceWidth = width.toFloat(),
                surfaceHeight = height.toFloat(),
                innerRows = tileMap.rowCount,
                innerCols = tileMap.columnCount,
            )

        renderer.rebuildStaticLayout(
            viewWidth = width,
            viewHeight = height,
            viewport = viewport,
            tileMap = tileMap,
        )

        val bitmap = renderer.getStaticFrameBitmap()

        return StaticBoardFrame(
            bitmap = bitmap,
            viewport = viewport,
            tileMap = tileMap,
            width = width,
            height = height,
        )
    }

    private fun Level.toBoardGeometry(): LevelBoardGeometry {
        val rowCount = grid.size
        val columnCount = grid.firstOrNull()?.size ?: 0
        val tiles =
            grid.flatMap { row ->
                row.map { tile ->
                    when (tile) {
                        Tile.FLOOR -> LevelBoardTile.FLOOR
                        Tile.GOAL -> LevelBoardTile.GOAL
                        Tile.VOID -> LevelBoardTile.VOID
                    }
                }
            }

        return LevelBoardGeometry(
            rowCount = rowCount,
            columnCount = columnCount,
            tiles = tiles,
            player = LevelBoardPoint(playerStart.row, playerStart.col),
            boxes = boxPositions.map { LevelBoardPoint(it.row, it.col) }.sortedWith(compareBy({ it.row }, { it.col })),
        )
    }
}
