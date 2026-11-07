//! Design tokens ported from src/styles.css.
//! Keep values in sync with the CSS custom properties.

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, Stroke, Vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Clone, Copy)]
pub struct UiColors {
    pub app_bg: Color32,
    pub app_surface: Color32,
    pub app_toolbar: Color32,
    pub app_hover: Color32,
    pub app_text: Color32,
    pub app_muted: Color32,
    pub app_border: Color32,
    pub app_accent: Color32,
    pub app_accent_hover: Color32,
    pub app_accent_soft: Color32,
    pub app_accent_ink: Color32,
    pub app_error: Color32,
    pub error_banner_bg: Color32,
    pub error_banner_fg: Color32,
    pub switch_track: Color32,
    pub modal_backdrop: Color32,
    pub focus_strip_bg: Color32,
    pub busy_bg: Color32,
}

impl UiColors {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self {
                app_bg: rgb(0xf1, 0xf3, 0xf7),
                app_surface: rgb(0xff, 0xff, 0xff),
                app_toolbar: rgb(0xea, 0xf0, 0xfa),
                app_hover: rgb(0xe9, 0xed, 0xf3),
                app_text: rgb(0x25, 0x2a, 0x32),
                app_muted: rgb(0x64, 0x70, 0x84),
                app_border: rgb(0xd9, 0xdf, 0xe8),
                app_accent: rgb(0x25, 0x63, 0xeb),
                app_accent_hover: rgb(0x1d, 0x4e, 0xd8),
                app_accent_soft: rgb(0xdb, 0xe7, 0xff),
                app_accent_ink: rgb(0x23, 0x4b, 0x93),
                app_error: rgb(0xb3, 0x26, 0x1e),
                error_banner_bg: rgb(0xff, 0xf0, 0xe5),
                error_banner_fg: rgb(0x8e, 0x41, 0x1b),
                switch_track: rgb(0x73, 0x7f, 0x90),
                modal_backdrop: Color32::from_rgba_unmultiplied(0x0b, 0x14, 0x2f, 0x66),
                focus_strip_bg: rgb(0xf8, 0xfa, 0xfd),
                busy_bg: rgb(0xdb, 0xe7, 0xff),
            },
            Theme::Dark => Self {
                app_bg: rgb(0x11, 0x13, 0x18),
                app_surface: rgb(0x1b, 0x1e, 0x24),
                app_toolbar: rgb(0x22, 0x26, 0x2e),
                app_hover: rgb(0x2d, 0x33, 0x3e),
                app_text: rgb(0xe7, 0xea, 0xf0),
                app_muted: rgb(0xaf, 0xb7, 0xc6),
                app_border: rgb(0x38, 0x3f, 0x4b),
                app_accent: rgb(0x93, 0xb8, 0xff),
                app_accent_hover: rgb(0xb1, 0xcc, 0xff),
                app_accent_soft: rgb(0x28, 0x3b, 0x5c),
                app_accent_ink: rgb(0xc6, 0xd9, 0xff),
                app_error: rgb(0xff, 0xb4, 0xab),
                error_banner_bg: rgb(0x40, 0x2b, 0x25),
                error_banner_fg: rgb(0xff, 0xbd, 0x9e),
                switch_track: rgb(0x75, 0x81, 0x96),
                modal_backdrop: Color32::from_rgba_unmultiplied(0x0b, 0x14, 0x2f, 0x66),
                focus_strip_bg: rgb(0x11, 0x13, 0x18),
                busy_bg: rgb(0x28, 0x3b, 0x5c),
            },
        }
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

// Layout metrics from styles.css (logical pixels at 100% scale).
pub const HEADER_HEIGHT: f32 = 78.0;
pub const STATUS_HEIGHT: f32 = 34.0;
pub const HEADER_PADDING_X: f32 = 20.0;
pub const HEADER_PADDING_LEFT: f32 = 16.0;
pub const FOCUS_STRIP_HEIGHT: f32 = 38.0;
pub const FONT_SIZE_BASE: f32 = 13.0;
pub const FONT_SIZE_SMALL: f32 = 12.0;
pub const FONT_SIZE_TITLE: f32 = 18.0;
pub const FONT_SIZE_MODAL_TITLE: f32 = 22.0;

pub fn ui_font_id(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Insyn Inter".into()))
}

/// Inter instances matching CSS font-weight 400 / 500 / 600.
pub fn ui_font_id_weight(size: f32, weight: u16) -> FontId {
    let family = match weight {
        500..=599 => "Insyn Inter Medium",
        600..=899 => "Insyn Inter SemiBold",
        _ => "Insyn Inter",
    };
    FontId::new(size, FontFamily::Name(family.into()))
}

/// Install the Inter font instances (instanced from the variable TTF) into
/// egui. Call once at startup.
pub fn install_fonts(ctx: &egui::Context, regular: &[u8], medium: &[u8], semibold: &[u8]) {
    let mut fonts = FontDefinitions::default();
    for (key, bytes) in [
        ("insyn_inter", regular),
        ("insyn_inter_medium", medium),
        ("insyn_inter_semibold", semibold),
    ] {
        fonts.font_data.insert(
            key.to_owned(),
            std::sync::Arc::new(FontData::from_owned(bytes.to_vec())),
        );
    }
    fonts
        .families
        .entry(FontFamily::Name("Insyn Inter".into()))
        .or_default()
        .insert(0, "insyn_inter".to_owned());
    fonts
        .families
        .entry(FontFamily::Name("Insyn Inter Medium".into()))
        .or_default()
        .insert(0, "insyn_inter_medium".to_owned());
    fonts
        .families
        .entry(FontFamily::Name("Insyn Inter SemiBold".into()))
        .or_default()
        .insert(0, "insyn_inter_semibold".to_owned());
    // Also make regular Inter the default proportional family.
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "insyn_inter".to_owned());
    let system_root = std::env::var_os("WINDIR").unwrap_or_else(|| "C:/Windows".into());
    let system_font_dir = std::path::PathBuf::from(system_root).join("Fonts");
    for (family, filenames) in [
        (
            "Arial",
            ["arial.ttf", "arialbd.ttf", "ariali.ttf", "arialbi.ttf"],
        ),
        (
            "Calibri",
            [
                "calibri.ttf",
                "calibrib.ttf",
                "calibrii.ttf",
                "calibriz.ttf",
            ],
        ),
        (
            "Cambria",
            [
                "cambria.ttc",
                "cambriab.ttf",
                "cambriai.ttf",
                "cambriaz.ttf",
            ],
        ),
        (
            "Consolas",
            [
                "consola.ttf",
                "consolab.ttf",
                "consolai.ttf",
                "consolaz.ttf",
            ],
        ),
        (
            "Courier New",
            ["cour.ttf", "courbd.ttf", "couri.ttf", "courbi.ttf"],
        ),
        (
            "Georgia",
            [
                "georgia.ttf",
                "georgiab.ttf",
                "georgiai.ttf",
                "georgiaz.ttf",
            ],
        ),
        (
            "Segoe UI",
            [
                "segoeui.ttf",
                "segoeuib.ttf",
                "segoeuii.ttf",
                "segoeuiz.ttf",
            ],
        ),
        (
            "Tahoma",
            ["tahoma.ttf", "tahomabd.ttf", "tahoma.ttf", "tahomabd.ttf"],
        ),
        (
            "Times New Roman",
            ["times.ttf", "timesbd.ttf", "timesi.ttf", "timesbi.ttf"],
        ),
        (
            "Trebuchet MS",
            ["trebuc.ttf", "trebucbd.ttf", "trebucit.ttf", "trebucbi.ttf"],
        ),
        (
            "Verdana",
            [
                "verdana.ttf",
                "verdanab.ttf",
                "verdanai.ttf",
                "verdanaz.ttf",
            ],
        ),
    ] {
        for (suffix, filename) in ["", " Bold", " Italic", " Bold Italic"]
            .into_iter()
            .zip(filenames)
        {
            if let Ok(bytes) = std::fs::read(system_font_dir.join(filename)) {
                register_document_font(&mut fonts, &format!("{family}{suffix}"), bytes);
            }
        }
    }
    for (family, bytes) in [
        (
            "Comic Neue Italic",
            include_bytes!("../../public/document-fonts/ComicNeue-Italic.ttf") as &[u8],
        ),
        (
            "Comic Neue",
            include_bytes!("../../public/document-fonts/ComicNeue-Regular.ttf") as &[u8],
        ),
        (
            "Comic Neue Bold Italic",
            include_bytes!("../../public/document-fonts/ComicNeue-BoldItalic.ttf") as &[u8],
        ),
        (
            "Comic Neue Bold",
            include_bytes!("../../public/document-fonts/ComicNeue-Bold.ttf") as &[u8],
        ),
        (
            "Crimson Text Italic",
            include_bytes!("../../public/document-fonts/CrimsonText-Italic.ttf") as &[u8],
        ),
        (
            "Crimson Text",
            include_bytes!("../../public/document-fonts/CrimsonText-Regular.ttf") as &[u8],
        ),
        (
            "Crimson Text Bold Italic",
            include_bytes!("../../public/document-fonts/CrimsonText-BoldItalic.ttf") as &[u8],
        ),
        (
            "Crimson Text Bold",
            include_bytes!("../../public/document-fonts/CrimsonText-Bold.ttf") as &[u8],
        ),
        (
            "Fira Sans Italic",
            include_bytes!("../../public/document-fonts/FiraSans-Italic.ttf") as &[u8],
        ),
        (
            "Fira Sans",
            include_bytes!("../../public/document-fonts/FiraSans-Regular.ttf") as &[u8],
        ),
        (
            "Fira Sans Bold Italic",
            include_bytes!("../../public/document-fonts/FiraSans-BoldItalic.ttf") as &[u8],
        ),
        (
            "Fira Sans Bold",
            include_bytes!("../../public/document-fonts/FiraSans-Bold.ttf") as &[u8],
        ),
        (
            "Lato Italic",
            include_bytes!("../../public/document-fonts/Lato-Italic.ttf") as &[u8],
        ),
        (
            "Lato",
            include_bytes!("../../public/document-fonts/Lato-Regular.ttf") as &[u8],
        ),
        (
            "Lato Bold Italic",
            include_bytes!("../../public/document-fonts/Lato-BoldItalic.ttf") as &[u8],
        ),
        (
            "Lato Bold",
            include_bytes!("../../public/document-fonts/Lato-Bold.ttf") as &[u8],
        ),
        (
            "Lobster Two Italic",
            include_bytes!("../../public/document-fonts/LobsterTwo-Italic.ttf") as &[u8],
        ),
        (
            "Lobster Two",
            include_bytes!("../../public/document-fonts/LobsterTwo-Regular.ttf") as &[u8],
        ),
        (
            "Lobster Two Bold Italic",
            include_bytes!("../../public/document-fonts/LobsterTwo-BoldItalic.ttf") as &[u8],
        ),
        (
            "Lobster Two Bold",
            include_bytes!("../../public/document-fonts/LobsterTwo-Bold.ttf") as &[u8],
        ),
        (
            "Montserrat Italic",
            include_bytes!("../../public/document-fonts/Montserrat-Italic.ttf") as &[u8],
        ),
        (
            "Montserrat",
            include_bytes!("../../public/document-fonts/Montserrat-Regular.ttf") as &[u8],
        ),
        (
            "Montserrat Bold Italic",
            include_bytes!("../../public/document-fonts/Montserrat-BoldItalic.ttf") as &[u8],
        ),
        (
            "Montserrat Bold",
            include_bytes!("../../public/document-fonts/Montserrat-Bold.ttf") as &[u8],
        ),
        (
            "Open Sans Italic",
            include_bytes!("../../public/document-fonts/OpenSans-Italic.ttf") as &[u8],
        ),
        (
            "Open Sans",
            include_bytes!("../../public/document-fonts/OpenSans-Regular.ttf") as &[u8],
        ),
        (
            "Open Sans Bold Italic",
            include_bytes!("../../public/document-fonts/OpenSans-BoldItalic.ttf") as &[u8],
        ),
        (
            "Open Sans Bold",
            include_bytes!("../../public/document-fonts/OpenSans-Bold.ttf") as &[u8],
        ),
        (
            "Poppins Italic",
            include_bytes!("../../public/document-fonts/Poppins-Italic.ttf") as &[u8],
        ),
        (
            "Poppins",
            include_bytes!("../../public/document-fonts/Poppins-Regular.ttf") as &[u8],
        ),
        (
            "Poppins Bold Italic",
            include_bytes!("../../public/document-fonts/Poppins-BoldItalic.ttf") as &[u8],
        ),
        (
            "Poppins Bold",
            include_bytes!("../../public/document-fonts/Poppins-Bold.ttf") as &[u8],
        ),
        (
            "PT Sans Italic",
            include_bytes!("../../public/document-fonts/PT_Sans-Web-Italic.ttf") as &[u8],
        ),
        (
            "PT Sans",
            include_bytes!("../../public/document-fonts/PT_Sans-Web-Regular.ttf") as &[u8],
        ),
        (
            "PT Sans Bold Italic",
            include_bytes!("../../public/document-fonts/PT_Sans-Web-BoldItalic.ttf") as &[u8],
        ),
        (
            "PT Sans Bold",
            include_bytes!("../../public/document-fonts/PT_Sans-Web-Bold.ttf") as &[u8],
        ),
        (
            "PT Serif Italic",
            include_bytes!("../../public/document-fonts/PT_Serif-Web-Italic.ttf") as &[u8],
        ),
        (
            "PT Serif",
            include_bytes!("../../public/document-fonts/PT_Serif-Web-Regular.ttf") as &[u8],
        ),
        (
            "PT Serif Bold Italic",
            include_bytes!("../../public/document-fonts/PT_Serif-Web-BoldItalic.ttf") as &[u8],
        ),
        (
            "PT Serif Bold",
            include_bytes!("../../public/document-fonts/PT_Serif-Web-Bold.ttf") as &[u8],
        ),
        (
            "Roboto Italic",
            include_bytes!("../../public/document-fonts/Roboto-Italic.ttf") as &[u8],
        ),
        (
            "Roboto",
            include_bytes!("../../public/document-fonts/Roboto-Regular.ttf") as &[u8],
        ),
        (
            "Roboto Bold Italic",
            include_bytes!("../../public/document-fonts/Roboto-BoldItalic.ttf") as &[u8],
        ),
        (
            "Roboto Bold",
            include_bytes!("../../public/document-fonts/Roboto-Bold.ttf") as &[u8],
        ),
        (
            "Space Mono Italic",
            include_bytes!("../../public/document-fonts/SpaceMono-Italic.ttf") as &[u8],
        ),
        (
            "Space Mono",
            include_bytes!("../../public/document-fonts/SpaceMono-Regular.ttf") as &[u8],
        ),
        (
            "Space Mono Bold Italic",
            include_bytes!("../../public/document-fonts/SpaceMono-BoldItalic.ttf") as &[u8],
        ),
        (
            "Space Mono Bold",
            include_bytes!("../../public/document-fonts/SpaceMono-Bold.ttf") as &[u8],
        ),
    ] {
        register_document_font(&mut fonts, family, bytes.to_vec());
    }
    ctx.set_fonts(fonts);
}

pub const DOCUMENT_FAMILIES: &[&str] = &[
    "Arial",
    "Calibri",
    "Cambria",
    "Comic Neue",
    "Consolas",
    "Courier New",
    "Crimson Text",
    "Fira Sans",
    "Georgia",
    "Lato",
    "Lobster Two",
    "Montserrat",
    "Open Sans",
    "Poppins",
    "PT Sans",
    "PT Serif",
    "Roboto",
    "Segoe UI",
    "Space Mono",
    "Tahoma",
    "Times New Roman",
    "Trebuchet MS",
    "Verdana",
];

fn register_document_font(fonts: &mut FontDefinitions, family: &str, bytes: Vec<u8>) {
    fonts.font_data.insert(
        family.to_owned(),
        std::sync::Arc::new(FontData::from_owned(bytes)),
    );
    let mut fallback = fonts.families[&FontFamily::Proportional].clone();
    fallback.insert(0, family.to_owned());
    fonts
        .families
        .insert(FontFamily::Name(family.into()), fallback);
}

/// Apply theme colors to egui's global style.
pub fn apply_theme(ctx: &egui::Context, theme: Theme) {
    let c = UiColors::for_theme(theme);
    let mut style = (*ctx.style()).clone();
    style.visuals = if theme == Theme::Dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    style.visuals.panel_fill = c.app_bg;
    style.visuals.window_fill = c.app_surface;
    style.visuals.extreme_bg_color = c.app_toolbar;
    style.visuals.faint_bg_color = c.app_hover;
    style.visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, c.app_muted);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.app_border);
    style.visuals.widgets.inactive.bg_fill = c.app_surface;
    style.visuals.widgets.inactive.weak_bg_fill = c.app_surface;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, c.app_border);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, c.app_text);
    style.visuals.widgets.hovered.bg_fill = c.app_hover;
    style.visuals.widgets.hovered.weak_bg_fill = c.app_hover;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, c.app_border);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, c.app_text);
    style.visuals.widgets.active.bg_fill = c.app_accent_soft;
    style.visuals.widgets.active.weak_bg_fill = c.app_accent_soft;
    style.visuals.widgets.open.bg_fill = c.app_surface;
    style.visuals.widgets.open.weak_bg_fill = c.app_surface;
    style.visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, c.app_text);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, c.app_accent_ink);
    style.visuals.selection.bg_fill = c.app_accent_soft;
    style.visuals.selection.stroke = Stroke::new(1.0, c.app_accent);
    style.visuals.hyperlink_color = c.app_accent;
    style.visuals.override_text_color = Some(c.app_text);
    style.visuals.window_stroke = Stroke::new(1.0, c.app_border);
    style.visuals.window_corner_radius = egui::CornerRadius::same(18);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(8);
    style.visuals.popup_shadow = match theme {
        Theme::Light => egui::Shadow {
            offset: [0, 8],
            blur: 32,
            spread: 0,
            color: Color32::from_rgba_unmultiplied(0x1e, 0x30, 0x51, 0x26),
        },
        Theme::Dark => egui::Shadow {
            offset: [0, 12],
            blur: 40,
            spread: 0,
            color: Color32::from_black_alpha(153),
        },
    };
    ctx.set_style(style);
}

/// Hit area helpers matching the CSS icon-button (36x36, radius 50%).
pub fn icon_button_size() -> Vec2 {
    Vec2::splat(36.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_document_faces_load_and_render_with_distinct_metrics() {
        let context = egui::Context::default();
        install_fonts(
            &context,
            crate::INTER_TTF,
            crate::INTER_MEDIUM_TTF,
            crate::INTER_SEMIBOLD_TTF,
        );
        let mut widths = Vec::new();
        let _ = context.run(egui::RawInput::default(), |context| {
            context.fonts(|fonts| {
                let available = fonts.families();
                for family in [
                    "Roboto",
                    "Roboto Bold",
                    "Roboto Italic",
                    "Roboto Bold Italic",
                    "Space Mono",
                ] {
                    let name = FontFamily::Name(family.into());
                    assert!(available.contains(&name), "Missing {family}");
                    let text = fonts.layout_no_wrap(
                        "iiii WWWW".into(),
                        FontId::new(24.0, name),
                        Color32::BLACK,
                    );
                    assert!(!text.is_empty(), "Empty Layout For {family}");
                    if family == "Roboto" || family == "Space Mono" {
                        widths.push(text.size().x);
                    }
                }
            });
        });
        assert!(
            (widths[0] - widths[1]).abs() > 1.0,
            "Distinct Families Must Render With Their Own Metrics"
        );
    }
}
