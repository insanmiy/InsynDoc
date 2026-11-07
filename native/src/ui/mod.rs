//! Native Application Controls. Commands Are Dispatched To The Rust Workspace.

use std::time::Instant;

use egui::{
    vec2, Align, Align2, Color32, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, Sense,
    Stroke, TopBottomPanel, Vec2,
};
use serde_json::Value;

use crate::icons::{self, IconName};
use crate::state::{AppState, Modal, UnsavedChoice};
use crate::theme::{self, Theme, UiColors, HEADER_HEIGHT, STATUS_HEIGHT};

pub mod banners;
pub mod header;
pub mod menubar;
pub mod modals;
pub mod statusbar;

/// Commands the chrome UI asks the integrator to perform.
#[derive(Clone)]
pub enum UiCommand {
    /// Application Command.
    Command { name: String, args: Value },
    /// Editor Or Workspace Command.
    Exec { name: String, args: Value },
    /// Quit the application (run the native unsaved-changes flow first).
    Quit,
    /// Update the native window title.
    SetTitle { name: String, dirty: bool },
    /// Ask For An Update Check.
    CheckUpdates,
    /// Install the pending update (`state.update`).
    InstallUpdate,
    /// Dismiss the update prompt and disable update notifications.
    DismissUpdate,
    /// Fetch the recent-documents list (see module docs).
    FetchRecent,
    /// Open the native "choose browser" file dialog.
    ChooseBrowser,
    /// Persist Application Preferences.
    SavePreferences,
    /// Resolve a pending unsaved-changes prompt.
    ConfirmChoice { choice: UnsavedChoice },
    /// Dismiss the workspace notice pill.
    ClearNotice,
}

/// Output produced by [`draw`]; the integrator drains `commands` each frame.
#[derive(Default)]
pub struct UiOutput {
    pub commands: Vec<UiCommand>,
}

impl UiOutput {
    pub fn command(&mut self, name: &str) {
        self.commands.push(UiCommand::Command {
            name: name.to_string(),
            args: Value::Null,
        });
    }

    pub fn command_args(&mut self, name: &str, args: Value) {
        self.commands.push(UiCommand::Command {
            name: name.to_string(),
            args,
        });
    }

    pub fn exec(&mut self, name: &str, args: Value) {
        self.commands.push(UiCommand::Exec {
            name: name.to_string(),
            args,
        });
    }
}

/// Assets the integrator supplies each frame.
pub struct ChromeAssets {
    /// Resolved theme (System already resolved to Light/Dark).
    pub theme: Theme,
    /// Logo texture for the 44x44 `.app-logo` image, if loaded.
    pub logo: Option<egui::TextureId>,
}

/// A recent document entry (mirrors `POST /api/recent/list`).
#[derive(Clone, serde::Deserialize)]
pub struct RecentEntry {
    pub token: String,
    pub name: String,
    pub path: String,
    pub available: bool,
}

pub(crate) const RECENTS_ID: &str = "insyn_recent_documents";
const RECENTS_FETCHED_ID: &str = "insyn_recent_fetched_at";

/// Store fetched recent documents for the File > Open Recent submenu.
/// Called by the integrator after handling [`UiCommand::FetchRecent`].
pub fn store_recents(ctx: &egui::Context, recents: Vec<RecentEntry>) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(RECENTS_ID), recents));
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(RECENTS_FETCHED_ID), Instant::now()));
}

/// Read the stored recent documents (empty when never fetched).
pub fn recents(ctx: &egui::Context) -> Vec<RecentEntry> {
    ctx.data(|d| d.get_temp::<Vec<RecentEntry>>(egui::Id::new(RECENTS_ID)))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Shared drawing helpers (pixel values from src/styles.css, px == points)
// ---------------------------------------------------------------------------

pub fn colors(assets: &ChromeAssets) -> UiColors {
    UiColors::for_theme(assets.theme)
}

pub fn text(size: f32) -> RichText {
    RichText::new("").font(FontId::proportional(size))
}

pub fn label(ui: &mut egui::Ui, size: f32, color: Color32, s: impl Into<String>) -> egui::Response {
    ui.add(egui::Label::new(
        RichText::new(s.into()).size(size).color(color),
    ))
}

/// `.icon-button`: 36x36 circle, muted icon, hover app_hover + app_text.
pub fn icon_button(
    ui: &mut egui::Ui,
    c: &UiColors,
    name: IconName,
    icon_size: f32,
    tooltip: &str,
) -> egui::Response {
    let size = theme::icon_button_size();
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let bg = if hovered {
        c.app_hover
    } else {
        Color32::TRANSPARENT
    };
    let fg = if hovered { c.app_text } else { c.app_muted };
    if bg != Color32::TRANSPARENT {
        ui.painter().circle_filled(rect.center(), size.x / 2.0, bg);
    }
    icons::paint_icon_at(
        ui.ctx(),
        &ui.painter(),
        name,
        rect.shrink((size.x - icon_size) / 2.0),
        fg,
    );
    response.on_hover_text(tooltip)
}

/// `.secondary-button` chrome variant used by Import (height 36, padding 8/12,
/// transparent bg, app_border stroke, accent-ink text, radius 8).
pub fn secondary_button(
    ui: &mut egui::Ui,
    c: &UiColors,
    icon: Option<IconName>,
    label_text: &str,
    height: f32,
    enabled_state: bool,
) -> egui::Response {
    let font = theme::ui_font_id_weight(13.0, 500);
    let icon_w = if icon.is_some() { 16.0 + 7.0 } else { 0.0 };
    let text_w = ui
        .painter()
        .layout_no_wrap(label_text.to_string(), font.clone(), c.app_accent_ink)
        .size()
        .x;
    let width = (text_w + icon_w + 24.0).max(32.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let hovered = response.hovered() && response.enabled() && enabled_state;
    let bg = if hovered {
        c.app_hover
    } else {
        Color32::TRANSPARENT
    };
    let alpha = if enabled_state { 1.0 } else { 0.4 };
    let fg = dim_color(c.app_accent_ink, alpha);
    let border = dim_color(c.app_border, alpha);
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(8), bg);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        Stroke::new(1.0_f32, border),
        egui::StrokeKind::Inside,
    );
    let mut x = rect.left() + 12.0;
    let cy = rect.center().y;
    if let Some(ic) = icon {
        icons::paint_icon_at(
            ui.ctx(),
            &ui.painter(),
            ic,
            Rect::from_center_size(Pos2::new(x + 8.0, cy), vec2(16.0, 16.0)),
            fg,
        );
        x += 16.0 + 7.0;
    }
    ui.painter()
        .text(Pos2::new(x, cy), Align2::LEFT_CENTER, label_text, font, fg);
    response
}

pub(crate) fn dim_color(color: Color32, f: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color.r() as f32 * f) as u8,
        (color.g() as f32 * f) as u8,
        (color.b() as f32 * f) as u8,
        color.a(),
    )
}

/// Paint an icon centered at `center` with `size` points (sibling-module
/// convenience wrapper around [`icons::paint_icon_at`]).
pub(crate) fn icons_paint(ui: &egui::Ui, name: IconName, size: f32, color: Color32, center: Pos2) {
    icons::paint_icon_at(
        ui.ctx(),
        &ui.painter(),
        name,
        Rect::from_center_size(center, Vec2::splat(size)),
        color,
    );
}

/// `.primary-button` chrome variant (accent bg; dark theme uses dark ink
/// #122449 per `[data-theme="dark"] .primary-button`).
pub fn primary_button(
    ui: &mut egui::Ui,
    c: &UiColors,
    icon: Option<IconName>,
    label_text: &str,
    height: f32,
    enabled_state: bool,
) -> egui::Response {
    let dark = c.app_bg == Color32::from_rgb(0x11, 0x13, 0x18);
    let fg = if dark {
        Color32::from_rgb(0x12, 0x24, 0x49)
    } else {
        Color32::WHITE
    };
    let font = theme::ui_font_id_weight(13.0, 500);
    let icon_w = if icon.is_some() { 16.0 + 9.0 } else { 0.0 };
    let text_w = ui
        .painter()
        .layout_no_wrap(label_text.to_string(), font.clone(), fg)
        .size()
        .x;
    let width = (text_w + icon_w + 40.0).max(40.0);
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if !enabled_state {
        response = response.on_disabled_hover_text("");
    }
    let hovered = response.hovered() && enabled_state;
    let bg = if !enabled_state {
        c.app_accent
    } else if hovered {
        c.app_accent_hover
    } else {
        c.app_accent
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(8), bg);
    let mut alpha = 1.0f32;
    if !enabled_state {
        alpha = 0.4;
    }
    let fg_dim = Color32::from_rgba_unmultiplied(
        (fg.r() as f32 * alpha) as u8,
        (fg.g() as f32 * alpha) as u8,
        (fg.b() as f32 * alpha) as u8,
        255,
    );
    let mut x = rect.left() + 20.0;
    let cy = rect.center().y;
    if let Some(ic) = icon {
        icons::paint_icon_at(
            ui.ctx(),
            &ui.painter(),
            ic,
            Rect::from_center_size(Pos2::new(x + 8.0, cy), vec2(16.0, 16.0)),
            fg_dim,
        );
        x += 16.0 + 9.0;
    }
    ui.painter().text(
        Pos2::new(x, cy),
        Align2::LEFT_CENTER,
        label_text,
        font,
        fg_dim,
    );
    response
}

/// `.text-button` (transparent, muted, hover app_hover).
pub fn text_button(
    ui: &mut egui::Ui,
    c: &UiColors,
    label_text: &str,
    font_size: f32,
) -> egui::Response {
    let font = theme::ui_font_id_weight(font_size, 500);
    let text_w = ui
        .painter()
        .layout_no_wrap(label_text.to_string(), font.clone(), c.app_muted)
        .size()
        .x;
    let (rect, response) = ui.allocate_exact_size(vec2(text_w + 28.0, 34.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(8), c.app_hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label_text,
        font,
        c.app_muted,
    );
    response
}

/// `.status-separator`: 1x10 app_border line with 2px side margins inside the
/// parent's 10px item gap (net 12px each side of the line).
pub fn status_separator(ui: &mut egui::Ui, c: &UiColors) {
    let (rect, _) = ui.allocate_exact_size(vec2(5.0, 10.0), Sense::hover());
    ui.painter().rect_filled(
        Rect::from_min_size(Pos2::new(rect.left() + 2.0, rect.top()), vec2(1.0, 10.0)),
        0.0,
        c.app_border,
    );
}

/// Format an integer with thousands separators (JS toLocaleString approx).
pub fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// `.setting-row input[role=switch]`: 34x20 track, 14px knob.
pub fn switch(ui: &mut egui::Ui, c: &UiColors, value: &mut bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(34.0, 20.0), Sense::click());
    if response.clicked() {
        *value = !*value;
    }
    let checked = *value;
    let track = if checked {
        c.app_accent
    } else {
        c.switch_track
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(10), track);
    let knob_r = 7.0;
    let knob_x = if checked {
        rect.right() - 3.0 - knob_r
    } else {
        rect.left() + 3.0 + knob_r
    };
    let knob_color = if checked {
        Color32::WHITE
    } else {
        c.app_surface
    };
    ui.painter()
        .circle_filled(Pos2::new(knob_x, rect.center().y), knob_r, knob_color);
    response.clicked()
}

/// `.kbd` styling: border, radius 4-5, muted small text.
pub fn kbd(ui: &mut egui::Ui, c: &UiColors, keys: &str, size: f32) -> egui::Response {
    let font = FontId::proportional(size);
    let text_w = ui
        .painter()
        .layout_no_wrap(keys.to_string(), font.clone(), c.app_muted)
        .size()
        .x;
    let (rect, response) = ui.allocate_exact_size(vec2(text_w + 10.0, size + 9.0), Sense::hover());
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(4),
        Stroke::new(1.0_f32, c.app_border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        keys,
        font,
        c.app_muted,
    );
    response
}

/// Borderless text input matching `.page-number-input` /
/// `.document-name-input` / `.setting-field input` (border app_border or
/// accent for rename, radius 6/4, surface bg).
pub fn framed_text_edit(
    ui: &mut egui::Ui,
    c: &UiColors,
    text: &mut String,
    width: f32,
    accent_border: bool,
) -> egui::Response {
    let frame = Frame::new()
        .fill(c.app_surface)
        .stroke(Stroke::new(
            1.0_f32,
            if accent_border {
                c.app_accent
            } else {
                c.app_border
            },
        ))
        .corner_radius(egui::CornerRadius::same(if accent_border { 4 } else { 6 }))
        .inner_margin(Margin::symmetric(10, 8));
    frame
        .show(ui, |ui| {
            ui.set_width((width - 20.0).max(1.0));
            egui::TextEdit::singleline(text)
                .font(FontId::proportional(13.0))
                .text_color(c.app_text)
                .desired_width((width - 20.0).max(1.0))
                .show(ui)
        })
        .inner
        .response
}

/// `.modal-description`: 13px muted, line-height ~1.7.
pub fn modal_description(ui: &mut egui::Ui, c: &UiColors, s: &str) {
    ui.add_space(4.0);
    ui.label(
        RichText::new(s)
            .size(13.0)
            .color(c.app_muted)
            .line_height(Some(22.1)),
    );
    ui.add_space(12.0);
}

/// `.modal-footer`: right-aligned row with 22px top padding.
pub fn modal_footer(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(22.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| add(ui));
}

/// `.settings-section` separator: 18px vertical padding + bottom border.
pub fn section_separator(ui: &mut egui::Ui, c: &UiColors) {
    ui.add_space(18.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, c.app_border);
}

// ---------------------------------------------------------------------------
// Menu memory helpers (egui temp data; survives across frames)
// ---------------------------------------------------------------------------

pub fn clear_all_submenus(ctx: &egui::Context) {
    menubar::clear_submenus(ctx);
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Draw all native chrome. See module docs for the integration contract.
pub fn draw(ctx: &egui::Context, state: &mut AppState, assets: &ChromeAssets, out: &mut UiOutput) {
    // Update The OS Window Title Only When It Changes.
    let title_key = egui::Id::new("window_title_state");
    let current = (state.surface.name.clone(), state.surface.dirty);
    let last = ctx.data(|d| d.get_temp::<(String, bool)>(title_key));
    if last.as_ref() != Some(&current) {
        ctx.data_mut(|d| d.insert_temp(title_key, current.clone()));
        out.commands.push(UiCommand::SetTitle {
            name: current.0.clone(),
            dirty: current.1,
        });
    }

    handle_escape(ctx, state, out);

    let focus = state.surface.focus_mode;

    // Top chrome: focus strip replaces the whole header+status bar in focus
    // mode (CSS `.focus-mode .app-header, .focus-mode .status-bar {display:none}`).
    if focus {
        TopBottomPanel::top("chrome-focus-strip")
            .exact_height(theme::FOCUS_STRIP_HEIGHT)
            .frame(egui::Frame::new().fill(colors(assets).app_bg))
            .show(ctx, |ui| banners::draw_focus_strip(ui, state, assets, out));
    } else {
        TopBottomPanel::top("chrome-header")
            .show_separator_line(false)
            .exact_height(HEADER_HEIGHT)
            .frame(
                egui::Frame::new()
                    .fill(colors(assets).app_surface)
                    .inner_margin(Margin {
                        left: 16,
                        right: 20,
                        top: 8,
                        bottom: 8,
                    }),
            )
            .show(ctx, |ui| header::draw(ui, ctx, state, assets, out));
    }

    // Error Banners Reserve Layout Space.
    let banner_h = banners::banners_height(state);
    if banner_h > 0.0 {
        TopBottomPanel::top("chrome-banners")
            .exact_height(banner_h)
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                banners::draw_error_banners(ui, state, assets, out)
            });
    }

    // Status bar hidden in focus mode.
    if !focus {
        let status = TopBottomPanel::bottom("chrome-status")
            .exact_height(STATUS_HEIGHT)
            .frame(
                egui::Frame::new()
                    .fill(colors(assets).app_surface)
                    .inner_margin(Margin {
                        left: 20,
                        right: 20,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .show(ctx, |ui| statusbar::draw(ui, state, assets, out));
        // 1px top border (.status-bar border-top: 1px solid app_border).
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("chrome_status_border"),
        ));
        painter.hline(
            status.response.rect.x_range(),
            status.response.rect.top(),
            Stroke::new(1.0, colors(assets).app_border),
        );
    }

    // Transient Busy And Notice Overlays.
    banners::draw_overlays(ctx, state, assets, out);

    // Modals (backdrop + window).
    modals::draw(ctx, state, assets, out);

    // Menus last so they layer above everything else.
    menubar::draw_open_menus(ctx, state, assets, out);
}

/// Total height reserved by transient bottom overlays (busy pill / notice).
/// Space Used By Busy And Notice Overlays.
pub fn bottom_overlay_height(state: &AppState) -> f32 {
    let mut h = 0.0f32;
    if state.surface.busy && state.modal == Modal::None && !state.quitting {
        h = h.max(64.0);
    }
    if state.surface.notice.is_some() && state.modal == Modal::None {
        h = h.max(46.0);
    }
    h
}

/// Height of the top chrome region above the editor surface:
/// focus strip (38) OR header (78) + error banners.
pub fn top_chrome_height(state: &AppState) -> f32 {
    if state.surface.focus_mode {
        return theme::FOCUS_STRIP_HEIGHT + banners::banners_height(state);
    }
    HEADER_HEIGHT + banners::banners_height(state)
}

fn handle_escape(ctx: &egui::Context, state: &mut AppState, out: &mut UiOutput) {
    let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    if !escape {
        return;
    }
    if state.open_menu.is_some() {
        state.open_menu = None;
        clear_all_submenus(ctx);
        return;
    }
    match state.modal {
        Modal::None => {}
        Modal::ConfirmClose => {
            state.modal = Modal::None;
            state.pending_close_choice = Some(UnsavedChoice::Cancel);
            out.commands.push(UiCommand::ConfirmChoice {
                choice: UnsavedChoice::Cancel,
            });
        }
        _ => {
            state.modal = Modal::None;
        }
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;

    #[test]
    fn clicked_switch_reports_the_change_to_its_caller() {
        let context = egui::Context::default();
        let colors = UiColors::for_theme(Theme::Light);
        let mut persisted = false;
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(300.0, 100.0))),
            ..Default::default()
        };
        for frame in 0..3 {
            if frame == 2 {
                let position = Pos2::new(20.0, 18.0);
                input.events = vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
            }
            let _ = context.run(input.clone(), |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    let mut draft = persisted;
                    if switch(ui, &colors, &mut draft) {
                        persisted = draft;
                    }
                });
            });
        }
        assert!(persisted);
    }
}
