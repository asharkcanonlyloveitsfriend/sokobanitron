# TODO

## Launch

- Add preference migration/versioning strategy for persisted JSON preferences. For now, development builds may drop old keys, but before launch we should define how renamed, removed, and client-specific preferences are migrated.
- Add client-specific preference schema support so persisted files only include preferences relevant to the current client. For example, desktop should not write Kindle-only sleep-screen preferences into its JSON file.
