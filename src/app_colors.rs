//! Shared Ventura-inspired colors. These fixed sRGB tokens do not query native macOS.
use crate::desktop_style::Palette;
use eframe::egui::Color32;

pub const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
pub const LIGHT: Palette = Palette {
    background: rgb(0xECECEC),
    surface: rgb(0xFFFFFF),
    control: rgb(0xFFFFFF),
    text: rgb(0x1D1D1F),
    muted: rgb(0x6E6E73),
    line: rgb(0xD2D2D7),
    accent: rgb(0x007AFF),
    on_accent: Color32::WHITE,
};
pub const DARK: Palette = Palette {
    background: rgb(0x282828),
    surface: rgb(0x1E1E1E),
    control: rgb(0x3A3A3A),
    text: rgb(0xF5F5F7),
    muted: rgb(0x98989D),
    line: rgb(0x48484A),
    accent: rgb(0x0A84FF),
    on_accent: Color32::WHITE,
};
// Punctuation, key, string, number, boolean, collection, null: seven roles.
pub const JSON_LIGHT: [Color32; 7] = [
    LIGHT.text,
    rgb(0x005CC5),
    rgb(0x237A36),
    rgb(0x9C6500),
    rgb(0x8944AB),
    rgb(0xC0392B),
    LIGHT.muted,
];
pub const JSON_DARK: [Color32; 7] = [
    DARK.text,
    rgb(0x64B5FF),
    rgb(0x7DDC91),
    rgb(0xF5C26B),
    rgb(0xCE9BFA),
    rgb(0xFF9F8A),
    DARK.muted,
];
// Information, success, warning, error.
pub const NOTICE_LIGHT: [Color32; 4] = [LIGHT.accent, rgb(0x34C759), rgb(0xFF9500), rgb(0xFF3B30)];
pub const NOTICE_DARK: [Color32; 4] = [DARK.accent, rgb(0x30D158), rgb(0xFF9F0A), rgb(0xFF453A)];
pub const FIND_ACTIVE: Color32 = rgb(0xFF9500);
pub const FIND_OTHER: Color32 = rgb(0xFFE29A);
pub const FIND_TEXT: Color32 = Color32::BLACK;
pub const PREVIEW_BACKGROUND: Color32 = rgb(0xD2D2D7);
pub const SWITCH_OFF_LIGHT: Color32 = rgb(0xD2D2D7);
pub const SWITCH_OFF_DARK: Color32 = rgb(0x5B5B60);
pub const SWITCH_THUMB: Color32 = Color32::WHITE;
pub const SWITCH_SHADOW: Color32 = Color32::from_black_alpha(35);

// Neutral dropdown hover fills stay visible against Settings surfaces.
pub const DROPDOWN_HOVER_LIGHT: Color32 = rgb(0xE7E7E7);
pub const DROPDOWN_HOVER_DARK: Color32 = rgb(0x454545);

// macOS-style popup surfaces are opaque and distinct from the Settings pane.
pub const DROPDOWN_MENU_LIGHT: Color32 = rgb(0xF6F6F6);
pub const DROPDOWN_MENU_DARK: Color32 = rgb(0x303030);

// Settings group surfaces and per-category icon tile/ink pairs.
pub const SETTINGS_GROUP_LIGHT: Color32 = rgb(0xF8F8F8);
pub const SETTINGS_GROUP_DARK: Color32 = rgb(0x303030);
pub const SETTINGS_MENU_ICONS: [(Color32, Color32); 3] = [
    (rgb(0x007AFF), Color32::WHITE),
    (rgb(0x34C759), rgb(0xF0FFF4)),
    (rgb(0x8E8E93), rgb(0xE6F0FF)),
];

pub const BUTTON_ON_WHITE_LIGHT: Color32 = rgb(0xF0F0F0);
pub const BUTTON_ON_GRAY_LIGHT: Color32 = rgb(0xE7E7E7);
pub const BUTTON_ON_WHITE_DARK: Color32 = rgb(0x3A3A3A);
pub const BUTTON_ON_GRAY_DARK: Color32 = rgb(0x505050);
