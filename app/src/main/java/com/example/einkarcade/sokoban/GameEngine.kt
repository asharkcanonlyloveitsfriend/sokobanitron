package com.example.einkarcade.sokoban

class GameEngine(private val level: Level) {
    private var gameState = GameState.fromLevel(level)
    private var lastSavedState: GameState? = null

    val playerPosition: Position
        get() = gameState.playerPosition

    val boxPositions: Set<Position>
        get() = gameState.boxPositions

    val isGameWon: Boolean
        get() = gameState.boxPositions.all { level.isGoal(it) }

    private fun hasBoxAt(position: Position): Boolean {
        return gameState.boxPositions.contains(position)
    }

    fun undo(): Boolean {
        val savedState = lastSavedState ?: return false
        gameState = savedState.deepCopy()
        lastSavedState = null
        return true
    }

    fun moveBoxTo(from: Position, to: Position): Boolean {
        if (isGameWon) return false
        if (!hasBoxAt(from)) return false

        // Plan a multi-push move using BoxMover. The walkable grid treats boxes as obstacles,
        // so mark the starting box square walkable for the planning step.
        val gridCopy = walkableGrid.map { it.copyOf() }.toTypedArray()
        gridCopy[from.row][from.col] = true

        val boxMover = BoxMover(gridCopy)
        val finalPlayerPosition = boxMover.canMoveBox(from, to, playerPosition) ?: return false

        // Apply the planned move.
        lastSavedState = gameState.deepCopy()
        gameState.moveBox(from, to)
        gameState.movePlayer(finalPlayerPosition)
        return true
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
        get() = Array(level.grid.size) { row ->
            Array(level.grid[0].size) { col ->
                val pos = Position(row, col)
                level.grid[row][col] != Tile.WALL && !gameState.boxPositions.contains(pos)
            }
        }
}
