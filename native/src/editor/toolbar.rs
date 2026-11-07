use egui::{pos2, vec2, Align, Align2, Color32, FontId, Layout, Rect, Response, Sense, Stroke, Ui};
use serde_json::{json, Value};

use super::Editor;
use crate::icons::{self, IconName};
use crate::state::AppState;
use crate::theme::UiColors;

impl Editor {
    pub(super) fn toolbar(&mut self, ui: &mut Ui, state: &mut AppState, colors: &UiColors) {
        let (shell, _) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::hover());
        ui.painter().rect_filled(shell, 10, colors.app_toolbar);
        ui.painter().rect_stroke(
            shell,
            10,
            Stroke::new(1.0_f32, colors.app_border),
            egui::StrokeKind::Inside,
        );
        let content = Rect::from_min_max(
            pos2(shell.left() + 8.0, shell.top() + 8.0),
            pos2(shell.right() - 8.0, shell.bottom() - 8.0),
        );
        let trailing_width = 86.0;
        let mut row = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    content.min,
                    pos2(content.right() - trailing_width - 40.0, content.bottom()),
                ))
                .layout(Layout::left_to_right(Align::Center)),
        );
        row.spacing_mut().item_spacing = vec2(2.0, 0.0);
        let mut overflow = Vec::new();
        let search = icon_button(
            &mut row,
            colors,
            IconName::Search,
            32.0,
            false,
            true,
            "Search The Menus (Alt+/)",
        );
        row.painter().rect_stroke(
            search.rect,
            7,
            Stroke::new(1.0_f32, colors.app_border),
            egui::StrokeKind::Inside,
        );
        if search.clicked() {
            self.toolbar_command("searchMenus", Value::Null, state);
        }
        for (icon, label, command, enabled) in [
            (IconName::Undo, "Undo", "undo", !self.undo.is_empty()),
            (IconName::Redo, "Redo", "redo", !self.redo.is_empty()),
            (IconName::Painter, "Format Painter", "copyFormatting", true),
        ] {
            if row.available_width() < 32.0 {
                overflow.push((label, command, Value::Null));
                continue;
            }
            if icon_button(&mut row, colors, icon, 30.0, false, enabled, label).clicked() && enabled
            {
                if command == "copyFormatting" {
                    self.toolbar_command(command, Value::Null, state);
                } else {
                    self.format_command(&json!({"type":command}), state);
                }
            }
        }
        separator(&mut row, colors);
        if row.available_width() >= 128.0 {
            let style = self
                .document
                .paragraphs
                .get(self.active)
                .and_then(|paragraph| paragraph.style.clone())
                .unwrap_or_else(|| "Normal".into());
            let label = paragraph_style_label(&style);
            let picked_style = style.clone();
            let response = text_picker(&mut row, colors, label, 128.0, "Paragraph Style");
            let mut picked = None;
            popup(&row, "toolbar-style", &response, |ui| {
                ui.label(egui::RichText::new("Paragraph Style").weak().small());
                for &(label, style, size, italic, color, before, after, spacing, indent) in
                    PARAGRAPH_STYLES
                {
                    if ui
                        .selectable_label(style == picked_style.as_str(), label)
                        .clicked()
                    {
                        picked = Some((style, size, italic, color, before, after, spacing, indent));
                    }
                }
            });
            if let Some((style, size, italic, color, before, after, spacing, indent)) = picked {
                self.apply_paragraph_style(
                    state, style, size, italic, color, before, after, spacing, indent,
                );
            }
        }
        separator(&mut row, colors);
        if row.available_width() >= 122.0 {
            let font = self
                .format
                .font_family
                .as_deref()
                .unwrap_or("Calibri")
                .to_string();
            let response = text_picker(&mut row, colors, &font, 120.0, "Font Family");
            let mut picked = None;
            let popup_id = row.id().with("toolbar-font");
            let search_id = popup_id.with("search");
            if response.clicked() {
                let is_open = row.memory(|memory| memory.is_popup_open(popup_id));
                if !is_open {
                    row.data_mut(|data| data.insert_temp(search_id, String::new()));
                }
                row.memory_mut(|memory| memory.toggle_popup(popup_id));
            }
            egui::popup_below_widget(
                &row,
                popup_id,
                &response,
                egui::PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    ui.set_min_width(220.0);
                    let mut search = ui
                        .data(|data| data.get_temp::<String>(search_id))
                        .unwrap_or_default();
                    ui.add(
                        egui::TextEdit::singleline(&mut search)
                            .hint_text("Search Fonts")
                            .desired_width(220.0),
                    );
                    ui.data_mut(|data| data.insert_temp(search_id, search.clone()));
                    let available = ui.fonts(|fonts| fonts.families());
                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .show(ui, |ui| {
                            for family in matching_document_fonts(&search) {
                                if !available.iter().any(|entry| matches!(entry, egui::FontFamily::Name(name) if name.as_ref() == family)) {
                                    continue;
                                }
                                if ui.selectable_label(font == family, family).clicked() {
                                    picked = Some(family.to_string());
                                    ui.memory_mut(|memory| memory.close_popup());
                                }
                            }
                        });
                },
            );
            if let Some(family) = picked.filter(|_| self.can_format_selection(state)) {
                self.format.font_family = Some(family.clone());
                self.apply_transform(state, |format| format.font_family = Some(family.clone()));
            }
        }
        if row.available_width() >= 92.0 {
            let mut size = self.format.font_size.unwrap_or(11.0);
            let mut changed = false;
            if text_button(&mut row, colors, "\u{2212}", 22.0).clicked() {
                size = (size - 1.0).max(6.0);
                changed = true;
            }
            let draft_id = row.id().with("font-size-draft");
            let input_id = row.id().with("font-size-input");
            let focused = row.memory(|memory| memory.has_focus(input_id));
            let mut draft = row
                .data(|data| data.get_temp::<String>(draft_id))
                .filter(|_| focused)
                .unwrap_or_else(|| format!("{}", size.round() as u32));
            let (rect, _) = row.allocate_exact_size(vec2(40.0, 32.0), Sense::hover());
            row.painter()
                .rect_filled(rect, 6, colors.app_surface);
            row.painter().rect_stroke(
                rect,
                6,
                Stroke::new(1.0_f32, colors.app_border),
                egui::StrokeKind::Inside,
            );
            let response = row
                .scope(|ui| {
                    macro_rules! clear_frame {
                        ($state:ident) => {{
                            let visuals = &mut ui.visuals_mut().widgets.$state;
                            visuals.bg_fill = Color32::TRANSPARENT;
                            visuals.weak_bg_fill = Color32::TRANSPARENT;
                            visuals.bg_stroke = Stroke::new(0.0_f32, Color32::TRANSPARENT);
                            visuals.corner_radius = egui::CornerRadius::same(0);
                        }};
                    }
                    clear_frame!(inactive);
                    clear_frame!(hovered);
                    clear_frame!(active);
                    ui.put(
                        rect,
                        egui::TextEdit::singleline(&mut draft)
                            .id(input_id)
                            .horizontal_align(Align::Center)
                            .font(FontId::proportional(13.0))
                            .margin(vec2(4.0, 8.0)),
                    )
                })
                .inner;
            row.data_mut(|data| data.insert_temp(draft_id, draft.clone()));
            if response.changed() {
                if let Ok(value) = draft.parse::<f32>() {
                    if (6.0..=96.0).contains(&value) {
                        size = value;
                        changed = true;
                    }
                }
            }
            if text_button(&mut row, colors, "+", 22.0).clicked() {
                size = (size + 1.0).min(96.0);
                changed = true;
            }
            if changed && self.can_format_selection(state) {
                self.format.font_size = Some(size);
                self.apply_transform(state, |format| format.font_size = Some(size));
            }
        }
        separator(&mut row, colors);
        for (icon, label, mark, active) in [
            (IconName::Bold, "Bold", "bold", self.format.bold),
            (IconName::Italic, "Italic", "italic", self.format.italic),
            (
                IconName::Underline,
                "Underline",
                "underline",
                self.format.underline,
            ),
        ] {
            if row.available_width() < 32.0 {
                overflow.push((label, mark, Value::Null));
                continue;
            }
            if icon_button(&mut row, colors, icon, 30.0, active, true, label).clicked() {
                self.format_command(&json!({"type":"toggleMark","mark":mark}), state);
            }
        }
        if row.available_width() >= 104.0 {
            let text_bar = self
                .format
                .color
                .as_deref()
                .and_then(hex_color)
                .unwrap_or(Color32::from_rgb(0x20, 0x21, 0x24));
            let highlight_bar = self
                .format
                .highlight
                .as_deref()
                .and_then(hex_color)
                .unwrap_or(Color32::from_rgb(0xFA, 0xCC, 0x15));
            for (icon, label, is_highlight, bar) in [
                (IconName::TextColor, "Text Color", false, text_bar),
                (IconName::Highlight, "Highlight", true, highlight_bar),
            ] {
                let response = colorsplit_button(&mut row, colors, icon, bar, label);
                let mut picked = None;
                popup(&row, label, &response, |ui| {
                    for (name, value) in [
                        ("Black", "202124"),
                        ("Red", "EF4444"),
                        ("Blue", "2563EB"),
                        ("Green", "15803D"),
                        ("Yellow", "FACC15"),
                        ("None", "none"),
                    ] {
                        if ui.button(name).clicked() {
                            picked = Some(value.to_string());
                        }
                    }
                });
                if let Some(value) = picked {
                    if is_highlight {
                        self.toolbar_command("setHighlight", json!({"color":value}), state);
                    } else if self.can_format_selection(state) {
                        let color = if value == "none" { None } else { Some(value) };
                        self.format.color = color.clone();
                        self.apply_transform(state, |format| format.color = color.clone());
                    }
                }
            }
        }
        separator(&mut row, colors);
        if row.available_width() >= 34.0 {
            let alignment = self
                .document
                .paragraphs
                .get(self.active)
                .map(|paragraph| paragraph.alignment)
                .unwrap_or_default();
            let icon = match alignment {
                crate::document::Alignment::Center => IconName::AlignCenter,
                crate::document::Alignment::Right => IconName::AlignRight,
                crate::document::Alignment::Justify => IconName::AlignJustify,
                _ => IconName::AlignLeft,
            };
            let response =
                dropdown_icon_button(&mut row, colors, icon, false, true, "Alignment");
            let mut picked = None;
            popup(&row, "toolbar-align", &response, |ui| {
                for (label, value) in [
                    ("Align Left", "left"),
                    ("Align Center", "center"),
                    ("Align Right", "right"),
                    ("Justify", "justify"),
                ] {
                    if ui.button(label).clicked() {
                        picked = Some(value);
                    }
                }
            });
            if let Some(align) = picked {
                self.format_command(&json!({"type":"setAlignment","align":align}), state);
            }
        }
        if row.available_width() >= 32.0 {
            let response = dropdown_icon_button(
                &mut row,
                colors,
                IconName::LineSpacing,
                false,
                true,
                "Line Spacing",
            );
            let active_spacing = self
                .document
                .paragraphs
                .get(self.active)
                .map(|paragraph| {
                    (
                        paragraph.line_spacing,
                        paragraph.space_before,
                        paragraph.space_after,
                    )
                })
                .unwrap_or((0.0, 0.0, 0.0));
            let mut picked = None;
            let mut dialog = false;
            let mut space_command = None;
            popup(&row, "toolbar-spacing", &response, |ui| {
                for value in [1.0, 1.08, 1.15, 1.5, 2.0, 2.5, 3.0] {
                    let current = if active_spacing.0 > 0.0 {
                        (active_spacing.0 - value).abs() < 0.005
                    } else {
                        (value - 1.08).abs() < 0.005
                    };
                    let label = if current {
                        format!("{value}\u{2713}")
                    } else {
                        value.to_string()
                    };
                    if ui.button(label).clicked() {
                        picked = Some(value);
                    }
                }
                ui.separator();
                if ui.button("Line Spacing Options…").clicked() {
                    dialog = true;
                }
                let before_label = if active_spacing.1 > 0.0 {
                    "Remove Space Before Paragraph"
                } else {
                    "Add Space Before Paragraph"
                };
                let after_label = if active_spacing.2 > 0.0 {
                    "Remove Space After Paragraph"
                } else {
                    "Add Space After Paragraph"
                };
                if ui.button(before_label).clicked() {
                    space_command = Some("spaceBefore");
                }
                if ui.button(after_label).clicked() {
                    space_command = Some("spaceAfter");
                }
            });
            if dialog {
                self.toolbar_command("openParagraphDialog", Value::Null, state);
            }
            if let Some(command) = space_command {
                self.toolbar_command(command, Value::Null, state);
            }
            if let Some(value) = picked {
                self.toolbar_command("setLineSpacing", json!({"value":value}), state);
            }
        }
        let paragraph_indent = self
            .document
            .paragraphs
            .get(self.active)
            .map(|paragraph| paragraph.indent)
            .unwrap_or(0.0);
        for (icon, label, command, args, enabled) in [
            (
                IconName::BulletList,
                "Bulleted List",
                "toggleList",
                json!({"kind":"bullet"}),
                true,
            ),
            (
                IconName::NumberedList,
                "Numbered List",
                "toggleList",
                json!({"kind":"number"}),
                true,
            ),
            (
                IconName::Outdent,
                "Decrease Indent",
                "changeIndent",
                json!({"delta":-1}),
                paragraph_indent > 0.0,
            ),
            (
                IconName::Indent,
                "Increase Indent",
                "changeIndent",
                json!({"delta":1}),
                true,
            ),
            (
                IconName::ClearFormatting,
                "Clear Formatting",
                "clearFormatting",
                Value::Null,
                true,
            ),
            (IconName::Link, "Insert Link", "insertLink", Value::Null, true),
            (IconName::Image, "Insert Image", "insertImage", Value::Null, true),
            (IconName::Table, "Insert Table", "insertTable", Value::Null, true),
        ] {
            if row.available_width() < 32.0 {
                overflow.push((label, command, args));
                continue;
            }
            if matches!(icon, IconName::Link) {
                separator(&mut row, colors);
            }
            if icon_button(&mut row, colors, icon, 30.0, false, enabled, label).clicked()
                && enabled
            {
                if command == "clearFormatting" {
                    self.format_command(&json!({"type":command}), state);
                } else {
                    self.toolbar_command(command, args, state);
                }
            }
        }
        let overflow_rect = Rect::from_center_size(
            pos2(content.right() - trailing_width - 20.0, content.center().y),
            vec2(30.0, 30.0),
        );
        let mut overflow_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(overflow_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let more = icon_button(
            &mut overflow_ui,
            colors,
            IconName::More,
            30.0,
            false,
            true,
            "More Formatting",
        );
        let mut picked = None;
        popup(&overflow_ui, "toolbar-more", &more, |ui| {
            for (label, command, args) in overflow.iter().cloned().chain([
                ("Strikethrough", "strike", Value::Null),
                ("Superscript", "superscript", Value::Null),
                ("Subscript", "subscript", Value::Null),
            ]) {
                if ui.button(label).clicked() {
                    picked = Some((command, args));
                }
            }
        });
        if let Some((command, args)) = picked {
            if [
                "bold",
                "italic",
                "underline",
                "strike",
                "superscript",
                "subscript",
            ]
            .contains(&command)
            {
                self.toolbar_command(
                    "editorExec",
                    json!({"type":"toggleMark","mark":command}),
                    state,
                );
            } else {
                self.toolbar_command(command, args, state);
            }
        }
        let separator_x = content.right() - trailing_width;
        ui.painter().vline(
            separator_x,
            content.top() + 3.0..=content.bottom() - 3.0,
            Stroke::new(1.0_f32, colors.app_border),
        );
        let mut end = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    pos2(separator_x + 19.0, content.top()),
                    content.max,
                ))
                .layout(Layout::left_to_right(Align::Center)),
        );
        end.spacing_mut().item_spacing = vec2(6.0, 0.0);
        if icon_button(
            &mut end,
            colors,
            IconName::Outline,
            30.0,
            state.preferences.show_outline,
            true,
            "Document Outline",
        )
        .clicked()
        {
            state.preferences.show_outline = !state.preferences.show_outline;
            self.queued_commands
                .push(crate::ui::UiCommand::SavePreferences);
        }
        if icon_button(
            &mut end,
            colors,
            IconName::ArrowUp,
            30.0,
            false,
            true,
            "Hide Menus (Ctrl+Shift+F)",
        )
        .clicked()
        {
            state.surface.focus_mode = true;
        }
    }

    fn toolbar_command(&mut self, name: &str, args: Value, state: &mut AppState) {
        if !self.execute(name, &args, state) {
            state.surface.error = Some(format!(
                "This Feature Is Not Yet Available In The Rust Editor: {name}"
            ));
        }
    }

    fn apply_paragraph_style(
        &mut self,
        state: &mut AppState,
        style: &str,
        size: f32,
        italic: bool,
        color: Option<&str>,
        space_before: f32,
        space_after: f32,
        line_spacing: f32,
        indent: f32,
    ) {
        self.checkpoint();
        let mut indices = self.selected_paragraph_indices();
        if indices.is_empty() {
            indices.push(self.active.min(self.document.paragraphs.len().saturating_sub(1)));
        }
        for index in indices {
            let Some(paragraph) = self.document.paragraphs.get_mut(index) else {
                continue;
            };
            paragraph.style = Some(style.into());
            paragraph.space_before = space_before;
            paragraph.space_after = space_after;
            paragraph.line_spacing = line_spacing;
            paragraph.indent = indent;
            if style == "Quote" {
                paragraph.alignment = crate::document::Alignment::Center;
            }
            for run in &mut paragraph.runs {
                run.format.font_size = Some(size);
                run.format.italic = italic;
                run.format.color = color.map(str::to_owned);
            }
        }
        self.format.font_size = Some(size);
        self.format.italic = italic;
        self.format.color = color.map(str::to_owned);
        self.changed(state);
    }
}

const DEFAULT_LINE_SPACING: f32 = 259.0 / 240.0;

// Spacing values mirror the default styles.xml: before/after in points,
// line_spacing as a multiple where 0 inherits, indent in native units.
const PARAGRAPH_STYLES: &[(&str, &str, f32, bool, Option<&str>, f32, f32, f32, f32)] = &[
    ("Normal", "Normal", 11.0, false, None, 0.0, 8.0, DEFAULT_LINE_SPACING, 0.0),
    ("Title", "Title", 28.0, false, None, 0.0, 0.0, 1.0, 0.0),
    ("Subtitle", "Subtitle", 11.0, false, Some("5A5A5A"), 0.0, 8.0, DEFAULT_LINE_SPACING, 0.0),
    ("Heading 1", "Heading1", 16.0, false, Some("2F5496"), 12.0, 0.0, DEFAULT_LINE_SPACING, 0.0),
    ("Heading 2", "Heading2", 13.0, false, Some("2F5496"), 2.0, 0.0, DEFAULT_LINE_SPACING, 0.0),
    ("Heading 3", "Heading3", 12.0, false, Some("1F3763"), 2.0, 0.0, DEFAULT_LINE_SPACING, 0.0),
    ("Heading 4", "Heading4", 11.0, true, Some("2F5496"), 2.0, 0.0, DEFAULT_LINE_SPACING, 0.0),
    ("Heading 5", "Heading5", 11.0, false, Some("2F5496"), 2.0, 0.0, DEFAULT_LINE_SPACING, 0.0),
    ("Quote", "Quote", 11.0, true, Some("404040"), 8.0, 8.0, DEFAULT_LINE_SPACING, 0.0),
    ("Caption", "Caption", 9.0, true, Some("404040"), 0.0, 0.0, DEFAULT_LINE_SPACING, 0.0),
    ("No Spacing", "NoSpacing", 11.0, false, None, 0.0, 0.0, 1.0, 0.0),
    ("List Paragraph", "ListParagraph", 11.0, false, None, 0.0, 8.0, DEFAULT_LINE_SPACING, 48.0),
];

fn paragraph_style_label(style: &str) -> &str {
    PARAGRAPH_STYLES
        .iter()
        .find(|(_, id, ..)| *id == style)
        .map(|(label, ..)| *label)
        .unwrap_or(style)
}

fn matching_document_fonts(search: &str) -> impl Iterator<Item = &'static str> + '_ {
    let search = search.trim().to_lowercase();
    crate::theme::DOCUMENT_FAMILIES
        .iter()
        .copied()
        .filter(move |family| family.to_lowercase().contains(&search))
}

#[cfg(test)]
mod font_picker_tests {
    use super::*;

    #[test]
    fn font_search_ignores_case_and_whitespace() {
        let results: Vec<_> = matching_document_fonts("  SANS ").collect();
        assert!(results.contains(&"Open Sans"));
        assert!(results.contains(&"PT Sans"));
        assert!(!results.contains(&"Roboto"));
    }
}

fn icon_button(
    ui: &mut Ui,
    colors: &UiColors,
    icon: IconName,
    width: f32,
    active: bool,
    enabled: bool,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 30.0), Sense::click());
    if active || response.hovered() && enabled {
        ui.painter().rect_filled(
            rect,
            if icon == IconName::Outline { 15 } else { 4 },
            if active {
                colors.app_accent_soft
            } else {
                colors.app_hover
            },
        );
    }
    let color = if !enabled {
        colors.app_text.linear_multiply(0.4)
    } else if active {
        colors.app_accent_ink
    } else {
        colors.app_text
    };
    icons::paint_icon_filled(
        ui.ctx(),
        ui.painter(),
        icon,
        Rect::from_center_size(rect.center(), vec2(18.0, 18.0)),
        color,
    );
    response.on_hover_text(tooltip)
}

/// 30px icon button with a small filled down-caret at the right edge, used by
/// the align and line-spacing triggers in the web toolbar.
fn dropdown_icon_button(
    ui: &mut Ui,
    colors: &UiColors,
    icon: IconName,
    active: bool,
    enabled: bool,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
    if active || response.hovered() && enabled {
        ui.painter().rect_filled(
            rect,
            4,
            if active {
                colors.app_accent_soft
            } else {
                colors.app_hover
            },
        );
    }
    let color = if !enabled {
        colors.app_text.linear_multiply(0.4)
    } else if active {
        colors.app_accent_ink
    } else {
        colors.app_text
    };
    icons::paint_icon_filled(
        ui.ctx(),
        ui.painter(),
        icon,
        Rect::from_center_size(
            pos2(rect.left() + 11.5, rect.center().y),
            vec2(16.0, 16.0),
        ),
        color,
    );
    paint_caret(
        ui,
        if enabled {
            colors.app_muted
        } else {
            colors.app_text.linear_multiply(0.4)
        },
        pos2(rect.right() - 8.0, rect.center().y + 1.0),
        8.0,
        6.0,
    );
    response.on_hover_text(tooltip)
}

/// 50px colorsplit composite: a 30px main icon button with a 16x4 color bar
/// near the bottom, a 2px gap, and an 18px caret zone. Both zones share one
/// click response so either side opens the same color popup.
fn colorsplit_button(
    ui: &mut Ui,
    colors: &UiColors,
    icon: IconName,
    bar: Color32,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(50.0, 30.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(
            Rect::from_min_max(rect.min, pos2(rect.left() + 32.0, rect.bottom())),
            4,
            colors.app_hover,
        );
    }
    icons::paint_icon_filled(
        ui.ctx(),
        ui.painter(),
        icon,
        Rect::from_center_size(
            pos2(rect.left() + 15.0, rect.center().y),
            vec2(18.0, 18.0),
        ),
        colors.app_text,
    );
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(rect.left() + 7.0, rect.bottom() - 7.0), vec2(16.0, 4.0)),
        1,
        bar,
    );
    paint_caret(
        ui,
        colors.app_muted,
        pos2(rect.right() - 9.0, rect.center().y + 1.0),
        8.0,
        5.0,
    );
    response.on_hover_text(tooltip)
}

/// Parse a "#RRGGBB" / "RRGGBB" / 8-digit hex string into a Color32.
fn hex_color(value: &str) -> Option<Color32> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() < 6 {
        return None;
    }
    let rgb = u32::from_str_radix(&hex[hex.len() - 6..], 16).ok()?;
    Some(Color32::from_rgb(
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    ))
}

fn text_button(ui: &mut Ui, colors: &UiColors, label: &str, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 5, colors.app_hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.0),
        colors.app_text,
    );
    response
}

fn text_picker(ui: &mut Ui, colors: &UiColors, label: &str, width: f32, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    ui.painter().rect_filled(
        rect,
        6,
        if response.hovered() {
            colors.app_hover
        } else {
            colors.app_surface
        },
    );
    ui.painter().rect_stroke(
        rect,
        6,
        Stroke::new(1.0_f32, colors.app_border),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .with_clip_rect(rect.shrink2(vec2(10.0, 0.0)))
        .text(
            pos2(rect.left() + 10.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.0),
            colors.app_text,
        );
    draw_chevron(ui, colors, &response);
    response.on_hover_text(tooltip)
}

fn draw_chevron(ui: &Ui, colors: &UiColors, response: &Response) {
    paint_caret(
        ui,
        colors.app_muted,
        pos2(response.rect.right() - 12.0, response.rect.center().y + 1.0),
        7.0,
        5.0,
    );
}

fn paint_caret(ui: &Ui, color: Color32, center: egui::Pos2, width: f32, height: f32) {
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            pos2(center.x - width / 2.0, center.y - height / 2.0),
            pos2(center.x + width / 2.0, center.y - height / 2.0),
            pos2(center.x, center.y + height / 2.0),
        ],
        color,
        Stroke::NONE,
    ));
}

fn separator(ui: &mut Ui, colors: &UiColors) {
    if ui.available_width() < 14.0 {
        return;
    }
    let (rect, _) = ui.allocate_exact_size(vec2(12.0, 23.0), Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.y_range(),
        Stroke::new(1.0_f32, colors.app_border),
    );
}

fn popup(ui: &Ui, key: &str, response: &Response, content: impl FnOnce(&mut Ui)) {
    let id = ui.id().with(key);
    if response.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(id));
    }
    egui::popup_below_widget(
        ui,
        id,
        response,
        egui::PopupCloseBehavior::CloseOnClick,
        content,
    );
}
