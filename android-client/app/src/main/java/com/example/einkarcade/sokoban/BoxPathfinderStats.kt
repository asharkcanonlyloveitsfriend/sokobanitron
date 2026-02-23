package com.example.einkarcade.sokoban

data class BoxPathfinderStats(
    var statesPushed: Long = 0,
    var statesExpanded: Long = 0,
    var pushAttempts: Long = 0,
    var playerPathfinderCalls: Long = 0,
    var playerPathfinderSuccesses: Long = 0,
)
