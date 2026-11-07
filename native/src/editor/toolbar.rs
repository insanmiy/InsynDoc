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
            let label = match style.as_str() {
                "Heading1" => "Heading 1",
                "Heading2" => "Heading 2",
                "Heading3" => "Heading 3",
                _ => style.as_str(),
            };
            let response = text_picker(&mut row, colors, label, 128.0, "Paragraph Style");
            let mut picked = None;
            popup(&row, "toolbar-style", &response, |ui| {
                for (label, style, size) in [
                    ("Normal", "Normal", 11.0),
                    ("Title", "Title", 28.0),
                    ("Heading 1", "Heading1", 20.0),
                    ("Heading 2", "Heading2", 16.0),
                    ("Heading 3", "Heading3", 14.0),
                ] {
                    if ui.selectable_label(false, label).clicked() {
                        picked = Some((style, size));
                    }
                }
            });
            if let Some((style, size)) = picked {
                self.checkpoint();
                if let Some(paragraph) = self.document.paragraphs.get_mut(self.active) {
                    paragraph.style = Some(style.into());
                    for run in &mut paragraph.runs {
                        run.format.font_size = Some(size);
                        run.format.bold = style.starts_with("Heading");
                    }
                }
                self.format.font_size = Some(size);
                self.format.bold = style.starts_with("Heading");
                self.changed(state);
            }
        }
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
            if text_button(&mut row, colors, "-", 22.0).clicked() {
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
            let response = row
                .scope(|ui| {
                    ui.visuals_mut().widgets.inactive.corner_radius = egui::CornerRadius::same(6);
                    ui.visuals_mut().widgets.hovered.corner_radius = egui::CornerRadius::same(6);
                    ui.visuals_mut().widgets.active.corner_radius = egui::CornerRadius::same(6);
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
        if row.available_width() >= 80.0 {
            for (icon, label, is_highlight, underline) in [
                (
                    IconName::TextColor,
                    "Text Color",
                    false,
                    Color32::from_rgb(239, 68, 68),
                ),
                (
                    IconName::Highlight,
                    "Highlight",
                    true,
                    Color32::from_rgb(250, 204, 21),
                ),
            ] {
                let response = icon_button(&mut row, colors, icon, 38.0, false, true, label);
                let bar = Rect::from_min_size(
                    pos2(response.rect.left() + 8.0, response.rect.bottom() - 5.0),
                    vec2(16.0, 3.0),
                );
                row.painter().rect_filled(bar, 0, underline);
                icons::paint_icon_at(
                    row.ctx(),
                    row.painter(),
                    IconName::ChevronDown,
                    Rect::from_center_size(
                        pos2(response.rect.right() - 5.0, response.rect.center().y),
                        vec2(7.0, 7.0),
                    ),
                    colors.app_muted,
                );
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
        if row.available_width() >= 44.0 {
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
            let response = icon_button(&mut row, colors, icon, 40.0, false, true, "Alignment");
            draw_chevron(&row, colors, &response);
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
        if row.available_width() >= 42.0 {
            let response = icon_button(
                &mut row,
                colors,
                IconName::LineSpacing,
                40.0,
                false,
                true,
                "Line Spacing",
            );
            draw_chevron(&row, colors, &response);
            let mut picked = None;
            popup(&row, "toolbar-spacing", &response, |ui| {
                for value in [1.0, 1.15, 1.5, 2.0] {
                    if ui.button(value.to_string()).clicked() {
                        picked = Some(value);
                    }
                }
            });
            if let Some(value) = picked {
                self.toolbar_command("setLineSpacing", json!({"value":value}), state);
            }
        }
        for (icon, label, command, args) in [
            (
                IconName::BulletList,
                "Bulleted List",
                "toggleList",
                json!({"kind":"bullet"}),
            ),
            (
                IconName::NumberedList,
                "Numbered List",
                "toggleList",
                json!({"kind":"number"}),
            ),
            (
                IconName::Outdent,
                "Decrease Indent",
                "changeIndent",
                json!({"delta":-1}),
            ),
            (
                IconName::Indent,
                "Increase Indent",
                "changeIndent",
                json!({"delta":1}),
            ),
            (
                IconName::ClearFormatting,
                "Clear Formatting",
                "clearFormatting",
                Value::Null,
            ),
            (IconName::Link, "Insert Link", "insertLink", Value::Null),
            (IconName::Image, "Insert Image", "insertImage", Value::Null),
            (IconName::Table, "Insert Table", "insertTable", Value::Null),
        ] {
            if row.available_width() < 32.0 {
                overflow.push((label, command, args));
                continue;
            }
            if matches!(icon, IconName::Link) {
                separator(&mut row, colors);
            }
            if icon_button(&mut row, colors, icon, 30.0, false, true, label).clicked() {
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
                    pos2(separator_x + 14.0, content.top()),
                    content.max,
                ))
                .layout(Layout::left_to_right(Align::Center)),
        );
        end.spacing_mut().item_spacing = vec2(10.0, 0.0);
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
        colors.app_text.gamma_multiply(0.4)
    } else if active {
        colors.app_accent_ink
    } else {
        colors.app_text
    };
    icons::paint_icon_at(
        ui.ctx(),
        ui.painter(),
        icon,
        Rect::from_center_size(rect.center(), vec2(17.0, 17.0)),
        color,
    );
    response.on_hover_text(tooltip)
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
    icons::paint_icon_at(
        ui.ctx(),
        ui.painter(),
        IconName::ChevronDown,
        Rect::from_center_size(
            pos2(response.rect.right() - 10.0, response.rect.center().y),
            vec2(8.0, 8.0),
        ),
        colors.app_muted,
    );
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
