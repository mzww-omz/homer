use gpui::prelude::*;
use gpui::{div, px, Div, FontWeight, SharedString};

use crate::theme;

pub fn button(label: impl Into<SharedString>, primary: bool) -> Div {
    let base = div()
        .h(px(30.))
        .px(px(12.))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .rounded(px(7.))
        .border_1()
        .text_size(px(12.5))
        .cursor_pointer()
        .child(label.into());
    if primary {
        base.bg(theme::text())
            .border_color(theme::text())
            .text_color(theme::bg())
            .hover(|s| s.bg(gpui::rgb(0x241f1a)).border_color(gpui::rgb(0x241f1a)))
    } else {
        base.bg(theme::surface())
            .border_color(theme::line())
            .text_color(theme::text())
            .hover(|s| s.bg(theme::surface_2()).border_color(theme::line_strong()))
    }
}

pub fn icon_button(glyph: impl Into<SharedString>, size: f32) -> Div {
    div()
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .text_color(theme::muted())
        .text_size(px(13.))
        .cursor_pointer()
        .hover(|s| s.bg(gpui::rgb(0x302c2712)).text_color(theme::text()))
        .child(glyph.into())
}

pub fn avatar(name: &str, accent: &str, size: f32) -> Div {
    let color = theme::parse_hex(accent);
    let initial = name.chars().next().unwrap_or('？').to_string();
    div()
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme::tint(color, 0.14))
        .border_1()
        .border_color(theme::tint(color, 0.24))
        .text_color(color)
        .text_size(px((size * 0.4).max(10.)))
        .font_weight(FontWeight::SEMIBOLD)
        .child(initial)
}

pub fn narrator_avatar(size: f32) -> Div {
    div()
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme::surface())
        .border_1()
        .border_color(theme::line_strong())
        .text_color(theme::faint())
        .font_family(theme::serif_font())
        .text_size(px((size * 0.4).max(10.)))
        .child("語")
}

pub fn label(text: &'static str) -> Div {
    div()
        .text_size(px(11.5))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme::muted())
        .mb(px(6.))
        .child(text)
}

pub fn section_title(text: &'static str) -> Div {
    div()
        .text_size(px(10.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::faint())
        .child(text)
}

pub fn divider() -> Div {
    div().h(px(1.)).w_full().bg(theme::line())
}
