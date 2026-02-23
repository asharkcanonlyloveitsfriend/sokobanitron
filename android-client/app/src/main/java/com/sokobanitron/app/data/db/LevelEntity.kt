package com.sokobanitron.app.data.db

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "levels",
    foreignKeys = [
        ForeignKey(
            entity = LevelSetEntity::class,
            parentColumns = ["id"],
            childColumns = ["level_set_id"],
            onDelete = ForeignKey.CASCADE,
        ),
        ForeignKey(
            entity = PuzzleEntity::class,
            parentColumns = ["id"],
            childColumns = ["puzzle_id"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
    indices = [
        Index("level_set_id"),
        Index("puzzle_id"),
    ],
)
data class LevelEntity(
    @PrimaryKey val id: Int,
    val title: String,
    @ColumnInfo(name = "level_set_id") val levelSetId: Int,
    @ColumnInfo(name = "puzzle_id") val puzzleId: Int,
)
