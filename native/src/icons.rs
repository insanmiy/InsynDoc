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

/// Filled Material-style glyph paths (viewBox "0 -960 960 960", fill only,
/// no stroke) used by the web toolbar buttons. Returns the raw `d` attribute
/// for the glyph, or `None` when no filled variant exists.
fn svg_body_filled(name: IconName) -> Option<&'static str> {
    let path = match name {
        IconName::Undo => "M280-200v-80h284q63 0 109.5-40T720-420q0-60-46.5-100T564-560H312l104 104-56 56-200-200 200-200 56 56-104 104h252q97 0 166.5 63T800-420q0 94-69.5 157T564-200H280Z",
        IconName::Redo => "M396-200q-97 0-166.5-63T160-420q0-94 69.5-157T396-640h252L544-744l56-56 200 200-200 200-56-56 104-104H396q-63 0-109.5 40T240-420q0 60 46.5 100T396-280h284v80H396Z",
        IconName::Painter => "M440-80q-33 0-56.5-23.5T360-160v-160H240q-33 0-56.5-23.5T160-400v-280q0-66 47-113t113-47h480v440q0 33-23.5 56.5T720-320H600v160q0 33-23.5 56.5T520-80h-80ZM240-560h480v-200h-40v160h-80v-160h-40v80h-80v-80H320q-33 0-56.5 23.5T240-680v120Zm0 160h480v-80H240v80Zm0 0v-80 80Z",
        IconName::Bold => "M272-200v-560h221q65 0 120 40t55 111q0 51-23 78.5T602-491q25 11 55.5 41t30.5 90q0 89-65 124.5T501-200H272Zm121-112h104q48 0 58.5-24.5T566-372q0-11-10.5-35.5T494-432H393v120Zm0-228h93q33 0 48-17t15-38q0-24-17-39t-44-15h-95v109Z",
        IconName::Italic => "M200-200v-100h160l120-360H320v-100h400v100H580L460-300h140v100H200Z",
        IconName::Underline => "M200-120v-80h560v80H200Zm123-223q-56-63-56-167v-330h103v336q0 56 28 91t82 35q54 0 82-35t28-91v-336h103v330q0 104-56 167t-157 63q-101 0-157-63Z",
        IconName::TextColor => "M80 0v-160h800V0H80Zm140-280 210-560h100l210 560h-96l-50-144H368l-52 144h-96Zm176-224h168l-82-232h-4l-82 232Z",
        IconName::Highlight => "M544-400 440-504 240-304l104 104 200-200Zm-47-161 104 104 199-199-104-104-199 199Zm-84-28 216 216-229 229q-24 24-56 24t-56-24l-2-2-26 26H60l126-126-2-2q-24-24-24-56t24-56l229-229Zm0 0 227-227q24-24 56-24t56 24l104 104q24 24 24 56t-24 56L629-373 413-589Z",
        IconName::AlignLeft => "M120-120v-80h720v80H120Zm0-160v-80h480v80H120Zm0-160v-80h720v80H120Zm0-160v-80h480v80H120Zm0-160v-80h720v80H120Z",
        // Derived from the Align Left bar pattern: middle bars centered,
        // trailing bars right-aligned, justify uses full-width bars.
        IconName::AlignCenter => "M120-120v-80h720v80H120Zm240-160v-80h480v80H240Zm0-160v-80h720v80H120Zm240-160v-80h480v80H240Zm0-160v-80h720v80H120Z",
        IconName::AlignRight => "M120-120v-80h720v80H120Zm360-160v-80h480v80H360Zm0-160v-80h720v80H120Zm360-160v-80h480v80H360Zm0-160v-80h720v80H120Z",
        IconName::AlignJustify => "M120-120v-80h720v80H120Zm0-160v-80h720v80H120Zm0-160v-80h720v80H120Zm0-160v-80h720v80H120Zm0-160v-80h720v80H120Z",
        IconName::LineSpacing => "M240-160 80-320l56-56 64 62v-332l-64 62-56-56 160-160 160 160-56 56-64-62v332l64-62 56 56-160 160Zm240-40v-80h400v80H480Zm0-240v-80h400v80H480Zm0-240v-80h400v80H480Z",
        IconName::BulletList => "M360-200v-80h480v80H360Zm0-240v-80h480v80H360Zm0-240v-80h480v80H360ZM200-160q-33 0-56.5-23.5T120-240q0-33 23.5-56.5T200-320q33 0 56.5 23.5T280-240q0 33-23.5 56.5T200-160Zm0-240q-33 0-56.5-23.5T120-480q0-33 23.5-56.5T200-560q33 0 56.5 23.5T280-480q0 33-23.5 56.5T200-400Zm-56.5-263.5Q120-687 120-720t23.5-56.5Q167-800 200-800t56.5 23.5Q280-753 280-720t-23.5 56.5Q233-640 200-640t-56.5-23.5Z",
        IconName::NumberedList => "M120-80v-60h100v-30h-60v-60h60v-30H120v-60h120q17 0 28.5 11.5T280-280v40q0 17-11.5 28.5T240-200q17 0 28.5 11.5T280-160v40q0 17-11.5 28.5T240-80H120Zm0-280v-110q0-17 11.5-28.5T160-510h60v-30H120v-60h120q17 0 28.5 11.5T280-560v70q0 17-11.5 28.5T240-450h-60v30h100v60H120Zm60-280v-180h-60v-60h120v240h-60Zm180 440v-80h480v80H360Zm0-240v-80h480v80H360Zm0-240v-80h480v80H360Z",
        IconName::Outdent => "M120-120v-80h720v80H120Zm320-160v-80h400v80H440Zm0-160v-80h400v80H440Zm0-160v-80h400v80H440ZM120-760v-80h720v80H120Zm160 440L120-480l160-160v320Z",
        IconName::Indent => "M120-120v-80h720v80H120Zm320-160v-80h400v80H440Zm0-160v-80h400v80H440Zm0-160v-80h400v80H440ZM120-760v-80h720v80H120Zm0 440v-320l160 160-160 160Z",
        IconName::ClearFormatting => "m528-546-93-93-121-121h486v120H568l-40 94ZM792-56 460-388l-80 188H249l119-280L56-792l56-56 736 736-56 56Z",
        IconName::Link => "M440-280H280q-83 0-141.5-58.5T80-480q0-83 58.5-141.5T280-680h160v80H280q-50 0-85 35t-35 85q0 50 35 85t85 35h160v80ZM320-440v-80h320v80H320Zm200 160v-80h160q50 0 85-35t35-85q0-50-35-85t-85-35H520v-80h160q83 0 141.5 58.5T880-480q0 83-58.5 141.5T680-280H520Z",
        IconName::Image => "M200-120q-33 0-56.5-23.5T120-200v-560q0-33 23.5-56.5T200-840h560q33 0 56.5 23.5T840-760v560q0 33-23.5 56.5T760-120H200Zm0-80h560v-560H200v560Zm40-80h480L570-480 450-320l-90-120-120 160Zm-40 80v-560 560Z",
        IconName::Table => "M200-120q-33 0-56.5-23.5T120-200v-560q0-33 23.5-56.5T200-840h560q33 0 56.5 23.5T840-760v560q0 33-23.5 56.5T760-120H200Zm240-240H200v160h240v-160Zm80 0v160h240v-160H520Zm-80-80v-160H200v160h240Zm80 0h240v-160H520v160ZM200-680h560v-80H200v80Z",
        IconName::More => "M240-400q-33 0-56.5-23.5T160-480q0-33 23.5-56.5T240-560q33 0 56.5 23.5T320-480q0 33-23.5 56.5T240-400Zm240 0q-33 0-56.5-23.5T400-480q0-33 23.5-56.5T480-560q33 0 56.5 23.5T560-480q0 33-23.5 56.5T480-400Zm240 0q-33 0-56.5-23.5T640-480q0-33 23.5-56.5T720-560q33 0 56.5 23.5T800-480q0 33-23.5 56.5T720-400Z",
        // Filled down caret (Material expand_more) for dropdown triggers and
        // colorsplit caret zones.
        IconName::ChevronDown => "M480-345 240-585l56-56 184 184 184-184 56 56-240 240Z",
        _ => return None,
    };
    Some(path)
}

fn svg_document_filled(name: IconName) -> Option<String> {
    svg_body_filled(name).map(|path| {
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 -960 960 960\" \
             fill=\"#ffffff\"><path d=\"{path}\"/></svg>"
        )
    })
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

type IconCache = Mutex<HashMap<(IconName, u32, bool), TextureHandle>>;

fn cache() -> &'static IconCache {
    static CACHE: OnceLock<IconCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn render_icon(ctx: &Context, name: IconName, color: Color32, filled: bool) -> TextureId {
    let key_color = ((color.r() as u32) << 24)
        | ((color.g() as u32) << 16)
        | ((color.b() as u32) << 8)
        | color.a() as u32;
    if let Some(handle) = cache()
        .lock()
        .unwrap()
        .get(&(name, key_color, filled))
    {
        return handle.id();
    }

    // Filled glyphs take precedence when requested and available; otherwise
    // fall back to the stroke icon so non-toolbar callers still work.
    let svg = if filled {
        svg_document_filled(name).unwrap_or_else(|| svg_document(name))
    } else {
        svg_document(name)
    };
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
        .insert((name, key_color, filled), handle.clone());
    handle.id()
}

/// Paint an icon glyph centered in a `size` x `size` box at the painter's
/// current cursor position is NOT used; instead this paints at `rect` center.
/// Call inside a sized ui region or pass an explicit rect via
/// [`paint_icon_at`].
fn paint_icon_at_inner(
    ctx: &Context,
    painter: &Painter,
    name: IconName,
    rect: egui::Rect,
    color: Color32,
    filled: bool,
) {
    let id = render_icon(ctx, name, color, filled);
    painter.image(
        id,
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

pub fn paint_icon_at(
    ctx: &Context,
    painter: &Painter,
    name: IconName,
    rect: egui::Rect,
    color: Color32,
) {
    paint_icon_at_inner(ctx, painter, name, rect, color, false);
}

/// Paint the filled Material-style glyph variant used by the web toolbar.
/// Falls back to the stroke glyph when no filled path exists for `name`.
pub fn paint_icon_filled(
    ctx: &Context,
    painter: &Painter,
    name: IconName,
    rect: egui::Rect,
    color: Color32,
) {
    paint_icon_at_inner(ctx, painter, name, rect, color, true);
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

    #[test]
    fn toolbar_filled_glyphs_parse_and_render_visible_pixels() {
        for name in [
            IconName::Undo, IconName::Redo, IconName::Painter, IconName::Bold,
            IconName::Italic, IconName::Underline, IconName::TextColor, IconName::Highlight,
            IconName::AlignLeft, IconName::AlignCenter, IconName::AlignRight, IconName::AlignJustify,
            IconName::LineSpacing, IconName::BulletList, IconName::NumberedList, IconName::Outdent,
            IconName::Indent, IconName::ClearFormatting, IconName::Link, IconName::Image,
            IconName::Table, IconName::More, IconName::ChevronDown,
        ] {
            let svg = svg_document_filled(name).unwrap_or_else(|| panic!("{name:?} has no filled glyph"));
            assert!(svg.contains("viewBox=\"0 -960 960 960\""), "{name:?}");
            assert!(svg.contains("fill=\"#ffffff\""), "{name:?}");
            assert!(!svg.contains("stroke"), "{name:?}");
            let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
            let size = tree.size();
            let scale = 48.0 / size.width().max(size.height());
            let w = (size.width() * scale).round().max(1.0) as u32;
            let h = (size.height() * scale).round().max(1.0) as u32;
            let mut pixmap = tiny_skia::Pixmap::new(w, h).unwrap();
            resvg::render(
                &tree,
                usvg::Transform::from_scale(scale, scale),
                &mut pixmap.as_mut(),
            );
            assert!(
                pixmap.pixels().iter().any(|pixel| pixel.alpha() > 0),
                "{name:?} filled glyph rendered empty"
            );
        }
    }
}
