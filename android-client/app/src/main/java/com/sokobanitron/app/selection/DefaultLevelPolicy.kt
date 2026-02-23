package com.sokobanitron.app.selection

import com.sokobanitron.app.sokoban.Level

object DefaultLevelPolicy {
    fun pickIndex(levels: List<Level>): Int {
        val firstIncompleteIndex = levels.indexOfFirst { !it.isCompleted }
        return if (firstIncompleteIndex != -1) firstIncompleteIndex else 0
    }
}
