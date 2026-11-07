use super::*;

impl Editor {
    pub(super) fn show_menu_search(
        &mut self,
        ctx: &egui::Context,
        state: &mut AppState,
        colors: &UiColors,
    ) {
        if !self.search_open {
            return;
        }
        let mut selected = None;
        let mut close = false;
        let response = egui::Modal::new(egui::Id::new("menu-search"))
            .backdrop_color(colors.modal_backdrop)
            .frame(
                Frame::new()
                    .fill(colors.app_surface)
                    .stroke(Stroke::new(1.0_f32, colors.app_border))
                    .corner_radius(18)
                    .inner_margin(28),
            )
            .show(ctx, |ui| {
                ui.set_width(462.0_f32.min(ctx.screen_rect().width() - 80.0));
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new("Search The Menus")
                            .font(crate::theme::ui_font_id_weight(22.0, 600)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if crate::ui::icon_button(
                            ui,
                            colors,
                            crate::icons::IconName::Close,
                            18.0,
                            "Close",
                        )
                        .clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.add_space(16.0);
                let search = Frame::new()
                    .inner_margin(Margin::symmetric(0, 12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 12.0;
                            let (icon, _) =
                                ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::hover());
                            crate::icons::paint_icon_at(
                                ui.ctx(),
                                ui.painter(),
                                crate::icons::IconName::Search,
                                icon,
                                colors.app_muted,
                            );
                            let edit = ui.add_sized(
                                vec2((ui.available_width() - 55.0).max(40.0), 24.0),
                                TextEdit::singleline(&mut self.search_query)
                                    .hint_text("Search For A Tool Or Action")
                                    .font(FontId::proportional(14.0))
                                    .frame(false),
                            );
                            if self.search_needs_focus {
                                edit.request_focus();
                                self.search_needs_focus = false;
                            }
                            ui.label(RichText::new("Alt+/").size(11.0).color(colors.app_muted));
                        });
                    });
                ui.painter().hline(
                    search.response.rect.x_range(),
                    search.response.rect.bottom(),
                    Stroke::new(2.0_f32, colors.app_accent),
                );
                ui.add_space(12.0);
                let query = self.search_query.trim().to_lowercase();
                let results_height = 360.0_f32.min(ctx.screen_rect().height() * 0.5);
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), results_height),
                    egui::Layout::top_down(Align::LEFT),
                    |ui| {
                        ui.set_min_height(results_height);
                        ui.spacing_mut().scroll.foreground_color = false;
                        ui.spacing_mut().scroll.floating = false;
                        ui.visuals_mut().widgets.inactive.bg_fill = colors.app_border;
                        egui::ScrollArea::vertical()
                            .max_height(results_height)
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let mut found = false;
                                for &(label, command, shortcut) in MENU_COMMANDS {
                                    if !label.to_lowercase().contains(&query) {
                                        continue;
                                    }
                                    found = true;
                                    let (rect, response) = ui.allocate_exact_size(
                                        vec2(ui.available_width(), 42.0),
                                        egui::Sense::click(),
                                    );
                                    if response.hovered() {
                                        ui.painter().rect_filled(rect, 5.0, colors.app_accent_soft);
                                    }
                                    ui.painter().text(
                                        rect.left_center() + vec2(10.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        label,
                                        FontId::proportional(13.0),
                                        colors.app_text,
                                    );
                                    if !shortcut.is_empty() {
                                        let font = FontId::proportional(11.0);
                                        ui.painter().text(
                                            rect.right_center() - vec2(10.0, 0.0),
                                            egui::Align2::RIGHT_CENTER,
                                            shortcut,
                                            font,
                                            colors.app_muted,
                                        );
                                    }
                                    if response.clicked() {
                                        selected = Some(command);
                                    }
                                }
                                if !found {
                                    ui.label(
                                        RichText::new("No Matching Tools. Try Another Search.")
                                            .size(13.0)
                                            .color(colors.app_muted),
                                    );
                                }
                            });
                    },
                );
            });
        if close || response.should_close() {
            self.search_open = false;
        }
        if let Some(command) = selected {
            self.search_open = false;
            self.activate_menu_command(command, state);
        }
    }

    fn activate_menu_command(&mut self, command: &str, state: &mut AppState) {
        match command {
            "settings" => state.modal = Modal::Settings,
            "shortcuts" => state.modal = Modal::Shortcuts,
            "outline" => {
                state.preferences.show_outline = !state.preferences.show_outline;
                self.queued_commands
                    .push(crate::ui::UiCommand::SavePreferences);
            }
            "focusMode" => state.surface.focus_mode = !state.surface.focus_mode,
            "bold" | "italic" | "underline" | "strike" | "superscript" | "subscript" => {
                self.execute(
                    "editorExec",
                    &serde_json::json!({"type":"toggleMark","mark":command}),
                    state,
                );
            }
            "left" | "center" | "right" | "justify" => {
                self.execute(
                    "editorExec",
                    &serde_json::json!({"type":"setAlignment","align":command}),
                    state,
                );
            }
            "bullet" | "number" => {
                self.execute("toggleList", &serde_json::json!({"kind":command}), state);
            }
            "indent" | "outdent" | "clearFormatting" => {
                self.execute("editorExec", &serde_json::json!({"type":command}), state);
            }
            _ => self.queued_commands.push(crate::ui::UiCommand::Command {
                name: command.into(),
                args: Value::Null,
            }),
        }
    }
}

const MENU_COMMANDS: &[(&str, &str, &str)] = &[
    ("New Document", "newDocument", "Ctrl+N"),
    ("Open Document", "open", "Ctrl+O"),
    ("Save Document", "save", "Ctrl+S"),
    ("Export DOCX", "saveAs", "Ctrl+Shift+S"),
    ("Check Spelling", "checkSpelling", "Ctrl+Alt+X"),
    ("Find In Document", "find", "Ctrl+F"),
    ("Find And Replace", "findReplace", "Ctrl+Shift+I"),
    ("Bold", "bold", "Ctrl+B"),
    ("Italic", "italic", "Ctrl+I"),
    ("Underline", "underline", "Ctrl+U"),
    ("Strikethrough", "strike", ""),
    ("Superscript", "superscript", ""),
    ("Subscript", "subscript", ""),
    ("Align Left", "left", ""),
    ("Align Center", "center", ""),
    ("Align Right", "right", ""),
    ("Justify", "justify", ""),
    ("Bulleted List", "bullet", ""),
    ("Numbered List", "number", ""),
    ("Increase Indent", "indent", ""),
    ("Decrease Indent", "outdent", ""),
    ("Clear Formatting", "clearFormatting", ""),
    ("Word Count", "wordCount", ""),
    ("Insert Date And Time", "insertDateTime", ""),
    ("Special Characters", "insertSymbols", ""),
    ("Insert Custom Table", "insertTable", ""),
    ("Math Equation", "insertMath", ""),
    ("Edit Header", "editHeader", ""),
    ("Edit Footer", "editFooter", ""),
    ("Go To Page", "goToPage", "Ctrl+G"),
    ("Document Outline", "outline", ""),
    ("Focus Mode", "focusMode", "Ctrl+Shift+F"),
    ("Settings", "settings", "Ctrl+,"),
    ("Keyboard Shortcuts", "shortcuts", "Ctrl+/"),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn palette_dispatches_formatting_and_view_commands() {
        let mut editor = Editor::default();
        let mut state = AppState::default();
        editor.document.replace_plain_text("Selected Text");
        editor.selection = 0..8;
        editor.activate_menu_command("bold", &mut state);
        assert!(editor.document.paragraphs[0].runs[0].format.bold);
        editor.activate_menu_command("center", &mut state);
        assert_eq!(editor.document.paragraphs[0].alignment, Alignment::Center);
        editor.activate_menu_command("number", &mut state);
        assert_eq!(
            editor.document.paragraphs[0].list.as_deref(),
            Some("number")
        );
        editor.activate_menu_command("outline", &mut state);
        assert!(!state.preferences.show_outline);
        assert!(matches!(
            editor.queued_commands.last(),
            Some(crate::ui::UiCommand::SavePreferences)
        ));
    }
}
