package com.example.einkarcade.content

import com.example.einkarcade.sokoban.Level

data class LevelSet(
    val id: Int,
    val name: String,
    val levels: List<Level>,
)
