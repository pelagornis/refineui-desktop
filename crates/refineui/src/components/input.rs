//! TextInput (Input / SearchField / password / Textarea) — web `Input`,
//! `SearchField` and `Textarea`.
//!
//! A single-line editable text field built on GPUI's `EntityInputHandler`, so
//! IME composition, the macOS character palette and clipboard all work.
//! Create it with `cx.new(|cx| TextInput::new(cx).placeholder("Name"))` and
//! subscribe to [`InputEvent`]s.

use std::ops::Range;

use gpui::{
    App, AvailableSpace, Bounds, ClipboardItem, Context, CursorStyle, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable,
    GlobalElementId, KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PaintQuad, Pixels, Point, ShapedLine, SharedString, Style, TextAlign, TextRun,
    UTF16Selection, UnderlineStyle, Window, WrappedLine, div, fill, point, prelude::*, px,
    relative, size,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::motion::{Motion, animate};
use crate::{ActiveTheme, ControlSize, Icon, IconName, IconSize, StyledExt};

#[allow(missing_docs)]
mod actions {
    gpui::actions!(
        refineui_input,
        [
            Backspace,
            Delete,
            Left,
            Right,
            SelectLeft,
            SelectRight,
            SelectAll,
            Home,
            End,
            SelectToHome,
            SelectToEnd,
            Up,
            Down,
            SelectUp,
            SelectDown,
            Paste,
            Cut,
            Copy,
            Submit,
            Clear,
            ShowCharacterPalette,
        ]
    );
}
use actions::*;

const KEY_CONTEXT: &str = "RefineUITextInput";

pub(crate) fn bind_keys(cx: &mut App) {
    let ctx = Some(KEY_CONTEXT);
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("left", Left, ctx),
        KeyBinding::new("right", Right, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("secondary-a", SelectAll, ctx),
        KeyBinding::new("secondary-v", Paste, ctx),
        KeyBinding::new("secondary-c", Copy, ctx),
        KeyBinding::new("secondary-x", Cut, ctx),
        KeyBinding::new("home", Home, ctx),
        KeyBinding::new("end", End, ctx),
        KeyBinding::new("cmd-left", Home, ctx),
        KeyBinding::new("cmd-right", End, ctx),
        KeyBinding::new("shift-home", SelectToHome, ctx),
        KeyBinding::new("shift-end", SelectToEnd, ctx),
        KeyBinding::new("cmd-shift-left", SelectToHome, ctx),
        KeyBinding::new("cmd-shift-right", SelectToEnd, ctx),
        KeyBinding::new("up", Up, ctx),
        KeyBinding::new("down", Down, ctx),
        KeyBinding::new("shift-up", SelectUp, ctx),
        KeyBinding::new("shift-down", SelectDown, ctx),
        KeyBinding::new("enter", Submit, ctx),
        KeyBinding::new("escape", Clear, ctx),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, ctx),
    ]);
}

/// Events emitted by [`TextInput`].
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// The text changed.
    Change(SharedString),
    /// Enter was pressed.
    Submit(SharedString),
}

/// Field chrome (web `Input` vs `SearchField appearance="filled"`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InputAppearance {
    /// 1px `borderDefault` on `backgroundPrimary` (web `Input`, default).
    #[default]
    Outline,
    /// Borderless `backgroundSurfaceHover` plate (web `SearchField`).
    Filled,
}

/// Editable text field: single line (web `Input` / `SearchField`) or
/// multi-line with soft wrap ([`TextInput::multiline`], web `Textarea`).
pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    size: ControlSize,
    disabled: bool,
    invalid: bool,
    masked: bool,
    leading_icon: Option<IconName>,
    clearable: bool,
    escape_clears: bool,
    multiline: bool,
    success: bool,
    hovered: bool,
    appearance: InputAppearance,
    last_lines: Vec<WrappedLine>,
    line_height: Pixels,
}

impl EventEmitter<InputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TextInput {
    /// Empty input.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle().tab_stop(true),
            content: SharedString::default(),
            placeholder: SharedString::default(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            // Web `Input` defaults to `lg`.
            size: ControlSize::Large,
            disabled: false,
            invalid: false,
            masked: false,
            leading_icon: None,
            clearable: false,
            escape_clears: false,
            multiline: false,
            success: false,
            hovered: false,
            appearance: InputAppearance::Outline,
            last_lines: Vec::new(),
            line_height: px(20.),
        }
    }

    /// Multi-line field with soft wrap (web `Textarea`): `bodyMd`, `roundedLarge`,
    /// 80px minimum height, Enter inserts a newline.
    pub fn multiline(cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(cx);
        this.multiline = true;
        this
    }

    /// Search field preset (web `SearchField`): search icon, clear button, Esc clears.
    pub fn search(cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(cx);
        this.leading_icon = Some(IconName::Search);
        this.clearable = true;
        this.escape_clears = true;
        this.appearance = InputAppearance::Filled;
        // Web `SearchField` defaults to `md`.
        this.size = SEARCH_DEFAULT_SIZE;
        this.placeholder = "Search".into();
        this
    }

    /// Placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Initial value.
    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.content = value.into();
        self.selected_range = self.content.len()..self.content.len();
        self
    }

    /// Control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Leading icon.
    pub fn leading_icon(mut self, icon: IconName) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    /// Show a clear (×) button when non-empty.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Password field: render bullets.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    /// Current text.
    pub fn text(&self) -> SharedString {
        self.content.clone()
    }

    /// Replace the text (emits [`InputEvent::Change`]).
    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = text.into();
        self.selected_range = self.content.len()..self.content.len();
        self.marked_range = None;
        cx.emit(InputEvent::Change(self.content.clone()));
        cx.notify();
    }

    /// Enable / disable.
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify();
    }

    /// Builder form of [`Self::set_disabled`].
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Mark as invalid (error border), e.g. from [`crate::Field`] validation.
    pub fn set_invalid(&mut self, invalid: bool, cx: &mut Context<Self>) {
        self.invalid = invalid;
        cx.notify();
    }

    /// Builder form of [`Self::set_invalid`].
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Mark as valid (success border, web `data-success`).
    pub fn set_success(&mut self, success: bool, cx: &mut Context<Self>) {
        self.success = success;
        cx.notify();
    }

    /// Builder form of [`Self::set_success`].
    pub fn success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    /// Field chrome.
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    // ---- editing -------------------------------------------------------

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn select_to_home(&mut self, _: &SelectToHome, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    fn select_to_end(&mut self, _: &SelectToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.content.len(), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline {
            self.replace_text_in_range(None, "\n", window, cx);
        } else {
            cx.emit(InputEvent::Submit(self.content.clone()));
        }
    }

    fn vertical_target(&self, delta_rows: f32) -> Option<usize> {
        if !self.multiline {
            return None;
        }
        let bounds = self.last_bounds?;
        let pos = self.cursor_point(self.cursor_offset())?;
        let target = point(
            bounds.left() + pos.x,
            bounds.top() + pos.y + self.line_height * (0.5 + delta_rows),
        );
        Some(self.index_for_mouse_position(target))
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        match self.vertical_target(-1.0) {
            Some(ix) => self.move_to(ix, cx),
            None => cx.propagate(),
        }
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        match self.vertical_target(1.0) {
            Some(ix) => self.move_to(ix, cx),
            None => cx.propagate(),
        }
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.vertical_target(-1.0) {
            self.select_to(ix, cx)
        }
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.vertical_target(1.0) {
            self.select_to(ix, cx)
        }
    }

    /// Byte offset where each logical (`\n`-separated) line starts.
    fn line_starts(&self) -> Vec<usize> {
        let mut starts = vec![0];
        starts.extend(self.content.match_indices('\n').map(|(i, _)| i + 1));
        starts
    }

    /// Caret position relative to the text origin (multi-line only).
    fn cursor_point(&self, offset: usize) -> Option<Point<Pixels>> {
        let starts = self.line_starts();
        let mut y = px(0.);
        for (i, line) in self.last_lines.iter().enumerate() {
            let start = *starts.get(i)?;
            let end = start + line.len();
            if offset <= end {
                let p = line.position_for_index(offset - start, self.line_height)?;
                return Some(point(p.x, y + p.y));
            }
            y += line.size(self.line_height).height;
        }
        None
    }

    fn clear(&mut self, _: &Clear, _: &mut Window, cx: &mut Context<Self>) {
        if self.escape_clears && !self.content.is_empty() {
            self.set_text("", cx);
        } else {
            cx.propagate();
        }
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.multiline {
                text
            } else {
                text.replace('\n', " ")
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && !self.masked {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        window.focus(&self.focus_handle);
        self.is_selecting = true;
        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn display_text(&self) -> SharedString {
        if self.masked {
            "•".repeat(self.content.chars().count()).into()
        } else {
            self.content.clone()
        }
    }

    /// Content byte offset → display byte offset.
    fn to_display_offset(&self, offset: usize) -> usize {
        if self.masked {
            self.content[..offset.min(self.content.len())]
                .chars()
                .count()
                * '•'.len_utf8()
        } else {
            offset
        }
    }

    /// Display byte offset → content byte offset.
    fn display_to_content_offset(&self, offset: usize) -> usize {
        if self.masked {
            let chars = offset / '•'.len_utf8();
            self.content
                .char_indices()
                .nth(chars)
                .map(|(i, _)| i)
                .unwrap_or(self.content.len())
        } else {
            offset
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        if self.multiline {
            let Some(bounds) = self.last_bounds.as_ref() else {
                return 0;
            };
            if position.y < bounds.top() {
                return 0;
            }
            let starts = self.line_starts();
            let mut top = bounds.top();
            for (i, line) in self.last_lines.iter().enumerate() {
                let h = line.size(self.line_height).height;
                if position.y < top + h || i + 1 == self.last_lines.len() {
                    let local = point(
                        position.x - bounds.left(),
                        (position.y - top).min(h - px(1.)),
                    );
                    let ix = match line.closest_index_for_position(local, self.line_height) {
                        Ok(ix) | Err(ix) => ix,
                    };
                    return (starts.get(i).copied().unwrap_or(0) + ix).min(self.content.len());
                }
                top += h;
            }
            return self.content.len();
        }
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        self.display_to_content_offset(line.closest_index_for_x(position.x - bounds.left()))
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
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
}

impl EntityInputHandler for TextInput {
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
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        cx.emit(InputEvent::Change(self.content.clone()));
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
        if self.disabled {
            return;
        }
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.marked_range = if new_text.is_empty() {
            None
        } else {
            Some(range.start..range.start + new_text.len())
        };
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        if self.multiline {
            let p = self.cursor_point(range.start)?;
            return Some(Bounds::new(
                point(bounds.left() + p.x, bounds.top() + p.y),
                size(px(2.), self.line_height),
            ));
        }
        let last_layout = self.last_layout.as_ref()?;
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(self.to_display_offset(range.start)),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(self.to_display_offset(range.end)),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        if self.multiline {
            return Some(self.offset_to_utf16(self.index_for_mouse_position(point)));
        }
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;
        let display_index = last_layout.index_for_x(point.x - line_point.x)?;
        Some(self.offset_to_utf16(self.display_to_content_offset(display_index)))
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    lines: Vec<WrappedLine>,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
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
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        if !self.input.read(cx).multiline {
            style.size.height = window.line_height().into();
            return (window.request_layout(style, [], cx), ());
        }
        // Textarea: measure wrapped height for the available width.
        let input = self.input.clone();
        let text_style = window.text_style();
        let line_height = window.line_height();
        let layout = window.request_measured_layout(style, move |known, available, window, cx| {
            let width = known.width.or(match available.width {
                AvailableSpace::Definite(w) => Some(w),
                _ => None,
            });
            let input = input.read(cx);
            let text = if input.content.is_empty() {
                input.placeholder.clone()
            } else {
                input.content.clone()
            };
            let font_size = text_style.font_size.to_pixels(window.rem_size());
            let run = TextRun {
                len: text.len(),
                font: text_style.font(),
                color: text_style.color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let height = window
                .text_system()
                .shape_text(text, font_size, &[run], width, None)
                .map(|lines| {
                    lines
                        .iter()
                        .map(|l| l.size(line_height).height)
                        .fold(px(0.), |a, b| a + b)
                })
                .unwrap_or(line_height)
                .max(line_height);
            size(width.unwrap_or(px(0.)), height)
        });
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let theme = cx.theme().clone();
        let input = self.input.read(cx);
        let content = input.display_text();
        let selected_range = input.to_display_offset(input.selected_range.start)
            ..input.to_display_offset(input.selected_range.end);
        let cursor = input.to_display_offset(input.cursor_offset());
        let style = window.text_style();

        let content_is_empty = content.is_empty();
        let (display_text, text_color) = if content.is_empty() {
            (
                input.placeholder.clone(),
                theme.colors.foreground_placeholder,
            )
        } else {
            (content, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) = input.marked_range.as_ref().filter(|_| !input.masked)
        {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        if input.multiline {
            let line_height = window.line_height();
            let lines: Vec<WrappedLine> = window
                .text_system()
                .shape_text(
                    display_text,
                    font_size,
                    &runs,
                    Some(bounds.size.width),
                    None,
                )
                .map(|l| l.into_iter().collect())
                .unwrap_or_default();
            let input = self.input.clone();
            let input_ref = input.read(cx);
            let starts = input_ref.line_starts();
            let mut quads = Vec::new();
            let mut cursor_quad = None;
            let mut top = bounds.top();
            let sel = selected_range.clone();
            for (i, line) in lines.iter().enumerate() {
                let start = starts.get(i).copied().unwrap_or(0);
                let end = start + line.len();
                let h = line.size(line_height).height;
                if !content_is_empty {
                    if sel.is_empty()
                        && cursor >= start
                        && cursor <= end
                        && cursor_quad.is_none()
                        && let Some(p) = line.position_for_index(cursor - start, line_height)
                    {
                        cursor_quad = Some(fill(
                            Bounds::new(
                                point(bounds.left() + p.x, top + p.y),
                                size(px(1.5), line_height),
                            ),
                            theme.colors.foreground_primary,
                        ));
                    }
                    let a = sel.start.max(start);
                    let b = sel.end.min(end);
                    if !sel.is_empty()
                        && a <= b
                        && sel.start <= end
                        && sel.end >= start
                        && let (Some(pa), Some(pb)) = (
                            line.position_for_index(a - start, line_height),
                            line.position_for_index(b - start, line_height),
                        )
                    {
                        let mut y = pa.y;
                        while y <= pb.y {
                            let x0 = if y == pa.y { pa.x } else { px(0.) };
                            let x1 = if y == pb.y { pb.x } else { bounds.size.width };
                            quads.push(fill(
                                Bounds::from_corners(
                                    point(bounds.left() + x0, top + y),
                                    point(
                                        bounds.left() + x1.max(x0 + px(4.)),
                                        top + y + line_height,
                                    ),
                                ),
                                theme.colors.background_info.opacity(0.3),
                            ));
                            y += line_height;
                        }
                    }
                }
                top += h;
            }
            if content_is_empty {
                cursor_quad = Some(fill(
                    Bounds::new(bounds.origin, size(px(1.5), line_height)),
                    theme.colors.foreground_primary,
                ));
            }
            return PrepaintState {
                line: None,
                lines,
                cursor: cursor_quad,
                selection: quads,
            };
        }
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            (
                Vec::new(),
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(1.5), bounds.bottom() - bounds.top()),
                    ),
                    theme.colors.foreground_primary,
                )),
            )
        } else {
            (
                vec![fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    theme.colors.background_info.opacity(0.3),
                )],
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            lines: Vec::new(),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        for selection in prepaint.selection.drain(..) {
            window.paint_quad(selection)
        }
        if prepaint.line.is_none() {
            let line_height = window.line_height();
            let mut origin = bounds.origin;
            for line in &prepaint.lines {
                line.paint(
                    origin,
                    line_height,
                    TextAlign::Left,
                    Some(bounds),
                    window,
                    cx,
                )
                .ok();
                origin.y += line.size(line_height).height;
            }
            if focus_handle.is_focused(window)
                && let Some(cursor) = prepaint.cursor.take()
            {
                window.paint_quad(cursor);
            }
            let lines = std::mem::take(&mut prepaint.lines);
            self.input.update(cx, |input, _cx| {
                input.last_lines = lines;
                input.last_bounds = Some(bounds);
                input.line_height = line_height;
            });
            return;
        }
        let Some(line) = prepaint.line.take() else {
            return;
        };
        line.paint(bounds.origin, window.line_height(), window, cx)
            .ok();

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let c = theme.colors;
        let focused = self.focus_handle.is_focused(window);
        let m = theme.field_metrics(self.size);
        let filled = self.appearance == InputAppearance::Filled;
        let hovered = self.hovered && !self.disabled;
        // refineui.css: Input / Textarea border + 0.5px focus halo; SearchField plate.
        let (border, halo, bg) = if filled {
            let bg = if self.disabled {
                c.background_surface_disabled
            } else if hovered && !focused {
                c.background_surface_active
            } else {
                c.background_surface_hover
            };
            let border = if self.disabled {
                c.border_disabled
            } else if focused {
                c.border_default
            } else {
                gpui::transparent_black()
            };
            (border, None, bg)
        } else if self.disabled {
            (c.border_disabled, None, c.background_surface_disabled)
        } else if self.invalid {
            if focused {
                (
                    c.background_error_hover,
                    Some(c.background_error_hover),
                    c.background_primary,
                )
            } else {
                (c.border_error, None, c.background_primary)
            }
        } else if self.success {
            if focused {
                (
                    c.background_success_hover,
                    Some(c.background_success_hover),
                    c.background_primary,
                )
            } else {
                (c.border_success, None, c.background_primary)
            }
        } else if focused {
            (c.border_focus, Some(c.border_focus), c.background_primary)
        } else if hovered {
            (c.border_hover, None, c.background_primary)
        } else {
            (c.border_default, None, c.background_primary)
        };
        let key = cx.entity_id();
        let motion = if filled {
            Motion::new(150, crate::motion::easing::EMPHASIZED)
        } else {
            Motion::FAST_EASE
        };
        let border = animate(("refineui-input-border", key), border, motion, window, cx);
        let bg = animate(("refineui-input-bg", key), bg, motion, window, cx);
        let halo_alpha = animate(
            ("refineui-input-halo", key),
            if halo.is_some() { 1.0 } else { 0.0 },
            Motion::FAST_EASE,
            window,
            cx,
        );
        let halo_color = halo.unwrap_or(border);
        // SearchField (filled) uses a larger leading glyph: small / medium / large.
        let shell = search_shell(theme.density, self.size);
        let icon_size = IconSize::Custom(px(if filled { shell.icon } else { m.icon }));
        let clear_size = IconSize::Custom(px(match self.size {
            ControlSize::Small => refineui_tokens::size::icon::X_SMALL,
            _ => refineui_tokens::size::icon::SMALL,
        }));
        let show_clear = self.clearable && !self.content.is_empty() && !self.disabled;
        let multiline = self.multiline;
        let text = if multiline {
            refineui_tokens::typography::role::BODY_MD
        } else {
            m.text
        };
        div()
            .id(("refineui-text-input", cx.entity_id()))
            .flex()
            // Web `Input` (`formControlInputSizeClass`): min-height +
            // block padding, so `lg` is 16 + 24 + 16 + 2 = 58px. SearchField
            // (`formControlRowShellSizeClass`) is a fixed-height row with
            // inline padding only (36 / 44 / 52).
            .when(!multiline && filled, |d| {
                d.items_center()
                    .h(px(shell.height))
                    .min_h(px(shell.height))
                    .max_h(px(shell.height))
                    .px(px(shell.padding_x))
            })
            .when(!multiline && !filled, |d| {
                d.items_center()
                    .min_h(px(m.height))
                    .px(px(m.padding_x))
                    .py(px(m.padding_y))
            })
            .when(multiline, |d| {
                d.items_start()
                    .min_h(px(refineui_tokens::size::component::TEXTAREA_MIN))
                    .px(px(refineui_tokens::spacing::X_SMALL))
                    .py(px(refineui_tokens::spacing::MEDIUM))
            })
            .gap(px(refineui_tokens::spacing::X_SMALL))
            .w_full()
            .rounded(px(if multiline {
                refineui_tokens::radius::LARGE
            } else if filled {
                shell.radius
            } else {
                m.radius
            }))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(if self.disabled {
                c.foreground_disabled
            } else {
                c.foreground_primary
            })
            .text_style_token(text)
            .overflow_hidden()
            .when(halo_alpha > 0.01, |d| {
                d.shadow(theme.ring(
                    halo_color.opacity(halo_alpha),
                    refineui_tokens::stroke::HAIRLINE,
                ))
            })
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if this.hovered != *hovered {
                    this.hovered = *hovered;
                    cx.notify();
                }
            }))
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .when(!self.disabled, |d| d.cursor(CursorStyle::IBeam))
            .when(self.disabled, |d| {
                d.cursor(CursorStyle::OperationNotAllowed)
            })
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_to_home))
            .on_action(cx.listener(Self::select_to_end))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::clear))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .when_some(self.leading_icon, |d, icon| {
                d.child(Icon::new(icon).size(icon_size).color(if self.disabled {
                    c.foreground_disabled
                } else {
                    c.foreground_tertiary
                }))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .child(TextElement { input: cx.entity() }),
            )
            .when(show_clear, |d| {
                d.child(
                    div()
                        .id("clear")
                        .flex_none()
                        .cursor_action()
                        .opacity(0.9)
                        .hover(|s| s.opacity(1.0))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.set_text("", cx);
                            window.focus(&this.focus_handle);
                        }))
                        .child(
                            Icon::new(IconName::DismissCircle)
                                .style(crate::IconStyle::Filled)
                                .size(clear_size)
                                .color(c.foreground_tertiary),
                        ),
                )
            })
    }
}

/// Web `SearchField` default size (`md`).
pub(crate) const SEARCH_DEFAULT_SIZE: ControlSize = ControlSize::Medium;

/// Resolved SearchField shell (web `formControlRowShellSizeClass` +
/// `searchFieldLeadingIconSize`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SearchShell {
    pub height: f32,
    pub padding_x: f32,
    pub radius: f32,
    pub icon: f32,
}

/// SearchField metrics for `size` at `density`: a fixed-height row (no block
/// padding), so web Comfortable is 36 / 44 / 52 tall, 12 / 16 / 20 inline
/// padding, `roundedLarge / XLarge / XXLarge`, and a 20 / 24 / 28 search glyph.
pub(crate) fn search_shell(
    density: refineui_tokens::size::Density,
    size: ControlSize,
) -> SearchShell {
    let m = density.field(size);
    let icon = match (density, size) {
        (refineui_tokens::size::Density::Compact, _) => m.icon,
        (_, ControlSize::Small) => refineui_tokens::size::icon::SMALL,
        (_, ControlSize::Medium) => refineui_tokens::size::icon::MEDIUM,
        (_, ControlSize::Large) => refineui_tokens::size::icon::LARGE,
    };
    SearchShell {
        height: m.height,
        padding_x: m.padding_x,
        radius: m.radius,
        icon,
    }
}

/// Web `SearchField` — a [`TextInput`] preset (filled plate, search icon,
/// clear button, Esc clears). `cx.new(|cx| SearchField::new(cx))`.
pub struct SearchField;

impl SearchField {
    /// New search field input.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(cx: &mut Context<TextInput>) -> TextInput {
        TextInput::search(cx)
    }
}

/// Web `Textarea` — a multi-line [`TextInput`]. `cx.new(|cx| Textarea::new(cx))`.
pub struct Textarea;

impl Textarea {
    /// New multi-line input.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(cx: &mut Context<TextInput>) -> TextInput {
        TextInput::multiline(cx)
    }
}

#[cfg(test)]
mod search_shell_tests {
    use super::*;
    use refineui_tokens::size::Density;

    /// Pins web `SearchField` (`formControlRowShellSizeClass`) so the size
    /// cannot regress silently.
    #[test]
    fn search_field_matches_web_sizes() {
        assert_eq!(SEARCH_DEFAULT_SIZE, ControlSize::Medium);
        let rows = [
            (ControlSize::Small, 36., 10., 8., 20.),
            (ControlSize::Medium, 44., 16., 12., 24.),
            (ControlSize::Large, 52., 20., 16., 28.),
        ];
        for (size, h, px_, r, icon) in rows {
            let s = search_shell(Density::Comfortable, size);
            assert_eq!(
                (s.height, s.padding_x, s.radius, s.icon),
                (h, px_, r, icon),
                "{size:?}"
            );
        }
    }
}
