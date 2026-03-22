use crate::{ScreenRect, draw_icon_bits_in_rect};

pub const UI_ICON_SIZE: usize = 9;
pub const UI_ICON_SCALE: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiIcon {
    Draw,
    Manipulate,
    Undo,
    Restart,
    MenuFirst,
    MenuPageUp,
    MenuCurrent,
    MenuPageDown,
    MenuLast,
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

const ICON_MANIPULATE_CURSOR: IconBits = [
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

const ICON_MENU_PAGE_UP: IconBits = [
    0b000100000,
    0b001000000,
    0b010000000,
    0b100000000,
    0b010000000,
    0b001000000,
    0b000100000,
    0b000000000,
    0b000000000,
];

const ICON_MENU_PAGE_DOWN: IconBits = [
    0b000100000,
    0b000010000,
    0b000001000,
    0b000000100,
    0b000001000,
    0b000010000,
    0b000100000,
    0b000000000,
    0b000000000,
];

const ICON_MENU_FIRST: IconBits = [
    0b100100000,
    0b101000000,
    0b110000000,
    0b100000000,
    0b110000000,
    0b101000000,
    0b100100000,
    0b100000000,
    0b100000000,
];

const ICON_MENU_LAST: IconBits = [
    0b000100001,
    0b000010001,
    0b000001001,
    0b000000101,
    0b000001001,
    0b000010001,
    0b000100001,
    0b000000001,
    0b000000001,
];

const ICON_MENU_CURRENT: IconBits = [
    0b000100000,
    0b000010000,
    0b000001000,
    0b000000100,
    0b000001000,
    0b000010000,
    0b000100000,
    0b000000000,
    0b000000000,
];

pub fn draw_ui_icon_in_rect(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    icon: UiIcon,
    color: [u8; 4],
) {
    let bits = match icon {
        UiIcon::Draw => ICON_DRAW_PENCIL,
        UiIcon::Manipulate => ICON_MANIPULATE_CURSOR,
        UiIcon::Undo => ICON_UNDO,
        UiIcon::Restart => ICON_RESTART,
        UiIcon::MenuFirst => ICON_MENU_FIRST,
        UiIcon::MenuPageUp => ICON_MENU_PAGE_UP,
        UiIcon::MenuCurrent => ICON_MENU_CURRENT,
        UiIcon::MenuPageDown => ICON_MENU_PAGE_DOWN,
        UiIcon::MenuLast => ICON_MENU_LAST,
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
