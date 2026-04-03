# Android Performance Notes

Only the non-obvious findings from Android bringup are captured here.

- The largest real win was moving Android presentation off the `IntArray -> Bitmap -> Canvas` path and presenting through `ANativeWindow`.
- `release` Rust mattered enough that early feel comparisons against the old client were misleading before switching to it.
- Caching the shared board-scene base is a good general optimization. It improves redraw cost without pushing client-specific policy into the renderer.
- We intentionally removed Android-only gameplay dirty-cell redraw. It was fast, but it put repaint rules in the Android runtime, which would get in the way of shared animation work later.

If partial gameplay redraw returns later, it should live in shared presentation code rather than in an individual client runtime.
