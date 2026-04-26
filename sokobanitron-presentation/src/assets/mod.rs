//! Asset-backed primitives used by the presentation system.

pub(crate) mod icons;
pub(crate) mod sprites;

pub(crate) use icons::{UI_ICON_SCALE, UI_ICON_SIZE, draw_ui_icon_scaled_in_rect};
pub use icons::{UiIcon, draw_ui_icon_in_rect};
pub(crate) use sprites::rasterize_svg;
