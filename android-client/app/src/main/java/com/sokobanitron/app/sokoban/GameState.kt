package com.sokobanitron.app.sokoban

data class GameState(
    var playerPosition: Position,
    val boxPositions: MutableSet<Position>,
) {
    companion object {
        fun fromLevel(level: Level): GameState =
            GameState(
                playerPosition = level.playerStart,
                boxPositions = level.boxPositions.toMutableSet(),
            )
    }

    fun movePlayer(to: Position) {
        playerPosition = to
    }

    fun moveBox(
        from: Position,
        to: Position,
    ) {
        if (!hasBoxAt(from)) {
            error("No box at position $from")
        }
        removeBox(from)
        addBox(to)
    }

    fun hasBoxAt(position: Position): Boolean = boxPositions.contains(position)

    fun addBox(position: Position) {
        boxPositions.add(position)
    }

    fun removeBox(position: Position) {
        boxPositions.remove(position)
    }
}
