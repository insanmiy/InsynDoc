//! All native modals, pixel-matched to `.modal` in src/styles.css and the
//! React modals in src/App.tsx / src/Settings.tsx / src/Modal.tsx.
//!
//! - `.modal`: width 580px (page-modal 380, statistics/unsaved 460),
//!   radius 18, border app_border, bg app_surface, shadow; `.modal-content`
//!   padding 24px 28px; `.modal-header` h2 22px/600 + icon-button close;
//!   `::backdrop` = theme.modal_backdrop.
//! - Escape closes the topmost modal (handled in `ui::draw`); clicking the
//!   backdrop closes too.
//!
//! Preferences mutations write directly into `state.preferences` and emit
//! [`UiCommand::SavePreferences`] so the integrator persists + pushes them.
//!
//! # Integrator hooks
//!
//! - After handling [`UiCommand::CheckUpdates`], report the outcome with
//!   [`set_update_status`] (`"current"`, `"available"`, `"error"` or
//!   `"unsupported"`) so the Settings modal status line updates.
//! - After handling [`UiCommand::ChooseBrowser`], report dialog failures
//!   with [`set_browser_error`]; on success set
//!   `state.preferences.browser_path` and emit `SavePreferences` (the
//!   "Choosing..." state clears automatically once `browser_path` is set,
//!   or after a short timeout if the dialog is cancelled).

use egui::{
    vec2, Align2, Color32, FontId, Frame, Id, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};
use serde_json::json;

use crate::icons::{self, IconName};
use crate::state::{AppState, Modal, Preferences, RulerUnit, ThemePref, UnsavedChoice};
use crate::theme::{self, UiColors};

use super::header;
use super::{
    colors, fmt_num, framed_text_edit, kbd, modal_description, modal_footer,
    primary_button, secondary_button, section_separator, switch, text_button, ChromeAssets,
    UiCommand, UiOutput,
};

// ---------------------------------------------------------------------------
// Settings scratch memory (egui temp data; survives across frames)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct SettingsMem {
    /// "idle" | "checking" | "current" | "available" | "error" | "unsupported"
    update_status: &'static str,
    browser_error: Option<String>,
    choosing_browser: bool,
    choose_browser_at: f64,
    scroll_section: Option<String>,
}

impl Default for SettingsMem {
    fn default() -> Self {
        Self {
            update_status: "idle",
            browser_error: None,
            choosing_browser: false,
            choose_browser_at: 0.0,
            scroll_section: None,
        }
    }
}

const SETTINGS_MEM_ID: &str = "insyn_settings_mem";
const MODAL_CLOSE_ID: &str = "insyn_modal_close_clicked";

fn settings_mem(ctx: &egui::Context) -> SettingsMem {
    ctx.data(|d| d.get_temp::<SettingsMem>(Id::new(SETTINGS_MEM_ID)))
        .unwrap_or_default()
}

fn set_settings_mem(ctx: &egui::Context, mem: SettingsMem) {
    ctx.data_mut(|d| d.insert_temp(Id::new(SETTINGS_MEM_ID), mem));
}

/// Report the result of a [`UiCommand::CheckUpdates`] round trip so the
/// Settings > Updates status line reflects it.
pub fn set_update_status(ctx: &egui::Context, status: &'static str) {
    let mut mem = settings_mem(ctx);
    mem.update_status = status;
    set_settings_mem(ctx, mem);
}

/// Report a [`UiCommand::ChooseBrowser`] dialog failure (or clear with `None`).
pub fn set_browser_error(ctx: &egui::Context, message: Option<String>) {
    let mut mem = settings_mem(ctx);
    mem.browser_error = message;
    mem.choosing_browser = false;
    set_settings_mem(ctx, mem);
}

fn request_close(ui: &egui::Ui) {
    ui.ctx().data_mut(|d| {
        d.insert_temp(Id::new(MODAL_CLOSE_ID), true);
    });
}

// ---------------------------------------------------------------------------
// Shell
// ---------------------------------------------------------------------------

/// Entry point called from `ui::draw`.
pub fn draw(ctx: &egui::Context, state: &mut AppState, assets: &ChromeAssets, out: &mut UiOutput) {
    let c = colors(assets);

    // DownloadMenu is a popup anchored to the header button, not a dialog.
    if state.modal == Modal::DownloadMenu {
        header::draw_download_popup(ctx, state, &c, out);
        close_download_on_outside_click(ctx, state);
        return;
    }

    let Some((width, title, tall)) = modal_spec(state.modal) else {
        return;
    };

    let screen = ctx.screen_rect();
    let content_width = (width - 58.0).min(screen.width() - 86.0).max(240.0);
    let bottom_margin = if tall { 10 } else { 24 };
    let content_height = if tall {
        (screen.height() - 110.0).max(200.0) + (24.0 - bottom_margin as f32)
    } else {
        (screen.height() - 103.0).max(200.0) + (24.0 - bottom_margin as f32)
    };
    let response = egui::Modal::new(Id::new(("application-modal", title)))
        .backdrop_color(c.modal_backdrop)
        .frame(
            Frame::new()
                .fill(c.app_surface)
                .stroke(Stroke::new(1.0_f32, c.app_border))
                .shadow(modal_shadow(&c))
                .corner_radius(egui::CornerRadius::same(18))
                .inner_margin(Margin {
                    left: 28,
                    right: 28,
                    top: 24,
                    bottom: bottom_margin,
                }),
        )
        .show(ctx, |ui| {
            ui.set_width(content_width);
            if tall {
                ui.set_height(content_height);
            }
            modal_header(ui, state, &c, title, out);
            match state.modal {
                Modal::Settings => draw_settings(ui, ctx, state, assets, &c, out),
                Modal::GoToPage => draw_go_to_page(ui, state, &c, out),
                Modal::Shortcuts => draw_shortcuts(ui, &c),
                Modal::Statistics => draw_statistics(ui, state, &c),
                Modal::Update => draw_update(ui, state, &c, out),
                Modal::ConfirmClose => draw_confirm_close(ui, state, &c, out),
                _ => {}
            }
        });
    if response.backdrop_response.clicked() {
        close_modal(state, out);
    }

    // "Done"-style buttons signal through temp data (they have no `out`).
    let close_clicked = ctx
        .data(|d| d.get_temp::<bool>(Id::new(MODAL_CLOSE_ID)))
        .unwrap_or(false);
    if close_clicked {
        ctx.data_mut(|d| d.remove::<bool>(Id::new(MODAL_CLOSE_ID)));
        close_modal(state, out);
    }
}

/// Close the download popup when the pointer is pressed outside both the
/// trigger button and the popup itself.
fn close_download_on_outside_click(ctx: &egui::Context, state: &mut AppState) {
    let Some(pos) = ctx.input(|i| i.pointer.press_origin()) else {
        return;
    };
    let trigger = ctx
        .data(|d| d.get_temp::<Rect>(Id::new("download_trigger_rect")))
        .unwrap_or(Rect::NOTHING);
    let popup = ctx
        .data(|d| d.get_temp::<Rect>(Id::new("download_popup_rect")))
        .unwrap_or(Rect::NOTHING);
    if !trigger.contains(pos) && !popup.contains(pos) {
        state.modal = Modal::None;
    }
}

fn modal_shadow(c: &UiColors) -> egui::Shadow {
    egui::Shadow {
        offset: [0, 12],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if c.app_bg.r() < 128 { 120 } else { 70 }),
    }
}

fn modal_spec(modal: Modal) -> Option<(f32, &'static str, bool)> {
    match modal {
        Modal::None | Modal::DownloadMenu | Modal::Rename => None,
        Modal::Settings => Some((580.0, "Settings", true)),
        Modal::GoToPage => Some((380.0, "Go To Page", false)),
        Modal::Shortcuts => Some((580.0, "Keyboard Shortcuts", false)),
        Modal::Statistics => Some((460.0, "Word Count", false)),
        Modal::Update => Some((580.0, "Update Available", false)),
        Modal::ConfirmClose => Some((460.0, "Save Your Changes?", false)),
        _ => None,
    }
}

fn close_modal(state: &mut AppState, out: &mut UiOutput) {
    if state.modal == Modal::ConfirmClose {
        state.modal = Modal::None;
        state.pending_close_choice = Some(UnsavedChoice::Cancel);
        out.commands.push(UiCommand::ConfirmChoice {
            choice: UnsavedChoice::Cancel,
        });
    } else {
        state.modal = Modal::None;
    }
}

/// `.modal-header`: h2 22px left, icon-button close right, margin-bottom 8.
fn modal_header(
    ui: &mut egui::Ui,
    state: &mut AppState,
    c: &UiColors,
    title: &str,
    out: &mut UiOutput,
) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.label(
            RichText::new(title)
                .font(theme::ui_font_id_weight(22.0, 600))
                .color(c.app_text),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            // .modal-header .icon-button { margin-right: -8px } — the 36px
            // circle extends 8px past the content edge.
            let (rect, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
            let shifted = rect.translate(vec2(8.0, 0.0));
            let hovered = ui.rect_contains_pointer(shifted);
            ui.painter().circle_stroke(
                shifted.center(),
                18.0,
                Stroke::new(1.0_f32, c.app_border),
            );
            if hovered {
                ui.painter()
                    .circle_filled(shifted.center(), 18.0, c.app_hover);
            }
            icons::paint_icon_at(
                ui.ctx(),
                &ui.painter(),
                IconName::Close,
                shifted.shrink(8.0),
                if hovered { c.app_text } else { c.app_muted },
            );
            if ui.interact(shifted, ui.make_persistent_id("modal-close"), Sense::click()).clicked() {
                close_modal(state, out);
            }
        });
    });
    ui.add_space(8.0);
}

// ---------------------------------------------------------------------------
// Settings (port of src/Settings.tsx)
// ---------------------------------------------------------------------------

fn draw_settings(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    _assets: &ChromeAssets,
    c: &UiColors,
    out: &mut UiOutput,
) {
    let mut mem = settings_mem(ctx);

    egui::ScrollArea::vertical()
        .id_salt("settings-scroll")
        .max_height(ui.available_height())
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // .settings-intro
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Personalize Your Writing Workspace. Changes Are Saved Automatically On This Device.",
                )
                .size(13.0)
                .color(c.app_muted)
                .line_height(Some(22.1)),
            );

            // Settings Navigation Scrolls To The Requested Section.
            ui.add_space(11.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for label in ["Appearance", "Writing", "Editor", "Updates", "Hyperlinks"] {
                    let font = FontId::proportional(12.0);
                    let w = ui
                        .painter()
                        .layout_no_wrap(label.to_string(), font.clone(), c.app_text)
                        .size()
                        .x
                        + 20.0;
                    let (rect, response) = ui.allocate_exact_size(vec2(w, 27.0), Sense::click());
                    if response.clicked() { mem.scroll_section = Some(label.to_string()); }
                    if response.hovered() { ui.painter().rect_filled(rect, 6.0, c.app_hover); }
                    ui.painter().rect_stroke(
                        rect,
                        egui::CornerRadius::same(6),
                        Stroke::new(1.0_f32, c.app_border),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        label,
                        font,
                        c.app_text,
                    );
                }
            });

            // ---- Appearance ----
            section_title(ui, c, "Appearance", "Appearance", &mut mem);
            ui.label(
                RichText::new("Choose A Theme For The App. Your Document Colors Stay The Same.")
                    .size(13.0)
                    .color(c.app_muted)
                    .line_height(Some(22.1)),
            );
            ui.add_space(16.0);
            theme_options(ui, ctx, state, &c, out);
            section_separator(ui, &c);

            // ---- Writing Preferences ----
            section_title(ui, c, "Writing Preferences", "Writing", &mut mem);
            ui.label(
                RichText::new("Set Your Measurement Units And Writing Target.")
                    .size(13.0)
                    .color(c.app_muted)
                    .line_height(Some(22.1)),
            );
            setting_field_select(
                ui,
                &c,
                "Ruler Units",
                if state.preferences.ruler_unit == RulerUnit::Cm { "Centimeters" } else { "Inches" },
                &["Inches", "Centimeters"],
                |ui, idx| {
                    state.preferences.ruler_unit =
                        if idx == 1 { RulerUnit::Cm } else { RulerUnit::Inch };
                    out.commands.push(UiCommand::SavePreferences);
                    ui.ctx().request_repaint();
                },
            );
            setting_field_number(
                ui,
                &c,
                "Word Goal",
                "Set To 0 To Turn Off. Up To 100,000 Words.",
                state.preferences.word_goal as i64,
                0,
                100000,
                |v| {
                    state.preferences.word_goal = v.max(0) as u32;
                    out.commands.push(UiCommand::SavePreferences);
                },
            );
            section_separator(ui, &c);

            // ---- Editor Preferences ----
            section_title(ui, c, "Editor Preferences", "Editor", &mut mem);
            ui.label(
                RichText::new("Choose The Tools Visible In Your Workspace.")
                    .size(13.0)
                    .color(c.app_muted)
                    .line_height(Some(22.1)),
            );
            ui.add_space(18.0);
            editor_toggle(
                ui,
                &c,
                state,
                out,
                "Check Spelling",
                "Check English (US) Spelling And Sentence Context On This Device.",
                |p| &mut p.check_spelling,
            );
            editor_toggle(
                ui,
                &c,
                state,
                out,
                "Show Ruler",
                "Adjust Page Margins And Indents.",
                |p| &mut p.show_ruler,
            );
            editor_toggle(
                ui,
                &c,
                state,
                out,
                "Show Document Outline",
                "Navigate Between Your Headings.",
                |p| &mut p.show_outline,
            );
            editor_toggle(
                ui,
                &c,
                state,
                out,
                "Show Word Count",
                "Keep Document Statistics In The Status Bar.",
                |p| &mut p.show_word_count,
            );

            if !state.preferences.spelling_words.is_empty() {
                dictionary_row(
                    ui,
                    &c,
                    state,
                    out,
                    &format!(
                        "{} {} In Your Personal Dictionary",
                        state.preferences.spelling_words.len(),
                        if state.preferences.spelling_words.len() == 1 { "Word" } else { "Words" }
                    ),
                    "Reset Personal Dictionary",
                    |p| p.spelling_words.clear(),
                );
            }
            if !state.preferences.spelling_corrections.is_empty() {
                dictionary_row(
                    ui,
                    &c,
                    state,
                    out,
                    "Suggestions Learn From Corrections You Accept On This Device.",
                    "Reset Learned Suggestions",
                    |p| p.spelling_corrections.clear(),
                );
            }
            section_separator(ui, &c);

            // ---- Updates ----
            section_title(ui, c, "Updates", "Updates", &mut mem);
            ui.label(
                RichText::new("Keep InsynDoc On The Latest Version.")
                    .size(13.0)
                    .color(c.app_muted)
                    .line_height(Some(22.1)),
            );
            editor_toggle(
                ui,
                &c,
                state,
                out,
                "Notify Me About Updates",
                "Show A Prompt When A New Version Is Available.",
                |p| &mut p.update_notifications,
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x=0.0;
                    ui.label(
                        RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                            .size(13.0)
                            .color(c.app_muted),
                    );
                    let checking = mem.update_status == "checking";
                    let btn = secondary_button(
                        ui,
                        &c,
                        None,
                        if checking { "Checking..." } else { "Check For Updates" },
                        36.0,
                        !checking,
                    );
                    if btn.clicked() && !checking {
                        mem.update_status = "checking";
                        set_settings_mem(ctx, mem.clone());
                        out.commands.push(UiCommand::CheckUpdates);
                    }
                });
            });
            ui.add_space(12.0);
            draw_update_status(ui, &c, &mem);
            section_separator(ui, &c);

            // ---- Hyperlinks ----
            section_title(ui, c, "Hyperlinks", "Hyperlinks", &mut mem);
            ui.label(
                RichText::new("Open Links With Your System Default Or A Browser You Choose.")
                    .size(13.0)
                    .color(c.app_muted)
                    .line_height(Some(22.1)),
            );
            let current_browser = if state.preferences.browser_path.is_some() { "Custom Browser" } else { "System Default" };
            setting_field_select(ui, c, "Link Browser", current_browser, &["System Default", "Custom Browser"], |ui, index| {
                if index == 0 {
                    state.preferences.browser_path = None;
                    out.commands.push(UiCommand::SavePreferences);
                } else {
                    mem.choosing_browser = true;
                    mem.choose_browser_at = ui.input(|input| input.time);
                    out.commands.push(UiCommand::ChooseBrowser);
                }
            });
            let mut actions_gap = 12.0;
            if let Some(path) = state.preferences.browser_path.clone() {
                ui.label(
                    RichText::new(path)
                        .size(13.0)
                        .color(c.app_muted)
                        .line_height(Some(22.1)),
                );
                actions_gap = 16.0;
            }
            ui.add_space(actions_gap);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let choosing = mem.choosing_browser;
                    let btn = secondary_button(
                        ui,
                        &c,
                        None,
                        if choosing { "Choosing Browser..." } else { "Choose Browser" },
                        36.0,
                        !choosing,
                    );
                    if btn.clicked() && !choosing {
                        mem.choosing_browser = true;
                        mem.choose_browser_at = ui.input(|i| i.time);
                        set_settings_mem(ctx, mem.clone());
                        out.commands.push(UiCommand::ChooseBrowser);
                    }
                });
            });
            ui.add_space(12.0);
            if let Some(err) = &mem.browser_error {
                ui.label(RichText::new(err).size(12.0).color(c.app_error));
            }

            // .modal-footer — Done.
            modal_footer(ui, |ui| {
                let done = primary_button(ui, &c, None, "Done", 38.0, true);
                if done.clicked() {
                    request_close(ui);
                }
            });
        });

    // Clear "Choosing..." once a browser_path arrives or after a timeout
    // (native dialog cancelled).
    let now = ctx.input(|i| i.time);
    if mem.choosing_browser
        && (state.preferences.browser_path.is_some() || now - mem.choose_browser_at > 3.0)
    {
        mem.choosing_browser = false;
    }
    set_settings_mem(ctx, mem);
}

fn draw_update_status(ui: &mut egui::Ui, c: &UiColors, mem: &SettingsMem) {
    let msg = match mem.update_status {
        "current" => Some(("You're Up To Date.", false)),
        "available" => Some((
            "A Newer Version Is Available. Finish Installing From The Update Prompt.",
            false,
        )),
        "error" => Some((
            "Could Not Check For Updates. Check Your Connection And Try Again.",
            true,
        )),
        "unsupported" => Some((
            "Update Checks Are Not Yet Available For The Rust App.",
            false,
        )),
        _ => None,
    };
    if let Some((text, is_error)) = msg {
        ui.label(
            RichText::new(text)
                .size(13.0)
                .color(if is_error { c.app_error } else { c.app_muted })
                .line_height(Some(22.1)),
        );
        ui.add_space(16.0);
    }
}

fn section_title(
    ui: &mut egui::Ui,
    c: &UiColors,
    title: &str,
    section: &str,
    mem: &mut SettingsMem,
) {
    ui.add_space(14.0);
    let response = ui.label(
        RichText::new(title)
            .font(theme::ui_font_id_weight(13.0, 600))
            .color(c.app_text),
    );
    if mem.scroll_section.as_deref() == Some(section) {
        response.scroll_to_me(Some(egui::Align::TOP));
        mem.scroll_section = None;
    }
    ui.add_space(5.0);
}

/// `.theme-options` — three selectable cards with mini preview drawings.
fn theme_options(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    out: &mut UiOutput,
) {
    let variants = [
        (ThemePref::Light, "Light", IconName::Sun, 0u8),
        (ThemePref::Dark, "Dark", IconName::Moon, 1u8),
        (ThemePref::System, "System", IconName::Monitor, 2u8),
    ];
    let card_width = ((ui.available_width() - 24.0) / 3.0).max(80.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(12.0, 0.0);
        for (pref, label, icon, variant) in variants {
            let selected = state.preferences.theme == pref;
            theme_card(
                ui, ctx, c, state, out, pref, label, icon, variant, selected, card_width,
            );
        }
    });
}

fn theme_card(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    c: &UiColors,
    state: &mut AppState,
    out: &mut UiOutput,
    pref: ThemePref,
    label: &str,
    icon: IconName,
    variant: u8,
    selected: bool,
    card_w: f32,
) {
    // .theme-option — flex 1, border 2px, padding 5, radius 10.
    let preview_h = 86.0;
    let caption_h = 32.0;
    let (rect, response) = ui.allocate_exact_size(
        vec2(card_w, 7.0 + preview_h + caption_h + 7.0),
        Sense::click(),
    );
    let border_color = if selected { c.app_accent } else { c.app_border };
    let text_color = if selected {
        c.app_accent_ink
    } else {
        c.app_muted
    };
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(10),
        Stroke::new(2.0_f32, border_color),
        egui::StrokeKind::Inside,
    );
    if response.clicked() {
        state.preferences.theme = pref;
        out.commands.push(UiCommand::SavePreferences);
    }
    // .theme-preview — inner rect, radius 4.
    let preview_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 7.0, rect.top() + 7.0),
        vec2(card_w - 14.0, preview_h),
    );
    draw_theme_preview(ui, preview_rect, variant);
    // .theme-caption — icon + label + check.
    let cap_y = preview_rect.bottom() + 18.0;
    icons::paint_icon_at(
        ctx,
        &ui.painter(),
        icon,
        Rect::from_center_size(
            Pos2::new(preview_rect.left() + 13.5, cap_y),
            vec2(17.0, 17.0),
        ),
        text_color,
    );
    ui.painter().text(
        Pos2::new(preview_rect.left() + 5.0 + 17.0 + 6.0, cap_y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.0),
        text_color,
    );
    if selected {
        icons::paint_icon_at(
            ctx,
            &ui.painter(),
            IconName::Check,
            Rect::from_center_size(
                Pos2::new(preview_rect.right() - 13.5, cap_y),
                vec2(17.0, 17.0),
            ),
            text_color,
        );
    }
}

/// Mini theme preview approximating `.theme-preview` + `.preview-*` shapes.
fn draw_theme_preview(ui: &mut egui::Ui, rect: Rect, variant: u8) {
    let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    // .preview-page is always white; toolbar/sidebar colors follow the theme.
    let (toolbar, sidebar_top, sidebar_bot, line) = match variant {
        1 => (
            Color32::from_rgb(0x23, 0x27, 0x30),
            Color32::from_rgb(0xc1, 0xcc, 0xdc),
            Color32::from_rgb(0xd9, 0xe0, 0xec),
            Color32::from_rgb(0xc1, 0xcc, 0xdc),
        ),
        _ => (
            Color32::from_rgb(0xd3, 0xe3, 0xfd),
            Color32::from_rgb(0xc1, 0xcc, 0xdc),
            Color32::from_rgb(0xd9, 0xe0, 0xec),
            Color32::from_rgb(0xd7, 0xdd, 0xe8),
        ),
    };

    // System variant: split the preview vertically light/dark.
    if variant == 2 {
        let mid = rect.center().x;
        p.rect_filled(
            Rect::from_min_max(rect.min, Pos2::new(mid, rect.max.y)),
            egui::CornerRadius::same(4),
            Color32::from_rgb(0xf1, 0xf4, 0xf9),
        );
        p.rect_filled(
            Rect::from_min_max(Pos2::new(mid, rect.min.y), rect.max),
            egui::CornerRadius::same(4),
            Color32::from_rgb(0x09, 0x0a, 0x0d),
        );
    } else {
        let bg = if variant == 1 {
            Color32::from_rgb(0x09, 0x0a, 0x0d)
        } else {
            Color32::from_rgb(0xf1, 0xf4, 0xf9)
        };
        p.rect_filled(rect, egui::CornerRadius::same(4), bg);
    }

    // .preview-toolbar — top 9, left/right 8, height 9, radius 5.
    let tb = Rect::from_min_size(
        Pos2::new(rect.left() + 8.0, rect.top() + 9.0),
        vec2(rect.width() - 16.0, 9.0),
    );
    if variant == 2 {
        let mid = tb.center().x;
        p.rect_filled(
            Rect::from_min_max(tb.min, Pos2::new(mid, tb.max.y)),
            egui::CornerRadius::same(5),
            Color32::from_rgb(0xd3, 0xe3, 0xfd),
        );
        p.rect_filled(
            Rect::from_min_max(Pos2::new(mid, tb.min.y), tb.max),
            egui::CornerRadius::same(5),
            Color32::from_rgb(0x23, 0x27, 0x30),
        );
    } else {
        p.rect_filled(tb, egui::CornerRadius::same(5), toolbar);
    }

    // .preview-sidebar — left 10, top 27, w23, bottom border.
    let sb = Rect::from_min_size(
        Pos2::new(rect.left() + 10.0, rect.top() + 27.0),
        vec2(23.0, 37.0),
    );
    p.hline(
        sb.x_range(),
        sb.top() + 1.5,
        Stroke::new(3.0_f32, sidebar_top),
    );
    p.hline(
        sb.x_range(),
        sb.bottom() - 1.5,
        Stroke::new(3.0_f32, sidebar_bot),
    );

    // .preview-page — left 45, right 22, top 26, white + 3 text lines.
    let page = Rect::from_min_size(
        Pos2::new(rect.left() + 45.0, rect.top() + 26.0),
        vec2((rect.width() - 67.0).max(10.0), 67.0),
    );
    p.rect_filled(page, 0.0, Color32::WHITE);
    p.rect_stroke(
        page,
        0.0,
        Stroke::new(
            1.0_f32,
            if variant == 1 {
                Color32::from_rgb(0x29, 0x2e, 0x38)
            } else {
                Color32::from_rgb(0xdd, 0xe4, 0xed)
            },
        ),
        egui::StrokeKind::Inside,
    );
    for i in 0..3 {
        let ly = page.top() + 12.0 + i as f32 * 7.0;
        if ly + 2.0 > page.bottom() {
            break;
        }
        let lw = if i == 2 {
            page.width() * 0.65
        } else {
            page.width()
        };
        p.rect_filled(
            Rect::from_min_size(Pos2::new(page.left() + 8.0, ly), vec2(lw - 16.0, 2.0)),
            1.0,
            line,
        );
    }
}

fn setting_field_select(
    ui: &mut egui::Ui,
    c: &UiColors,
    label: &str,
    current: &str,
    options: &[&str],
    mut on_pick: impl FnMut(&mut egui::Ui, usize),
) {
    ui.add_space(13.0);
    let width = ui.available_width();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        ui.allocate_ui_with_layout(
            vec2((width - 168.0).max(60.0), 34.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width((width - 168.0).max(60.0));
                ui.label(RichText::new(label).size(13.0).color(c.app_text));
            },
        );
        ui.allocate_ui_with_layout(
            vec2(150.0, 34.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(150.0, 34.0), Sense::click());
                ui.painter().rect_filled(rect, 6.0, c.app_surface);
                ui.painter().rect_stroke(
                    rect,
                    6.0,
                    Stroke::new(1.0_f32, c.app_border),
                    egui::StrokeKind::Inside,
                );
                let clip = rect.shrink2(vec2(10.0, 0.0));
                ui.painter()
                    .with_clip_rect(clip)
                    .text(
                        Pos2::new(rect.left() + 10.0, rect.center().y),
                        Align2::LEFT_CENTER,
                        current,
                        FontId::proportional(13.0),
                        c.app_text,
                    );
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(rect.right() - 16.0, rect.center().y - 2.5),
                        Pos2::new(rect.right() - 10.0, rect.center().y - 2.5),
                        Pos2::new(rect.right() - 13.0, rect.center().y + 2.5),
                    ],
                    c.app_muted,
                    Stroke::NONE,
                ));
                let popup_id = ui.id().with(("setting-select", label));
                if response.clicked() {
                    ui.memory_mut(|memory| memory.toggle_popup(popup_id));
                }
                egui::popup_below_widget(
                    ui,
                    popup_id,
                    &response,
                    egui::PopupCloseBehavior::CloseOnClick,
                    |ui| {
                        ui.set_min_width(150.0);
                        for (index, option) in options.iter().enumerate() {
                            if ui.selectable_label(*option == current, *option).clicked() {
                                on_pick(ui, index);
                                ui.memory_mut(|memory| memory.close_popup());
                            }
                        }
                    },
                );
            },
        );
    });
}

#[derive(Clone)]
struct NumberDraft {
    text: String,
    last_value: i64,
}

fn setting_field_number(
    ui: &mut egui::Ui,
    c: &UiColors,
    label: &str,
    hint: &str,
    value: i64,
    min: i64,
    max: i64,
    mut on_change: impl FnMut(i64),
) {
    ui.add_space(11.0);
    let width = ui.available_width();
    let draft_id = ui.make_persistent_id(("number-draft", label));
    let edit_id = ui.make_persistent_id(("number-edit", label));
    let mut draft = ui
        .ctx()
        .data(|data| data.get_temp::<NumberDraft>(draft_id))
        .unwrap_or_else(|| NumberDraft {
            text: value.to_string(),
            last_value: value,
        });
    if draft.last_value != value && !ui.memory(|memory| memory.has_focus(edit_id)) {
        draft.text = value.to_string();
    }
    draft.last_value = value;
    let mut step = 0;
    let mut lost_focus = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        ui.allocate_ui_with_layout(
            vec2((width - 168.0).max(60.0), 42.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_min_width((width - 168.0).max(60.0));
                ui.label(RichText::new(label).size(13.0).color(c.app_text));
                ui.add_space(5.0);
                ui.label(
                    RichText::new(hint)
                        .size(12.0)
                        .color(c.app_muted)
                        .line_height(Some(19.2)),
                );
            },
        );
        ui.allocate_ui_with_layout(
            vec2(150.0, 42.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(150.0, 34.0), Sense::hover());
                ui.painter().rect_filled(rect, 6.0, c.app_surface);
                ui.painter().rect_stroke(
                    rect,
                    6.0,
                    Stroke::new(1.0_f32, c.app_border),
                    egui::StrokeKind::Inside,
                );
                let edit_rect =
                    Rect::from_min_max(rect.min + vec2(10.0, 8.0), rect.max - vec2(28.0, 7.0));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit_rect));
                let response = child.add(
                    egui::TextEdit::singleline(&mut draft.text)
                        .id(edit_id)
                        .frame(false)
                        .margin(Vec2::ZERO)
                        .font(FontId::proportional(13.0))
                        .text_color(c.app_text)
                        .desired_width(edit_rect.width()),
                );
                lost_focus = response.lost_focus();
                let show_arrows = ui.rect_contains_pointer(rect) || response.has_focus();
                if show_arrows {
                    for (direction, center_y) in
                        [(1, rect.center().y - 6.0), (-1, rect.center().y + 6.0)]
                    {
                        let center = Pos2::new(rect.right() - 13.0, center_y);
                        let button_rect = Rect::from_center_size(center, vec2(20.0, 13.0));
                        let arrow_response =
                            ui.interact(button_rect, edit_id.with(direction), Sense::click());
                        if arrow_response.clicked() {
                            step = direction;
                        }
                        let points = if direction > 0 {
                            vec![
                                center + vec2(-3.0, 2.0),
                                center + vec2(3.0, 2.0),
                                center + vec2(0.0, -2.0),
                            ]
                        } else {
                            vec![
                                center + vec2(-3.0, -2.0),
                                center + vec2(3.0, -2.0),
                                center + vec2(0.0, 2.0),
                            ]
                        };
                        ui.painter().add(egui::Shape::convex_polygon(
                            points,
                            c.app_muted,
                            Stroke::NONE,
                        ));
                    }
                }
            },
        );
    });
    if step != 0 {
        let next = draft
            .text
            .trim()
            .parse::<i64>()
            .unwrap_or(value)
            .saturating_add(step)
            .clamp(min, max);
        draft.text = next.to_string();
        draft.last_value = next;
        if next != value {
            on_change(next);
        }
    } else if let Ok(next) = draft.text.trim().parse::<i64>() {
        if (min..=max).contains(&next) && next != value {
            on_change(next);
            draft.last_value = next;
        } else if lost_focus {
            draft.text = value.to_string();
        }
    } else if lost_focus {
        draft.text = value.to_string();
    }
    ui.ctx().data_mut(|data| data.insert_temp(draft_id, draft));
}

fn editor_toggle(
    ui: &mut egui::Ui,
    c: &UiColors,
    state: &mut AppState,
    out: &mut UiOutput,
    label: &str,
    description: &str,
    field: fn(&mut Preferences) -> &mut bool,
) {
    ui.add_space(11.0);
    let width = ui.available_width();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.allocate_ui_with_layout(
            vec2((width - 53.0).max(80.0), 38.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_min_width((width - 53.0).max(80.0));
                ui.label(
                    RichText::new(label)
                        .font(theme::ui_font_id_weight(13.0, 500))
                        .color(c.app_text),
                );
                ui.add_space(5.0);
                ui.label(
                    RichText::new(description)
                        .size(12.0)
                        .color(c.app_muted)
                        .line_height(Some(19.2)),
                );
            },
        );
        ui.allocate_ui_with_layout(
            vec2(34.0, 38.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let value_ref = field(&mut state.preferences);
                let mut value = *value_ref;
                if switch(ui, c, &mut value) {
                    *value_ref = value;
                    out.commands.push(UiCommand::SavePreferences);
                }
            },
        );
    });
}

fn dictionary_row(
    ui: &mut egui::Ui,
    c: &UiColors,
    state: &mut AppState,
    out: &mut UiOutput,
    text: &str,
    button_label: &str,
    clear: impl FnOnce(&mut Preferences),
) {
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(12.0, 0.0);
        ui.label(RichText::new(text).size(12.0).color(c.app_muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let btn = text_button(ui, c, button_label, 12.0);
            if btn.clicked() {
                clear(&mut state.preferences);
                out.commands.push(UiCommand::SavePreferences);
            }
        });
    });
    ui.add_space(12.0);
}

// ---------------------------------------------------------------------------
// Go To Page (.page-modal 380px)
// ---------------------------------------------------------------------------

fn draw_go_to_page(ui: &mut egui::Ui, state: &mut AppState, c: &UiColors, out: &mut UiOutput) {
    // .page-input-label + .page-number-input
    ui.label(RichText::new("Page Number").size(12.0).color(c.app_text));
    ui.add_space(8.0);
    let resp = framed_text_edit(
        ui,
        c,
        &mut state.page_number_input,
        ui.available_width(),
        false,
    );
    ui.add_space(12.0);
    let pages = state.surface.page_count.max(1);
    modal_description(
        ui,
        c,
        &format!(
            "{} {} In This Document.",
            pages,
            if pages == 1 { "Page" } else { "Pages" }
        ),
    );
    if let Some(err) = &state.page_error {
        ui.label(RichText::new(err).size(12.0).color(c.app_error));
        ui.add_space(8.0);
    }
    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
    modal_footer(ui, |ui| {
        let go = primary_button(ui, c, None, "Go To Page", 38.0, true);
        if go.clicked() || (resp.lost_focus() && enter) || enter {
            let page: i64 = state.page_number_input.trim().parse().unwrap_or(0);
            if page < 1 || page > pages as i64 {
                state.page_error = Some(format!("Choose A Page From 1 To {}.", pages));
            } else {
                out.exec("scrollToPage", json!({ "page": page }));
                state.modal = Modal::None;
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Keyboard Shortcuts (.shortcuts-modal — 15 entries, two columns)
// ---------------------------------------------------------------------------

fn draw_shortcuts(ui: &mut egui::Ui, c: &UiColors) {
    ui.add_space(3.0);
    const SHORTCUTS: &[(&str, &str)] = &[
        ("New Document", "Ctrl+N"),
        ("Open Document", "Ctrl+O"),
        ("Save", "Ctrl+S"),
        ("Export DOCX", "Ctrl+Shift+S"),
        ("Find", "Ctrl+F"),
        ("Find And Replace", "Ctrl+Shift+I"),
        ("Go To Page", "Ctrl+G"),
        ("Focus Mode", "Ctrl+Shift+F"),
        ("Settings", "Ctrl+,"),
        ("Keyboard Shortcuts", "Ctrl+/"),
        ("Undo", "Ctrl+Z"),
        ("Redo", "Ctrl+Y"),
        ("Bold", "Ctrl+B"),
        ("Italic", "Ctrl+I"),
        ("Underline", "Ctrl+U"),
    ];
    // .shortcut-list — 2 columns, gap 28, 11px rows, kbd right.
    let col_w = (ui.available_width() - 28.0) / 2.0;
    ui.spacing_mut().item_spacing.y = 0.0;
    for row in 0..8 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(28.0, 0.0);
            for col in 0..2 {
                let idx = row * 2 + col;
                if idx >= SHORTCUTS.len() {
                    break;
                }
                let (action, keys) = SHORTCUTS[idx];
                let (cell, _) = ui.allocate_exact_size(vec2(col_w, 43.0), Sense::hover());
                ui.painter().text(
                    cell.left_center(),
                    Align2::LEFT_CENTER,
                    action,
                    FontId::proportional(11.0),
                    c.app_text,
                );
                let keys_font = FontId::proportional(10.0);
                let key_width = ui
                    .painter()
                    .layout_no_wrap(keys.to_string(), keys_font.clone(), c.app_muted)
                    .size()
                    .x
                    + 10.0;
                let key_rect = Rect::from_center_size(
                    Pos2::new(cell.right() - key_width * 0.5, cell.center().y),
                    vec2(key_width, 19.0),
                );
                ui.painter().rect_stroke(
                    key_rect,
                    4.0,
                    Stroke::new(1.0_f32, c.app_border),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    key_rect.center(),
                    Align2::CENTER_CENTER,
                    keys,
                    keys_font,
                    c.app_muted,
                );
                ui.painter().hline(
                    cell.x_range(),
                    cell.bottom(),
                    Stroke::new(1.0_f32, c.app_border),
                );
            }
        });
    }
    modal_footer(ui, |ui| {
        let done = primary_button(ui, c, None, "Done", 38.0, true);
        if done.clicked() {
            request_close(ui);
        }
    });
}

// ---------------------------------------------------------------------------
// Word Count (.statistics-modal 460px)
// ---------------------------------------------------------------------------

fn draw_statistics(ui: &mut egui::Ui, state: &AppState, c: &UiColors) {
    let stats = state.surface.statistics;
    let pages = state.surface.page_count.max(1);
    modal_description(ui, c, "Your Document At A Glance.");
    // .statistics-grid — 2 cols, gap 22.
    ui.add_space(9.0);
    let items: [(&str, String); 5] = [
        ("Words", fmt_num(stats.words)),
        ("Pages", pages.to_string()),
        ("Characters", fmt_num(stats.characters)),
        (
            "Characters Without Spaces",
            fmt_num(stats.characters_without_spaces),
        ),
        (
            "Estimated Reading Time",
            format!("{} Min", (stats.words + 199) / 200),
        ),
    ];
    let col_w = (ui.available_width() - 22.0) / 2.0;
    for row in 0..3 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(22.0, 0.0);
            for col in 0..2 {
                let idx = row * 2 + col;
                if idx >= items.len() {
                    break;
                }
                let item = &items[idx];
                ui.allocate_ui_with_layout(
                    vec2(col_w, 60.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.label(RichText::new(item.0).size(11.0).color(c.app_muted));
                        ui.add_space(7.0);
                        ui.label(
                            RichText::new(item.1.clone())
                                .size(26.0)
                                .color(c.app_accent_ink),
                        );
                    },
                );
            }
        });
    }
    ui.add_space(25.0);
    // .writing-goal — only when word_goal > 0.
    let goal = state.preferences.word_goal as u64;
    if goal > 0 {
        let pct = (stats.words as f64 / goal as f64 * 100.0).min(100.0);
        let pct_int = (stats.words as f64 / goal as f64 * 100.0).floor() as u64;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Writing Goal").size(12.0).color(c.app_text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{}%", pct_int.min(100)))
                        .size(12.0)
                        .color(c.app_accent_ink),
                );
            });
        });
        let (bar_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
        ui.painter()
            .rect_filled(bar_rect, egui::CornerRadius::same(4), c.app_border);
        let fill_w = bar_rect.width() * pct as f32 / 100.0;
        if fill_w > 0.0 {
            ui.painter().rect_filled(
                Rect::from_min_size(bar_rect.min, vec2(fill_w, 8.0)),
                egui::CornerRadius::same(4),
                c.app_accent,
            );
        }
        ui.add_space(6.0);
        let remaining = goal.saturating_sub(stats.words);
        let msg = if stats.words >= goal {
            "Goal Reached.".to_string()
        } else {
            format!("{} Words To Go.", fmt_num(remaining))
        };
        ui.label(RichText::new(msg).size(11.0).color(c.app_muted));
        ui.add_space(12.0);
    }
    modal_footer(ui, |ui| {
        let done = primary_button(ui, c, None, "Done", 38.0, true);
        if done.clicked() {
            request_close(ui);
        }
    });
}

// ---------------------------------------------------------------------------
// Update Available (.update-modal — default 580px)
// ---------------------------------------------------------------------------

fn draw_update(ui: &mut egui::Ui, state: &mut AppState, c: &UiColors, out: &mut UiOutput) {
    let version = state
        .update
        .as_ref()
        .map(|u| u.version.clone())
        .unwrap_or_default();
    modal_description(ui, c, &format!("InsynDoc {} Is Available.", version));
    if let Some(update) = &state.update {
        if let Some(notes) = &update.notes {
            if !notes.is_empty() {
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(notes)
                                .size(13.0)
                                .color(c.app_muted)
                                .line_height(Some(19.2)),
                        );
                    });
                ui.add_space(8.0);
            }
        }
    }
    if state.update_installing {
        modal_description(ui, c, "Downloading Update...");
    }
    if let Some(err) = &state.update_error {
        ui.label(RichText::new(err).size(12.0).color(c.app_error));
        ui.add_space(8.0);
    }
    modal_footer(ui, |ui| {
        if state.update_installing {
            let _ = secondary_button(ui, c, None, "Installing...", 40.0, false);
            return;
        }
        // Right-to-left order yields [Later][Don't Ask Again][Update Now].
        let update_now = primary_button(ui, c, None, "Update Now", 38.0, true);
        if update_now.clicked() {
            out.commands.push(UiCommand::InstallUpdate);
        }
        let dont = text_button(ui, c, "Don't Ask Again", 13.0);
        if dont.clicked() {
            state.preferences.update_notifications = false;
            out.commands.push(UiCommand::SavePreferences);
            out.commands.push(UiCommand::DismissUpdate);
            state.modal = Modal::None;
            state.update = None;
        }
        let later = text_button(ui, c, "Later", 13.0);
        if later.clicked() {
            state.modal = Modal::None;
        }
    });
}

// ---------------------------------------------------------------------------
// Save Your Changes? (.unsaved-dialog 460px)
// ---------------------------------------------------------------------------

fn draw_confirm_close(ui: &mut egui::Ui, state: &mut AppState, c: &UiColors, out: &mut UiOutput) {
    let name = state.surface.name.clone();
    let closing = state.surface.message == "Closing Document";
    // .unsaved-dialog p — 13px muted with a strong name span.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.label(
            RichText::new("Your Changes To ")
                .size(13.0)
                .color(c.app_muted),
        );
        ui.label(RichText::new(name).size(13.0).color(c.app_muted).strong());
        ui.label(
            RichText::new(" Have Not Been Saved.")
                .size(13.0)
                .color(c.app_muted),
        );
    });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(7.0, 0.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let save = primary_button(ui, c, None, "Save Changes", 38.0, true);
            if save.clicked() {
                choose(state, out, UnsavedChoice::Save);
            }
            let cancel = secondary_button(ui, c, None, "Cancel", 38.0, true);
            if cancel.clicked() {
                choose(state, out, UnsavedChoice::Cancel);
            }
            let discard_label = if closing {
                "Close Without Saving"
            } else {
                "Discard Changes"
            };
            let discard = text_button(ui, c, discard_label, 12.0);
            if discard.clicked() {
                choose(state, out, UnsavedChoice::Discard);
            }
        });
    });
}

fn choose(state: &mut AppState, out: &mut UiOutput, choice: UnsavedChoice) {
    state.modal = Modal::None;
    state.pending_close_choice = Some(choice);
    out.commands.push(UiCommand::ConfirmChoice { choice });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 900.0))),
            events,
            ..Default::default()
        }
    }

    fn visit_shapes(shapes: &[egui::epaint::ClippedShape], visitor: &mut impl FnMut(&egui::Shape)) {
        fn visit(shape: &egui::Shape, visitor: &mut impl FnMut(&egui::Shape)) {
            visitor(shape);
            if let egui::Shape::Vec(children) = shape {
                for child in children {
                    visit(child, visitor);
                }
            }
        }
        for shape in shapes {
            visit(&shape.shape, visitor);
        }
    }

    #[test]
    fn settings_dialog_matches_bounds_and_keeps_description_above_cards() {
        let context = egui::Context::default();
        theme::install_fonts(
            &context,
            include_bytes!("../../fonts/Inter-Regular.ttf"),
            include_bytes!("../../fonts/Inter-Medium.ttf"),
            include_bytes!("../../fonts/Inter-SemiBold.ttf"),
        );
        context.style_mut(|style| style.animation_time = 0.0);
        let mut state = AppState::default();
        state.modal = Modal::Settings;
        state.preferences.theme = ThemePref::Dark;
        theme::apply_theme(&context, theme::Theme::Dark);
        let assets = ChromeAssets {
            theme: theme::Theme::Dark,
            logo: None,
        };
        let mut output = None;
        for _ in 0..4 {
            output = Some(context.run(input(Vec::new()), |context| {
                draw(context, &mut state, &assets, &mut UiOutput::default());
            }));
        }
        let mut modal_rect = None;
        let mut description_bottom = 0.0_f32;
        let mut card_top = f32::INFINITY;
        let colors = UiColors::for_theme(theme::Theme::Dark);
        visit_shapes(&output.unwrap().shapes, &mut |shape| match shape {
            egui::Shape::Rect(rectangle)
                if rectangle.fill == colors.app_surface && rectangle.rect.width() > 550.0 =>
            {
                modal_rect = Some(rectangle.rect)
            }
            egui::Shape::Rect(rectangle)
                if rectangle.stroke.width == 2.0 && rectangle.rect.width() > 100.0 =>
            {
                card_top = card_top.min(rectangle.rect.top())
            }
            egui::Shape::Text(text) if text.galley.job.text.starts_with("Choose A Theme") => {
                description_bottom = text.pos.y + text.galley.size().y
            }
            _ => {}
        });
        let bounds = modal_rect.expect("Settings Modal Frame");
        assert!((bounds.width() - 580.0).abs() <= 2.0, "{bounds:?}");
        assert!((bounds.height() - 840.0).abs() <= 2.0, "{bounds:?}");
        assert!((bounds.top() - 31.0).abs() <= 2.0, "{bounds:?}");
        assert!(
            card_top > description_bottom + 10.0,
            "Card {card_top}, Description {description_bottom}"
        );
    }

    #[test]
    fn word_goal_draft_keeps_empty_text_until_the_next_keystroke() {
        let context = egui::Context::default();
        let colors = UiColors::for_theme(theme::Theme::Light);
        let mut value = 0;
        let mut edit_id = Id::NULL;
        let mut draft_id = Id::NULL;
        let mut frame = |events: Vec<egui::Event>| {
            let _ = context.run(input(events), |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    ui.set_width(522.0);
                    edit_id = ui.make_persistent_id(("number-edit", "Word Goal"));
                    draft_id = ui.make_persistent_id(("number-draft", "Word Goal"));
                    setting_field_number(
                        ui,
                        &colors,
                        "Word Goal",
                        "Writing Target",
                        value,
                        0,
                        100000,
                        |next| value = next,
                    );
                });
            });
        };
        frame(Vec::new());
        drop(frame);
        context.memory_mut(|memory| memory.request_focus(edit_id));
        let mut text_state = egui::text_edit::TextEditState::load(&context, edit_id).unwrap();
        text_state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(1),
            )));
        text_state.store(&context, edit_id);
        let mut draw_frame = |events: Vec<egui::Event>| {
            let _ = context.run(input(events), |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    ui.set_width(522.0);
                    setting_field_number(
                        ui,
                        &colors,
                        "Word Goal",
                        "Writing Target",
                        value,
                        0,
                        100000,
                        |next| value = next,
                    );
                });
            });
        };
        draw_frame(vec![egui::Event::Key {
            key: egui::Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert_eq!(
            context
                .data(|data| data.get_temp::<NumberDraft>(draft_id))
                .unwrap()
                .text,
            ""
        );
        draw_frame(vec![egui::Event::Text("12".into())]);
        drop(draw_frame);
        assert_eq!(value, 12);
    }
    #[test]
    fn shortcuts_do_not_inherit_the_settings_dialog_height() {
        let context = egui::Context::default();
        theme::install_fonts(
            &context,
            crate::INTER_TTF,
            crate::INTER_MEDIUM_TTF,
            crate::INTER_SEMIBOLD_TTF,
        );
        context.style_mut(|style| style.animation_time = 0.0);
        theme::apply_theme(&context, theme::Theme::Dark);
        let assets = ChromeAssets {
            theme: theme::Theme::Dark,
            logo: None,
        };
        let colors = UiColors::for_theme(theme::Theme::Dark);
        let mut state = AppState::default();
        let mut heights = Vec::new();
        for modal in [Modal::Settings, Modal::Shortcuts] {
            state.modal = modal;
            let mut output = None;
            for _ in 0..4 {
                output = Some(context.run(input(Vec::new()), |context| {
                    draw(context, &mut state, &assets, &mut UiOutput::default())
                }));
            }
            let mut height = None;
            visit_shapes(&output.unwrap().shapes, &mut |shape| {
                if let egui::Shape::Rect(rectangle) = shape {
                    if rectangle.fill == colors.app_surface
                        && (rectangle.rect.width() - 580.0).abs() < 2.0
                        && rectangle.rect.height() > 100.0
                    {
                        height = Some(rectangle.rect.height());
                    }
                }
            });
            heights.push(height.expect("Modal Frame"));
        }
        assert!(heights[0] > 800.0);
        assert!(heights[1] < 650.0);
    }
}
