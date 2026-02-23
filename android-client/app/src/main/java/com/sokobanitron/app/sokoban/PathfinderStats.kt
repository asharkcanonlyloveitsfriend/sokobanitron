package com.sokobanitron.app.sokoban

data class PathfinderStats(
    var nodesExpanded: Long = 0,
    var nodesPushed: Long = 0,
)
