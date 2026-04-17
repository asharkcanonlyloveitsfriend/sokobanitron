# Renderer Color Inventory

This document records the current shared grayscale palette model for the renderer.

The renderer now uses a fixed 16-color palette:

- `white`
- `gray_1` through `gray_14`
- `black`

The intent is ordinal, not semantic:

- `white` and `black` are fixed endpoints
- `gray_1` is the lightest non-white gray
- `gray_14` is the darkest non-black gray
- every renderer color must come from one of these 16 slots
- Android may override the 14 interior grays, but the relative ordering must stay the same

## Shared Palette Definition

Primary palette definitions live in:

- `sokobanitron-presentation/src/renderer/mod.rs`
  - `RendererTheme`
  - `RendererOverrides`
  - `RendererTheme::default()`

Current default palette, used by desktop and Kindle:

- `white = 255`
- `gray_1 = 238`
- `gray_2 = 221`
- `gray_3 = 204`
- `gray_4 = 187`
- `gray_5 = 170`
- `gray_6 = 153`
- `gray_7 = 136`
- `gray_8 = 119`
- `gray_9 = 102`
- `gray_10 = 85`
- `gray_11 = 68`
- `gray_12 = 51`
- `gray_13 = 34`
- `gray_14 = 17`
- `black = 0`

## Android Overrides

Android is the only client that overrides the shared palette.

Override definitions live in:

- `sokobanitron-android-jni/src/runtime.rs`
  - `android_renderer_overrides()`

Current Android override values:

- `gray_1 = 214`
- `gray_2 = 202`
- `gray_3 = 189`
- `gray_4 = 177`
- `gray_5 = 164`
- `gray_6 = 152`
- `gray_7 = 140`
- `gray_8 = 127`
- `gray_9 = 115`
- `gray_10 = 102`
- `gray_11 = 90`
- `gray_12 = 78`
- `gray_13 = 65`
- `gray_14 = 53`

`white` remains `255` and `black` remains `0` on Android.

## Client Behavior

- desktop uses the default shared palette
- Kindle uses the default shared palette
- Android uses the default shared palette plus the 14 interior overrides above

## Renderer Usage Notes

The renderer no longer uses `light_*`, `mid_*`, or `dark_*` palette names.

Representative current palette usage:

- floor tile stroke: `gray_1`
- goal tile fill: `gray_2`
- box/path outline and selection brackets: `gray_3`
- player body: `gray_5`
- selected box highlight: `gray_7`
- standard box body: `gray_8`
- player limb and selected box body: `gray_9`
- standard shadow and dark flash: `gray_10`
- selected box shadow: `gray_11`
- UI text: `gray_2`
- editor hint text: `gray_5`
- scrollbar track: `gray_4`
- scrollbar jump marker: `gray_2`
- scrollbar thumb: `gray_1`
- scrollbar current indicator: `white`

Not every palette slot is currently used. That is acceptable; the palette is meant to stay
complete and stable even if some entries are temporarily unused.
