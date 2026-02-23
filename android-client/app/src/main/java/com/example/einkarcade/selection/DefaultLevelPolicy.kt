package com.example.einkarcade.selection

import com.example.einkarcade.sokoban.Level

object DefaultLevelPolicy {
    fun pickIndex(levels: List<Level>): Int {
        val firstIncompleteIndex = levels.indexOfFirst { !it.isCompleted }
        return if (firstIncompleteIndex != -1) firstIncompleteIndex else 0
    }
}
