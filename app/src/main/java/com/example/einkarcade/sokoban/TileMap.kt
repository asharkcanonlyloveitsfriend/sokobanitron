package com.example.einkarcade.sokoban

class TileMap(
    private val tiles: List<List<Tile>>
) {
    fun tileAt(position: Position): Tile? {
        val (row, col) = position
        return tileAt(row, col)
    }

    fun tileAt(row: Int, col: Int): Tile? {
        return tiles.getOrNull(row)?.getOrNull(col)
    }

    fun isVoid(row: Int, col: Int): Boolean {
        return tileAt(row, col) == Tile.VOID
    }

    fun isVoid(position: Position): Boolean {
        val (row, col) = position
        return isVoid(row, col)
    }

    fun isGoal(position: Position): Boolean {
        return tileAt(position) == Tile.GOAL
    }

    fun isWalkable(position: Position): Boolean {
        return !isVoid(position)
    }

    val rowCount: Int
        get() = tiles.size

    val columnCount: Int
        get() = tiles.firstOrNull()?.size ?: 0
}