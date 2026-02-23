package com.sokobanitron.app.sokoban

import kotlin.math.abs

class GameEngine(
    private val level: Level,
) {
    private var gameState = GameState.fromLevel(level)
    private val boxMoveHistory: MutableList<List<Position>> = mutableListOf()
    private var hasUndoneOnce: Boolean = false

    val playerPosition: Position
        get() = gameState.playerPosition

    val boxPositions: Set<Position>
        get() = gameState.boxPositions

    val isLevelSolved: Boolean
        get() = gameState.boxPositions.all { level.tileMap.isGoal(it) }

    val isCleanSolution: Boolean
        get() = isLevelSolved && gameState.boxPositions.size == level.boxPositions.size

    val isAtStart: Boolean
        get() =
            gameState.playerPosition == level.playerStart &&
                gameState.boxPositions == level.boxPositions

    fun getBoxMoveHistory(): List<List<Position>> = boxMoveHistory.toList()

    fun undo(): List<Position>? {
        if (hasUndoneOnce) return null
        val path = boxMoveHistory.removeLastOrNull() ?: return null

        val boxFrom = path.first()
        val boxTo = path.last()
        val firstStep =
            Position(
                row = path[1].row - boxFrom.row,
                col = path[1].col - boxFrom.col,
            )
        val newPlayerPosition =
            Position(
                row = boxFrom.row - firstStep.row,
                col = boxFrom.col - firstStep.col,
            )

        gameState.removeBox(boxTo)
        gameState.addBox(boxFrom)
        gameState.movePlayer(newPlayerPosition)
        hasUndoneOnce = true
        return path
    }

    fun moveBoxTo(
        from: Position,
        to: Position,
    ): List<Position>? {
        if (isLevelSolved) return null
        if (!gameState.hasBoxAt(from)) return null

        val boxPathfinder =
            BoxPathfinder(
                fullGrid = walkableGrid,
                boxStart = from,
                playerStart = playerPosition,
            )

        val boxPath = boxPathfinder.findBoxPath(to) ?: return null
        val finalPlayerPosition =
            if (boxPath.size >= 2) {
                boxPath[boxPath.size - 2]
            } else {
                boxPath.last()
            }

        // Apply the planned move.
        boxMoveHistory.add(boxPath)
        hasUndoneOnce = false
        gameState.moveBox(from, to)
        gameState.movePlayer(finalPlayerPosition)
        return boxPath
    }

    fun pushBoxIntoVoid(
        from: Position,
        to: Position,
    ): Boolean {
        if (isLevelSolved) return false
        if (!gameState.hasBoxAt(from)) return false

        val dirRow = from.row - playerPosition.row
        val dirCol = from.col - playerPosition.col
        val isAdjacentPush = abs(dirRow) + abs(dirCol) == 1
        val pushedTo = Position(from.row + dirRow, from.col + dirCol)

        if (!isAdjacentPush) return false
        if (pushedTo != to) return false
        if (!level.tileMap.isVoid(to)) return false

        boxMoveHistory.add(listOf(from, to))
        hasUndoneOnce = false
        gameState.removeBox(from)
        gameState.movePlayer(from)
        return true
    }

    fun movePlayerTo(position: Position): Boolean {
        if (isLevelSolved) return false

        val pathfinder = Pathfinder(walkableGrid)
        if (!pathfinder.canFindPath(playerPosition, position)) return false
        if (position == playerPosition) return false

        gameState.movePlayer(position)
        return true
    }

    private val walkableGrid: Array<Array<Boolean>>
        get() =
            WalkableGrid.withObstacles(
                baseGrid = level.tileMap.walkableGrid(),
                obstacles = gameState.boxPositions,
            )
}
