package com.example.einkarcade.data.db

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "puzzles")
data class PuzzleEntity(
    @PrimaryKey val id: Int,
    val grid: String,
    @ColumnInfo(name = "last_completed_at") val lastCompletedAt: String?,
    @ColumnInfo(defaultValue = "0") val rating: Int,
    @ColumnInfo(name = "is_locally_edited", defaultValue = "0") val isLocallyEdited: Boolean = false
)
