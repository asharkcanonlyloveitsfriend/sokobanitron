package com.sokobanitron.app.sokoban

object WalkableGrid {
    fun withObstacles(
        baseGrid: Array<Array<Boolean>>,
        obstacles: Set<Position>,
    ): Array<Array<Boolean>> {
        val grid = baseGrid.map { it.copyOf() }.toTypedArray()
        for (pos in obstacles) {
            grid[pos.row][pos.col] = false
        }
        return grid
    }
}
