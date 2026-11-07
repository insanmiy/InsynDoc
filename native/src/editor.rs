mod find;
mod menu_search;
mod outline;
mod toolbar;
mod tools;

use std::ops::Range;

use egui::{vec2, Align, Color32, FontId, Frame, Margin, RichText, Stroke, TextEdit};
use serde_json::Value;

use crate::document::{Alignment, Document, Paragraph, Run, TextFormat};
use crate::state::{AppState, Modal, TextStatistics, ViewMode};
use crate::theme::{Theme, UiColors};

pub struct Editor {
    pub document: Document,
    undo: Vec<EditorSnapshot>,
    redo: Vec<EditorSnapshot>,
    active: usize,
    selection: Range<usize>,
    document_selected: bool,
    cross_selection: Option<DocumentSelection>,
    drag_anchor: Option<DocumentPosition>,
    drag_best: Option<(f32, usize, usize)>,
    format: TextFormat,
    find: String,
    replacement: String,
    replace_open: bool,
    match_case: bool,
    whole_words: bool,
    find_needs_focus: bool,
    pending_selection: Option<(usize, Range<usize>)>,
    scroll_paragraph: Option<usize>,
    page_count: u32,
    active_id: Option<egui::Id>,
    tools: tools::Tools,
    search_open: bool,
    search_query: String,
    search_needs_focus: bool,
    search_selected: usize,
    horizontal_scroll: f32,
    viewport_width: f32,
    pub queued_commands: Vec<crate::ui::UiCommand>,
}

struct EditorSnapshot {
    document: Document,
    active: usize,
    selection: Range<usize>,
    document_selected: bool,
    cross_selection: Option<DocumentSelection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DocumentPosition {
    paragraph: usize,
    offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DocumentSelection {
    anchor: DocumentPosition,
    head: DocumentPosition,
}

impl DocumentSelection {
    fn bounds(self) -> (DocumentPosition, DocumentPosition) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            document: Document::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            active: 0,
            selection: 0..0,
            document_selected: false,
            cross_selection: None,
            drag_anchor: None,
            drag_best: None,
            format: TextFormat::default(),
            find: String::new(),
            replacement: String::new(),
            replace_open: false,
            match_case: false,
            whole_words: false,
            find_needs_focus: false,
            pending_selection: Some((0, 0..0)),
            scroll_paragraph: None,
            page_count: 1,
            active_id: None,
            tools: tools::Tools::default(),
            search_open: false,
            search_query: String::new(),
            search_needs_focus: false,
            search_selected: 0,
            horizontal_scroll: 0.0,
            viewport_width: 1050.0,
            queued_commands: Vec::new(),
        }
    }
}

impl Editor {
    pub fn reset_history(&mut self) {
        self.tools = tools::Tools::default();
        self.search_open = false;
        self.horizontal_scroll = 0.0;
        self.undo.clear();
        self.redo.clear();
        self.active = 0;
        self.selection = 0..0;
        self.document_selected = false;
        self.cross_selection = None;
        self.drag_anchor = None;
        self.drag_best = None;
        self.format = TextFormat::default();
        self.pending_selection = Some((0, 0..0));
        self.scroll_paragraph = Some(0);
        self.active_id = None;
    }

    pub fn sync_state(&self, state: &mut AppState) {
        let text = self.document.plain_text();
        state.surface.ready = true;
        state.surface.editor_ready = true;
        state.surface.busy = false;
        state.surface.page_count = self.page_count.max(1);
        let statistics = crate::backend::statistics::count_text(&text);
        state.surface.statistics = TextStatistics {
            words: statistics.words as u64,
            characters: statistics.characters as u64,
            characters_without_spaces: statistics.characters_without_spaces as u64,
        };
        state.can_undo = !self.undo.is_empty();
        state.can_redo = !self.redo.is_empty();
        state.show_bookmarks = self.tools.show_bookmarks;
        state.show_paragraph_breaks = self.tools.paragraph_marks;
        if !state.surface.zoom.is_finite() || state.surface.zoom <= 0.0 {
            state.surface.zoom = 1.0;
        }
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            document: self.document.clone(),
            active: self.active,
            selection: self.selection.clone(),
            document_selected: self.document_selected,
            cross_selection: self.cross_selection,
        }
    }

    fn restore_snapshot(&mut self, snapshot: EditorSnapshot) {
        self.document = snapshot.document;
        self.active = snapshot.active.min(self.document.paragraphs.len().saturating_sub(1));
        let length = self.document.paragraphs.get(self.active)
            .map(|paragraph| paragraph.plain_text().chars().count()).unwrap_or(0);
        self.selection = snapshot.selection.start.min(length)..snapshot.selection.end.min(length);
        self.document_selected = snapshot.document_selected;
        self.cross_selection = snapshot.cross_selection;
        self.pending_selection = Some((self.active, self.selection.clone()));
        self.scroll_paragraph = Some(self.active);
    }

    fn changed(&self, state: &mut AppState) {
        state.surface.dirty = true;
        state.surface.message = "Unsaved Changes".into();
    }

    fn can_format_selection(&self, state: &mut AppState) -> bool {
        if self.document_selected || self.cross_selection.is_some() {
            if let Err(error) = self.document.can_replace_all_content() {
                state.surface.error = Some(error);
                return false;
            }
        }
        true
    }

    fn selected_range_in(&self, index: usize) -> Option<Range<usize>> {
        if self.document_selected {
            return self.document.paragraphs.get(index).map(|paragraph| 0..paragraph.plain_text().chars().count());
        }
        let (start, end) = self.cross_selection?.bounds();
        if index < start.paragraph || index > end.paragraph {
            return None;
        }
        let length = self.document.paragraphs.get(index)?.plain_text().chars().count();
        Some(if index == start.paragraph { start.offset } else { 0 }..
            if index == end.paragraph { end.offset } else { length })
    }

    fn selected_paragraph_indices(&self) -> Vec<usize> {
        (0..self.document.paragraphs.len()).filter(|&index| {
            self.selected_range_in(index).is_some_and(|range| {
                !range.is_empty() || self.document_selected || self.cross_selection.is_some_and(|selection| {
                    let (start, end) = selection.bounds();
                    index >= start.paragraph && index < end.paragraph
                })
            })
        }).collect()
    }

    pub fn execute(&mut self, name: &str, args: &Value, state: &mut AppState) -> bool {
        match name {
            "editorExec" => return self.format_command(args, state),
            "find" | "findReplace" => {
                self.document_selected = false;
                self.cross_selection = None;
                state.surface.find_open = true;
                self.find_needs_focus = true;
                self.replace_open = name == "findReplace";
                if let Some(query) = args.get("query").and_then(Value::as_str) {
                    self.find = query.to_owned();
                    self.find_next();
                }
            }
            "focusMode" => {
                state.surface.focus_mode = args
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(!state.surface.focus_mode)
            }
            "setView" => {
                state.surface.view =
                    if args.get("view").and_then(Value::as_str) == Some("continuous") {
                        ViewMode::Continuous
                    } else {
                        ViewMode::Pages
                    }
            }
            "zoomStep" => {
                let delta = if args.get("direction").and_then(Value::as_str) == Some("out") {
                    -0.1
                } else {
                    0.1
                };
                state.surface.zoom = (state.surface.zoom + delta).clamp(0.25, 3.0);
            }
            "setZoom" => {
                state.surface.zoom = args
                    .get("zoom")
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0)
                    .clamp(0.25, 3.0) as f32
            }
            "zoomFit" => {
                state.surface.zoom =
                    ((self.viewport_width - 48.0) / self.document.page_setup.width).clamp(0.25, 3.0)
            }
            "searchMenus" => {
                self.search_open = true;
                self.search_needs_focus = true;
                self.search_query.clear();
            }
            "scrollToPage" => {
                let page = args
                    .get("page")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .saturating_sub(1) as usize;
                self.scroll_paragraph = Some(
                    (page * self.document.paragraphs.len() / self.page_count.max(1) as usize)
                        .min(self.document.paragraphs.len().saturating_sub(1)),
                );
            }
            "wordCount" => state.modal = Modal::Statistics,
            "goToPage" => state.modal = Modal::GoToPage,
            "clearNotice" => state.surface.notice = None,
            _ => return self.execute_tool(name, args, state),
        }
        true
    }

    fn format_command(&mut self, args: &Value, state: &mut AppState) -> bool {
        let command = args.get("type").and_then(Value::as_str).unwrap_or("");
        if matches!(command, "toggleMark" | "clearFormatting" | "setAlignment")
            && !self.can_format_selection(state)
        {
            return false;
        }
        match command {
            "undo" => {
                if let Some(snapshot) = self.undo.pop() {
                    self.redo.push(self.snapshot());
                    self.restore_snapshot(snapshot);
                    self.changed(state);
                }
            }
            "redo" => {
                if let Some(snapshot) = self.redo.pop() {
                    self.undo.push(self.snapshot());
                    self.restore_snapshot(snapshot);
                    self.changed(state);
                }
            }
            "selectAll" => {
                self.document_selected = !self.document.paragraphs.is_empty();
                self.cross_selection = None;
                self.pending_selection = Some((self.active, self.selection.clone()));
            }
            "toggleMark" => {
                let mark = args.get("mark").and_then(Value::as_str).unwrap_or("");
                match mark {
                    "bold" => self.format.bold = !self.format.bold,
                    "italic" => self.format.italic = !self.format.italic,
                    "underline" => self.format.underline = !self.format.underline,
                    "strike" => self.format.strike = !self.format.strike,
                    _ => return self.execute_tool("toggleMark", args, state),
                }
                let enabled = match mark {
                    "bold" => self.format.bold,
                    "italic" => self.format.italic,
                    "underline" => self.format.underline,
                    "strike" => self.format.strike,
                    _ => false,
                };
                self.apply_transform(state, |format| match mark {
                    "bold" => format.bold = enabled,
                    "italic" => format.italic = enabled,
                    "underline" => format.underline = enabled,
                    "strike" => format.strike = enabled,
                    _ => {}
                });
            }
            "clearFormatting" => {
                self.format = TextFormat::default();
                self.apply_format(state);
            }
            "setAlignment" => {
                self.checkpoint();
                let alignment = match args.get("align").and_then(Value::as_str) {
                        Some("center") => Alignment::Center,
                        Some("right") => Alignment::Right,
                        Some("justify") => Alignment::Justify,
                        _ => Alignment::Left,
                    };
                if self.document_selected || self.cross_selection.is_some() {
                    let selected = self.selected_paragraph_indices();
                    for index in selected {
                        self.document.paragraphs[index].alignment = alignment;
                    }
                    self.changed(state);
                } else if let Some(paragraph) = self.document.paragraphs.get_mut(self.active) {
                    paragraph.alignment = alignment;
                    self.changed(state);
                }
            }
            "openGoTo" => state.modal = Modal::GoToPage,
            _ => return self.execute_tool(command, args, state),
        }
        true
    }

    fn apply_format(&mut self, state: &mut AppState) {
        let format = self.format.clone();
        self.apply_transform(state, |selected| *selected = format.clone());
    }

    fn apply_transform(&mut self, state: &mut AppState, transform: impl Fn(&mut TextFormat)) {
        if !self.can_format_selection(state) {
            return;
        }
        if self.document_selected || self.cross_selection.is_some() {
            let ranges: Vec<_> = (0..self.document.paragraphs.len())
                .filter_map(|index| self.selected_range_in(index).map(|range| (index, range)))
                .collect();
            self.checkpoint();
            for (index, range) in ranges {
                let paragraph = &mut self.document.paragraphs[index];
                let mut selected = slice_runs(&paragraph.runs, range.clone());
                for run in &mut selected {
                    transform(&mut run.format);
                }
                let mut runs = slice_runs(&paragraph.runs, 0..range.start);
                runs.extend(selected);
                runs.extend(slice_runs(&paragraph.runs, range.end..usize::MAX));
                if !runs.is_empty() {
                    paragraph.runs = runs;
                }
            }
            self.changed(state);
            return;
        }
        if self.selection.is_empty() {
            return;
        }
        self.checkpoint();
        if let Some(paragraph) = self.document.paragraphs.get_mut(self.active) {
            let before = slice_runs(&paragraph.runs, 0..self.selection.start);
            let selected = slice_runs(&paragraph.runs, self.selection.clone());
            let after = slice_runs(&paragraph.runs, self.selection.end..usize::MAX);
            paragraph.runs = before;
            paragraph.runs.extend(selected.into_iter().map(|mut run| {
                transform(&mut run.format);
                run
            }));
            paragraph.runs.extend(after);
            self.pending_selection = Some((self.active, self.selection.clone()));
            self.changed(state);
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState, theme: Theme) {
        self.sync_state(state);
        self.shortcuts(ctx, state);
        let colors = UiColors::for_theme(theme);
        if !state.surface.focus_mode {
            egui::TopBottomPanel::top("document-toolbar")
                .exact_height(56.0)
                .show_separator_line(false)
                .frame(Frame::new().fill(colors.app_bg).inner_margin(Margin {
                    left: 12,
                    right: 12,
                    top: 0,
                    bottom: 8,
                }))
                .show(ctx, |ui| self.toolbar(ui, state, &colors));
        }
        if state.surface.find_open {
            egui::TopBottomPanel::top("document-find")
                .show_separator_line(false)
                .frame(Frame::new().fill(colors.app_bg).inner_margin(Margin {
                    left: 12,
                    right: 12,
                    top: 0,
                    bottom: 8,
                }))
                .show(ctx, |ui| self.find_bar(ui, state));
        }
        if state.preferences.show_outline && !state.surface.focus_mode {
            egui::SidePanel::left("document-outline")
                .exact_width(230.0)
                .resizable(false)
                .frame(
                    Frame::new()
                        .fill(colors.app_surface)
                        .inner_margin(Margin::symmetric(14, 20)),
                )
                .show(ctx, |ui| self.outline(ui, state, &colors, theme));
        }
        egui::CentralPanel::default()
            .frame(Frame::new().fill(colors.app_bg))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                self.viewport_width = ui.available_width();
                let zoom = state.surface.zoom;
                let page_width = self.document.page_setup.width * zoom;
                let margin = self.document.page_setup.margin_left * zoom;
                let document_width = (page_width + 48.0).max(ui.available_width());
                let inset = (document_width - page_width) * 0.5;
                if state.preferences.show_ruler {
                    let (area, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), 43.0),
                        egui::Sense::hover(),
                    );
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(
                            area.left() + inset - self.horizontal_scroll,
                            area.top() + 8.0,
                        ),
                        vec2(page_width, 20.0),
                    );
                    ruler(
                        ui,
                        rect,
                        zoom,
                        margin,
                        self.document.page_setup.margin_right * zoom,
                        state.preferences.ruler_unit,
                        &colors,
                        theme,
                    );
                } else {
                    ui.add_space(8.0);
                }
                let scroll = egui::ScrollArea::both()
                    .id_salt("document-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(document_width);
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.add_space(inset);
                            ui.vertical(|ui| {
                                ui.set_width(page_width);
                                self.pages(ui, state, page_width, margin, zoom, theme);
                            });
                        });
                    });
                self.horizontal_scroll = scroll.state.offset.x;
            });
        self.finish_text_drag(ctx);
        self.show_tools(ctx, state);
        self.show_menu_search(ctx, state, &colors);
        self.sync_state(state);
    }

    fn pages(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut AppState,
        width: f32,
        margin: f32,
        zoom: f32,
        theme: Theme,
    ) {
        if self.document.paragraphs.is_empty() {
            self.document.paragraphs.push(Paragraph::default());
        }
        let content_height = (self.document.page_setup.height
            - self.document.page_setup.margin_top
            - self.document.page_setup.margin_bottom)
            * zoom;
        let right_margin = self.document.page_setup.margin_right * zoom;
        let content_width = width - margin - right_margin;
        let mut groups = vec![Vec::new()];
        let mut height = 0.0;
        for (index, paragraph) in self.document.paragraphs.iter().enumerate() {
            let estimated = paragraph_height(ui, paragraph, content_width, zoom);
            if height > 0.0
                && height + estimated > content_height
                && state.surface.view == ViewMode::Pages
            {
                groups.push(Vec::new());
                height = 0.0;
            }
            groups.last_mut().unwrap().push(index);
            height += estimated;
        }
        self.page_count = groups.len() as u32;
        for group in groups {
            let page = Frame::new()
                .fill(Color32::WHITE)
                .shadow(if state.surface.view == ViewMode::Pages {
                    match theme {
                        Theme::Light => egui::Shadow {
                            offset: [0, 6],
                            blur: 26,
                            spread: 0,
                            color: Color32::from_rgba_unmultiplied(0x26, 0x36, 0x50, 0x1a),
                        },
                        Theme::Dark => egui::Shadow {
                            offset: [0, 4],
                            blur: 24,
                            spread: 0,
                            color: Color32::from_black_alpha(30),
                        },
                    }
                } else {
                    egui::Shadow::NONE
                })
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.add_space(self.document.page_setup.margin_top * zoom);
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(margin);
                        ui.vertical(|ui| {
                            ui.set_min_size(vec2(content_width, content_height));
                            ui.set_max_width(content_width);
                            ui.spacing_mut().item_spacing.y = 8.0 * zoom;
                            ui.visuals_mut().override_text_color =
                                Some(Color32::from_rgb(32, 32, 32));
                            ui.visuals_mut().selection.bg_fill = Color32::from_rgb(180, 208, 248);
                            ui.visuals_mut().text_cursor.stroke =
                                Stroke::new(1.5_f32, Color32::from_rgb(0x20, 0x21, 0x24));
                            for index in group {
                                self.paragraph(ui, state, index, content_width, zoom);
                            }
                        });
                        ui.add_space(right_margin);
                    });
                    ui.add_space(self.document.page_setup.margin_bottom * zoom);
                });
            if state.surface.view == ViewMode::Pages {
                ui.painter().rect_stroke(
                    page.response.rect,
                    0.0,
                    Stroke::new(1.0_f32, ui.visuals().window_stroke.color),
                    egui::StrokeKind::Outside,
                );
            }
            ui.add_space(if state.surface.view == ViewMode::Pages {
                24.0 * zoom
            } else {
                0.0
            });
        }
    }

    fn paragraph(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut AppState,
        index: usize,
        width: f32,
        zoom: f32,
    ) {
        let paragraph = &self.document.paragraphs[index];
        let indent = paragraph.indent.max(0.0) * zoom;
        let line_spacing = paragraph.line_spacing;
        let marker = match paragraph.list.as_deref() {
            Some("bullet") => Some("\u{2022}".to_string()),
            Some("number") => {
                let count = self.document.paragraphs[..=index]
                    .iter()
                    .rev()
                    .take_while(|paragraph| paragraph.list.as_deref() == Some("number"))
                    .count();
                Some(format!("{count}."))
            }
            _ => None,
        };
        let available_width =
            (width - indent - if marker.is_some() { 24.0 * zoom } else { 0.0 }).max(30.0);
        let runs = paragraph.runs.clone();
        let alignment = paragraph.alignment.clone();
        let old_text = paragraph.plain_text();
        let mut text = old_text.clone();
        let insertion_format = self.format.clone();
        let selected_range = self.selected_range_in(index);
        let mut layouter = |ui: &egui::Ui, text: &str, wrap: f32| {
            let mut job = layout_job(ui, text, &runs, &alignment, wrap, zoom);
            if let Some(range) = &selected_range {
                highlight_range(&mut job, text, range.clone());
            }
            if line_spacing > 0.0 {
                for section in &mut job.sections {
                    section.format.line_height = Some(section.format.font_id.size * line_spacing);
                }
            }
            ui.fonts(|fonts| fonts.layout_job(job))
        };
        let id = ui.make_persistent_id(("document-paragraph", index));
        if let Some((pending_index, range)) = &self.pending_selection {
            if *pending_index == index {
                let mut edit_state =
                    egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
                edit_state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(range.start),
                        egui::text::CCursor::new(range.end),
                    )));
                edit_state.store(ui.ctx(), id);
                if !state.surface.find_open && self.cross_selection.is_none() {
                    if !ui.memory(|memory| memory.has_focus(id)) {
                        ui.memory_mut(|memory| memory.request_focus(id));
                    }
                    self.active_id = Some(id);
                }
                self.pending_selection = None;
            }
        }
        if self.cross_selection.is_some() && ui.memory(|memory| memory.has_focus(id)) {
            let anchor = self.cross_selection.and_then(|selection| {
                (selection.anchor.paragraph == index).then_some(selection.anchor.offset)
            }).unwrap_or(0);
            let mut edit_state =
                egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
            edit_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(anchor),
            )));
            edit_state.store(ui.ctx(), id);
        }
        let selection = egui::text_edit::TextEditState::load(ui.ctx(), id)
                .and_then(|edit| edit.cursor.char_range())
                .map(|range| range.primary.index.min(range.secondary.index)..range.primary.index.max(range.secondary.index))
                .unwrap_or_else(|| self.selection.clone());
        let mut paste = None;
        ui.input_mut(|input| {
                for event in &mut input.events {
                    if let egui::Event::Paste(text) = event {
                        *text = text.replace("\r\n", "\n").replace('\r', "\n");
                        paste = Some((selection.clone(), text.clone()));
                    }
                }
            });
        let output = ui
            .horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(indent);
                if let Some(marker) = &marker {
                    ui.add_sized(
                        vec2(24.0 * zoom, 18.0 * zoom),
                        egui::Label::new(RichText::new(marker).size(11.0 * 4.0 / 3.0 * zoom)),
                    );
                }
                TextEdit::multiline(&mut text)
                    .id(id)
                    .frame(false)
                    .margin(vec2(0.0, 0.0))
                    .desired_width(available_width)
                    .desired_rows(1)
                    .font(FontId::proportional(11.0 * 4.0 / 3.0 * zoom))
                    .text_color(Color32::from_rgb(32, 32, 32))
                    .layouter(&mut layouter)
                    .show(ui)
            })
            .inner;
        self.track_text_drag(ui, index, &output);
        if output.response.clicked() {
            self.document_selected = false;
            self.cross_selection = None;
        }
        if self.tools.paragraph_marks {
            ui.painter().text(
                output.response.rect.right_top() + vec2(3.0, 0.0),
                egui::Align2::LEFT_TOP,
                "\u{00b6}",
                FontId::proportional(12.0 * zoom),
                Color32::GRAY,
            );
        }
        if self.tools.show_bookmarks
            && self
                .tools
                .bookmarks
                .iter()
                .any(|(_, marked)| *marked == index)
        {
            crate::icons::paint_icon_filled(
                ui.ctx(),
                ui.painter(),
                crate::icons::IconName::Bookmark,
                egui::Rect::from_center_size(
                    output.response.rect.left_top() + vec2(-11.0, 9.0 * zoom),
                    vec2(13.0, 13.0),
                ),
                Color32::from_rgb(0x1e, 0x88, 0xe5),
            );
        }
        if self.scroll_paragraph == Some(index) {
            output.response.scroll_to_me(None);
            self.scroll_paragraph = None;
        }
        let mut selection_changed = false;
        if output.response.has_focus() {
            let previous_active = self.active;
            self.active = index;
            self.active_id = Some(id);
            if let Some(range) = output.cursor_range {
                let next = range.as_sorted_char_range();
                if self.selection != next || previous_active != index {
                    self.selection = next;
                    if !output.response.changed() {
                        selection_changed = true;
                        self.format = format_at(&runs, self.selection.start);
                    }
                }
            }
        }
        if selection_changed {
            self.paint_selection_if_armed(state);
        }
        if output.response.changed() && text != old_text {
            if let Some((range, pasted)) = paste {
                let expected = format!("{}{}{}", &old_text[..char_byte(&old_text, range.start)], pasted, &old_text[char_byte(&old_text, range.end)..]);
                if pasted.contains('\n') {
                    if expected == text {
                        self.paste_paragraphs(index, &runs, range, &pasted, &insertion_format, state);
                    } else {
                        state.surface.error = Some("Could Not Determine The Paste Position. Try Pasting Again.".into());
                        self.pending_selection = Some((index, selection));
                    }
                    ui.ctx().request_repaint();
                    return;
                }
            }
            self.checkpoint();
            self.document.paragraphs[index].runs =
                edit_runs(&runs, &old_text, &text, &insertion_format);
            self.changed(state);
        }
    }

    fn track_text_drag(
        &mut self,
        ui: &egui::Ui,
        index: usize,
        output: &egui::text_edit::TextEditOutput,
    ) {
        if output.response.drag_started() && self.drag_anchor.is_none() {
            if let Some(pos) = ui.input(|input| input.pointer.interact_pos()) {
                self.document_selected = false;
                let offset = output
                    .galley
                    .cursor_from_pos(pos.to_vec2() - output.galley_pos.to_vec2())
                    .ccursor
                    .index;
                self.drag_anchor = Some(DocumentPosition {
                    paragraph: index,
                    offset,
                });
                self.drag_best = None;
            }
        }
        if self.drag_anchor.is_none() {
            return;
        }
        let Some(pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return;
        };
        if !ui.input(|input| input.pointer.primary_down()) {
            return;
        }
        let offset = output
            .galley
            .cursor_from_pos(pos.to_vec2() - output.galley_pos.to_vec2())
            .ccursor
            .index;
        let rect = output.galley.rect.translate(output.galley_pos.to_vec2());
        let distance = if pos.y < rect.top() {
            rect.top() - pos.y
        } else if pos.y > rect.bottom() {
            pos.y - rect.bottom()
        } else {
            0.0
        };
        let candidate = (distance, index, offset);
        if self.drag_best.is_none_or(|best| candidate.0 < best.0) {
            self.drag_best = Some(candidate);
        }
    }

    fn finish_text_drag(&mut self, ctx: &egui::Context) {
        let Some(anchor) = self.drag_anchor else {
            return;
        };
        let down = ctx.input(|input| input.pointer.primary_down());
        let Some((_, paragraph, offset)) = self.drag_best else {
            if !down {
                self.drag_anchor = None;
            }
            return;
        };
        let head = DocumentPosition { paragraph, offset };
        self.cross_selection = if anchor == head {
            None
        } else {
            Some(DocumentSelection { anchor, head })
        };
        self.active = head.paragraph;
        self.selection = head.offset..head.offset;
        self.scroll_paragraph = Some(head.paragraph);
        if !down {
            self.pending_selection = Some((head.paragraph, head.offset..head.offset));
            self.drag_anchor = None;
            self.drag_best = None;
        }
        ctx.request_repaint();
    }

    fn paste_paragraphs(
        &mut self,
        index: usize,
        runs: &[Run],
        selection: Range<usize>,
        pasted: &str,
        insertion_format: &TextFormat,
        state: &mut AppState,
    ) {
        let mut edited_runs = slice_runs(runs, 0..selection.start);
        edited_runs.push(Run { text: pasted.into(), format: insertion_format.clone() });
        edited_runs.extend(slice_runs(runs, selection.end..usize::MAX));
        let breaks: Vec<usize> = pasted.chars().enumerate()
            .filter_map(|(index, character)| (character == '\n').then_some(selection.start + index))
            .collect();
        let mut document = self.document.clone();
        let mut start = 0;
        for (offset, &line_break) in breaks.iter().enumerate() {
            let mut first = document.paragraphs[index + offset].clone();
            let mut second = first.clone();
            first.runs = slice_runs(&edited_runs, start..line_break);
            second.runs = slice_runs(&edited_runs, line_break + 1..usize::MAX);
            if first.runs.is_empty() {
                first.runs.push(Run { text: String::new(), format: insertion_format.clone() });
            }
            if second.runs.is_empty() {
                second.runs.push(Run { text: String::new(), format: insertion_format.clone() });
            }
            if let Err(error) = document.split_paragraph(index + offset, first, second) {
                state.surface.error = Some(error);
                self.selection = selection.clone();
                self.pending_selection = Some((index, selection));
                return;
            }
            start = line_break + 1;
        }
        self.selection = selection.clone();
        self.checkpoint();
        self.document = document;
        self.active = index + breaks.len();
        let inserted_end = selection.start + pasted.chars().count();
        self.selection = (inserted_end - start)..(inserted_end - start);
        self.pending_selection = Some((self.active, self.selection.clone()));
        self.scroll_paragraph = Some(self.active);
        self.changed(state);
    }

    fn shortcuts(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if state.surface.focus_mode
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            state.surface.focus_mode = false;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::Slash)) {
            self.search_open = true;
            self.search_needs_focus = true;
        }
        if state.modal != Modal::None || self.search_open {
            return;
        }
        let document_keys = !state.surface.find_open;
        let editing_document = document_keys
            && self
                .active_id
                .is_some_and(|id| ctx.memory(|memory| memory.has_focus(id)));
        if editing_document
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::A))
        {
            self.format_command(&serde_json::json!({"type":"selectAll"}), state);
            ctx.request_repaint();
        }
        if editing_document && !self.document_selected {
            self.cross_selection_keys(ctx);
        }
        if editing_document && (self.document_selected || self.cross_selection.is_some()) {
            for (key, to_end) in [
                (egui::Key::ArrowLeft, false),
                (egui::Key::ArrowUp, false),
                (egui::Key::Home, false),
                (egui::Key::ArrowRight, true),
                (egui::Key::ArrowDown, true),
                (egui::Key::End, true),
            ] {
                if !ctx.input(|input| input.modifiers.shift)
                    && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
                    let bounds = self.selection_bounds();
                    self.document_selected = false;
                    self.cross_selection = None;
                    let (start, end) = bounds.unwrap();
                    let position = if to_end { end } else { start };
                    self.active = position.paragraph;
                    self.selection = position.offset..position.offset;
                    self.pending_selection = Some((self.active, self.selection.clone()));
                    self.scroll_paragraph = Some(self.active);
                    break;
                }
            }
        }
        if editing_document && (self.document_selected || self.cross_selection.is_some()) {
            self.document_selection_input(ctx, state);
        }
        if editing_document
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            self.split_paragraph(state);
        }
        if editing_document
            && self.active > 0
            && self.selection == (0..0)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Backspace))
        {
            self.merge_paragraph(state);
        }
        if document_keys {
            for (key, command) in [
                (egui::Key::B, "bold"),
                (egui::Key::I, "italic"),
                (egui::Key::U, "underline"),
            ] {
                if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, key)) {
                    self.format_command(
                        &serde_json::json!({"type":"toggleMark","mark":command}),
                        state,
                    );
                }
            }
            for (key, name) in [(egui::Key::Z, "undo"), (egui::Key::Y, "redo")] {
                if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, key)) {
                    self.format_command(&serde_json::json!({"type":name}), state);
                }
            }
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.document_selected = false;
            self.cross_selection = None;
            state.surface.find_open = true;
        }
    }

    fn selection_bounds(&self) -> Option<(DocumentPosition, DocumentPosition)> {
        if let Some(selection) = self.cross_selection {
            return Some(selection.bounds());
        }
        if !self.document_selected || self.document.paragraphs.is_empty() {
            return None;
        }
        let last = self.document.paragraphs.len() - 1;
        Some((DocumentPosition { paragraph: 0, offset: 0 }, DocumentPosition {
            paragraph: last,
            offset: self.document.paragraphs[last].plain_text().chars().count(),
        }))
    }

    fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_bounds()?;
        Some((start.paragraph..=end.paragraph).map(|index| {
            let text = self.document.paragraphs[index].plain_text();
            let from = if index == start.paragraph { start.offset } else { 0 };
            let to = if index == end.paragraph { end.offset } else { text.chars().count() };
            text[char_byte(&text, from)..char_byte(&text, to)].to_owned()
        }).collect::<Vec<_>>().join("\n"))
    }

    fn cross_selection_keys(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|input| input.events.clone());
        if events.iter().any(|event| matches!(event, egui::Event::PointerButton { pressed: true, .. })) {
            return;
        }
        let keys: Vec<_> = events.iter().filter_map(|event| match event {
            egui::Event::Key { key: key @ (egui::Key::ArrowLeft | egui::Key::ArrowRight), pressed: true, modifiers, .. }
                if modifiers.shift && !modifiers.ctrl && !modifiers.alt && !modifiers.command && !modifiers.mac_cmd => Some(*key),
            _ => None,
        }).collect();
        if keys.is_empty() {
            return;
        }
        let Some(id) = self.active_id else { return; };
        let Some(cursor) = egui::text_edit::TextEditState::load(ctx, id).and_then(|edit| edit.cursor.char_range()) else { return; };
        let initial = DocumentSelection {
            anchor: DocumentPosition { paragraph: self.active, offset: cursor.secondary.index },
            head: DocumentPosition { paragraph: self.active, offset: cursor.primary.index },
        };
        let mut selection = self.cross_selection.unwrap_or(initial);
        let first = keys[0];
        let text = self.document.paragraphs[selection.head.paragraph].plain_text();
        if self.cross_selection.is_none()
            && !((first == egui::Key::ArrowLeft && selection.head.offset == 0 && selection.head.paragraph > 0)
                || (first == egui::Key::ArrowRight && selection.head.offset == text.chars().count() && selection.head.paragraph + 1 < self.document.paragraphs.len()))
        {
            return;
        }
        for key in &keys {
            selection.head = move_document_position(&self.document, selection.head, *key == egui::Key::ArrowRight);
        }
        ctx.input_mut(|input| input.events.retain(|event| !matches!(event,
            egui::Event::Key { key: egui::Key::ArrowLeft | egui::Key::ArrowRight, pressed: true, modifiers, .. }
                if modifiers.shift && !modifiers.ctrl && !modifiers.alt && !modifiers.command && !modifiers.mac_cmd
        )));
        self.cross_selection = if selection.anchor == selection.head { None } else { Some(selection) };
        self.active = selection.head.paragraph;
        self.selection = selection.head.offset..selection.head.offset;
        self.pending_selection = Some((self.active, self.selection.clone()));
        self.scroll_paragraph = Some(self.active);
        ctx.request_repaint();
    }

    fn document_selection_input(&mut self, ctx: &egui::Context, state: &mut AppState) {
        let mut actions = Vec::new();
        let mut blocked_navigation = false;
        ctx.input_mut(|input| {
            input.events.retain(|event| {
                if matches!(event, egui::Event::Key {
                    key: egui::Key::ArrowLeft | egui::Key::ArrowRight | egui::Key::ArrowUp | egui::Key::ArrowDown | egui::Key::Home | egui::Key::End | egui::Key::PageUp | egui::Key::PageDown,
                    pressed: true, ..
                }) {
                    blocked_navigation = true;
                    return false;
                }
                let action = match event {
                    egui::Event::Copy => Some((false, None)),
                    egui::Event::Cut => Some((true, Some(String::new()))),
                    egui::Event::Paste(text) => Some((false, Some(text.replace("\r\n", "\n").replace('\r', "\n")))),
                    egui::Event::Text(text) if !text.is_empty() => Some((false, Some(text.clone()))),
                    egui::Event::Key { key: egui::Key::Backspace | egui::Key::Delete, pressed: true, .. } => Some((false, Some(String::new()))),
                    egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } => Some((false, Some("\n".into()))),
                    _ => None,
                };
                if let Some(action) = action {
                    actions.push(action);
                    false
                } else {
                    true
                }
            });
        });
        if blocked_navigation {
            state.surface.error = Some("This Selection Navigation Is Not Yet Supported.".into());
        }
        for (cut, replacement) in actions {
            if cut || replacement.is_none() {
                if let Err(error) = self.document.can_replace_all_content() {
                    state.surface.error = Some(error);
                    return;
                }
                ctx.copy_text(self.selected_text().unwrap_or_default());
            }
            if let Some(replacement) = replacement {
                self.replace_document_selection(&replacement, state);
            }
        }
    }

    fn replace_document_selection(&mut self, replacement: &str, state: &mut AppState) {
        if let Err(error) = self.document.can_replace_all_content() {
            state.surface.error = Some(error);
            return;
        }
        let mut document = self.document.clone();
        let Some((start, end)) = self.selection_bounds() else { return; };
        let mut runs = slice_runs(&document.paragraphs[start.paragraph].runs, 0..start.offset);
        runs.push(Run { text: replacement.into(), format: self.format.clone() });
        runs.extend(slice_runs(&document.paragraphs[end.paragraph].runs, end.offset..usize::MAX));
        for _ in start.paragraph..end.paragraph {
            let merged = document.paragraphs[start.paragraph].clone();
            if let Err(error) = document.merge_paragraphs(start.paragraph, merged) {
                state.surface.error = Some(error);
                return;
            }
        }
        document.paragraphs[start.paragraph].runs = runs.clone();
        let mut index = start.paragraph;
        let mut segment_start = 0;
        let breaks: Vec<_> = replacement.chars().enumerate()
            .filter_map(|(offset, character)| (character == '\n').then_some(start.offset + offset)).collect();
        for line_break in breaks {
            let first = document.paragraphs[index].clone();
            let mut second = first.clone();
            let mut first = first;
            first.runs = slice_runs(&runs, segment_start..line_break);
            second.runs = slice_runs(&runs, line_break + 1..usize::MAX);
            if first.runs.is_empty() { first.runs.push(Run { text: String::new(), format: self.format.clone() }); }
            if second.runs.is_empty() { second.runs.push(Run { text: String::new(), format: self.format.clone() }); }
            if let Err(error) = document.split_paragraph(index, first, second) {
                state.surface.error = Some(error);
                return;
            }
            segment_start = line_break + 1;
            index += 1;
        }
        if let Err(error) = document.to_bytes() {
            state.surface.error = Some(error.to_string());
            return;
        }
        self.checkpoint();
        self.document = document;
        self.document_selected = false;
        self.cross_selection = None;
        self.active = index;
        let offset = start.offset + replacement.chars().count() - segment_start;
        self.selection = offset..offset;
        self.pending_selection = Some((index, self.selection.clone()));
        self.scroll_paragraph = Some(index);
        self.changed(state);
    }

    fn split_paragraph(&mut self, state: &mut AppState) {
        let Some(paragraph) = self.document.paragraphs.get(self.active) else {
            return;
        };
        let before = slice_runs(&paragraph.runs, 0..self.selection.start);
        let after = slice_runs(&paragraph.runs, self.selection.end..usize::MAX);
        let next = Paragraph {
            runs: after,
            alignment: paragraph.alignment,
            ..paragraph.clone()
        };
        let mut first = paragraph.clone();
        first.runs = before;
        let mut document = self.document.clone();
        if let Err(error) = document.split_paragraph(self.active, first, next) {
            state.surface.error = Some(error);
            return;
        }
        self.checkpoint();
        self.document = document;
        self.active += 1;
        self.selection = 0..0;
        self.pending_selection = Some((self.active, 0..0));
        self.changed(state);
    }

    fn merge_paragraph(&mut self, state: &mut AppState) {
        let mut merged = self.document.paragraphs[self.active - 1].clone();
        let cursor = merged.plain_text().chars().count();
        merged.runs.extend(self.document.paragraphs[self.active].runs.clone());
        let mut document = self.document.clone();
        if let Err(error) = document.merge_paragraphs(self.active - 1, merged) {
            state.surface.error = Some(error);
            return;
        }
        self.checkpoint();
        self.document = document;
        self.active -= 1;
        self.selection = cursor..cursor;
        self.pending_selection = Some((self.active, self.selection.clone()));
        self.changed(state);
    }
}

fn ruler(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    zoom: f32,
    left_margin: f32,
    right_margin: f32,
    unit: crate::state::RulerUnit,
    colors: &UiColors,
    theme: Theme,
) {
    let painter = ui.painter().with_clip_rect(ui.clip_rect());
    let margin_color = if theme == Theme::Dark {
        Color32::from_rgb(13, 15, 19)
    } else {
        Color32::from_rgb(225, 229, 235)
    };
    painter.rect_filled(
        egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.left() + left_margin, rect.bottom()),
        ),
        0.0,
        margin_color,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.right() - right_margin, rect.top()),
            rect.max,
        ),
        0.0,
        margin_color,
    );
    let (unit_width, subdivisions) = match unit {
        crate::state::RulerUnit::Inch => (96.0 * zoom, 8),
        crate::state::RulerUnit::Cm => (96.0 / 2.54 * zoom, 4),
    };
    let origin = rect.left() + left_margin;
    let start = ((-left_margin / unit_width).floor() as i32) * subdivisions;
    let end = (((rect.width() - left_margin) / unit_width).ceil() as i32) * subdivisions;
    for tick in start..=end {
        let x = origin + tick as f32 * unit_width / subdivisions as f32;
        if x < rect.left() || x > rect.right() {
            continue;
        }
        let whole = tick % subdivisions == 0;
        let length = if whole {
            8.0
        } else if tick % 2 == 0 {
            5.0
        } else {
            3.0
        };
        painter.vline(
            x,
            rect.bottom() - length..=rect.bottom(),
            Stroke::new(0.6_f32, colors.app_muted),
        );
        if whole && tick != 0 && (tick > 0 || matches!(unit, crate::state::RulerUnit::Cm)) {
            painter.text(
                egui::pos2(x, rect.top() + 6.0),
                egui::Align2::CENTER_CENTER,
                (tick / subdivisions).abs().to_string(),
                FontId::proportional(9.0),
                colors.app_muted,
            );
        }
    }
    for (x, top) in [
        (origin, true),
        (origin, false),
        (rect.right() - right_margin, false),
    ] {
        let marker_h = 9.0;
        let y = if top { rect.top() } else { rect.bottom() - marker_h };
        let points = if top {
            vec![
                egui::pos2(x - 7.0, y),
                egui::pos2(x + 7.0, y),
                egui::pos2(x, y + marker_h),
            ]
        } else {
            vec![
                egui::pos2(x - 7.0, y + marker_h),
                egui::pos2(x + 7.0, y + marker_h),
                egui::pos2(x, y),
            ]
        };
        painter.add(egui::Shape::convex_polygon(
            points,
            colors.app_accent,
            Stroke::NONE,
        ));
    }
    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(origin - 4.0, rect.bottom() - 1.0),
            vec2(8.0, 3.0),
        ),
        0.0,
        colors.app_accent,
    );
}

fn paragraph_height(ui: &egui::Ui, paragraph: &Paragraph, width: f32, zoom: f32) -> f32 {
    let job = layout_job(
        ui,
        &paragraph.plain_text(),
        &paragraph.runs,
        &paragraph.alignment,
        width,
        zoom,
    );
    ui.fonts(|fonts| fonts.layout_job(job))
        .size()
        .y
        .max(18.0 * zoom)
        + 8.0 * zoom
}

fn layout_job(
    ui: &egui::Ui,
    text: &str,
    runs: &[Run],
    alignment: &Alignment,
    width: f32,
    zoom: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    job.halign = match alignment {
        Alignment::Center => Align::Center,
        Alignment::Right => Align::RIGHT,
        _ => Align::LEFT,
    };
    job.justify = matches!(alignment, Alignment::Justify);
    let mut offset = 0;
    for run in runs {
        let count = run.text.chars().count();
        let start = char_byte(text, offset);
        let end = char_byte(text, offset + count);
        if start < end {
            job.append(&text[start..end], 0.0, text_format(ui, &run.format, zoom));
        }
        offset += count;
    }
    let remaining = char_byte(text, offset);
    if remaining < text.len() || job.text.is_empty() {
        job.append(
            &text[remaining..],
            0.0,
            text_format(ui, &format_at(runs, offset), zoom),
        );
    }
    job
}

fn highlight_range(job: &mut egui::text::LayoutJob, text: &str, range: Range<usize>) {
    let start = char_byte(text, range.start);
    let end = char_byte(text, range.end);
    if start >= end {
        return;
    }
    let mut sections = Vec::new();
    for section in job.sections.drain(..) {
        let from = section.byte_range.start;
        let to = section.byte_range.end;
        let mut leading_space = section.leading_space;
        for segment in [from..start.min(to).max(from), start.max(from)..end.min(to), end.max(from).min(to)..to] {
            if segment.start >= segment.end || segment.start < from || segment.end > to {
                continue;
            }
            let mut format = section.format.clone();
            if segment.start >= start && segment.end <= end {
                format.background = Color32::from_rgb(180, 208, 248);
            }
            sections.push(egui::text::LayoutSection { leading_space, byte_range: segment, format });
            leading_space = 0.0;
        }
    }
    job.sections = sections;
}

fn text_format(ui: &egui::Ui, format: &TextFormat, zoom: f32) -> egui::TextFormat {
    let font_name = format.font_family.as_deref().unwrap_or("Calibri");
    let family_name = match (format.bold, format.italic) {
        (true, true) => format!("{} Bold Italic", font_name),
        (true, false) => format!("{} Bold", font_name),
        (false, true) => format!("{} Italic", font_name),
        _ => font_name.to_string(),
    };
    let family = ui.fonts(|fonts| fonts.families().into_iter().find(|family| matches!(family, egui::FontFamily::Name(name) if name.as_ref() == family_name))).unwrap_or(egui::FontFamily::Proportional);
    let dedicated_italic = format.italic && matches!(&family, egui::FontFamily::Name(name) if name.as_ref() == family_name);
    let color = format
        .color
        .as_deref()
        .and_then(|hex| u32::from_str_radix(hex.trim_start_matches('#'), 16).ok())
        .map(|value| Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8))
        .unwrap_or(Color32::from_rgb(32, 32, 32));
    let script_scale = if format.script.is_some() { 0.7 } else { 1.0 };
    let background = format
        .highlight
        .as_deref()
        .and_then(|hex| u32::from_str_radix(hex.trim_start_matches('#'), 16).ok())
        .map(|value| Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8))
        .unwrap_or(Color32::TRANSPARENT);
    egui::TextFormat {
        font_id: FontId::new(
            format.font_size.unwrap_or(11.0) * 4.0 / 3.0 * zoom * script_scale,
            family,
        ),
        color,
        background,
        valign: if format.script.as_deref() == Some("subscript") {
            Align::BOTTOM
        } else {
            Align::TOP
        },
        italics: format.italic && !dedicated_italic,
        underline: if format.underline {
            Stroke::new(1.0, color)
        } else {
            Stroke::NONE
        },
        strikethrough: if format.strike {
            Stroke::new(1.0, color)
        } else {
            Stroke::NONE
        },
        ..Default::default()
    }
}

fn char_byte(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map(|(byte, _)| byte)
        .unwrap_or(text.len())
}

fn move_document_position(document: &Document, position: DocumentPosition, forward: bool) -> DocumentPosition {
    use unicode_segmentation::UnicodeSegmentation;
    let text = document.paragraphs[position.paragraph].plain_text();
    let byte = char_byte(&text, position.offset);
    if forward {
        if byte == text.len() {
            if position.paragraph + 1 < document.paragraphs.len() {
                return DocumentPosition { paragraph: position.paragraph + 1, offset: 0 };
            }
            return position;
        }
        let next = text.grapheme_indices(true).map(|(index, _)| index)
            .chain(std::iter::once(text.len())).find(|&index| index > byte).unwrap_or(text.len());
        return DocumentPosition { paragraph: position.paragraph, offset: text[..next].chars().count() };
    }
    if byte == 0 {
        if position.paragraph > 0 {
            let paragraph = position.paragraph - 1;
            return DocumentPosition { paragraph, offset: document.paragraphs[paragraph].plain_text().chars().count() };
        }
        return position;
    }
    let previous = text.grapheme_indices(true).map(|(index, _)| index)
        .take_while(|&index| index < byte).last().unwrap_or(0);
    DocumentPosition { paragraph: position.paragraph, offset: text[..previous].chars().count() }
}

fn format_at(runs: &[Run], index: usize) -> TextFormat {
    let mut offset = 0;
    for run in runs {
        offset += run.text.chars().count();
        if index < offset {
            return run.format.clone();
        }
    }
    runs.last()
        .map(|run| run.format.clone())
        .unwrap_or_default()
}

fn slice_runs(runs: &[Run], range: Range<usize>) -> Vec<Run> {
    let mut offset = 0;
    let mut result = Vec::new();
    for run in runs {
        let end = offset + run.text.chars().count();
        let start = range.start.max(offset);
        let stop = range.end.min(end);
        if start < stop {
            result.push(Run {
                text: run.text
                    [char_byte(&run.text, start - offset)..char_byte(&run.text, stop - offset)]
                    .into(),
                format: run.format.clone(),
            });
        }
        offset = end;
    }
    result
}

fn edit_runs(runs: &[Run], old: &str, new: &str, insertion_format: &TextFormat) -> Vec<Run> {
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    let prefix = old_chars
        .iter()
        .zip(&new_chars)
        .take_while(|(old, new)| old == new)
        .count();
    let suffix = old_chars[prefix..]
        .iter()
        .rev()
        .zip(new_chars[prefix..].iter().rev())
        .take_while(|(old, new)| old == new)
        .count();
    let mut result = slice_runs(runs, 0..prefix);
    let inserted: String = new_chars[prefix..new_chars.len() - suffix].iter().collect();
    if !inserted.is_empty() {
        result.push(Run {
            text: inserted,
            format: insertion_format.clone(),
        });
    }
    result.extend(slice_runs(runs, old_chars.len() - suffix..usize::MAX));
    if result.is_empty() {
        result.push(Run {
            text: String::new(),
            format: insertion_format.clone(),
        });
    }
    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_paragraph_selection_formats_replaces_and_undoes() {
        let mut editor = Editor::default();
        let mut state = AppState::default();
        editor.document.replace_plain_text("ab\ncd");
        editor.document = Document::from_bytes(&editor.document.to_bytes().unwrap()).unwrap();
        editor.cross_selection = Some(DocumentSelection {
            anchor: DocumentPosition { paragraph: 0, offset: 1 },
            head: DocumentPosition { paragraph: 1, offset: 1 },
        });
        assert_eq!(editor.selected_text().as_deref(), Some("b\nc"));
        editor.format_command(&serde_json::json!({"type":"toggleMark","mark":"bold"}), &mut state);
        assert!(!editor.document.paragraphs[0].runs[0].format.bold);
        assert!(editor.document.paragraphs[0].runs[1].format.bold);
        assert!(editor.document.paragraphs[1].runs[0].format.bold);
        assert!(!editor.document.paragraphs[1].runs[1].format.bold);
        editor.replace_document_selection("X\nY", &mut state);
        assert_eq!(editor.document.plain_text(), "aX\nYd");
        let reopened = Document::from_bytes(&editor.document.to_bytes().unwrap()).unwrap();
        assert_eq!(reopened.plain_text(), "aX\nYd");
        editor.format_command(&serde_json::json!({"type":"undo"}), &mut state);
        assert_eq!(editor.document.plain_text(), "ab\ncd");
        assert_eq!(editor.selected_text().as_deref(), Some("b\nc"));
    }

    #[test]
    fn shift_arrows_extend_and_shrink_across_paragraphs() {
        let context = egui::Context::default();
        crate::theme::install_fonts(&context, crate::INTER_TTF, crate::INTER_MEDIUM_TTF, crate::INTER_SEMIBOLD_TTF);
        let mut editor = Editor::default();
        editor.document.replace_plain_text("ab\ncd");
        editor.pending_selection = Some((0, 1..1));
        let mut state = AppState::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))),
            ..Default::default()
        };
        let _ = context.run(input.clone(), |context| editor.show(context, &mut state, Theme::Light));
        let _ = context.run(input.clone(), |context| editor.show(context, &mut state, Theme::Light));
        let key = |key| {
            let mut event = input.clone();
            event.modifiers = egui::Modifiers::SHIFT;
            event.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::SHIFT });
            event
        };
        for _ in 0..3 {
            let _ = context.run(key(egui::Key::ArrowRight), |context| editor.show(context, &mut state, Theme::Light));
        }
        assert_eq!(editor.selected_text().as_deref(), Some("b\nc"));
        let mut copy = input.clone();
        copy.events.push(egui::Event::Copy);
        let output = context.run(copy, |context| editor.show(context, &mut state, Theme::Light));
        assert!(output.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text == "b\nc")));
        for _ in 0..3 {
            let _ = context.run(key(egui::Key::ArrowLeft), |context| editor.show(context, &mut state, Theme::Light));
        }
        assert!(editor.cross_selection.is_none());
        assert_eq!(editor.active, 0);
        assert_eq!(editor.selection, 1..1);
    }

    #[test]
    fn drag_release_keeps_cross_paragraph_selection() {
        let context = egui::Context::default();
        crate::theme::install_fonts(&context, crate::INTER_TTF, crate::INTER_MEDIUM_TTF, crate::INTER_SEMIBOLD_TTF);
        let mut editor = Editor::default();
        editor.document.replace_plain_text("ab\ncd");
        editor.drag_anchor = Some(DocumentPosition { paragraph: 0, offset: 1 });
        editor.drag_best = Some((0.0, 1, 1));
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))),
            ..Default::default()
        };
        let _ = context.run(input, |ctx| editor.finish_text_drag(ctx));
        assert_eq!(editor.selected_text().as_deref(), Some("b\nc"));
        assert!(editor.drag_anchor.is_none());
        assert!(editor.drag_best.is_none());
        assert_eq!(editor.pending_selection, Some((1, 1..1)));
        let _ = context.run(egui::RawInput::default(), |ctx| editor.finish_text_drag(ctx));
        assert_eq!(editor.selected_text().as_deref(), Some("b\nc"));
    }

    #[test]
    fn cross_paragraph_arrow_movement_respects_graphemes() {
        let mut document = Document::default();
        document.replace_plain_text("A\ne\u{301}B");
        let start = DocumentPosition { paragraph: 0, offset: 1 };
        let next = move_document_position(&document, start, true);
        assert_eq!(next, DocumentPosition { paragraph: 1, offset: 0 });
        let after_grapheme = move_document_position(&document, next, true);
        assert_eq!(after_grapheme.offset, 2);
        assert_eq!(move_document_position(&document, after_grapheme, false), next);
    }

    #[test]
    fn select_all_formats_and_replaces_every_paragraph_with_undo() {
        let mut editor = Editor::default();
        let mut state = AppState::default();
        editor.document.replace_plain_text("First\nSecond");
        editor.document = Document::from_bytes(&editor.document.to_bytes().unwrap()).unwrap();
        editor.format_command(&serde_json::json!({"type":"selectAll"}), &mut state);
        editor.format_command(&serde_json::json!({"type":"toggleMark","mark":"bold"}), &mut state);
        assert!(editor.document.paragraphs.iter().all(|paragraph| paragraph.runs[0].format.bold));
        editor.replace_document_selection("New\nText", &mut state);
        assert_eq!(editor.document.plain_text(), "New\nText");
        assert!(!editor.document_selected);
        let reopened = Document::from_bytes(&editor.document.to_bytes().unwrap()).unwrap();
        assert_eq!(reopened.plain_text(), "New\nText");
        editor.format_command(&serde_json::json!({"type":"undo"}), &mut state);
        assert_eq!(editor.document.plain_text(), "First\nSecond");
        assert!(editor.document_selected);
        editor.format_command(&serde_json::json!({"type":"redo"}), &mut state);
        assert_eq!(editor.document.plain_text(), "New\nText");
    }

    #[test]
    fn select_all_replacement_rejects_unrepresented_document_content() {
        use std::io::{Cursor, Write};
        for body in [
            "<w:p><w:r><w:t>Text</w:t></w:r></w:p><w:tbl/>",
            "<w:p><w:bookmarkStart w:id=\"0\" w:name=\"test\"/><w:r><w:t>Text</w:t></w:r></w:p>",
        ] {
            let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
            writer.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
            write!(writer, "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{body}</w:body></w:document>").unwrap();
            let mut editor = Editor::default();
            editor.document = Document::from_bytes(&writer.finish().unwrap().into_inner()).unwrap();
            let mut state = AppState::default();
            editor.format_command(&serde_json::json!({"type":"selectAll"}), &mut state);
            editor.replace_document_selection("Text", &mut state);
            assert_eq!(editor.document.plain_text(), "Text");
            assert!(state.surface.error.is_some());
            assert!(editor.document_selected);
            assert!(editor.undo.is_empty());
        }
    }

    #[test]
    fn select_all_formatting_rejects_unsupported_content_without_mutation() {
        use std::io::{Cursor, Write};
        for body in [
            "<w:p><w:r><w:t>Text</w:t></w:r></w:p><w:tbl/>",
            "<w:p><w:bookmarkStart w:id=\"0\" w:name=\"test\"/><w:r><w:t>Text</w:t></w:r></w:p>",
        ] {
            let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
            writer.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
            write!(writer, "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{body}</w:body></w:document>").unwrap();
            let mut editor = Editor::default();
            editor.document = Document::from_bytes(&writer.finish().unwrap().into_inner()).unwrap();
            let original = editor.document.clone();
            let original_format = editor.format.clone();
            editor.redo.push(editor.snapshot());
            let mut state = AppState::default();
            editor.format_command(&serde_json::json!({"type":"selectAll"}), &mut state);
            for command in [
                serde_json::json!({"type":"toggleMark","mark":"bold"}),
                serde_json::json!({"type":"clearFormatting"}),
                serde_json::json!({"type":"setAlignment","align":"center"}),
                serde_json::json!({"type":"toggleList","kind":"bullet"}),
                serde_json::json!({"type":"indent"}),
                serde_json::json!({"type":"setLineSpacing","value":2}),
                serde_json::json!({"type":"setColor","color":"FF0000"}),
            ] {
                state.surface.error = None;
                editor.format_command(&command, &mut state);
                assert_eq!(editor.document, original);
                assert_eq!(editor.format, original_format);
                assert!(state.surface.error.is_some());
                assert!(editor.undo.is_empty());
                assert_eq!(editor.redo.len(), 1);
            }
            state.surface.error = None;
            editor.apply_transform(&mut state, |format| format.bold = true);
            assert_eq!(editor.document, original);
            assert!(state.surface.error.is_some());
            assert!(editor.undo.is_empty());
        }
    }

    #[test]
    fn select_all_enter_and_backspace_replace_once() {
        for (key, expected, paragraph_count) in [
            (egui::Key::Enter, "\n", 2),
            (egui::Key::Backspace, "", 1),
        ] {
            let context = egui::Context::default();
            crate::theme::install_fonts(&context, crate::INTER_TTF, crate::INTER_MEDIUM_TTF, crate::INTER_SEMIBOLD_TTF);
            let mut editor = Editor::default();
            editor.document.replace_plain_text("First\nSecond");
            let mut state = AppState::default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))),
                ..Default::default()
            };
            let _ = context.run(input.clone(), |context| editor.show(context, &mut state, Theme::Light));
            let mut select = input.clone();
            select.events.push(egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::CTRL });
            let _ = context.run(select, |context| editor.show(context, &mut state, Theme::Light));
            assert!(editor.document_selected);
            let mut pressed = input;
            pressed.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
            let _ = context.run(pressed, |context| editor.show(context, &mut state, Theme::Light));
            assert_eq!(editor.document.plain_text(), expected);
            assert_eq!(editor.document.paragraphs.len(), paragraph_count);
            assert_eq!(editor.undo.len(), 1);
            assert!(!editor.document_selected);
        }
    }

    #[test]
    fn select_all_keyboard_replaces_all_paragraphs() {
        let context = egui::Context::default();
        crate::theme::install_fonts(&context, crate::INTER_TTF, crate::INTER_MEDIUM_TTF, crate::INTER_SEMIBOLD_TTF);
        let mut editor = Editor::default();
        editor.document.replace_plain_text("First\nSecond");
        let mut state = AppState::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))),
            ..Default::default()
        };
        let _ = context.run(input.clone(), |context| editor.show(context, &mut state, Theme::Light));
        let mut selected = input.clone();
        selected.events.push(egui::Event::Key {
            key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::CTRL,
        });
        let _ = context.run(selected, |context| editor.show(context, &mut state, Theme::Light));
        assert!(editor.document_selected);
        let mut copied = input.clone();
        copied.events.push(egui::Event::Copy);
        let output = context.run(copied, |context| editor.show(context, &mut state, Theme::Light));
        assert!(output.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text == "First\nSecond")));
        let mut typed = input;
        typed.events.push(egui::Event::Text("Replacement".into()));
        let _ = context.run(typed, |context| editor.show(context, &mut state, Theme::Light));
        assert_eq!(editor.document.plain_text(), "Replacement");
        assert_eq!(editor.document.paragraphs.len(), 1);
    }

    #[test]
    fn multiline_paste_creates_paragraphs_and_preserves_surrounding_formatting() {
        let mut editor = Editor::default();
        let mut state = AppState::default();
        let bold = TextFormat { bold: true, ..Default::default() };
        let runs = vec![Run { text: "BeforeAfter".into(), format: bold.clone() }];
        editor.document.paragraphs[0].runs = runs.clone();
        editor.document = Document::from_bytes(&editor.document.to_bytes().unwrap()).unwrap();
        editor.paste_paragraphs(0, &runs, 6..6, "One\nTwo\nThree", &TextFormat::default(), &mut state);

        assert_eq!(editor.document.plain_text(), "BeforeOne\nTwo\nThreeAfter");
        assert_eq!(editor.document.paragraphs.len(), 3);
        assert!(editor.document.paragraphs[0].runs[0].format.bold);
        assert!(editor.document.paragraphs[2].runs.last().unwrap().format.bold);
        assert_eq!(editor.selection, 5..5);
        let reopened = Document::from_bytes(&editor.document.to_bytes().unwrap()).unwrap();
        assert_eq!(reopened.plain_text(), editor.document.plain_text());
        assert!(reopened.paragraphs[0].runs[0].format.bold);
        assert!(!reopened.paragraphs[1].runs[0].format.bold);
        assert!(reopened.paragraphs[2].runs.last().unwrap().format.bold);
        editor.format_command(&serde_json::json!({"type":"undo"}), &mut state);
        assert_eq!(editor.document.plain_text(), "BeforeAfter");
        assert_eq!(editor.active, 0);
        assert_eq!(editor.selection, 6..6);
        editor.format_command(&serde_json::json!({"type":"redo"}), &mut state);
        assert_eq!(editor.document.paragraphs.len(), 3);
        assert_eq!(editor.selection, 5..5);
    }

    #[test]
    fn multiline_paste_keeps_repeated_existing_text_format() {
        let mut editor = Editor::default();
        let mut state = AppState::default();
        let bold = TextFormat { bold: true, ..Default::default() };
        let runs = vec![Run { text: "abc".into(), format: bold }];
        editor.document.paragraphs[0].runs = runs.clone();
        editor.paste_paragraphs(0, &runs, 0..0, "abc\n", &TextFormat::default(), &mut state);
        assert!(!editor.document.paragraphs[0].runs[0].format.bold);
        assert!(editor.document.paragraphs[1].runs[0].format.bold);
        assert_eq!(editor.selection, 0..0);
    }

    #[test]
    fn multiline_paste_keeps_format_on_empty_paragraphs() {
        let mut editor = Editor::default();
        let mut state = AppState::default();
        let bold = TextFormat { bold: true, ..Default::default() };
        let runs = editor.document.paragraphs[0].runs.clone();
        editor.paste_paragraphs(0, &runs, 0..0, "A\n\n", &bold, &mut state);
        assert_eq!(editor.document.paragraphs.len(), 3);
        assert!(editor.document.paragraphs.iter().all(|paragraph| paragraph.runs[0].format.bold));
    }

    #[test]
    fn rejected_split_preserves_redo_history() {
        use std::io::{Cursor, Write};
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:bookmarkStart w:id="0" w:name="test"/><w:r><w:t>Text</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
        let mut editor = Editor::default();
        editor.document = Document::from_bytes(&writer.finish().unwrap().into_inner()).unwrap();
        editor.redo.push(editor.snapshot());
        let mut state = AppState::default();
        editor.split_paragraph(&mut state);
        assert_eq!(editor.redo.len(), 1);
        assert_eq!(editor.document.plain_text(), "Text");
        assert!(state.surface.error.is_some());
    }

    #[test]
    fn native_surface_pastes_multiple_paragraphs() {
        let context = egui::Context::default();
        crate::theme::install_fonts(&context, crate::INTER_TTF, crate::INTER_MEDIUM_TTF, crate::INTER_SEMIBOLD_TTF);
        let mut editor = Editor::default();
        let mut state = AppState::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))),
            ..Default::default()
        };
        let _ = context.run(input.clone(), |context| editor.show(context, &mut state, Theme::Light));
        let mut pasted = input;
        pasted.events.push(egui::Event::Paste("First\r\nSecond".into()));
        let _ = context.run(pasted, |context| editor.show(context, &mut state, Theme::Light));
        assert_eq!(editor.document.paragraphs.len(), 2);
        assert_eq!(editor.document.plain_text(), "First\nSecond");
    }

    #[test]
    fn native_surface_renders_and_types_with_pending_marks() {
        let context = egui::Context::default();
        crate::theme::install_fonts(
            &context,
            crate::INTER_TTF,
            crate::INTER_MEDIUM_TTF,
            crate::INTER_SEMIBOLD_TTF,
        );
        let mut editor = Editor::default();
        let mut state = AppState::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(1280.0, 800.0),
            )),
            ..Default::default()
        };
        let _ = context.run(input.clone(), |context| {
            editor.show(context, &mut state, Theme::Light)
        });
        editor.execute(
            "editorExec",
            &serde_json::json!({"type":"toggleMark","mark":"bold"}),
            &mut state,
        );
        let mut typed = input;
        typed.events.push(egui::Event::Text("Hello 世界".into()));
        let _ = context.run(typed, |context| {
            editor.show(context, &mut state, Theme::Light)
        });
        assert_eq!(editor.document.plain_text(), "Hello 世界");
        assert!(editor.document.paragraphs[0].runs[0].format.bold);
        assert!(state.surface.dirty);
        assert!(state.surface.ready);
        assert_eq!(state.surface.statistics.words, 3);
    }

    #[test]
    fn editing_preserves_unmodified_run_formats_and_unicode() {
        let bold = TextFormat {
            bold: true,
            ..Default::default()
        };
        let normal = TextFormat::default();
        let runs = vec![
            Run {
                text: "Hello".into(),
                format: bold.clone(),
            },
            Run {
                text: " 世界".into(),
                format: normal.clone(),
            },
        ];
        let result = edit_runs(&runs, "Hello 世界", "Hello Rust 世界", &normal);
        assert_eq!(
            result
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            "Hello Rust 世界"
        );
        assert_eq!(result[0].format, bold);
        assert_eq!(result.last().unwrap().text, "世界");
    }

    #[test]
    fn deleting_across_runs_keeps_surviving_formats() {
        let bold = TextFormat {
            bold: true,
            ..Default::default()
        };
        let runs = vec![
            Run {
                text: "abc".into(),
                format: bold.clone(),
            },
            Run {
                text: "def".into(),
                format: TextFormat::default(),
            },
        ];
        let result = edit_runs(&runs, "abcdef", "af", &TextFormat::default());
        assert_eq!(result[0].text, "a");
        assert_eq!(result[0].format, bold);
        assert_eq!(result[1].text, "f");
    }

    #[test]
    fn toggling_a_mark_preserves_other_run_properties() {
        let mut editor = Editor::default();
        let mut state = AppState::default();
        let format = TextFormat {
            italic: true,
            color: Some("FF0000".into()),
            font_size: Some(18.0),
            ..Default::default()
        };
        editor.document.paragraphs[0].runs = vec![Run {
            text: "Selected".into(),
            format: format.clone(),
        }];
        editor.selection = 0..8;
        assert!(editor.execute(
            "editorExec",
            &serde_json::json!({"type":"toggleMark","mark":"bold"}),
            &mut state
        ));
        let actual = &editor.document.paragraphs[0].runs[0].format;
        assert!(actual.bold);
        assert!(actual.italic);
        assert_eq!(actual.color, format.color);
        assert_eq!(actual.font_size, format.font_size);
        assert!(state.surface.dirty);
    }

    #[test]
    fn splitting_and_merging_paragraphs_preserves_formatted_text() {
        let mut editor = Editor::default();
        let mut state = AppState::default();
        let format = TextFormat {
            bold: true,
            ..Default::default()
        };
        editor.document.paragraphs[0].runs = vec![Run {
            text: "Hello世界".into(),
            format: format.clone(),
        }];
        editor.selection = 5..5;
        editor.split_paragraph(&mut state);
        assert_eq!(editor.document.paragraphs.len(), 2);
        assert_eq!(editor.document.paragraphs[0].plain_text(), "Hello");
        assert_eq!(editor.document.paragraphs[1].plain_text(), "世界");
        assert_eq!(editor.document.paragraphs[1].runs[0].format, format);
        editor.merge_paragraph(&mut state);
        assert_eq!(editor.document.paragraphs.len(), 1);
        assert_eq!(editor.document.plain_text(), "Hello世界");
        assert_eq!(editor.selection, 5..5);
    }
}
