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
