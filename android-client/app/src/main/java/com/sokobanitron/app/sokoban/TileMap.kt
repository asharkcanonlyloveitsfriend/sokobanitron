package com.sokobanitron.app.sokoban

class TileMap(
    private val tiles: List<List<Tile>>,
) {
    fun tileAt(
        row: Int,
        col: Int,
    ): Tile? = tiles.getOrNull(row)?.getOrNull(col)

    fun isVoid(
        row: Int,
        col: Int,
    ): Boolean = tileAt(row, col) == Tile.VOID

    fun isVoid(position: Position): Boolean {
        val (row, col) = position
        return isVoid(row, col)
    }

    val rowCount: Int
        get() = tiles.size

    val columnCount: Int
        get() = tiles.firstOrNull()?.size ?: 0
}
