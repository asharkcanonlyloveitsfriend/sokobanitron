package com.example.einkarcade.sokoban

import kotlin.math.abs

class GameEngine(private val level: Level) {
    private var gameState = GameState.fromLevel(level)
    private val boxMoveHistory: MutableList<List<Position>> = mutableListOf()
    private var hasUndoneOnce: Boolean = false

    val playerPosition: Position
        get() = gameState.playerPosition

    val boxPositions: Set<Position>
        get() = gameState.boxPositions

    val isGameWon: Boolean
        get() = gameState.boxPositions.all { level.tileMap.isGoal(it) }

    val isCleanWin: Boolean
        get() = isGameWon && gameState.boxPositions.size == level.boxPositions.size

    val isAtStart: Boolean
        get() = gameState.playerPosition == level.playerStart &&
            gameState.boxPositions == level.boxPositions

    fun getBoxMoveHistory(): List<List<Position>> {
        return boxMoveHistory.toList()
    }

    fun undo(): List<Position>? {
        if (hasUndoneOnce) return null
        val path = boxMoveHistory.removeLastOrNull() ?: return null

        val boxFrom = path.first()
        val boxTo = path.last()
        val firstStep = Position(
            row = path[1].row - boxFrom.row,
            col = path[1].col - boxFrom.col
        )
        val newPlayerPosition = Position(
            row = boxFrom.row - firstStep.row,
            col = boxFrom.col - firstStep.col
        )

        gameState.removeBox(boxTo)
        gameState.addBox(boxFrom)
        gameState.movePlayer(newPlayerPosition)
        hasUndoneOnce = true
        return path
    }

    fun moveBoxTo(from: Position, to: Position): List<Position>? {
        if (isGameWon) return null
        if (!gameState.hasBoxAt(from)) return null

        val dirRow = from.row - playerPosition.row
        val dirCol = from.col - playerPosition.col
        val isAdjacentPush = abs(dirRow) + abs(dirCol) == 1
        val pushedTo = Position(from.row + dirRow, from.col + dirCol)
        val pushedIntoVoid = isAdjacentPush &&
            pushedTo == to &&
            level.tileMap.isVoid(to)

        if (pushedIntoVoid) {
            boxMoveHistory.add(listOf(from, to))
            hasUndoneOnce = false
            gameState.removeBox(from)
            gameState.movePlayer(from)
            return listOf(from, to)
        }

        // Plan a multi-push move using BoxMover. The walkable grid treats boxes as obstacles,
        // so mark the starting box square walkable for the planning step.
        val gridCopy = walkableGrid.map { it.copyOf() }.toTypedArray()
        gridCopy[from.row][from.col] = true

        val boxMover = BoxMover(gridCopy)
        val boxPath = boxMover.findBoxPath(from, to, playerPosition) ?: return null
        val finalPlayerPosition = if (boxPath.size >= 2) {
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

    fun movePlayerTo(position: Position): Boolean {
        if (isGameWon) return false

        val pathfinder = Pathfinder(walkableGrid)
        if (!pathfinder.canFindPath(playerPosition, position)) return false
        if (position == playerPosition) return false

        gameState.movePlayer(position)
        return true
    }

    private val walkableGrid: Array<Array<Boolean>>
        get() = Array(level.tileMap.rowCount) { row ->
            Array(level.tileMap.columnCount) { col ->
                val pos = Position(row, col)
                level.tileMap.isWalkable(pos) && !gameState.hasBoxAt(pos)
            }
        }
}
