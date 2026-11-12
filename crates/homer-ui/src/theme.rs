use chrono::{Datelike, Local, TimeZone, Utc};
use gpui::{rgb, Hsla, Rgba};

pub const BG: u32 = 0xf8f6f1;
pub const SIDEBAR: u32 = 0xefebe3;
pub const SURFACE: u32 = 0xfffdf9;
pub const SURFACE_2: u32 = 0xf3efe7;
pub const TEXT: u32 = 0x302c27;
pub const MUTED: u32 = 0x7c7469;
pub const FAINT: u32 = 0xa79e91;
pub const LINE: u32 = 0xe0d9cd;
pub const LINE_STRONG: u32 = 0xcec5b6;
pub const ACCENT: u32 = 0xa4563f;
pub const DANGER: u32 = 0x9c4436;
pub const USER_BUBBLE: u32 = 0xeae3d6;
pub const USER_BUBBLE_LINE: u32 = 0xe2d9ca;
pub const ERROR_BG: u32 = 0xf8ece7;
pub const ERROR_LINE: u32 = 0xe2c8bf;

pub fn bg() -> Rgba {
    rgb(BG)
}
pub fn sidebar() -> Rgba {
    rgb(SIDEBAR)
}
pub fn surface() -> Rgba {
    rgb(SURFACE)
}
pub fn surface_2() -> Rgba {
    rgb(SURFACE_2)
}
pub fn text() -> Rgba {
    rgb(TEXT)
}
pub fn muted() -> Rgba {
    rgb(MUTED)
}
pub fn faint() -> Rgba {
    rgb(FAINT)
}
pub fn line() -> Rgba {
    rgb(LINE)
}
pub fn line_strong() -> Rgba {
    rgb(LINE_STRONG)
}
pub fn accent() -> Rgba {
    rgb(ACCENT)
}
pub fn danger() -> Rgba {
    rgb(DANGER)
}

pub fn ui_font() -> &'static str {
    ".SystemUIFont"
}

pub fn serif_font() -> &'static str {
    "Hiragino Mincho ProN"
}

pub fn parse_hex(value: &str) -> Rgba {
    let raw = value.trim().trim_start_matches('#');
    if raw.len() == 6 {
        if let Ok(v) = u32::from_str_radix(raw, 16) {
            return rgb(v);
        }
    }
    accent()
}

pub fn format_time(ts_ms: i64) -> String {
    let Some(dt) = Utc.timestamp_millis_opt(ts_ms).single() else {
        return String::new();
    };
    let local = dt.with_timezone(&Local);
    let now = Local::now();
    if local.date_naive() == now.date_naive() {
        local.format("%H:%M").to_string()
    } else if local.year() == now.year() {
        local.format("%-m/%-d").to_string()
    } else {
        local.format("%Y/%-m/%-d").to_string()
    }
}

pub fn tint(color: Rgba, factor: f32) -> Hsla {
    let color: Hsla = color.into();
    color.opacity(factor)
}
