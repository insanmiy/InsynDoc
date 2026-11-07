//! Icon set ported from src/Icon.tsx.
//!
//! Each glyph is the same SVG path data used by the React app (24x24 viewBox,
//! stroke width 1.7, round caps/joins, currentColor). Paths are rasterized once
//! per (icon, color) pair into egui textures via resvg/usvg and cached.
//!
//! Integration: declare `mod icons;` in main.rs (crate root). Call
//! `icons::warm(ctx)` once after fonts/theme are installed to pre-render the
//! common colors, or just call `icons::paint_icon` lazily — it caches on demand.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use egui::{Color32, Context, Painter, TextureHandle, TextureId, TextureOptions, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum IconName {
    Document,
    Edit,
    Search,
    Outline,
    Save,
    Download,
    Check,
    Close,
    ChevronDown,
    ArrowUp,
    ArrowDown,
    Focus,
    Moon,
    Sun,
    Monitor,
    Page,
    Folder,
    Info,
    Settings,
    Undo,
    Redo,
    Painter,
    Bold,
    Italic,
    Underline,
    TextColor,
    Highlight,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignJustify,
    LineSpacing,
    BulletList,
    NumberedList,
    Outdent,
    Indent,
    ClearFormatting,
    Link,
    Image,
    Table,
    More,
}

fn svg_body(name: IconName) -> &'static str {
    match name {
        IconName::Document => "M6 3h8l4 4v14H6z M14 3v5h4 M9 12h6 M9 15h6 M9 18h4",
        IconName::Edit => "m4 16 12-12 4 4L8 20H4z M14 6l4 4",
        IconName::Search => "M21 21l-5-5 M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0",
        IconName::Outline => "M8 5h13 M8 12h13 M8 19h9 M3 5h.01 M3 12h.01 M3 19h.01",
        IconName::Save => "M5 3h13l3 3v15H3V3z M7 3v6h10V3 M7 21v-8h10v8",
        IconName::Download => "M12 3v12 M7 10l5 5 5-5 M4 16v5h16v-5",
        IconName::Check => "M5 12l4 4L19 6",
        IconName::Close => "M6 6l12 12 M18 6L6 18",
        IconName::ChevronDown => "M6 9l6 6 6-6",
        IconName::ArrowUp => "M6 14l6-6 6 6",
        IconName::ArrowDown => "M6 10l6 6 6-6",
        IconName::Focus => "M8 3H3v5 M16 3h5v5 M21 16v5h-5 M3 16v5h5",
        IconName::Moon => "M20.9 13A9 9 0 0 1 11 3.1 9 9 0 1 0 20.9 13z",
        IconName::Sun => "M12 3V1 M12 23v-2 M3 12H1 M23 12h-2 M4.2 4.2l1.4 1.4 M18.4 18.4l1.4 1.4 M19.8 4.2l-1.4 1.4 M5.6 18.4l-1.4 1.4 M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0",
        IconName::Monitor => "M3 4h18v13H3z M8 21h8 M12 17v4",
        IconName::Page => "M5 2h14v20H5z M8 7h8 M8 11h8 M8 15h5",
        IconName::Folder => "M3 6V4h7l2 3h9v13H3z",
        IconName::Info => "M12 11v6 M12 7h.01 M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0",
        IconName::Undo => "M9 7H5V3 M5 7l3-3a7 7 0 1 1 1 13",
        IconName::Redo => "M15 7h4V3 M19 7l-3-3a7 7 0 1 0-1 13",
        IconName::Painter => "M5 3h14v6H5z M8 3v6 M17 9v4h-5v8H9v-8h5V9",
        IconName::Bold => "M7 4h6a4 4 0 0 1 0 8H7z M7 12h7a4 4 0 0 1 0 8H7z",
        IconName::Italic => "M10 4h9 M5 20h9 M15 4L9 20",
        IconName::Underline => "M6 3v8a6 6 0 0 0 12 0V3 M4 21h16",
        IconName::TextColor => "M5 18L12 3l7 15 M8 13h8 M4 22h16",
        IconName::Highlight => "M5 15l9-11 5 4-9 11H5z M12 6l5 4 M3 22h15",
        IconName::AlignLeft => "M3 4h18 M3 9h12 M3 14h18 M3 19h12",
        IconName::AlignCenter => "M3 4h18 M6 9h12 M3 14h18 M6 19h12",
        IconName::AlignRight => "M3 4h18 M9 9h12 M3 14h18 M9 19h12",
        IconName::AlignJustify => "M3 4h18 M3 9h18 M3 14h18 M3 19h18",
        IconName::LineSpacing => "M4 3v18 M1 6l3-3 3 3 M1 18l3 3 3-3 M11 5h10 M11 12h10 M11 19h10",
        IconName::BulletList => "M8 5h13 M8 12h13 M8 19h13 M3 5h.01 M3 12h.01 M3 19h.01",
        IconName::NumberedList => "M9 5h12 M9 12h12 M9 19h12 M2 3h2v5 M2 8h4 M2 12q4-3 4 0l-4 4h4 M2 19h3l-2 2h3",
        IconName::Outdent => "M10 4h11 M10 9h11 M10 14h11 M3 19h18 M7 7l-4 4 4 4",
        IconName::Indent => "M10 4h11 M10 9h11 M10 14h11 M3 19h18 M3 7l4 4-4 4",
        IconName::ClearFormatting => "M5 4h14 M12 4l-5 16 M3 3l18 18",
        IconName::Link => "M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-2 2 M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l2-2",
        IconName::Image => "M3 3h18v18H3z M3 17l6-6 4 4 3-3 5 5 M8 7h.01",
        IconName::Table => "M3 3h18v18H3z M3 9h18 M3 15h18 M9 3v18 M15 3v18",
        IconName::More => "M5 12h.01 M12 12h.01 M19 12h.01",
        IconName::Settings => {
            return "<path d=\"m9.5 3-.6 2.1-1.8 1-2.1-.5-2.5 4.3L4 11.5v2L2.5 15 5 19.3l2.1-.5 1.8 1 .6 2.2h5l.6-2.2 1.8-1 2.1.5 2.5-4.3-1.5-1.5v-2l1.5-1.6L19 5.6l-2.1.5-1.8-1L14.5 3z\"/><circle cx=\"12\" cy=\"12.5\" r=\"3.2\"/>"
        }
    }
}

fn svg_document(name: IconName) -> String {
    let body = svg_body(name);
    // IconName::Settings already returns full <path>/<circle> elements; all
    // other variants return bare path data that must be wrapped in <path d=""/>.
    let inner = if body.starts_with('<') {
        body.to_string()
    } else {
        format!("<path d=\"{body}\"/>")
    };
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" \
         stroke=\"#ffffff\" stroke-width=\"1.7\" stroke-linecap=\"round\" \
         stroke-linejoin=\"round\">{inner}</svg>"
    )
}

const TEX_SIZE: u32 = 64;

type IconCache = Mutex<HashMap<(IconName, u32), TextureHandle>>;

fn cache() -> &'static IconCache {
    static CACHE: OnceLock<IconCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn render_icon(ctx: &Context, name: IconName, color: Color32) -> TextureId {
    let key_color = ((color.r() as u32) << 24)
        | ((color.g() as u32) << 16)
        | ((color.b() as u32) << 8)
        | color.a() as u32;
    if let Some(handle) = cache().lock().unwrap().get(&(name, key_color)) {
        return handle.id();
    }

    let svg = svg_document(name);
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("icon svg must parse");
    let size = tree.size();
    let scale = TEX_SIZE as f32 / size.width().max(size.height());
    let w = (size.width() * scale).round().max(1.0) as u32;
    let h = (size.height() * scale).round().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).expect("pixmap alloc");
    resvg::render(
        &tree,
        usvg::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    // Source is a white stroke on transparency, so each premultiplied pixel is
    // (a, a, a, a). Re-tint to the requested UNMULTIPLIED color channels,
    // keeping the stroke coverage as alpha.
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for px in pixmap.pixels() {
        rgba.extend_from_slice(&[
            color.r(),
            color.g(),
            color.b(),
            ((u16::from(px.alpha()) * u16::from(color.a())) / 255) as u8,
        ]);
    }
    let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    let handle = ctx.load_texture(
        format!("icon_{:?}_{:08x}", name, key_color),
        image,
        TextureOptions::LINEAR,
    );
    cache()
        .lock()
        .unwrap()
        .insert((name, key_color), handle.clone());
    handle.id()
}

/// Paint an icon glyph centered in a `size` x `size` box at the painter's
/// current cursor position is NOT used; instead this paints at `rect` center.
/// Call inside a sized ui region or pass an explicit rect via
/// [`paint_icon_at`].
pub fn paint_icon_at(
    ctx: &Context,
    painter: &Painter,
    name: IconName,
    rect: egui::Rect,
    color: Color32,
) {
    let id = render_icon(ctx, name, color);
    painter.image(
        id,
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Spec-compatible helper: paint `name` at `size` points centered on `center`.
pub fn paint_icon(
    ctx: &Context,
    painter: &Painter,
    name: IconName,
    size: f32,
    color: Color32,
    center: egui::Pos2,
) {
    paint_icon_at(
        ctx,
        painter,
        name,
        egui::Rect::from_center_size(center, Vec2::splat(size)),
        color,
    );
}

/// Convenience: allocate `size` points, paint the icon centered, return the
/// response (useful for icon buttons).
pub fn icon_widget(ui: &mut egui::Ui, name: IconName, size: f32, color: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    paint_icon_at(ui.ctx(), &ui.painter(), name, rect, color);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_svg_icons_parse_and_contain_visible_paths() {
        for name in [
            IconName::Undo, IconName::Redo, IconName::Painter, IconName::Bold,
            IconName::Italic, IconName::Underline, IconName::TextColor, IconName::Highlight,
            IconName::AlignLeft, IconName::AlignCenter, IconName::AlignRight, IconName::AlignJustify,
            IconName::LineSpacing, IconName::BulletList, IconName::NumberedList, IconName::Outdent,
            IconName::Indent, IconName::ClearFormatting, IconName::Link, IconName::Image,
            IconName::Table, IconName::More,
        ] {
            let tree = usvg::Tree::from_str(&svg_document(name), &usvg::Options::default()).unwrap();
            let mut pixmap = tiny_skia::Pixmap::new(24, 24).unwrap();
            resvg::render(&tree, usvg::Transform::identity(), &mut pixmap.as_mut());
            assert!(pixmap.pixels().iter().any(|pixel| pixel.alpha() > 0), "{name:?}");
        }
    }
}
