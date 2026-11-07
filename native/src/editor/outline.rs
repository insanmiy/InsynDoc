use super::*;

impl Editor {
    pub(super) fn outline(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut AppState,
        colors: &UiColors,
        theme: Theme,
    ) {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.painter().vline(
            ui.max_rect().right() + 14.0,
            ui.max_rect().y_range(),
            Stroke::new(1.0_f32, colors.app_border),
        );
        let (heading, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 26.0), egui::Sense::hover());
        ui.painter().text(
            heading.left_center() + vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "Document Outline",
            crate::theme::ui_font_id_weight(12.0, 600),
            colors.app_text,
        );
        let close_rect = egui::Rect::from_min_size(
            egui::pos2(heading.right() - 26.0, heading.top()),
            vec2(26.0, 26.0),
        );
        let close = ui.interact(
            close_rect,
            ui.make_persistent_id("outline-close"),
            egui::Sense::click(),
        );
        if close.hovered() {
            ui.painter()
                .circle_filled(close_rect.center(), 13.0, colors.app_hover);
        }
        crate::icons::paint_icon_at(
            ui.ctx(),
            ui.painter(),
            crate::icons::IconName::Close,
            close_rect.shrink(5.0),
            colors.app_muted,
        );
        if close.clicked() {
            state.preferences.show_outline = false;
            self.queued_commands
                .push(crate::ui::UiCommand::SavePreferences);
        }
        let headings: Vec<_> = self
            .document
            .paragraphs
            .iter()
            .enumerate()
            .filter(|(_, paragraph)| {
                paragraph
                    .style
                    .as_deref()
                    .is_some_and(|style| style.to_ascii_lowercase().starts_with("heading"))
            })
            .map(|(index, paragraph)| (index, paragraph.plain_text()))
            .collect();
        if !headings.is_empty() {
            ui.add_space(16.0);
            for (index, text) in headings {
                if ui.selectable_label(index == self.active, text).clicked() {
                    self.scroll_paragraph = Some(index);
                }
            }
            return;
        }
        ui.add_space(30.0);
        let (empty, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 125.0), egui::Sense::hover());
        let start = empty.min + vec2(12.0, 0.0);
        let bar_color = if theme == Theme::Dark {
            colors.app_muted
        } else {
            colors.app_border
        };
        for (index, (offset, width)) in [(0.0, 50.0), (8.0, 35.0), (8.0, 42.0)]
            .into_iter()
            .enumerate()
        {
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    start + vec2(offset, index as f32 * 10.0),
                    vec2(width, 3.0),
                ),
                1.5,
                bar_color,
            );
        }
        ui.painter().text(
            start + vec2(0.0, 43.0),
            egui::Align2::LEFT_TOP,
            "Your Document, At A Glance",
            crate::theme::ui_font_id_weight(13.0, 500),
            colors.app_text,
        );
        for (index, text) in ["Add Headings To Navigate", "Your Document Here."]
            .into_iter()
            .enumerate()
        {
            ui.painter().text(
                start + vec2(0.0, 73.0 + index as f32 * 21.6),
                egui::Align2::LEFT_TOP,
                text,
                FontId::proportional(12.0),
                colors.app_muted,
            );
        }
    }
}
