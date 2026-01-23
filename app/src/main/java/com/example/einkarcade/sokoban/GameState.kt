package com.example.einkarcade.sokoban

data class GameState(
    var playerPosition: Position,
    val boxPositions: MutableSet<Position>
) {
    companion object {
        fun fromLevel(level: Level): GameState {
            return GameState(
                playerPosition = level.playerStart,
                boxPositions = level.boxPositions.toMutableSet()
            )
        }
    }

    fun moveBox(from: Position, to: Position) {
        if (!boxPositions.contains(from)) {
            error("No box at position $from")
        }
        boxPositions.remove(from)
        boxPositions.add(to)
    }

    fun removeBox(position: Position) {
        boxPositions.remove(position)
    }

    fun movePlayer(to: Position) {
        playerPosition = to
    }

    fun deepCopy(): GameState {
        return GameState(
            playerPosition = playerPosition,
            boxPositions = boxPositions.toMutableSet()
        )
    }
}
