# TODO

## Near Term

- Kindle: when the custom app sleep screen is disabled or the app hands control back, make sure the system screensaver module is restored without requiring a full device reboot; restarting `lab126_gui` may be an acceptable fallback.
- Refactor the level editor tile model so it is closer to gameplay: keep base tiles and box occupancy aligned with gameplay semantics instead of treating boxes as part of `EditableTile`, while still preserving convenient authoring flows like drawing a box on a goal.

## Launch

- Add preference migration/versioning strategy for persisted JSON preferences. For now, development builds may drop old keys, but before launch we should define how renamed, removed, and client-specific preferences are migrated.
- Add client-specific preference schema support so persisted files only include preferences relevant to the current client. For example, desktop should not write Kindle-only sleep-screen preferences into its JSON file.
