package com.example.einkarcade.ui.rendering.anim

class QueuedAnimator(
    private val tickDelayMs: Long,
    private val postTick: (Runnable, Long) -> Unit,
) {

    private val queue: ArrayDeque<Animation> = ArrayDeque()
    private var ticking = false

    private val tickRunnable = object : Runnable {
        override fun run() {
            if (queue.isEmpty()) {
                ticking = false
                return
            }

            val current = queue.first()
            val keepGoing = current.tick()

            if (!keepGoing) {
                queue.removeFirst()
            }

            if (queue.isNotEmpty()) {
                postTick(this, tickDelayMs)
            } else {
                ticking = false
            }
        }
    }

    fun enqueue(animation: Animation) {
        queue.addLast(animation)

        if (!ticking) {
            ticking = true
            postTick(tickRunnable, 0L)
        }
    }

}