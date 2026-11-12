//! A small native text input / textarea for GPUI.
//!
//! It implements `EntityInputHandler` so macOS IME (including Japanese) works,
//! supports selection, clipboard, undo-free simple editing, and wraps text in a
//! fixed-height viewport. Adapted from the GPUI `input` example (Apache-2.0).

use std::ops::Range;

use gpui::{
    actions, div, fill, point, prelude::*, px, relative, size, App, Bounds, ClipboardItem,
    ContentMask, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, Focusable, GlobalElementId, InspectorElementId, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    SharedString, Style, TextAlign, TextRun, UTF16Selection, UnderlineStyle, Window,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::theme;

actions!(
    homer_input,
    [
        InputBackspace,
        InputDelete,
        InputLeft,
        InputRight,
        InputSelectLeft,
        InputSelectRight,
        InputSelectAll,
        InputHome,
        InputEnd,
        InputShowCharacterPalette,
        InputPaste,
        InputCut,
        InputCopy,
        InputNewline,
    ]
);

const PAD_X: f32 = 12.0;

pub struct TextArea {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<Vec<gpui::WrappedLine>>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    multiline: bool,
    height: Pixels,
    font_size: Pixels,
    line_height: Pixels,
    pad_y: Pixels,
    emphasized: bool,
    scroll_y: Pixels,
    context: SharedString,
}

impl TextArea {
    pub fn new(
        placeholder: impl Into<SharedString>,
        height: f32,
        multiline: bool,
        context: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let font_size = px(14.);
        let line_height = px(23.);
        let pad_y = if multiline {
            px(9.)
        } else {
            px(((height - 23.) / 2.).max(4.))
        };
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            placeholder: placeholder.into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            multiline,
            height: px(height),
            font_size,
            line_height,
            pad_y,
            emphasized: false,
            scroll_y: px(0.),
            context: context.into(),
        }
    }

    pub fn emphasized(mut self) -> Self {
        self.emphasized = true;
        self
    }

    pub fn content(&self) -> String {
        self.content.to_string()
    }

    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    pub fn set_content(&mut self, text: &str, cx: &mut Context<Self>) {
        self.content = text.to_string().into();
        self.selected_range = self.content.len()..self.content.len();
        self.selection_reversed = false;
        self.marked_range = None;
        self.scroll_y = px(0.);
        self.last_layout = None;
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.set_content("", cx);
    }

    pub fn set_placeholder(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.placeholder = text.into();
        cx.notify();
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn offset_from_utf16(offset: usize, text: &str) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in text.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(offset: usize, text: &str) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in text.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        Self::offset_to_utf16(range.start, &self.content)
            ..Self::offset_to_utf16(range.end, &self.content)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        Self::offset_from_utf16(range_utf16.start, &self.content)
            ..Self::offset_from_utf16(range_utf16.end, &self.content)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    fn backspace(&mut self, _: &InputBackspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &InputDelete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn left(&mut self, _: &InputLeft, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn right(&mut self, _: &InputRight, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn select_left(&mut self, _: &InputSelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &InputSelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &InputSelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        cx.notify();
    }

    fn home(&mut self, _: &InputHome, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &InputEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn newline(&mut self, _: &InputNewline, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline {
            self.replace_text_in_range(None, "\n", window, cx);
        }
    }

    fn show_character_palette(
        &mut self,
        _: &InputShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &InputPaste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.multiline {
                text.replace("\r\n", "\n")
            } else {
                text.replace(['\r', '\n'], " ")
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &InputCopy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &InputCut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.is_selecting = true;
        if event.modifiers.shift {
            let index = self.index_for_mouse_position(event.position);
            self.select_to(index, cx);
        } else {
            let index = self.index_for_mouse_position(event.position);
            self.move_to(index, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            let index = self.index_for_mouse_position(event.position);
            self.select_to(index, cx);
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let (Some(bounds), Some(lines)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        let mut y = position.y - bounds.top() - self.pad_y + self.scroll_y;
        let x = position.x - bounds.left() - px(PAD_X);
        let mut offset = 0;
        for (i, line) in lines.iter().enumerate() {
            let height = line.size(self.line_height).height;
            let last = i + 1 == lines.len();
            if y < height || last {
                let index = line
                    .closest_index_for_position(point(x, y), self.line_height)
                    .unwrap_or_else(|index| index);
                return (offset + index).min(self.content.len());
            }
            offset += line.len() + 1;
            y -= height;
        }
        self.content.len()
    }
}

impl EntityInputHandler for TextArea {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let range = range.start.min(self.content.len())..range.end.min(self.content.len());
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.selection_reversed = false;
        self.marked_range.take();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let range = range.start.min(self.content.len())..range.end.min(self.content.len());
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|new_range| {
                let start = Self::offset_from_utf16(new_range.start, new_text);
                let end = Self::offset_from_utf16(new_range.end, new_text);
                range.start + start..range.start + end
            })
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.selection_reversed = false;
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let lines = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        let (_, paragraph_start, position) = locate_position(lines, range.start, self.line_height)?;
        let _ = paragraph_start;
        let origin = point(
            bounds.left() + px(PAD_X),
            bounds.top() + self.pad_y - self.scroll_y,
        );
        Some(Bounds::new(
            origin + position,
            size(px(2.), self.line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.index_for_mouse_position(point))
    }
}

/// Returns (paragraph index, byte offset of paragraph, position within paragraph).
fn locate_position(
    lines: &[gpui::WrappedLine],
    index: usize,
    line_height: Pixels,
) -> Option<(usize, usize, Point<Pixels>)> {
    let mut offset = 0;
    for (i, line) in lines.iter().enumerate() {
        let end = offset + line.len();
        if index <= end {
            let within = index - offset;
            let position = line.position_for_index(within, line_height)?;
            return Some((i, offset, position));
        }
        offset = end + 1;
    }
    lines
        .last()
        .map(|_| (lines.len().saturating_sub(1), offset, point(px(0.), px(0.))))
}

struct TextAreaElement {
    input: Entity<TextArea>,
}

struct PrepaintState {
    lines: Vec<gpui::WrappedLine>,
    selection: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
    text_origin: Point<Pixels>,
    bounds: Bounds<Pixels>,
}

impl IntoElement for TextAreaElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextAreaElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let (
            content,
            placeholder,
            selected_range,
            cursor,
            marked_range,
            font_size,
            line_height,
            pad_y,
            scroll_y,
            focused,
        ) = {
            let input = self.input.read(cx);
            (
                input.content.clone(),
                input.placeholder.clone(),
                input.selected_range.clone(),
                input.cursor_offset(),
                input.marked_range.clone(),
                input.font_size,
                input.line_height,
                input.pad_y,
                input.scroll_y,
                input.focus_handle.is_focused(window),
            )
        };

        let (display_text, text_color) = if content.is_empty() {
            (placeholder, theme::faint())
        } else {
            (content.clone(), theme::text())
        };
        let style = window.text_style();
        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color.into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked) = marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked.end - marked.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|r| r.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let wrap_width = (bounds.size.width - px(PAD_X * 2.)).max(px(20.));
        let lines = window
            .text_system()
            .shape_text(display_text, font_size, &runs, Some(wrap_width), None)
            .unwrap_or_default()
            .to_vec();

        let content_height = lines
            .iter()
            .map(|line| line.size(line_height).height)
            .fold(px(0.), |acc, h| acc + h);
        let view_height = bounds.size.height - pad_y * 2.0;
        let max_scroll = (content_height - view_height).max(px(0.));
        let mut scroll = scroll_y.clamp(px(0.), max_scroll);

        let mut selection = Vec::new();
        if !content.is_empty() && !selected_range.is_empty() {
            let mut paragraph_offset = 0;
            for line in &lines {
                let para_start = selected_range.start.max(paragraph_offset);
                let para_end = selected_range.end.min(paragraph_offset + line.len());
                if para_start < para_end {
                    let rel_start = para_start - paragraph_offset;
                    let rel_end = para_end - paragraph_offset;
                    if let (Some(a), Some(b)) = (
                        line.position_for_index(rel_start, line_height),
                        line.position_for_index(rel_end, line_height),
                    ) {
                        let origin =
                            point(bounds.left() + px(PAD_X), bounds.top() + pad_y - scroll);
                        let rows = line.wrap_boundaries().len() + 1;
                        let start_row = (a.y / line_height).floor() as usize;
                        let end_row =
                            ((b.y / line_height).floor() as usize).min(rows.saturating_sub(1));
                        for row in start_row..=end_row {
                            let x0 = if row == start_row { a.x } else { px(0.) };
                            let x1 = if row == end_row { b.x } else { wrap_width };
                            if x1 > x0 {
                                selection.push(fill(
                                    Bounds::new(
                                        origin + point(x0, line_height * row as f32),
                                        size(x1 - x0, line_height),
                                    ),
                                    theme::tint(theme::accent(), 0.12),
                                ));
                            }
                        }
                    }
                }
                paragraph_offset += line.len() + 1;
            }
        }

        let mut cursor_quad = None;
        if focused && !content.is_empty() {
            if let Some((_, _, pos)) = locate_position(&lines, cursor, line_height) {
                let absolute_y = pos.y;
                if absolute_y < scroll {
                    scroll = absolute_y;
                } else if absolute_y + line_height > scroll + view_height {
                    scroll = absolute_y + line_height - view_height;
                }
                scroll = scroll.clamp(px(0.), max_scroll);
                let origin = point(bounds.left() + px(PAD_X), bounds.top() + pad_y - scroll);
                cursor_quad = Some(fill(
                    Bounds::new(origin + pos, size(px(1.5), line_height)),
                    theme::text(),
                ));
            }
        } else if focused {
            let origin = point(bounds.left() + px(PAD_X), bounds.top() + pad_y - scroll);
            cursor_quad = Some(fill(
                Bounds::new(origin, size(px(1.5), line_height)),
                theme::text(),
            ));
        }

        self.input.update(cx, |input, _| {
            input.last_layout = Some(lines.clone());
            input.last_bounds = Some(bounds);
            input.scroll_y = scroll;
        });

        PrepaintState {
            lines,
            selection,
            cursor: cursor_quad,
            text_origin: point(bounds.left() + px(PAD_X), bounds.top() + pad_y - scroll),
            bounds,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(prepaint.bounds, self.input.clone()),
            cx,
        );

        let line_height = self.input.read(cx).line_height;
        window.with_content_mask(
            Some(ContentMask {
                bounds: prepaint.bounds,
            }),
            |window| {
                for quad in prepaint.selection.drain(..) {
                    window.paint_quad(quad);
                }
                let mut y = px(0.);
                for line in &prepaint.lines {
                    line.paint(
                        prepaint.text_origin + point(px(0.), y),
                        line_height,
                        TextAlign::Left,
                        None,
                        window,
                        cx,
                    )
                    .ok();
                    y += line.size(line_height).height;
                }
                if let Some(cursor) = prepaint.cursor.take() {
                    window.paint_quad(cursor);
                }
            },
        );
    }
}

impl Render for TextArea {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border = if self.emphasized {
            theme::line_strong()
        } else {
            theme::line()
        };
        div()
            .w_full()
            .h(self.height)
            .bg(theme::surface())
            .border_1()
            .border_color(border)
            .rounded(px(if self.emphasized { 9. } else { 7. }))
            .overflow_hidden()
            .key_context(self.context.as_ref())
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .focus(|s| s.border_color(gpui::rgb(0xb8ad9c)))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(TextAreaElement { input: cx.entity() })
    }
}

impl Focusable for TextArea {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
