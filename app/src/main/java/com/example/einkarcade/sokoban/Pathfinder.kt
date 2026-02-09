package com.example.einkarcade.sokoban

class Pathfinder(
    private val walkableGrid: Array<Array<Boolean>>,
    private val stats: PathfinderStats? = null,
) {
    private val numRows = walkableGrid.size
    private val numCols = walkableGrid[0].size
    private val visitedStamp = IntArray(numRows * numCols)
    private var currentStamp = 1

    fun canFindPath(
        from: Position,
        to: Position,
    ): Boolean {
        if (from == to) return true
        val stamp = currentStamp++

        val queue: ArrayDeque<Int> = ArrayDeque()

        queue.add(from.row * numCols + from.col)
        stats?.nodesPushed = stats?.nodesPushed?.plus(1) ?: 0

        val targetIndex = to.row * numCols + to.col

        while (queue.isNotEmpty()) {
            val current = queue.removeFirst()
            stats?.nodesExpanded = stats?.nodesExpanded?.plus(1) ?: 0

            if (current == targetIndex) return true

            val row = current / numCols
            val col = current % numCols

            val idx = row * numCols + col
            if (visitedStamp[idx] == stamp) continue
            visitedStamp[idx] = stamp

            // up
            val up = row - 1
            if (up >= 0 && walkableGrid[up][col] && visitedStamp[up * numCols + col] != stamp) {
                queue.add(up * numCols + col)
                stats?.nodesPushed = stats?.nodesPushed?.plus(1) ?: 0
            }

            // down
            val down = row + 1
            if (down < numRows && walkableGrid[down][col] && visitedStamp[down * numCols + col] != stamp) {
                queue.add(down * numCols + col)
                stats?.nodesPushed = stats?.nodesPushed?.plus(1) ?: 0
            }

            // left
            val left = col - 1
            if (left >= 0 && walkableGrid[row][left] && visitedStamp[row * numCols + left] != stamp) {
                queue.add(row * numCols + left)
                stats?.nodesPushed = stats?.nodesPushed?.plus(1) ?: 0
            }

            // right
            val right = col + 1
            if (right < numCols && walkableGrid[row][right] && visitedStamp[row * numCols + right] != stamp) {
                queue.add(row * numCols + right)
                stats?.nodesPushed = stats?.nodesPushed?.plus(1) ?: 0
            }
        }

        return false
    }
}
