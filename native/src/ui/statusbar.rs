//! `.status-bar` — 34px, border-top app_border, bg app_surface, padding 0 20,
//! 12px muted.
//!
//! Left (.document-status): "{n} Words" (or "{n} / {goal} Words" when
//! word_goal > 0) -> Statistics modal, only when prefs.show_word_count;
//! separator; "{n} Page(s)" -> Go To Page modal (disabled when !ready).
//! Right: zoom stepper (`- 100% +`, % click fits width) -> zoomStep/setZoom
//! execs, separator, then "View: Separate Pages|Continuous" toggle with page
//! icon -> Command setView.

use egui::{vec2, Align, Align2, FontId, Layout, Pos2, Sense};

use crate::icons::IconName;
use crate::state::{AppState, Modal};
use crate::theme::UiColors;

use super::icons_paint;
use super::{colors, fmt_num, status_separator, ChromeAssets, UiOutput};

pub fn draw(ui: &mut egui::Ui, state: &mut AppState, assets: &ChromeAssets, out: &mut UiOutput) {
    let c = colors(assets);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
        ui.set_height(34.0);
        ui.vertical(|ui| {
            ui.set_height(34.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
                if state.preferences.show_word_count {
                    draw_word_count(ui, &c, state);
                    status_separator(ui, &c);
                }
                draw_page_button(ui, &c, state);
            });
        });

        // Right group, drawn right-to-left so the view toggle is rightmost.
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
            draw_view_toggle(ui, &c, state, out);
            status_separator(ui, &c);
            draw_zoom(ui, &c, state, out);
        });
    });
}

fn status_button(
    ui: &mut egui::Ui,
    c: &UiColors,
    content: impl FnOnce(&mut egui::Ui, egui::Rect, egui::Response),
    width: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(5), c.app_hover);
    }
    content(ui, rect, response.clone());
    response
}

fn draw_word_count(ui: &mut egui::Ui, c: &UiColors, state: &mut AppState) {
    let words = state.surface.statistics.words;
    let goal = state.preferences.word_goal as u64;
    let label_text = if goal > 0 {
        format!("{} / {} Words", fmt_num(words), fmt_num(goal))
    } else if words == 1 {
        "1 Word".to_string()
    } else {
        format!("{} Words", fmt_num(words))
    };
    let font = FontId::proportional(12.0);
    let w = ui
        .painter()
        .layout_no_wrap(label_text.clone(), font.clone(), c.app_muted)
        .size()
        .x
        + 14.0;
    let resp = status_button(
        ui,
        c,
        |ui, rect, _| {
            ui.painter().text(
                Pos2::new(rect.left() + 7.0, rect.center().y),
                Align2::LEFT_CENTER,
                label_text,
                font,
                c.app_muted,
            );
        },
        w.max(40.0),
    );
    if resp.clicked() {
        state.modal = Modal::Statistics;
    }
}

fn draw_page_button(ui: &mut egui::Ui, c: &UiColors, state: &mut AppState) {
    let pages = state.surface.page_count.max(1);
    let label_text = if pages == 1 {
        "1 Page".to_string()
    } else {
        format!("{} Pages", fmt_num(pages as u64))
    };
    let font = FontId::proportional(12.0);
    let w = ui
        .painter()
        .layout_no_wrap(label_text.clone(), font.clone(), c.app_muted)
        .size()
        .x
        + 14.0;
    let enabled = state.surface.ready;
    let resp = status_button(
        ui,
        c,
        |ui, rect, _| {
            let color = if enabled {
                c.app_muted
            } else {
                dim(c.app_muted, 0.4)
            };
            ui.painter().text(
                Pos2::new(rect.left() + 7.0, rect.center().y),
                Align2::LEFT_CENTER,
                label_text,
                font,
                color,
            );
        },
        w.max(40.0),
    );
    if resp.clicked() && enabled {
        state.modal = Modal::GoToPage;
        state.page_error = None;
    }
}

fn zoom_glyph(ui: &egui::Ui, glyph: &str, size: f32, color: egui::Color32) -> f32 {
    let font = FontId::proportional(size);
    ui.painter()
        .layout_no_wrap(glyph.to_string(), font, color)
        .size()
        .x
}

fn draw_zoom(ui: &mut egui::Ui, c: &UiColors, state: &mut AppState, out: &mut UiOutput) {
    let enabled = state.surface.ready && !state.surface.busy;
    let zoom = state.surface.zoom.max(0.1);
    let percent = format!("{}%", (zoom * 100.0).round() as i64);
    let glyph_w = zoom_glyph(ui, "-", 14.0, c.app_muted).max(10.0);
    let label_w = 70.0;
    let plus_w = zoom_glyph(ui, "+", 14.0, c.app_muted).max(10.0);
    let group_width = glyph_w + plus_w + label_w + 20.0;
    ui.allocate_ui_with_layout(
        vec2(group_width, 28.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
            let minus = zoom_button(
                ui,
                c,
                enabled,
                vec2(glyph_w + 8.0, 28.0),
                |ui, rect, color| {
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "-",
                        FontId::proportional(14.0),
                        color,
                    );
                },
            );
            if minus.clicked() && enabled {
                out.exec("zoomStep", serde_json::json!({"direction":"out"}));
            }
            let label = zoom_button(ui, c, enabled, vec2(label_w, 28.0), |ui, rect, color| {
                ui.painter().text(
                    Pos2::new(rect.left() + 10.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    &percent,
                    FontId::proportional(12.0),
                    color,
                );
                crate::icons::paint_icon_at(
                    ui.ctx(),
                    ui.painter(),
                    IconName::ChevronDown,
                    egui::Rect::from_center_size(
                        Pos2::new(rect.right() - 11.0, rect.center().y),
                        vec2(10.0, 10.0),
                    ),
                    color,
                );
            });
            let popup_id = ui.make_persistent_id("status_zoom_menu");
            if label.clicked() && enabled {
                ui.memory_mut(|memory| memory.toggle_popup(popup_id));
            }
            egui::popup_below_widget(
                ui,
                popup_id,
                &label,
                egui::PopupCloseBehavior::CloseOnClick,
                |ui| {
                    for zoom in [0.5, 0.75, 1.0, 1.25, 1.5, 2.0] {
                        if ui
                            .selectable_label(
                                (state.surface.zoom - zoom).abs() < 0.01,
                                format!("{}%", (zoom * 100.0) as u32),
                            )
                            .clicked()
                        {
                            out.exec("setZoom", serde_json::json!({"zoom":zoom}));
                        }
                    }
                    if ui.button("Fit Width").clicked() {
                        out.exec("zoomFit", serde_json::json!({}));
                    }
                },
            );
            let plus = zoom_button(
                ui,
                c,
                enabled,
                vec2(plus_w + 8.0, 28.0),
                |ui, rect, color| {
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "+",
                        FontId::proportional(14.0),
                        color,
                    );
                },
            );
            if plus.clicked() && enabled {
                out.exec("zoomStep", serde_json::json!({"direction":"in"}));
            }
        },
    );
}

fn zoom_button(
    ui: &mut egui::Ui,
    c: &UiColors,
    enabled: bool,
    size: egui::Vec2,
    content: impl FnOnce(&mut egui::Ui, egui::Rect, egui::Color32),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let color = if enabled {
        c.app_muted
    } else {
        dim(c.app_muted, 0.4)
    };
    if response.hovered() && enabled {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(5), c.app_hover);
    }
    content(ui, rect, color);
    response
}

fn draw_view_toggle(ui: &mut egui::Ui, c: &UiColors, state: &mut AppState, out: &mut UiOutput) {
    let continuous = state.surface.view == crate::state::ViewMode::Continuous;
    let label_text = if continuous {
        "View: Continuous"
    } else {
        "View: Separate Pages"
    };
    let font = FontId::proportional(12.0);
    let text_w = ui
        .painter()
        .layout_no_wrap(label_text.to_string(), font.clone(), c.app_muted)
        .size()
        .x;
    let width = 14.0 + 7.0 + text_w + 14.0;
    let enabled = !state.unavailable();
    let (rect, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
    if response.hovered() && enabled {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(5), c.app_hover);
    }
    let color = if enabled {
        c.app_muted
    } else {
        dim(c.app_muted, 0.4)
    };
    icons_paint(
        ui,
        IconName::Page,
        14.0,
        color,
        Pos2::new(rect.left() + 7.0 + 7.0, rect.center().y),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 7.0 + 14.0 + 7.0, rect.center().y),
        Align2::LEFT_CENTER,
        label_text,
        font,
        color,
    );
    if response.clicked() && enabled {
        let view = if continuous { "pages" } else { "continuous" };
        out.command_args("setView", serde_json::json!({ "view": view }));
    }
}

fn dim(color: egui::Color32, f: f32) -> egui::Color32 {
    super::dim_color(color, f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_buttons_keep_their_order_inside_the_right_aligned_group() {
        let context = egui::Context::default();
        let mut state = AppState::default();
        state.surface.ready = true;
        state.surface.zoom = 1.0;
        let colors = UiColors::for_theme(crate::theme::Theme::Dark);
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(500.0, 100.0))),
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        draw_zoom(ui, &colors, &mut state, &mut UiOutput::default())
                    });
                });
            },
        );
        let positions: std::collections::HashMap<_, _> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    Some((text.galley.job.text.as_str(), text.pos.x))
                } else {
                    None
                }
            })
            .collect();
        assert!(positions["-"] < positions["100%"]);
        assert!(positions["100%"] < positions["+"]);
    }
}
