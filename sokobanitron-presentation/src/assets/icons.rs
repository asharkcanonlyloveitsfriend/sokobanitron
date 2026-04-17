use crate::layout::ScreenRect;
use crate::renderer::{Gray, draw_icon_bits_in_rect};

pub const UI_ICON_SIZE: usize = 9;
pub const UI_ICON_SCALE: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiIcon {
    Draw,
    Select,
    Undo,
    Restart,
}

type IconBits = [u16; UI_ICON_SIZE];

const ICON_DRAW_PENCIL: IconBits = [
    0b110000000,
    0b111000000,
    0b011100000,
    0b001110000,
    0b000111000,
    0b000011100,
    0b000001111,
    0b000000101,
    0b000000111,
];

const ICON_SELECT_CURSOR: IconBits = [
    0b001100000,
    0b000110001,
    0b000011011,
    0b000111111,
    0b000011111,
    0b000001111,
    0b000000111,
    0b000000011,
    0b000000001,
];

const ICON_UNDO: IconBits = [
    0b001000000,
    0b011000000,
    0b111111100,
    0b011000110,
    0b001000011,
    0b000000011,
    0b000000110,
    0b000001100,
    0b000110000,
];

const ICON_RESTART: IconBits = [
    0b001111000,
    0b011111100,
    0b110000110,
    0b110000011,
    0b110000011,
    0b010100011,
    0b001100010,
    0b011100100,
    0b000001000,
];

pub fn draw_ui_icon_in_rect(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    icon: UiIcon,
    color: Gray,
) {
    let bits = match icon {
        UiIcon::Draw => ICON_DRAW_PENCIL,
        UiIcon::Select => ICON_SELECT_CURSOR,
        UiIcon::Undo => ICON_UNDO,
        UiIcon::Restart => ICON_RESTART,
    };
    draw_icon_bits_in_rect(
        frame,
        width,
        height,
        rect,
        &bits,
        UI_ICON_SIZE,
        UI_ICON_SCALE,
        color,
    );
}
