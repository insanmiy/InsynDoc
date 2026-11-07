use egui::{Align, RichText};
use serde_json::Value;

use super::{slice_runs, Editor};
use crate::document::{Alignment, PageSetup, Paragraph, Run, TabStop, TextFormat};
use crate::state::AppState;

#[derive(Clone, Copy, PartialEq)]
enum ToolDialog {
    PageSetup,
    Paragraph,
    Symbols,
    DateTime,
    MathText,
    Bookmarks,
}

#[derive(Default)]
pub(super) struct Tools {
    pub paragraph_marks: bool,
    dialog: Option<ToolDialog>,
    page: PageSetup,
    text: String,
    bookmark_name: String,
    pub bookmarks: Vec<(String, usize)>,
    date_index: usize,
    page_paper: usize,
    page_landscape: bool,
    pub show_bookmarks: bool,
    copied_format: Option<TextFormat>,
    painter_armed: bool,
    para_draft: Option<ParagraphDraft>,
}

#[derive(Clone)]
struct ParagraphDraft {
    alignment: String,
    direction: String,
    indent_left: f64,
    indent_right: f64,
    special: String,
    special_amount: f64,
    tab_position: f64,
    tab_alignment: String,
    tab_leader: String,
    tabs: Vec<TabStop>,
    space_before: f64,
    space_after: f64,
    line_rule: String,
    line_value: f64,
    contextual_spacing: bool,
    keep_next: bool,
    keep_lines: bool,
    widow_control: bool,
    page_break_before: bool,
}

impl ParagraphDraft {
    fn from_paragraph(paragraph: Option<&Paragraph>) -> Self {
        let Some(paragraph) = paragraph else {
            return Self::default();
        };
        let alignment = match paragraph.alignment {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
            Alignment::Justify => "justify",
        };
        Self {
            alignment: alignment.into(),
            direction: if paragraph.bidi { "rtl" } else { "ltr" }.into(),
            indent_left: (paragraph.indent / 96.0) as f64,
            indent_right: (paragraph.indent_right / 96.0) as f64,
            special: paragraph
                .special
                .clone()
                .unwrap_or_else(|| "none".into()),
            special_amount: (paragraph.special_amount / 96.0) as f64,
            tab_position: 0.0,
            tab_alignment: "left".into(),
            tab_leader: "none".into(),
            tabs: paragraph.tabs.clone(),
            space_before: paragraph.space_before as f64,
            space_after: paragraph.space_after as f64,
            line_rule: paragraph
                .line_rule
                .clone()
                .unwrap_or_else(|| "multiple".into()),
            line_value: if paragraph.line_rule.is_some() {
                paragraph.line_spacing as f64
            } else {
                (paragraph.line_spacing.max(0.01)) as f64
            },
            contextual_spacing: paragraph.contextual_spacing,
            keep_next: paragraph.keep_next,
            keep_lines: paragraph.keep_lines,
            widow_control: paragraph.widow_control,
            page_break_before: paragraph.page_break_before,
        }
    }
}

impl Default for ParagraphDraft {
    fn default() -> Self {
        Self {
            alignment: "left".into(),
            direction: "ltr".into(),
            indent_left: 0.0,
            indent_right: 0.0,
            special: "none".into(),
            special_amount: 0.0,
            tab_position: 0.0,
            tab_alignment: "left".into(),
            tab_leader: "none".into(),
            tabs: Vec::new(),
            space_before: 0.0,
            space_after: 8.0,
            line_rule: "multiple".into(),
            line_value: 1.08,
            contextual_spacing: false,
            keep_next: false,
            keep_lines: false,
            widow_control: true,
            page_break_before: false,
        }
    }
}

impl Editor {
    pub(super) fn execute_tool(&mut self, name: &str, args: &Value, state: &mut AppState) -> bool {
        let command = if name == "editorExec" {
            args.get("type").and_then(Value::as_str).unwrap_or("")
        } else {
            name
        };
        if matches!(command, "setTextColor" | "setColor" | "setHighlight" | "superscript" | "subscript" | "toggleMark" | "pasteFormatting" | "toggleList" | "toggleBulletList" | "toggleNumberedList" | "changeIndent" | "indent" | "outdent" | "setLineSpacing" | "spaceBefore" | "spaceAfter")
            && !self.can_format_selection(state)
        {
            return false;
        }
        match command {
            "pageSetup" => {
                self.tools.page = self.document.page_setup.clone();
                self.tools.page_landscape =
                    self.tools.page.width > self.tools.page.height;
                self.tools.page_paper = PAPER_PRESETS
                    .iter()
                    .position(|(_, width, height)| {
                        let (width, height) = if self.tools.page_landscape {
                            (*height, *width)
                        } else {
                            (*width, *height)
                        };
                        (self.tools.page.width - width).abs() < 3.0
                            && (self.tools.page.height - height).abs() < 3.0
                    })
                    .unwrap_or(0);
                self.tools.dialog = Some(ToolDialog::PageSetup);
            }
            "paragraphMarks" => self.tools.paragraph_marks = !self.tools.paragraph_marks,
            "showBookmarks" => self.tools.show_bookmarks = !self.tools.show_bookmarks,
            "bookmarks" | "openBookmarkDialog" => self.tools.dialog = Some(ToolDialog::Bookmarks),
            "insertDateTime" => {
                self.tools.date_index = 0;
                self.tools.dialog = Some(ToolDialog::DateTime);
            }
            "insertSymbols" => {
                self.tools.text.clear();
                self.tools.dialog = Some(ToolDialog::Symbols);
            }
            "insertMath" => {
                self.tools.text.clear();
                self.tools.dialog = Some(ToolDialog::MathText);
            }
            "insertBreak" => {
                if args.get("kind").and_then(Value::as_str) != Some("page") {
                    return false;
                }
                self.checkpoint();
                let index = self
                    .selected_paragraph_indices()
                    .last()
                    .copied()
                    .unwrap_or(self.active)
                    .min(self.document.paragraphs.len().saturating_sub(1));
                if let Some(paragraph) = self.document.paragraphs.get_mut(index) {
                    paragraph.page_break_before = true;
                    self.changed(state);
                }
            }
            "openParagraphDialog" => {
                self.tools.para_draft = None;
                self.tools.dialog = Some(ToolDialog::Paragraph);
            }
            "setTextColor" | "setColor" => {
                let color = color_argument(args);
                self.format.color = color.clone();
                self.apply_transform(state, |format| format.color = color.clone());
            }
            "setHighlight" => {
                let color = color_argument(args);
                self.format.highlight = color.clone();
                self.apply_transform(state, |format| format.highlight = color.clone());
            }
            "superscript" | "subscript" | "toggleMark"
                if command != "toggleMark"
                    || matches!(
                        args.get("mark").and_then(Value::as_str),
                        Some("superscript" | "subscript")
                    ) =>
            {
                let mark = if command == "toggleMark" {
                    args.get("mark").and_then(Value::as_str).unwrap()
                } else {
                    command
                };
                let script = if self.format.script.as_deref() == Some(mark) {
                    None
                } else {
                    Some(mark.to_owned())
                };
                self.format.script = script.clone();
                self.apply_transform(state, |format| format.script = script.clone());
            }
            "copyFormatting" => {
                self.tools.copied_format = Some(self.format.clone());
                self.tools.painter_armed = true;
            }
            "pasteFormatting" => {
                if let Some(format) = self.tools.copied_format.clone() {
                    self.format = format;
                    self.apply_format(state);
                }
            }
            "toggleList" | "toggleBulletList" | "toggleNumberedList" => {
                let kind = if command == "toggleBulletList" {
                    "bullet"
                } else if command == "toggleNumberedList" {
                    "number"
                } else {
                    args.get("kind").and_then(Value::as_str).unwrap_or("bullet")
                };
                let selected = self.selected_paragraph_indices();
                self.checkpoint();
                if self.document_selected || self.cross_selection.is_some() {
                    let enabled = selected.iter().any(|&index| self.document.paragraphs[index].list.as_deref() != Some(kind));
                    for index in selected {
                        self.document.paragraphs[index].list = enabled.then(|| kind.to_owned());
                    }
                    self.changed(state);
                    return true;
                }
                if let Some(paragraph) = self.document.paragraphs.get_mut(self.active) {
                    paragraph.list = if paragraph.list.as_deref() == Some(kind) {
                        None
                    } else {
                        Some(kind.to_owned())
                    };
                    self.changed(state);
                }
            }
            "changeIndent" | "indent" | "outdent" => {
                let delta = if command == "outdent" {
                    -48.0
                } else if command == "indent" {
                    48.0
                } else {
                    args.get("delta").and_then(Value::as_f64).unwrap_or(1.0) as f32 * 48.0
                };
                let selected = self.selected_paragraph_indices();
                self.checkpoint();
                if self.document_selected || self.cross_selection.is_some() {
                    for index in selected {
                        let paragraph = &mut self.document.paragraphs[index];
                        paragraph.indent = (paragraph.indent + delta).clamp(0.0, 384.0);
                    }
                    self.changed(state);
                    return true;
                }
                if let Some(paragraph) = self.document.paragraphs.get_mut(self.active) {
                    paragraph.indent = (paragraph.indent + delta).clamp(0.0, 384.0);
                    self.changed(state);
                }
            }
            "setLineSpacing" => {
                let value = args.get("value").and_then(Value::as_f64).unwrap_or(1.15) as f32;
                if !value.is_finite() || !(0.5..=5.0).contains(&value) {
                    return false;
                }
                let selected = self.selected_paragraph_indices();
                self.checkpoint();
                if self.document_selected || self.cross_selection.is_some() {
                    for index in selected {
                        self.document.paragraphs[index].line_spacing = value;
                    }
                    self.changed(state);
                    return true;
                }
                if let Some(paragraph) = self.document.paragraphs.get_mut(self.active) {
                    paragraph.line_spacing = value;
                    self.changed(state);
                }
            }
            "spaceBefore" | "spaceAfter" => {
                let before = command == "spaceBefore";
                let selected = self.selected_paragraph_indices();
                self.checkpoint();
                let toggle = |paragraph: &mut crate::document::Paragraph| {
                    let spacing = if before {
                        &mut paragraph.space_before
                    } else {
                        &mut paragraph.space_after
                    };
                    *spacing = if *spacing > 0.0 { 0.0 } else { 8.0 };
                };
                if self.document_selected || self.cross_selection.is_some() {
                    for index in selected {
                        toggle(&mut self.document.paragraphs[index]);
                    }
                    self.changed(state);
                    return true;
                }
                if let Some(paragraph) = self.document.paragraphs.get_mut(self.active) {
                    toggle(paragraph);
                    self.changed(state);
                }
            }
            _ => return false,
        }
        true
    }

    pub(super) fn paint_selection_if_armed(&mut self, state: &mut AppState) {
        if !self.tools.painter_armed || self.selection.is_empty() {
            return;
        }
        if !self.can_format_selection(state) {
            return;
        }
        self.tools.painter_armed = false;
        if let Some(format) = self.tools.copied_format.clone() {
            self.format = format;
            self.apply_format(state);
        }
    }

    pub(super) fn show_tools(&mut self, ctx: &egui::Context, state: &mut AppState) {
        let Some(dialog) = self.tools.dialog else {
            return;
        };
        let title = match dialog {
            ToolDialog::PageSetup => "Page Setup",
            ToolDialog::Paragraph => "Paragraph",
            ToolDialog::Symbols => "Special Characters",
            ToolDialog::DateTime => "Date And Time",
            ToolDialog::MathText => "Insert Math Text",
            ToolDialog::Bookmarks => "Document Bookmarks",
        };
        let mut open = true;
        let mut close = false;
        egui::Window::new(title)
            .id(egui::Id::new("native-writing-tool"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(410.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                match dialog {
                    ToolDialog::PageSetup => {
                        ui.label(
                            RichText::new("PAGE SIZE").weak().small(),
                        );
                        let mut picked_paper = None;
                        ui.horizontal(|ui| {
                            ui.label("Size");
                            ui.with_layout(
                                egui::Layout::right_to_left(Align::Center),
                                |ui| {
                                    egui::ComboBox::from_id_salt("page-size")
                                        .selected_text(PAPER_PRESETS[self.tools.page_paper].0)
                                        .show_ui(ui, |ui| {
                                            for (index, (name, _, _)) in
                                                PAPER_PRESETS.iter().enumerate()
                                            {
                                                if ui
                                                    .selectable_label(
                                                        self.tools.page_paper == index,
                                                        *name,
                                                    )
                                                    .clicked()
                                                {
                                                    picked_paper = Some(index);
                                                }
                                            }
                                        });
                                },
                            );
                        });
                        if let Some(index) = picked_paper {
                            self.tools.page_paper = index;
                            let (_, width, height) = PAPER_PRESETS[index];
                            let (width, height) = if self.tools.page_landscape {
                                (height, width)
                            } else {
                                (width, height)
                            };
                            self.tools.page.width = width;
                            self.tools.page.height = height;
                        }
                        let mut picked_orientation = None;
                        ui.horizontal(|ui| {
                            ui.label("Orientation");
                            ui.with_layout(
                                egui::Layout::right_to_left(Align::Center),
                                |ui| {
                                    egui::ComboBox::from_id_salt("page-orientation")
                                        .selected_text(if self.tools.page_landscape {
                                            "Landscape"
                                        } else {
                                            "Portrait"
                                        })
                                        .show_ui(ui, |ui| {
                                            for (landscape, name) in
                                                [(false, "Portrait"), (true, "Landscape")]
                                            {
                                                if ui
                                                    .selectable_label(
                                                        self.tools.page_landscape == landscape,
                                                        name,
                                                    )
                                                    .clicked()
                                                {
                                                    picked_orientation = Some(landscape);
                                                }
                                            }
                                        });
                                },
                            );
                        });
                        if let Some(landscape) = picked_orientation {
                            self.tools.page_landscape = landscape;
                            let (_, width, height) = PAPER_PRESETS[self.tools.page_paper];
                            let (width, height) = if landscape {
                                (height, width)
                            } else {
                                (width, height)
                            };
                            self.tools.page.width = width;
                            self.tools.page.height = height;
                        }
                        ui.add_space(4.0);
                        ui.label(RichText::new("MARGINS").weak().small());
                        for (label, value) in [
                            ("Top", &mut self.tools.page.margin_top),
                            ("Bottom", &mut self.tools.page.margin_bottom),
                            ("Left", &mut self.tools.page.margin_left),
                            ("Right", &mut self.tools.page.margin_right),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(label);
                                ui.with_layout(
                                    egui::Layout::right_to_left(Align::Center),
                                    |ui| {
                                        let mut inches = *value / 96.0;
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut inches)
                                                    .speed(0.05)
                                                    .range(0.0..=22.0)
                                                    .max_decimals(2),
                                            )
                                            .changed()
                                        {
                                            *value = inches * 96.0;
                                        }
                                        ui.label("in");
                                    },
                                );
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.label("Apply To");
                            ui.with_layout(
                                egui::Layout::right_to_left(Align::Center),
                                |ui| {
                                    egui::ComboBox::from_id_salt("page-scope")
                                        .selected_text("Whole Document")
                                        .show_ui(ui, |ui| {
                                            ui.selectable_label(true, "Whole Document");
                                        });
                                },
                            );
                        });
                        let page = &self.tools.page;
                        let valid = page.width >= page.margin_left + page.margin_right + 96.0
                            && page.height >= page.margin_top + page.margin_bottom + 96.0;
                        if !valid {
                            ui.label(
                                RichText::new("Leave At Least One Inch Of Editable Space.")
                                    .color(ui.visuals().error_fg_color),
                            );
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                            if ui
                                .add_enabled(valid, egui::Button::new("Apply"))
                                .clicked()
                            {
                                self.checkpoint();
                                self.document.page_setup = self.tools.page.clone();
                                self.changed(state);
                                close = true;
                            }
                        });
                    }
                    ToolDialog::Paragraph => {
                        if self.tools.para_draft.is_none() {
                            self.tools.para_draft = Some(ParagraphDraft::from_paragraph(
                                self.document.paragraphs.get(self.active),
                            ));
                        }
                        let mut confirmed = false;
                        let mut cancelled = false;
                        {
                            let draft = self.tools.para_draft.as_mut().unwrap();
                            ui.columns(2, |columns| {
                                columns[0].label(RichText::new("General").weak().small());
                                field_row(&mut columns[0], "Alignment", |ui| {
                                    combo(ui, "paragraph-align", &mut draft.alignment, &[
                                        ("left", "Left"),
                                        ("center", "Centered"),
                                        ("right", "Right"),
                                        ("justify", "Justified"),
                                    ]);
                                });
                                field_row(&mut columns[0], "Direction", |ui| {
                                    combo(ui, "paragraph-direction", &mut draft.direction, &[
                                        ("rtl", "Right-to-left"),
                                        ("ltr", "Left-to-right"),
                                    ]);
                                });
                                columns[0]
                                    .label(RichText::new("Indentation").weak().small());
                                field_row(&mut columns[0], "Before text", |ui| {
                                    unit_drag(ui, &mut draft.indent_left, "in");
                                });
                                field_row(&mut columns[0], "After text", |ui| {
                                    unit_drag(ui, &mut draft.indent_right, "in");
                                });
                                field_row(&mut columns[0], "Special", |ui| {
                                    combo(ui, "paragraph-special", &mut draft.special, &[
                                        ("none", "(none)"),
                                        ("firstLine", "First line"),
                                        ("hanging", "Hanging"),
                                    ]);
                                });
                                columns[0]
                                    .label(RichText::new("Tab stops").weak().small());
                                if draft.tabs.is_empty() {
                                    columns[0].label(RichText::new("No tab stops set.").weak().small());
                                } else {
                                    let mut remove_tab = None;
                                    for index in 0..draft.tabs.len() {
                                        let tab = draft.tabs[index].clone();
                                        columns[0].horizontal(|ui| {
                                            ui.label(format!(
                                                "{} in {} {}",
                                                tab.position / 96.0,
                                                tab.alignment,
                                                tab.leader
                                            ));
                                            if ui.small_button("Remove").clicked() {
                                                remove_tab = Some(index);
                                            }
                                        });
                                    }
                                    if let Some(index) = remove_tab {
                                        draft.tabs.remove(index);
                                    }
                                }
                                field_row(&mut columns[0], "Position", |ui| {
                                    unit_drag(ui, &mut draft.tab_position, "in");
                                });
                                field_row(&mut columns[0], "Alignment", |ui| {
                                    combo(ui, "tab-align", &mut draft.tab_alignment, &[
                                        ("left", "Left"),
                                        ("center", "Center"),
                                        ("right", "Right"),
                                        ("decimal", "Decimal"),
                                    ]);
                                });
                                field_row(&mut columns[0], "Leader", |ui| {
                                    combo(ui, "tab-leader", &mut draft.tab_leader, &[
                                        ("none", "None"),
                                        ("dot", "Dotted"),
                                        ("hyphen", "Dashed"),
                                        ("underscore", "Underlined"),
                                    ]);
                                });
                                columns[0].horizontal(|ui| {
                                    if ui.button("Set").clicked() {
                                        draft.tabs.retain(|tab| {
                                            (tab.position as f64 - draft.tab_position * 96.0).abs() > 0.5
                                        });
                                        draft.tabs.push(TabStop {
                                            position: (draft.tab_position * 96.0) as f32,
                                            alignment: draft.tab_alignment.clone(),
                                            leader: draft.tab_leader.clone(),
                                        });
                                        draft.tabs.sort_by(|a, b| {
                                            a.position.partial_cmp(&b.position).unwrap()
                                        });
                                    }
                                    if ui.button("Clear all").clicked() {
                                        draft.tabs.clear();
                                    }
                                });

                                columns[1].label(RichText::new("Spacing").weak().small());
                                field_row(&mut columns[1], "Before", |ui| {
                                    unit_drag(ui, &mut draft.space_before, "pt");
                                });
                                field_row(&mut columns[1], "After", |ui| {
                                    unit_drag(ui, &mut draft.space_after, "pt");
                                });
                                field_row(&mut columns[1], "Line spacing", |ui| {
                                    combo(ui, "line-rule", &mut draft.line_rule, &[
                                        ("multiple", "Multiple"),
                                        ("atLeast", "At least"),
                                        ("exact", "Exactly"),
                                    ]);
                                });
                                field_row(&mut columns[1], "At", |ui| {
                                    unit_drag(ui, &mut draft.line_value, "");
                                });
                                columns[1].checkbox(
                                    &mut draft.contextual_spacing,
                                    "Don't add space between paragraphs of the same style",
                                );
                                columns[1]
                                    .label(RichText::new("Pagination").weak().small());
                                columns[1].checkbox(&mut draft.keep_next, "Keep with next");
                                columns[1].checkbox(&mut draft.widow_control, "Widow/Orphan control");
                                columns[1].checkbox(&mut draft.keep_lines, "Keep lines together");
                                columns[1]
                                    .checkbox(&mut draft.page_break_before, "Page break before");
                            });
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                cancelled = true;
                            }
                            if ui.button("OK").clicked() {
                                confirmed = true;
                            }
                        });
                        if confirmed {
                            let draft = self.tools.para_draft.take().unwrap();
                            self.apply_paragraph_draft(&draft, state);
                            close = true;
                        } else if cancelled {
                            self.tools.para_draft = None;
                            close = true;
                        }
                    }
                    ToolDialog::Symbols => {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tools.text)
                                .hint_text("Search By Name Or Character")
                                .desired_width(390.0),
                        );
                        let query = self.tools.text.trim().to_lowercase();
                        egui::Grid::new("native-symbol-grid")
                            .spacing([10.0, 10.0])
                            .show(ui, |ui| {
                                let mut column = 0;
                                for (symbol, name) in SYMBOLS {
                                    if !query.is_empty()
                                        && !format!("{symbol} {name}")
                                            .to_lowercase()
                                            .contains(&query)
                                    {
                                        continue;
                                    }
                                    if ui.button(*symbol).on_hover_text(*name).clicked() {
                                        self.insert_tool_text(symbol, state);
                                        close = true;
                                    }
                                    column += 1;
                                    if column % 6 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                        if !query.is_empty()
                            && SYMBOLS.iter().all(|(symbol, name)| {
                                !format!("{symbol} {name}")
                                    .to_lowercase()
                                    .contains(&query)
                            })
                        {
                            ui.label("No Matching Characters.");
                        }
                    }
                    ToolDialog::DateTime => {
                        ui.label("Insert A Fixed Date Or Time At Your Cursor.");
                        let formats = local_date_formats();
                        for (index, format) in formats.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    self.tools.date_index == index,
                                    format.as_str(),
                                )
                                .clicked()
                            {
                                self.tools.date_index = index;
                            }
                        }
                        if ui.button("Insert Date And Time").clicked() {
                            let text = formats
                                [self.tools.date_index.min(formats.len() - 1)]
                                .clone();
                            self.insert_tool_text(&text, state);
                            close = true;
                        }
                    }
                    ToolDialog::MathText => {
                        ui.label("Enter Mathematical Symbols As Document Text.");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tools.text).desired_width(390.0),
                        );
                        if ui
                            .add_enabled(!self.tools.text.is_empty(), egui::Button::new("Insert"))
                            .clicked()
                        {
                            let text = self.tools.text.clone();
                            self.insert_tool_text(&text, state);
                            close = true;
                        }
                    }
                    ToolDialog::Bookmarks => {
                        ui.label("Bookmarks For This Editing Session");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.tools.bookmark_name)
                                    .hint_text("Bookmark Name")
                                    .desired_width(260.0),
                            );
                            if ui
                                .add_enabled(
                                    !self.tools.bookmark_name.trim().is_empty(),
                                    egui::Button::new("Add"),
                                )
                                .clicked()
                            {
                                let name = self.tools.bookmark_name.trim().to_owned();
                                self.tools
                                    .bookmarks
                                    .retain(|(existing, _)| existing != &name);
                                self.tools.bookmarks.push((name, self.active));
                                self.tools.bookmark_name.clear();
                            }
                        });
                        for (name, paragraph) in self.tools.bookmarks.clone() {
                            if ui.button(name).clicked() {
                                let paragraph =
                                    paragraph.min(self.document.paragraphs.len().saturating_sub(1));
                                self.active = paragraph;
                                self.document_selected = false;
                                self.cross_selection = None;
                                self.scroll_paragraph = Some(paragraph);
                                self.pending_selection = Some((paragraph, 0..0));
                                close = true;
                            }
                        }
                    }
                }
            });
        if !open || close {
            self.tools.dialog = None;
        }
    }

    fn insert_tool_text(&mut self, text: &str, state: &mut AppState) {
        if self.document_selected || self.cross_selection.is_some() {
            self.replace_document_selection(text, state);
            return;
        }
        if self.document.paragraphs.get(self.active).is_none() {
            return;
        }
        self.checkpoint();
        let paragraph = &mut self.document.paragraphs[self.active];
        let content = paragraph.plain_text();
        let start = self.selection.start.min(content.chars().count());
        let end = self.selection.end.min(content.chars().count());
        let mut runs = slice_runs(&paragraph.runs, 0..start);
        runs.push(Run {
            text: text.into(),
            format: self.format.clone(),
        });
        runs.extend(slice_runs(&paragraph.runs, end..usize::MAX));
        paragraph.runs = runs;
        let cursor = start + text.chars().count();
        self.selection = cursor..cursor;
        self.pending_selection = Some((self.active, cursor..cursor));
        self.changed(state);
    }

    fn apply_paragraph_draft(&mut self, draft: &ParagraphDraft, state: &mut AppState) {
        self.checkpoint();
        let mut indices = self.selected_paragraph_indices();
        if indices.is_empty() {
            indices.push(self.active.min(self.document.paragraphs.len().saturating_sub(1)));
        }
        for index in indices {
            let Some(paragraph) = self.document.paragraphs.get_mut(index) else {
                continue;
            };
            paragraph.alignment = match draft.alignment.as_str() {
                "center" => Alignment::Center,
                "right" => Alignment::Right,
                "justify" => Alignment::Justify,
                _ => Alignment::Left,
            };
            paragraph.bidi = draft.direction == "rtl";
            paragraph.indent = (draft.indent_left.max(0.0) * 96.0) as f32;
            paragraph.indent_right = (draft.indent_right.max(0.0) * 96.0) as f32;
            paragraph.special = (draft.special != "none").then(|| draft.special.clone());
            paragraph.special_amount = (draft.special_amount.max(0.0) * 96.0) as f32;
            paragraph.space_before = draft.space_before.max(0.0) as f32;
            paragraph.space_after = draft.space_after.max(0.0) as f32;
            paragraph.line_rule =
                (draft.line_rule != "multiple").then(|| draft.line_rule.clone());
            paragraph.line_spacing = draft.line_value.max(0.01) as f32;
            paragraph.contextual_spacing = draft.contextual_spacing;
            paragraph.keep_next = draft.keep_next;
            paragraph.keep_lines = draft.keep_lines;
            paragraph.widow_control = draft.widow_control;
            paragraph.page_break_before = draft.page_break_before;
            paragraph.tabs = draft.tabs.clone();
        }
        self.changed(state);
    }
}

fn field_row(ui: &mut egui::Ui, label: &str, content: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            egui::vec2(90.0, 24.0),
            egui::Label::new(RichText::new(label)),
        );
        content(ui);
    });
}

fn combo(ui: &mut egui::Ui, id: &str, value: &mut String, options: &[(&str, &str)]) {
    let selected = options
        .iter()
        .find(|(key, _)| *key == value.as_str())
        .map(|(_, label)| *label)
        .unwrap_or(options[0].1);
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (key, label) in options {
                if ui.selectable_label(value.as_str() == *key, *label).clicked() {
                    *value = (*key).to_string();
                }
            }
        });
}

fn unit_drag(ui: &mut egui::Ui, value: &mut f64, unit: &str) {
    ui.add(egui::DragValue::new(value).speed(0.05).range(0.0..=999.0));
    if !unit.is_empty() {
        ui.label(unit);
    }
}

fn color_argument(args: &Value) -> Option<String> {
    args.get("color")
        .and_then(Value::as_str)
        .map(|color| color.trim_start_matches('#'))
        .filter(|color| color.len() == 6 && color.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(str::to_uppercase)
}

const PAPER_PRESETS: &[(&str, f32, f32)] = &[
    ("Letter (8.5\" × 11\")", 816.0, 1056.0),
    ("A4 (8.27\" × 11.69\")", 793.92, 1122.24),
    ("Legal (8.5\" × 14\")", 816.0, 1344.0),
    ("A3 (11.69\" × 16.54\")", 1122.24, 1587.84),
    ("A5 (5.83\" × 8.27\")", 559.68, 793.92),
    ("B5 (6.93\" × 9.84\")", 665.28, 944.64),
    ("Executive (7.25\" × 10.5\")", 696.0, 1008.0),
];

const SYMBOLS: &[(&str, &str)] = &[
    ("©", "Copyright"),
    ("®", "Registered Trademark"),
    ("™", "Trademark"),
    ("§", "Section"),
    ("¶", "Paragraph"),
    ("•", "Bullet"),
    ("…", "Ellipsis"),
    ("–", "En Dash"),
    ("—", "Em Dash"),
    ("“", "Opening Double Quote"),
    ("”", "Closing Double Quote"),
    ("‘", "Opening Single Quote"),
    ("’", "Closing Single Quote"),
    ("€", "Euro"),
    ("£", "Pound"),
    ("¥", "Yen"),
    ("¢", "Cent"),
    ("±", "Plus Or Minus"),
    ("×", "Multiplication"),
    ("÷", "Division"),
    ("≠", "Not Equal"),
    ("≤", "Less Than Or Equal"),
    ("≥", "Greater Than Or Equal"),
    ("∞", "Infinity"),
    ("√", "Square Root"),
    ("π", "Pi"),
    ("°", "Degree"),
    ("←", "Left Arrow"),
    ("→", "Right Arrow"),
    ("↑", "Up Arrow"),
    ("↓", "Down Arrow"),
    ("✓", "Check Mark"),
];

fn local_date_formats() -> Vec<String> {
    use chrono::Datelike;
    let now = chrono::Local::now();
    let (year, month, day) = (now.year(), now.month(), now.day());
    let month_name = now.format("%B").to_string();
    let long_date = format!("{month_name} {day}, {year}");
    let short_date = format!("{month}/{day}/{year}");
    let iso_date = format!("{year:04}-{month:02}-{day:02}");
    let time = now.format("%I:%M %p").to_string();
    let long = format!("{long_date}, {time}");
    vec![long_date, short_date, iso_date, time, long]
}

fn utc_date_time(seconds: u64) -> String {
    let days = (seconds / 86400) as i64 + 719468;
    let era = days / 146097;
    let day_of_era = days - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = month_index + if month_index < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        (seconds / 3600) % 24,
        (seconds / 60) % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_date_conversion_handles_epoch_and_leap_day() {
        assert_eq!(utc_date_time(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc_date_time(1709208000), "2024-02-29 12:00 UTC");
    }

    #[test]
    fn text_tools_preserve_surrounding_runs_and_pending_format() {
        let mut editor = Editor::default();
        editor.document.replace_plain_text("Before After");
        editor.selection = 7..7;
        editor.format.bold = true;
        let mut state = AppState::default();
        editor.insert_tool_text("π ", &mut state);
        assert_eq!(editor.document.plain_text(), "Before π After");
        assert!(editor.document.paragraphs[0].runs[1].format.bold);
        assert!(state.surface.dirty);
    }

    #[test]
    fn toolbar_commands_apply_real_paragraph_and_script_properties() {
        let mut editor = Editor::default();
        let mut state = AppState::default();
        assert!(editor.execute_tool("changeIndent", &serde_json::json!({"delta":1}), &mut state));
        assert_eq!(editor.document.paragraphs[0].indent, 48.0);
        assert!(editor.execute_tool("changeIndent", &serde_json::json!({"delta":-1}), &mut state));
        assert_eq!(editor.document.paragraphs[0].indent, 0.0);
        assert!(editor.execute_tool(
            "toggleList",
            &serde_json::json!({"kind":"bullet"}),
            &mut state
        ));
        assert_eq!(
            editor.document.paragraphs[0].list.as_deref(),
            Some("bullet")
        );
        assert!(editor.execute_tool(
            "editorExec",
            &serde_json::json!({"type":"toggleMark","mark":"superscript"}),
            &mut state
        ));
        assert_eq!(editor.format.script.as_deref(), Some("superscript"));
        assert!(editor.execute_tool("superscript", &Value::Null, &mut state));
        assert_eq!(editor.format.script, None);
    }

    #[test]
    fn formatting_painter_applies_once_after_a_new_selection() {
        let mut editor = Editor::default();
        editor.document.replace_plain_text("One Two");
        let mut state = AppState::default();
        editor.format.bold = true;
        editor.execute_tool("copyFormatting", &Value::Null, &mut state);
        editor.format.bold = false;
        editor.selection = 4..7;
        editor.paint_selection_if_armed(&mut state);
        assert!(
            editor.document.paragraphs[0]
                .runs
                .last()
                .unwrap()
                .format
                .bold
        );
        assert!(!editor.tools.painter_armed);
        assert!(state.surface.dirty);
    }
}
