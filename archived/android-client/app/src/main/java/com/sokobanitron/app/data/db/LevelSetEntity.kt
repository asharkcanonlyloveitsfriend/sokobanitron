package com.sokobanitron.app.data.db

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "level_sets")
data class LevelSetEntity(
    @PrimaryKey val id: Int,
    val title: String,
)
