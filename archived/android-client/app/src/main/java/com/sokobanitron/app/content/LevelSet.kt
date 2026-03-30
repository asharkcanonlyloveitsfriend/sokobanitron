package com.sokobanitron.app.content

import com.sokobanitron.app.sokoban.Level

data class LevelSet(
    val id: Int,
    val name: String,
    val levels: List<Level>,
)
