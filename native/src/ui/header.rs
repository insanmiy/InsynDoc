//! `.app-header` — logo, document title row (rename + save indicator), menu
//! bar triggers and the right-side actions (Import / Settings / Download).
//!
//! Layout (styles.css):
//! - min-height 78px, padding 8px 20px 8px 16px (panel frame supplies this),
//!   gap 12px, background app_surface.
//! - `.app-logo` 44x48, image 44x44 at top (+5 margin).
//! - `.document-heading` flex-1: `.document-title-row` height 32, gap 18 —
//!   h1 18px/600/ls-0.4 max-width 40vw ellipsis; `.document-save-indicator`
//!   12px muted gap 6.
//! - `.main-menu-host` min-height 30 — real menus or `.menu-placeholder`
//!   (File Edit View Insert Format Tools, muted 13px, gap 24, pl 5) when
//!   `!state.surface.ready`.
//! - `.header-actions` gap 7, self-center: Import (secondary-button h36),
//!   Settings (icon-button 36 circle), DownloadMenu (primary-button h36 +
//!   270px popup).

use egui::{vec2, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke};
use serde_json::json;

use crate::icons::{self, IconName};
use crate::state::{AppState, Modal};
use crate::theme::{self, UiColors};

use super::menubar;
use super::{colors, framed_text_edit, icon_button, secondary_button, ChromeAssets, UiOutput};

const DOWNLOAD_FORMATS: &[(&str, &str)] = &[
    ("docx", "Microsoft Word (.docx)"),
    ("pdf", "PDF Document (.pdf)"),
    ("odt", "OpenDocument Format (.odt)"),
    ("txt", "Plain Text (.txt)"),
    ("rtf", "Rich Text Format (.rtf)"),
    ("html", "Web Page (.html, Zipped)"),
    ("epub", "EPUB Publication (.epub)"),
    ("md", "Markdown (.md)"),
];

pub fn draw(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    assets: &ChromeAssets,
    out: &mut UiOutput,
) {
    let c = colors(assets);
    let unavailable = state.unavailable();

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(12.0, 0.0);

        // .app-logo — 44x48 area, 44x44 image 5px from top.
        let (logo_rect, _) = ui.allocate_exact_size(vec2(44.0, 48.0), Sense::hover());
        let img_rect = Rect::from_min_size(
            Pos2::new(logo_rect.left(), logo_rect.top() + 7.0),
            vec2(44.0, 44.0),
        );
        if let Some(tex) = assets.logo {
            ui.painter().image(
                tex,
                img_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            icons::paint_icon_at(
                ctx,
                &ui.painter(),
                IconName::Document,
                img_rect.shrink(6.0),
                c.app_muted,
            );
        }

        // .document-heading — flex 1 (reserve ~285px for .header-actions).
        let heading_w = (ui.available_width() - 285.0).max(120.0);
        ui.allocate_ui_with_layout(
            vec2(heading_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_min_height(32.0);
                draw_title_row(ui, ctx, state, &c, out, unavailable);
                draw_menu_row(ui, ctx, state, &c, out);
            },
        );

        // .header-actions — pushed right via right_to_left layout.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(7.0, 0.0);
            draw_download(ui, ctx, state, &c, out, unavailable);
            let settings = icon_button(ui, &c, IconName::Settings, 20.0, "Settings (Ctrl+,)");
            if settings.clicked() {
                state.modal = Modal::Settings;
            }
            let import =
                secondary_button(ui, &c, Some(IconName::Folder), "Import", 36.0, !unavailable);
            if import.clicked() && !unavailable {
                out.command("open");
            }
        });
    });
}

// ---------------------------------------------------------------------------
// .document-title-row
// ---------------------------------------------------------------------------

fn draw_title_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    out: &mut UiOutput,
    unavailable: bool,
) {
    ui.set_height(32.0);
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(18.0, 0.0);
        draw_name_control(ui, ctx, state, c, out, unavailable);
        draw_save_indicator(ui, c, state);
    });
}

fn draw_name_control(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    out: &mut UiOutput,
    unavailable: bool,
) {
    let max_w = (ctx.screen_rect().width() * 0.40).min(560.0);
    if state.renaming {
        // .document-name-input — single line, accent border, radius 4.
        let edit = framed_text_edit(ui, c, &mut state.title_draft, 220.0_f32.max(180.0), true);
        let focus_key = egui::Id::new("rename_needs_focus");
        if ctx
            .data(|data| data.get_temp::<bool>(focus_key))
            .unwrap_or(false)
        {
            edit.request_focus();
            ctx.data_mut(|data| data.remove::<bool>(focus_key));
        }
        // Commit on Enter; cancel on Escape; commit on blur (focus loss).
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            commit_rename(state, out);
        }
        if state.renaming && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.renaming = false;
        }
        if state.renaming && edit.lost_focus() {
            commit_rename(state, out);
        }
        return;
    }

    // .document-name-button — 18px/600, ls -0.4, ellipsis, edit icon 14.
    let name = state.surface.name.clone();
    let font = theme::ui_font_id_weight(18.0, 600);
    let galley = ui
        .painter()
        .layout_no_wrap(name.clone(), font.clone(), c.app_text);
    let text_w = galley.size().x.min(max_w);
    let (rect, response) = ui.allocate_exact_size(
        vec2((text_w + 10.0 + 14.0 + 10.0).min(max_w + 34.0), 30.0),
        Sense::click(),
    );
    if response.hovered() && !unavailable {
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(4),
            Stroke::new(1.0_f32, c.app_border),
            egui::StrokeKind::Inside,
        );
    }
    // Ellipsized name.
    let shown = ellipsize(ui, &name, max_w, &font, c.app_text);
    ui.painter().text(
        Pos2::new(rect.left() + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        shown,
        font,
        if unavailable {
            dim(c.app_text, 0.4)
        } else {
            c.app_text
        },
    );
    let icon_x = rect.left() + 4.0 + text_w.min(max_w) + 10.0;
    icons::paint_icon_at(
        ctx,
        &ui.painter(),
        IconName::Edit,
        Rect::from_center_size(Pos2::new(icon_x + 7.0, rect.center().y), vec2(14.0, 14.0)),
        c.app_muted,
    );
    if response.clicked() && !unavailable {
        state.renaming = true;
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("rename_needs_focus"), true));
        state.title_draft = strip_docx(&state.surface.name);
    }
}

fn commit_rename(state: &mut AppState, out: &mut UiOutput) {
    state.renaming = false;
    let name = state.title_draft.trim().to_string();
    if !name.is_empty() {
        out.exec("rename", json!({ "name": name }));
    }
}

fn strip_docx(name: &str) -> String {
    name.strip_suffix(".docx")
        .or_else(|| name.strip_suffix(".DOCX"))
        .unwrap_or(name)
        .to_string()
}

fn ellipsize(ui: &egui::Ui, text: &str, max_w: f32, font: &FontId, color: Color32) -> String {
    if ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), color)
        .size()
        .x
        <= max_w
    {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars() {
        let candidate = format!("{}…", out);
        if ui
            .painter()
            .layout_no_wrap(candidate.clone(), font.clone(), color)
            .size()
            .x
            > max_w
        {
            break;
        }
        out.push(ch);
    }
    format!("{}…", out)
}

fn dim(color: Color32, f: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color.r() as f32 * f) as u8,
        (color.g() as f32 * f) as u8,
        (color.b() as f32 * f) as u8,
        color.a(),
    )
}

fn draw_save_indicator(ui: &mut egui::Ui, c: &UiColors, state: &AppState) {
    let (icon, label_text, dirty) = if state.surface.dirty {
        (IconName::Save, "Unsaved Changes", true)
    } else if state.surface.token.is_some() {
        (IconName::Check, "Saved Locally", false)
    } else {
        (IconName::Check, "New Document", false)
    };
    let color = if dirty { c.app_accent_ink } else { c.app_muted };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
        ui.set_height(20.0);
        let (rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
        icons::paint_icon_at(ui.ctx(), &ui.painter(), icon, rect, color);
        ui.label(RichText::new(label_text).size(12.0).color(color));
    });
}

// ---------------------------------------------------------------------------
// .main-menu-host
// ---------------------------------------------------------------------------

fn draw_menu_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    out: &mut UiOutput,
) {
    if !state.surface.ready {
        // .menu-placeholder — muted 13px, gap 24, padding-left 5, height 30.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(24.0, 0.0);
            ui.add_space(5.0);
            ui.set_height(30.0);
            ui.vertical(|ui| {
                ui.set_height(30.0);
                ui.horizontal_centered(|ui| {
                    for label in ["File", "Edit", "View", "Insert", "Format", "Tools", "Help"] {
                        ui.label(RichText::new(label).size(13.0).color(c.app_muted));
                    }
                });
            });
        });
        return;
    }
    ui.set_min_height(30.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        menubar::draw_triggers(ui, ctx, state, c, out);
    });
}

// ---------------------------------------------------------------------------
// DownloadMenu (.download-menu-container + .download-format-menu)
// ---------------------------------------------------------------------------

fn draw_download(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    _out: &mut UiOutput,
    unavailable: bool,
) {
    // Trigger: primary-button h36, gap 7 — download icon 16 + "Download" +
    // chevronDown 14. Split into two painted halves is unnecessary; the whole
    // button toggles `Modal::DownloadMenu`.
    let font = theme::ui_font_id_weight(13.0, 500);
    let dark = c.app_bg == Color32::from_rgb(0x11, 0x13, 0x18);
    let fg = if dark {
        Color32::from_rgb(0x12, 0x24, 0x49)
    } else {
        Color32::WHITE
    };
    let text_w = ui
        .painter()
        .layout_no_wrap("Download".to_string(), font.clone(), fg)
        .size()
        .x;
    let width = 17.0 + 8.0 + 16.0 + 8.0 + text_w + 8.0 + 16.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width.max(96.0), 36.0), Sense::click());
    let open = state.modal == Modal::DownloadMenu;
    let hovered = response.hovered() && !unavailable;
    let bg = if !unavailable && hovered {
        c.app_accent_hover
    } else {
        c.app_accent
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(8), bg);
    let alpha = if unavailable { 0.4 } else { 1.0 };
    let fg_dim = Color32::from_rgba_unmultiplied(
        (fg.r() as f32 * alpha) as u8,
        (fg.g() as f32 * alpha) as u8,
        (fg.b() as f32 * alpha) as u8,
        255,
    );
    let mut x = rect.left() + 17.0;
    let cy = rect.center().y;
    icons::paint_icon_at(
        ctx,
        &ui.painter(),
        IconName::Download,
        Rect::from_center_size(Pos2::new(x + 8.0, cy), vec2(16.0, 16.0)),
        fg_dim,
    );
    x += 16.0 + 8.0;
    ui.painter().text(
        Pos2::new(x, cy),
        Align2::LEFT_CENTER,
        "Download",
        font.clone(),
        fg_dim,
    );
    x += text_w + 8.0;
    icons::paint_icon_at(
        ctx,
        &ui.painter(),
        IconName::ChevronDown,
        Rect::from_center_size(Pos2::new(x + 7.0, cy), vec2(14.0, 14.0)),
        fg_dim,
    );
    if response.clicked() && !unavailable {
        state.modal = if open {
            Modal::None
        } else {
            Modal::DownloadMenu
        };
    }
    // Stash trigger rect for the popup.
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("download_trigger_rect"), rect);
    });
}

/// Render the 270px download format popup (called from modals::draw when
/// `state.modal == Modal::DownloadMenu`). Also records the popup rect in
/// `download_popup_rect` so the caller can close on outside clicks.
pub fn draw_download_popup(
    ctx: &egui::Context,
    state: &mut AppState,
    c: &UiColors,
    out: &mut UiOutput,
) {
    let rect = ctx
        .data(|d| d.get_temp::<Rect>(egui::Id::new("download_trigger_rect")))
        .unwrap_or(Rect::NOTHING);
    if rect == Rect::NOTHING {
        return;
    }
    let pos = Pos2::new(rect.right() - 270.0, rect.bottom() + 6.0);
    let selected_id = egui::Id::new("download_selected_index");
    let mut selected = ctx
        .data(|d| d.get_temp::<usize>(selected_id))
        .unwrap_or(0)
        .min(DOWNLOAD_FORMATS.len() - 1);
    use egui::Key;
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::ArrowDown)) {
        selected = (selected + 1) % DOWNLOAD_FORMATS.len();
    }
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::ArrowUp)) {
        selected = (selected + DOWNLOAD_FORMATS.len() - 1) % DOWNLOAD_FORMATS.len();
    }
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Home)) {
        selected = 0;
    }
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::End)) {
        selected = DOWNLOAD_FORMATS.len() - 1;
    }
    let mut activated = None;
    if ctx.input_mut(|input| {
        input.consume_key(egui::Modifiers::NONE, Key::Enter)
            || input.consume_key(egui::Modifiers::NONE, Key::Space)
    }) {
        activated = Some(selected);
    }
    ctx.data_mut(|d| d.insert_temp(selected_id, selected));
    let inner = egui::Area::new(egui::Id::new("download_popup"))
        .order(egui::Order::Foreground)
        .pivot(egui::Align2::RIGHT_TOP)
        .fixed_pos(Pos2::new(rect.right(), rect.bottom() + 6.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(c.app_surface)
                .stroke(Stroke::new(1.0_f32, c.app_border))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::same(5))
                .show(ui, |ui| {
                    ui.set_width(270.0);
                    for (index, (format, label)) in DOWNLOAD_FORMATS.iter().enumerate() {
                        let highlighted = index == selected;
                        let (item_rect, response) =
                            ui.allocate_exact_size(vec2(260.0, 34.0), Sense::click());
                        if response.hovered() || highlighted {
                            ui.painter().rect_filled(
                                item_rect,
                                egui::CornerRadius::same(4),
                                c.app_hover,
                            );
                        }
                        ui.painter().text(
                            Pos2::new(item_rect.left() + 12.0, item_rect.center().y),
                            Align2::LEFT_CENTER,
                            *label,
                            FontId::proportional(13.0),
                            c.app_text,
                        );
                        if response.clicked() {
                            out.command_args("download", json!({ "format": format }));
                            state.modal = Modal::None;
                        }
                    }
                });
        });
    if let Some(index) = activated {
        let (format, _) = DOWNLOAD_FORMATS[index];
        out.command_args("download", json!({ "format": format }));
        state.modal = Modal::None;
    }
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("download_popup_rect"), inner.response.rect);
    });
    let _ = pos;
}
