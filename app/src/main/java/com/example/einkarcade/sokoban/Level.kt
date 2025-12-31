package com.example.einkarcade.sokoban

data class Level(
    val name: String,
    val ascii: String,
    val grid: List<List<Tile>>,
    val playerStart: Position,
    val boxPositions: Set<Position>,
    val puzzleId: Int = -1
)
{
    // -1 = thumbs down, 0 = none, 1 = thumbs up. Not part of equality/hashCode.
    var rating: Int = 0
        private set
    var completedAt: String? = null
        private set

    val isCompleted: Boolean
        get() = completedAt != null


    fun setRating(value: Int) {
        rating = value
    }


    fun markCompleted(timestamp: String) {
        completedAt = timestamp
    }

    fun setCompletedAt(value: String?) {
        completedAt = value
    }

    fun toggleThumbUp(): Int {
        rating = if (rating == 1) 0 else 1
        return rating
    }

    fun toggleThumbDown(): Int {
        rating = if (rating == -1) 0 else -1
        return rating
    }


    companion object {
        fun fromAscii(name: String, ascii: String, puzzleId: Int = -1): Level {
            val lines = ascii.lines().dropLastWhile { it.isBlank() }
            val maxWidth = lines.maxOfOrNull { it.length } ?: 0
            var playerStart: Position? = null
            val boxes = mutableSetOf<Position>()

            // Initial parse: replace interior walkable spaces with FLOOR
            val initialGrid = lines.mapIndexed { rowIndex, line ->
                line.padEnd(maxWidth).mapIndexed { colIndex, char ->
                    val position = Position(rowIndex, colIndex)
                    when (char) {
                        '#' -> Tile.WALL
                        '.' -> Tile.GOAL
                        '$' -> {
                            boxes.add(position)
                            Tile.FLOOR
                        }
                        '*' -> {
                            boxes.add(position)
                            Tile.GOAL
                        }
                        '@' -> {
                            playerStart = position
                            Tile.FLOOR
                        }
                        '+' -> {
                            playerStart = position
                            Tile.GOAL
                        }
                        ' ' -> Tile.FLOOR
                        else -> Tile.FLOOR
                    }
                }
            }

            requireNotNull(playerStart) { "Player start '@' not found in level" }

            // Flood-fill to distinguish exterior EMPTY from interior FLOOR
            val numRows = initialGrid.size
            val numCols = if (numRows > 0) initialGrid[0].size else 0
            val visited = Array(numRows) { BooleanArray(numCols) { false } }
            val queue = ArrayDeque<Position>()

            // Enqueue all boundary cells that are not WALL
            for (col in 0 until numCols) {
                if (initialGrid[0][col] != Tile.WALL) {
                    queue.add(Position(0, col))
                    visited[0][col] = true
                }
                if (numRows > 1 && initialGrid[numRows - 1][col] != Tile.WALL) {
                    queue.add(Position(numRows - 1, col))
                    visited[numRows - 1][col] = true
                }
            }
            for (row in 1 until numRows - 1) {
                if (initialGrid[row][0] != Tile.WALL) {
                    queue.add(Position(row, 0))
                    visited[row][0] = true
                }
                if (numCols > 1 && initialGrid[row][numCols - 1] != Tile.WALL) {
                    queue.add(Position(row, numCols - 1))
                    visited[row][numCols - 1] = true
                }
            }

            // Directions for BFS
            val directions = listOf(
                Position(-1, 0),
                Position(1, 0),
                Position(0, -1),
                Position(0, 1)
            )

            while (queue.isNotEmpty()) {
                val pos = queue.removeFirst()
                for (dir in directions) {
                    val newRow = pos.row + dir.row
                    val newCol = pos.col + dir.col
                    if (newRow in 0 until numRows && newCol in 0 until numCols) {
                        if (!visited[newRow][newCol] && initialGrid[newRow][newCol] != Tile.WALL) {
                            visited[newRow][newCol] = true
                            queue.add(Position(newRow, newCol))
                        }
                    }
                }
            }

            // Build final grid: convert visited exterior cells to EMPTY, others remain the same
            val finalGrid = List(numRows) { row ->
                List(numCols) { col ->
                    if (visited[row][col]) {
                        Tile.EMPTY
                    } else {
                        initialGrid[row][col]
                    }
                }
            }

            return Level(name, ascii, finalGrid, playerStart, boxes, puzzleId)
        }
    }

    fun isGoal(position: Position): Boolean {
        return grid.getOrNull(position.row)?.getOrNull(position.col) == Tile.GOAL
    }
}
