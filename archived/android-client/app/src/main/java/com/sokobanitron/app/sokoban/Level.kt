package com.sokobanitron.app.sokoban

data class Level(
    val name: String,
    val ascii: String,
    val grid: List<List<Tile>>,
    val playerStart: Position,
    val boxPositions: Set<Position>,
    val puzzleId: Int = -1,
) {
    // -1 = thumbs down, 0 = none, 1 = thumbs up. Not part of equality/hashCode.
    var rating: Int = 0
        private set
    var isStarred: Boolean = false
        private set
    var completedAt: String? = null
        private set

    val isCompleted: Boolean
        get() = completedAt != null

    val tileMap: TileMap
        get() = TileMap(grid)

    fun setRating(value: Int) {
        rating = value
    }

    fun setStarred(value: Boolean) {
        isStarred = value
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
        fun fromAscii(
            name: String,
            ascii: String,
            puzzleId: Int = -1,
        ): Level {
            val parsed = parseAscii(ascii)
            return Level(
                name,
                ascii,
                parsed.initialGrid,
                parsed.playerStart,
                parsed.boxes,
                puzzleId,
            )
        }

        private data class ParsedAscii(
            val initialGrid: List<List<Tile>>,
            val playerStart: Position,
            val boxes: Set<Position>,
        )

        /** Parses Sokoban ASCII into a base grid (VOID/FLOOR/GOAL) and extracts player + boxes. */
        private fun parseAscii(ascii: String): ParsedAscii {
            val lines = ascii.lines()
            val maxWidth = lines.maxOfOrNull { it.length } ?: 0

            var playerStart: Position? = null
            val boxes = mutableSetOf<Position>()

            val grid =
                lines.mapIndexed { rowIndex, line ->
                    line.padEnd(maxWidth).mapIndexed { colIndex, ch ->
                        val position = Position(rowIndex, colIndex)
                        when (ch) {
                            '#' -> {
                                Tile.VOID
                            }

                            ' ' -> {
                                Tile.FLOOR
                            }

                            '.' -> {
                                Tile.GOAL
                            }

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

                            else -> {
                                Tile.FLOOR
                            }
                        }
                    }
                }

            val start = requireNotNull(playerStart) { "Player start '@' not found in level" }
            return ParsedAscii(grid, start, boxes)
        }
    }
}
