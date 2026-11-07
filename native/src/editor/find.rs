use super::*;
use crate::icons::{self, IconName};
use unicode_segmentation::UnicodeSegmentation;

impl Editor {
    pub(super) fn find_bar(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        let colors = UiColors::for_theme(if ui.visuals().dark_mode {
            Theme::Dark
        } else {
            Theme::Light
        });
        Frame::new()
            .fill(colors.app_surface)
            .stroke(Stroke::new(1.0_f32, colors.app_border))
            .corner_radius(12)
            .inner_margin(Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let (heading, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 26.0), egui::Sense::hover());
                ui.painter().text(
                    heading.left_center(),
                    egui::Align2::LEFT_CENTER,
                    "Find In Document",
                    crate::theme::ui_font_id_weight(12.0, 600),
                    colors.app_text,
                );
                let close = egui::Rect::from_min_size(
                    egui::pos2(heading.right() - 26.0, heading.top()),
                    vec2(26.0, 26.0),
                );
                if find_icon(ui, close, IconName::Close, "Close Find", true, &colors).clicked() {
                    state.surface.find_open = false;
                }
                ui.add_space(8.0);
                let (matches, truncated) = self.find_matches();
                let plus = if truncated { "+" } else { "" };
                let count = format!("{}{}", matches.len(), plus);
                let wide = ui.available_width() >= 740.0;
                let mut navigate = 0;
                let mut query_changed = false;
                let mut options_changed = false;
                let mut input_focused = false;
                ui.horizontal(|ui| {
                    let input_width = if wide {
                        ui.available_width() - 370.0
                    } else {
                        ui.available_width()
                    };
                    let input_frame = Frame::new()
                        .stroke(Stroke::new(1.0_f32, colors.app_border))
                        .corner_radius(5)
                        .inner_margin(Margin::symmetric(10, 4))
                        .show(ui, |ui| {
                            ui.set_width((input_width - 20.0).max(80.0));
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                let (icon, _) =
                                    ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::hover());
                                icons::paint_icon_at(
                                    ui.ctx(),
                                    ui.painter(),
                                    IconName::Search,
                                    icon,
                                    colors.app_muted,
                                );
                                let count_width = ui
                                    .painter()
                                    .layout_no_wrap(
                                        count.clone(),
                                        FontId::proportional(12.0),
                                        colors.app_muted,
                                    )
                                    .size()
                                    .x
                                    .max(18.0);
                                let edit = ui.add_sized(
                                    vec2(
                                        (ui.available_width() - count_width - 8.0).max(30.0),
                                        18.0,
                                    ),
                                    TextEdit::singleline(&mut self.find)
                                        .id_salt("find-text")
                                        .frame(false)
                                        .margin(egui::Vec2::ZERO)
                                        .font(FontId::proportional(12.0))
                                        .hint_text("Search Text"),
                                );
                                if self.find_needs_focus {
                                    edit.request_focus();
                                    self.find_needs_focus = false;
                                }
                                query_changed = edit.changed();
                                input_focused = edit.has_focus();
                                if (edit.has_focus() || edit.lost_focus())
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                                {
                                    edit.request_focus();
                                    navigate = if ui.input(|input| input.modifiers.shift) {
                                        -1
                                    } else {
                                        1
                                    };
                                }
                                ui.label(RichText::new(&count).size(12.0).color(colors.app_muted));
                            });
                        });
                    if input_focused {
                        ui.painter().rect_stroke(
                            input_frame.response.rect,
                            5,
                            Stroke::new(2.0_f32, colors.app_accent),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if wide {
                        self.find_options(
                            ui,
                            &colors,
                            !matches.is_empty(),
                            &mut options_changed,
                            &mut navigate,
                        );
                    }
                });
                if !wide {
                    ui.horizontal_wrapped(|ui| {
                        self.find_options(
                            ui,
                            &colors,
                            !matches.is_empty(),
                            &mut options_changed,
                            &mut navigate,
                        )
                    });
                }
                if query_changed || options_changed {
                    self.active = 0;
                    self.selection = 0..0;
                    self.pending_selection = None;
                    self.find_next();
                } else if navigate > 0 {
                    self.find_next();
                } else if navigate < 0 {
                    self.find_previous();
                }
                if self.replace_open {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let width = (ui.available_width() - 190.0).max(80.0);
                        let replacement = ui.add_sized(
                            vec2(width, 34.0),
                            TextEdit::singleline(&mut self.replacement)
                                .hint_text("Replace With")
                                .font(FontId::proportional(12.0)),
                        );
                        if (replacement.has_focus() || replacement.lost_focus())
                            && ui.input(|input| input.key_pressed(egui::Key::Enter))
                        {
                            replacement.request_focus();
                            if ui.input(|input| input.modifiers.ctrl) {
                                self.replace_all(state);
                            } else {
                                self.replace_selection(state);
                            }
                        }
                        let enabled = !matches.is_empty();
                        let dark = colors.app_bg == Color32::from_rgb(0x11, 0x13, 0x18);
                        for (label, primary, action) in [
                            ("Replace", false, 0),
                            ("Replace All", true, 1),
                        ] {
                            let font = FontId::proportional(11.0);
                            let text_w = ui
                                .painter()
                                .layout_no_wrap(label.to_string(), font.clone(), colors.app_text)
                                .size()
                                .x;
                            let (rect, response) = ui.allocate_exact_size(
                                vec2(text_w + 28.0, 24.0),
                                egui::Sense::click(),
                            );
                            let hovered = enabled && response.hovered();
                            let alpha = if enabled { 1.0 } else { 0.4 };
                            let (bg, fg, stroke) = if primary {
                                let ink = if dark {
                                    Color32::from_rgb(0x12, 0x24, 0x49)
                                } else {
                                    Color32::WHITE
                                };
                                (
                                    if hovered {
                                        colors.app_accent_hover
                                    } else {
                                        colors.app_accent
                                    },
                                    ink,
                                    Stroke::NONE,
                                )
                            } else {
                                (
                                    if hovered {
                                        colors.app_hover
                                    } else {
                                        Color32::TRANSPARENT
                                    },
                                    colors.app_accent_ink,
                                    Stroke::new(1.0_f32, colors.app_border),
                                )
                            };
                            ui.painter().rect_filled(rect, 6.0, bg);
                            if stroke.width > 0.0 {
                                ui.painter().rect_stroke(
                                    rect,
                                    6.0,
                                    Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgba_unmultiplied(
                                            colors.app_border.r(),
                                            colors.app_border.g(),
                                            colors.app_border.b(),
                                            (colors.app_border.a() as f32 * alpha) as u8,
                                        ),
                                    ),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                label,
                                font,
                                Color32::from_rgba_unmultiplied(
                                    (fg.r() as f32 * alpha) as u8,
                                    (fg.g() as f32 * alpha) as u8,
                                    (fg.b() as f32 * alpha) as u8,
                                    255,
                                ),
                            );
                            if enabled && response.clicked() {
                                if action == 0 {
                                    self.replace_selection(state);
                                } else {
                                    self.replace_all(state);
                                }
                            }
                        }
                    });
                }
            });
    }

    fn find_options(
        &mut self,
        ui: &mut egui::Ui,
        colors: &UiColors,
        enabled: bool,
        changed: &mut bool,
        navigate: &mut i32,
    ) {
        ui.spacing_mut().item_spacing.x = 8.0;
        *changed |= ui
            .checkbox(&mut self.match_case, RichText::new("Match Case").size(12.0))
            .changed();
        *changed |= ui
            .checkbox(
                &mut self.whole_words,
                RichText::new("Whole Words").size(12.0),
            )
            .changed();
        for (icon, label, direction) in [
            (IconName::ArrowUp, "Previous Match", -1),
            (IconName::ArrowDown, "Next Match", 1),
        ] {
            let (rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), egui::Sense::hover());
            if find_icon(ui, rect, icon, label, enabled, colors).clicked() && enabled {
                *navigate = direction;
            }
        }
        let (rect, response) = ui.allocate_exact_size(vec2(64.0, 30.0), egui::Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, 6.0, colors.app_hover);
        }
        icons::paint_icon_at(
            ui.ctx(),
            ui.painter(),
            IconName::ChevronDown,
            egui::Rect::from_center_size(
                egui::pos2(rect.left() + 8.0, rect.center().y),
                vec2(12.0, 12.0),
            ),
            colors.app_accent_ink,
        );
        ui.painter().text(
            egui::pos2(rect.left() + 18.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Replace",
            FontId::proportional(12.0),
            colors.app_accent_ink,
        );
        if response.clicked() {
            self.replace_open = !self.replace_open;
        }
    }

    fn find_matches(&self) -> (Vec<(usize, Range<usize>)>, bool) {
        if self.find.is_empty() {
            return (Vec::new(), false);
        }
        let mut matches: Vec<_> = self
            .document
            .paragraphs
            .iter()
            .enumerate()
            .flat_map(|(index, paragraph)| {
                match_ranges(
                    &paragraph.plain_text(),
                    &self.find,
                    self.match_case,
                    self.whole_words,
                )
                .into_iter()
                .map(move |range| (index, range))
            })
            .collect();
        let truncated = matches.len() > 1000;
        matches.truncate(1000);
        (matches, truncated)
    }

    pub(super) fn find_next(&mut self) {
        self.navigate_match(false);
    }
    fn find_previous(&mut self) {
        self.navigate_match(true);
    }

    fn navigate_match(&mut self, previous: bool) {
        let (matches, _) = self.find_matches();
        if matches.is_empty() {
            return;
        }
        let current = matches
            .iter()
            .position(|(index, range)| *index == self.active && *range == self.selection);
        let index = if previous {
            current
                .map(|current| (current + matches.len() - 1) % matches.len())
                .unwrap_or_else(|| {
                    matches
                        .iter()
                        .rposition(|(index, range)| {
                            *index < self.active
                                || (*index == self.active && range.end <= self.selection.start)
                        })
                        .unwrap_or(matches.len() - 1)
                })
        } else {
            current
                .map(|current| (current + 1) % matches.len())
                .unwrap_or_else(|| {
                    matches
                        .iter()
                        .position(|(index, range)| {
                            *index > self.active
                                || (*index == self.active && range.start >= self.selection.end)
                        })
                        .unwrap_or(0)
                })
        };
        self.active = matches[index].0;
        self.document_selected = false;
        self.cross_selection = None;
        self.selection = matches[index].1.clone();
        self.pending_selection = Some((self.active, self.selection.clone()));
        self.scroll_paragraph = Some(self.active);
    }

    fn replace_selection(&mut self, state: &mut AppState) {
        if !self
            .find_matches()
            .0
            .iter()
            .any(|(index, range)| *index == self.active && *range == self.selection)
        {
            self.find_next();
            return;
        }
        self.checkpoint();
        let paragraph = &mut self.document.paragraphs[self.active];
        let text = paragraph.plain_text();
        let mut next = text.clone();
        next.replace_range(
            char_byte(&text, self.selection.start)..char_byte(&text, self.selection.end),
            &self.replacement,
        );
        paragraph.runs = edit_runs(
            &paragraph.runs,
            &text,
            &next,
            &format_at(&paragraph.runs, self.selection.start),
        );
        self.selection =
            self.selection.start..self.selection.start + self.replacement.chars().count();
        self.changed(state);
        self.find_next();
    }

    fn replace_all(&mut self, state: &mut AppState) {
        let (matches, _) = self.find_matches();
        if matches.is_empty() {
            return;
        }
        self.checkpoint();
        for (index, range) in matches.into_iter().rev() {
            let paragraph = &mut self.document.paragraphs[index];
            let mut runs = slice_runs(&paragraph.runs, 0..range.start);
            runs.push(Run {
                text: self.replacement.clone(),
                format: format_at(&paragraph.runs, range.start),
            });
            runs.extend(slice_runs(&paragraph.runs, range.end..usize::MAX));
            paragraph.runs = runs;
        }
        self.selection = 0..0;
        self.document_selected = false;
        self.cross_selection = None;
        self.pending_selection = None;
        self.changed(state);
    }
}

pub(super) fn match_ranges(
    text: &str,
    query: &str,
    match_case: bool,
    whole_words: bool,
) -> Vec<Range<usize>> {
    let query_length = query.chars().count();
    if query_length == 0 {
        return Vec::new();
    }
    let boundaries: Vec<_> = text
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .collect();
    let words: Vec<_> = if whole_words {
        text.unicode_word_indices()
            .map(|(index, word)| index..index + word.len())
            .collect()
    } else {
        Vec::new()
    };
    let lowercase = query.to_lowercase();
    let mut matches = Vec::new();
    let mut index = 0;
    while index + query_length < boundaries.len() {
        let range = boundaries[index]..boundaries[index + query_length];
        let candidate = &text[range.clone()];
        let equal = if match_case {
            candidate == query
        } else {
            candidate.to_lowercase() == lowercase
        };
        let bounded = !whole_words
            || (words.iter().any(|word| word.start == range.start)
                && words.iter().any(|word| word.end == range.end));
        if equal && bounded {
            matches.push(index..index + query_length);
            index += query_length;
        } else {
            index += 1;
        }
    }
    matches
}

fn find_icon(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    icon: IconName,
    label: &str,
    enabled: bool,
    colors: &UiColors,
) -> egui::Response {
    let response = ui.interact(rect, ui.make_persistent_id(label), egui::Sense::click());
    if enabled && response.hovered() {
        ui.painter()
            .circle_filled(rect.center(), rect.width() * 0.5, colors.app_hover);
    }
    icons::paint_icon_at(
        ui.ctx(),
        ui.painter(),
        icon,
        egui::Rect::from_center_size(rect.center(), vec2(18.0, 18.0)),
        if enabled {
            colors.app_muted
        } else {
            colors.app_muted.gamma_multiply(0.4)
        },
    );
    response.on_hover_text(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_options_respect_case_words_and_unicode_positions() {
        assert_eq!(
            match_ranges("Cat cat scatter", "cat", false, true),
            vec![0..3, 4..7]
        );
        assert_eq!(
            match_ranges("Cat cat scatter", "cat", true, true),
            vec![4..7]
        );
        assert_eq!(match_ranges("é Café CAFE", "café", false, true), vec![2..6]);
        assert_eq!(match_ranges("你好 世界", "世界", true, true), vec![3..5]);
    }

    #[test]
    fn navigation_wraps_and_filtered_replace_preserves_formatting() {
        let mut editor = Editor::default();
        editor.document.replace_plain_text("Cat cat scatter\ncat");
        editor.document.paragraphs[1].runs[0].format.bold = true;
        editor.find = "cat".into();
        editor.whole_words = true;
        editor.find_next();
        assert_eq!(editor.selection, 0..3);
        editor.find_previous();
        assert_eq!(editor.active, 1);
        editor.match_case = true;
        editor.replacement = "Dog".into();
        let mut state = AppState::default();
        editor.replace_all(&mut state);
        assert_eq!(editor.document.plain_text(), "Cat Dog scatter\nDog");
        assert!(editor.document.paragraphs[1].runs[0].format.bold);
        assert!(state.surface.dirty);
    }
}
