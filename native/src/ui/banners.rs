//! Banners and transient overlays.
//!
//! - `.error-banner` (link_error then surface.error): bg error_banner_bg,
//!   fg error_banner_fg, 12px, padding 9px 24px, info icon 18, dismiss X.
//!   Drawn In A TopBottomPanel To Reserve Layout Space.
//! - `.focus-strip` (focus mode only): 38px, muted 11px, focus icon +
//!   "Focus Mode" left; "Exit Focus Mode" + Esc kbd right -> Command
//!   focusMode {enabled:false}.
//! - `.busy-indicator`: bottom-center pill (accent_soft bg, border, radius
//!   10, padding 12px 22px, 12px accent-ink, "{message}…") when busy and no
//!   modal is open.
//! - `.workspace-notice`: bottom-center pill (surface bg, border, radius 10,
//!   padding 6px 10px 6px 18px, 12px) with notice text + dismiss X ->
//!   [`UiCommand::ClearNotice`].
//!

use egui::{vec2, Align2, FontId, Pos2, Sense, Stroke};

use crate::icons::IconName;
use crate::state::{AppState, Modal};
use crate::theme::{UiColors, FOCUS_STRIP_HEIGHT, STATUS_HEIGHT};

use super::{colors, icons_paint, kbd, ChromeAssets, UiCommand, UiOutput};

/// Height of all visible error banners (0, 1 or 2 stacked).
pub fn banners_height(state: &AppState) -> f32 {
    let mut n = 0;
    if state.link_error.is_some() {
        n += 1;
    }
    if state.surface.error.is_some() {
        n += 1;
    }
    // padding 9+9 + ~17px line = ~35 per banner.
    n as f32 * 35.0
}

/// Draw the stacked error banners inside the "chrome-banners" panel.
pub fn draw_error_banners(
    ui: &mut egui::Ui,
    state: &mut AppState,
    assets: &ChromeAssets,
    out: &mut UiOutput,
) {
    let c = colors(assets);
    if state.link_error.is_some() {
        let msg = state.link_error.clone().unwrap_or_default();
        if banner_row(ui, &c, &msg) {
            state.link_error = None;
        }
    }
    if state.surface.error.is_some() {
        let msg = state.surface.error.clone().unwrap_or_default();
        if banner_row(ui, &c, &msg) {
            state.surface.error = None;
            // Best effort: tell the surface to clear its error too.
            out.commands.push(UiCommand::Command {
                name: "clearError".to_string(),
                args: serde_json::Value::Null,
            });
        }
    }
}

/// Draw one `.error-banner` row; returns true when its dismiss X was clicked.
fn banner_row(ui: &mut egui::Ui, c: &UiColors, message: &str) -> bool {
    let height = 35.0;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, c.error_banner_bg);
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(9.0, 0.0);
            // padding-left 24 + icon 18 + gap 9 = 51 before the text.
            ui.add_space(51.0);
            icons_paint(
                ui,
                IconName::Info,
                18.0,
                c.error_banner_fg,
                Pos2::new(rect.left() + 24.0 + 9.0, rect.center().y),
            );
            ui.label(
                egui::RichText::new(message)
                    .size(12.0)
                    .color(c.error_banner_fg),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(24.0 - 14.0);
                let (x_rect, x_resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
                icons_paint(ui, IconName::Close, 18.0, c.error_banner_fg, x_rect.center());
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new("banner_dismiss_hit"), x_resp.clicked());
                });
            });
        });
    });
    let hit = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(egui::Id::new("banner_dismiss_hit")))
        .unwrap_or(false);
    ui.ctx()
        .data_mut(|d| d.remove::<bool>(egui::Id::new("banner_dismiss_hit")));
    hit
}

/// `.focus-strip` — shown instead of header+status bar in focus mode.
pub fn draw_focus_strip(
    ui: &mut egui::Ui,
    _state: &mut AppState,
    assets: &ChromeAssets,
    out: &mut UiOutput,
) {
    let c = colors(assets);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.add_space(24.0);
        ui.set_height(FOCUS_STRIP_HEIGHT);
        // Left: icon + "Focus Mode" (11px muted, gap 8).
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
            let (icon_rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
            icons_paint(ui, IconName::Focus, 16.0, c.app_muted, icon_rect.center());
            ui.label(egui::RichText::new("Focus Mode").size(11.0).color(c.app_muted));
        });
        // Right: "Exit Focus Mode" + Esc kbd.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
                kbd(ui, &c, "Esc", 10.0);
                let (btn_rect, response) = ui.allocate_exact_size(
                    vec2(110.0, 24.0),
                    Sense::click(),
                );
                ui.painter().text(
                    Pos2::new(btn_rect.left() + 4.0, btn_rect.center().y),
                    Align2::LEFT_CENTER,
                    "Exit Focus Mode",
                    FontId::proportional(11.0),
                    c.app_accent_ink,
                );
                if response.hovered() {
                    ui.painter().rect_filled(
                        btn_rect,
                        egui::CornerRadius::same(4),
                        c.app_hover,
                    );
                }
                if response.clicked() {
                    out.command_args(
                        "focusMode",
                        serde_json::json!({ "enabled": false }),
                    );
                }
            });
        });
        ui.add_space(24.0);
    });
}

/// Bottom overlays: busy pill + workspace notice. Drawn as Foreground Areas.
pub fn draw_overlays(
    ctx: &egui::Context,
    state: &mut AppState,
    assets: &ChromeAssets,
    out: &mut UiOutput,
) {
    let c = colors(assets);
    let screen = ctx.screen_rect();

    // .workspace-notice — bottom center, above status bar (bottom: 18px in
    // CSS relative to the editor area; we anchor above the status bar).
    if let Some(notice) = state.surface.notice.clone() {
        if state.modal == Modal::None {
            let pill_h = 38.0;
            let pos = Pos2::new(
                screen.center().x,
                screen.bottom() - STATUS_HEIGHT - 18.0 - pill_h / 2.0,
            );
            egui::Area::new(egui::Id::new("workspace_notice"))
                .order(egui::Order::Foreground)
                .pivot(Align2::CENTER_CENTER)
                .fixed_pos(pos)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(c.app_surface)
                        .stroke(Stroke::new(1.0_f32, c.app_border))
                        .corner_radius(egui::CornerRadius::same(10))
                        .inner_margin(egui::Margin {
                            left: 18,
                            right: 10,
                            top: 6,
                            bottom: 6,
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(16.0, 0.0);
                                ui.label(
                                    egui::RichText::new(&notice)
                                        .size(12.0)
                                        .color(c.app_text),
                                );
                                let (x_rect, x_resp) = ui.allocate_exact_size(
                                    vec2(28.0, 28.0),
                                    Sense::click(),
                                );
                                icons_paint(
                                    ui,
                                    IconName::Close,
                                    16.0,
                                    c.app_muted,
                                    x_rect.center(),
                                );
                                if x_resp.clicked() {
                                    out.commands.push(UiCommand::ClearNotice);
                                }
                            });
                        });
                });
        }
    }

    // .busy-indicator — bottom center pill, hidden while a modal is open or
    // while quitting.
    if state.surface.busy && state.modal == Modal::None && !state.quitting {
        let message = format!("{}…", state.surface.message);
        let pill_h = 46.0;
        let pos = Pos2::new(
            screen.center().x,
            screen.bottom() - STATUS_HEIGHT - 20.0 - pill_h / 2.0,
        );
        egui::Area::new(egui::Id::new("busy_indicator"))
            .order(egui::Order::Foreground)
            .pivot(Align2::CENTER_CENTER)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(c.busy_bg)
                    .stroke(Stroke::new(1.0_f32, c.app_border))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(22, 12))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&message)
                                .size(12.0)
                                .color(c.app_accent_ink),
                        );
                    });
            });
    }
}
