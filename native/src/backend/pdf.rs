use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use egui::epaint::text::{Glyph as EguiGlyph, Row as EguiRow};
use egui::{Color32, Galley, Pos2, Stroke, Vec2};
use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Rect as PdfRect, Transform};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule, Stroke as PdfStroke};
use krilla::surface::Surface;
use krilla::text::{Font as PdfFont, GlyphId as PdfGlyphId, KrillaGlyph};
use krilla::Document as PdfDocument;

use crate::document::Document;
use crate::page_layout::{self, ParagraphFragment, ResolvedFont};

// The Editor Layout Uses Pixels At 96 Dpi While Pdf Points Are 1/72 Inch.
const POINTS_PER_PIXEL: f32 = 0.75;
// Approximates The Shear Egui Applies When Synthesizing Italic Text.
const ITALIC_SHEAR: f32 = 0.2;

type FontCache = HashMap<(usize, u32), PdfFont>;

pub fn export(ctx: &egui::Context, document: &Document) -> Result<Vec<u8>, String> {
    if document.has_unrepresented_body_content() {
        return Err(
            "The Document Contains Content That Cannot Yet Be Exported To PDF.".into(),
        );
    }
    let layout = page_layout::layout_document(ctx, document)?;
    let mut pdf = PdfDocument::new();
    let mut fonts = FontCache::default();
    for page_layout in &layout.pages {
        let settings = PageSettings::from_wh(
            layout.page_size.x * POINTS_PER_PIXEL,
            layout.page_size.y * POINTS_PER_PIXEL,
        )
        .ok_or_else(|| "The Page Dimensions Are Invalid.".to_string())?;
        let mut page = pdf.start_page_with(settings);
        let mut surface = page.surface();
        surface.push_transform(&Transform::from_scale(POINTS_PER_PIXEL, POINTS_PER_PIXEL));
        let result: Result<(), String> = (|| {
            for fragment in &page_layout.fragments {
                draw_fragment(ctx, &mut surface, document, fragment, &mut fonts)?;
            }
            Ok(())
        })();
        surface.pop();
        surface.finish();
        page.finish();
        result?;
    }
    pdf.finish()
        .map_err(|error| format!("Could Not Create The PDF Document. {error}"))
}

fn draw_fragment(
    ctx: &egui::Context,
    surface: &mut Surface<'_>,
    document: &Document,
    fragment: &ParagraphFragment,
    fonts: &mut FontCache,
) -> Result<(), String> {
    draw_rows(
        surface,
        &fragment.galley,
        fragment.glyph_fonts.as_slice(),
        fragment.rows.clone(),
        fragment.origin,
        fonts,
    )?;
    if let Some(marker) = &fragment.marker {
        draw_marker(ctx, surface, document, fragment, marker, fonts)?;
    }
    Ok(())
}

fn draw_marker(
    ctx: &egui::Context,
    surface: &mut Surface<'_>,
    document: &Document,
    fragment: &ParagraphFragment,
    marker: &str,
    fonts: &mut FontCache,
) -> Result<(), String> {
    let paragraph = document
        .paragraphs
        .get(fragment.paragraph_index)
        .ok_or_else(|| "The Document Layout Is Invalid.".to_string())?;
    let format = paragraph
        .runs
        .first()
        .map(|run| &run.format)
        .cloned()
        .unwrap_or_default();
    let mut job = egui::text::LayoutJob::default();
    job.wrap = egui::text::TextWrapping::no_max_width();
    job.append(
        marker,
        0.0,
        page_layout::text_format(ctx, &format, paragraph.line_spacing)?,
    );
    let galley = ctx.fonts(|fonts| fonts.layout_job(job));
    let glyph_fonts = page_layout::resolve_glyph_fonts(ctx, &galley);
    let first_row = fragment
        .galley
        .rows
        .get(fragment.rows.start)
        .ok_or_else(|| "The Document Layout Is Invalid.".to_string())?;
    let baseline = first_row
        .glyphs
        .first()
        .map(|glyph| glyph.pos.y)
        .unwrap_or(first_row.rect.top() + first_row.rect.height() * 0.8);
    let marker_baseline = galley
        .rows
        .first()
        .and_then(|row| row.glyphs.first())
        .map(|glyph| glyph.pos.y)
        .unwrap_or(0.0);
    let origin = Pos2::new(
        fragment.marker_origin.x,
        fragment.origin.y + baseline - marker_baseline,
    );
    draw_rows(
        surface,
        &galley,
        glyph_fonts.as_slice(),
        0..galley.rows.len(),
        origin,
        fonts,
    )
}

fn draw_rows(
    surface: &mut Surface<'_>,
    galley: &Galley,
    glyph_fonts: &[Vec<Option<ResolvedFont>>],
    rows: Range<usize>,
    origin: Pos2,
    fonts: &mut FontCache,
) -> Result<(), String> {
    for row_index in rows {
        let row = galley
            .rows
            .get(row_index)
            .ok_or_else(|| "The Document Layout Is Invalid.".to_string())?;
        let row_fonts = glyph_fonts
            .get(row_index)
            .ok_or_else(|| "The Document Layout Is Invalid.".to_string())?;
        draw_backgrounds(surface, origin, galley, row);
        draw_row_text(surface, origin, galley, row, row_fonts, fonts)?;
        draw_row_lines(surface, origin, galley, row, LineKind::Underline);
        draw_row_lines(surface, origin, galley, row, LineKind::Strikethrough);
    }
    Ok(())
}

fn logical_rect(glyph: &EguiGlyph) -> egui::Rect {
    egui::Rect::from_min_size(
        glyph.pos - Vec2::new(0.0, glyph.font_ascent),
        Vec2::new(glyph.advance_width, glyph.line_height),
    )
}

fn fill_page_rect(
    surface: &mut Surface<'_>,
    origin: Pos2,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    color: Color32,
) {
    let Some(rect) = PdfRect::from_xywh(
        origin.x + left,
        origin.y + top,
        right - left,
        bottom - top,
    ) else {
        return;
    };
    let mut path = PathBuilder::new();
    path.push_rect(rect);
    let Some(path) = path.finish() else {
        return;
    };
    surface.set_stroke(None);
    surface.set_fill(Some(solid_fill(color)));
    surface.draw_path(&path);
}

fn draw_backgrounds(
    surface: &mut Surface<'_>,
    origin: Pos2,
    galley: &Galley,
    row: &EguiRow,
) {
    // Merges Consecutive Glyphs Sharing A Background Color And Vertical Extent,
    // Matching How Egui Builds Its Row Backgrounds.
    let mut pending: Option<(Color32, f32, f32, f32, f32)> = None;
    for glyph in &row.glyphs {
        let format = &galley.job.sections[glyph.section_index as usize].format;
        let color = format.background;
        let rect = logical_rect(glyph);
        if color == Color32::TRANSPARENT {
            if let Some((run_color, left, top, bottom, right)) = pending.take() {
                fill_page_rect(
                    surface,
                    origin,
                    left - 1.0,
                    top - 1.0,
                    right + 1.0,
                    bottom + 1.0,
                    run_color,
                );
            }
            continue;
        }
        match pending {
            Some((run_color, left, top, bottom, right))
                if run_color == color && top == rect.top() && bottom == rect.bottom() =>
            {
                pending = Some((color, left, top, bottom, right.max(rect.right())));
            }
            _ => {
                if let Some((run_color, left, top, bottom, right)) = pending.take() {
                    fill_page_rect(
                        surface,
                        origin,
                        left - 1.0,
                        top - 1.0,
                        right + 1.0,
                        bottom + 1.0,
                        run_color,
                    );
                }
                pending = Some((
                    color,
                    rect.left(),
                    rect.top(),
                    rect.bottom(),
                    rect.right(),
                ));
            }
        }
    }
    if let Some((run_color, left, top, bottom, right)) = pending.take() {
        fill_page_rect(
            surface,
            origin,
            left - 1.0,
            top - 1.0,
            right + 1.0,
            bottom + 1.0,
            run_color,
        );
    }
}

#[derive(Clone, Copy)]
enum LineKind {
    Underline,
    Strikethrough,
}

fn draw_row_lines(
    surface: &mut Surface<'_>,
    origin: Pos2,
    galley: &Galley,
    row: &EguiRow,
    kind: LineKind,
) {
    // Merges Consecutive Glyphs Sharing A Stroke And Line Position,
    // Matching How Egui Builds Its Row Lines.
    let mut pending: Option<(Stroke, f32, f32, f32)> = None;
    for glyph in &row.glyphs {
        let format = &galley.job.sections[glyph.section_index as usize].format;
        let stroke = match kind {
            LineKind::Underline => format.underline,
            LineKind::Strikethrough => format.strikethrough,
        };
        let rect = logical_rect(glyph);
        let y = match kind {
            LineKind::Underline => rect.bottom(),
            LineKind::Strikethrough => rect.center().y,
        };
        if stroke == Stroke::NONE {
            if let Some((line_stroke, line_y, left, right)) = pending.take() {
                draw_hline(surface, origin, left, line_y, right, line_stroke);
            }
            continue;
        }
        match pending {
            Some((line_stroke, line_y, left, right))
                if line_stroke == stroke && line_y == y =>
            {
                pending = Some((stroke, y, left, right.max(glyph.max_x())));
            }
            _ => {
                if let Some((line_stroke, line_y, left, right)) = pending.take() {
                    draw_hline(surface, origin, left, line_y, right, line_stroke);
                }
                pending = Some((stroke, y, glyph.pos.x, glyph.max_x()));
            }
        }
    }
    if let Some((line_stroke, line_y, left, right)) = pending.take() {
        draw_hline(surface, origin, left, line_y, right, line_stroke);
    }
}

fn draw_hline(
    surface: &mut Surface<'_>,
    origin: Pos2,
    left: f32,
    y: f32,
    right: f32,
    stroke: Stroke,
) {
    let mut path = PathBuilder::new();
    path.move_to(origin.x + left, origin.y + y);
    path.line_to(origin.x + right, origin.y + y);
    let Some(path) = path.finish() else {
        return;
    };
    surface.set_fill(None);
    surface.set_stroke(Some(PdfStroke {
        paint: rgb_color(stroke.color).into(),
        width: stroke.width,
        opacity: alpha_opacity(stroke.color),
        ..Default::default()
    }));
    surface.draw_path(&path);
}

fn draw_row_text(
    surface: &mut Surface<'_>,
    origin: Pos2,
    galley: &Galley,
    row: &EguiRow,
    row_fonts: &[Option<ResolvedFont>],
    fonts: &mut FontCache,
) -> Result<(), String> {
    let mut run_start = 0;
    let mut run_key: Option<(u32, usize, u32)> = None;
    for index in 0..=row.glyphs.len() {
        let key = if index < row.glyphs.len() {
            glyph_run_key(&row.glyphs[index], row_fonts.get(index))?
        } else {
            None
        };
        if key != run_key {
            if run_key.is_some() {
                draw_text_run(
                    surface,
                    origin,
                    galley,
                    row,
                    row_fonts,
                    run_start..index,
                    fonts,
                )?;
            }
            run_start = index;
            run_key = key;
        }
    }
    Ok(())
}

// Zero Width Characters Without A Resolved Font Are Skipped Silently.
fn glyph_run_key(
    glyph: &EguiGlyph,
    resolved: Option<&Option<ResolvedFont>>,
) -> Result<Option<(u32, usize, u32)>, String> {
    let resolved = match resolved.and_then(|resolved| resolved.as_ref()) {
        Some(resolved) => resolved,
        None => {
            if glyph.advance_width == 0.0 && glyph.uv_rect.is_nothing() {
                return Ok(None);
            }
            return Err(
                "The Document Contains Characters That No Available Font Can Display.".into(),
            );
        }
    };
    Ok(Some((
        glyph.section_index,
        Arc::as_ptr(&resolved.data) as usize,
        resolved.data.index,
    )))
}

fn draw_text_run(
    surface: &mut Surface<'_>,
    origin: Pos2,
    galley: &Galley,
    row: &EguiRow,
    row_fonts: &[Option<ResolvedFont>],
    range: Range<usize>,
    fonts: &mut FontCache,
) -> Result<(), String> {
    let mut first: Option<(&EguiGlyph, &ResolvedFont)> = None;
    for index in range.clone() {
        if let Some(resolved) = row_fonts[index].as_ref() {
            first = Some((&row.glyphs[index], resolved));
            break;
        }
    }
    let Some((first_glyph, resolved)) = first else {
        return Ok(());
    };
    let section = &galley.job.sections[first_glyph.section_index as usize];
    let format = &section.format;
    let font_size = format.font_id.size;
    if !font_size.is_finite() || font_size <= 0.0 {
        return Err("The Text Size Is Invalid.".to_string());
    }
    let pdf_font = pdf_font(fonts, resolved)?;
    let face = ttf_parser::Face::parse(resolved.data.font.as_ref(), resolved.data.index)
        .map_err(|_| "A Font Used By The Document Could Not Be Read.".to_string())?;
    let mut text = String::new();
    let mut glyphs = Vec::new();
    let baseline_y = first_glyph.pos.y;
    for index in range {
        let egui_glyph = &row.glyphs[index];
        let Some(_) = row_fonts[index].as_ref() else {
            continue;
        };
        let glyph_id = face
            .glyph_index(egui_glyph.chr)
            .or_else(|| face.glyph_index(' '))
            .ok_or_else(|| {
                "The Document Contains Characters That No Available Font Can Display."
                    .to_string()
            })?;
        let range_start = text.len();
        text.push(egui_glyph.chr);
        // Krilla Advances Are Normalized To The Font Size, While Y Offsets
        // Compensate For Per Glyph Baseline Differences Such As Font Fallback.
        let x_advance = if index + 1 < row.glyphs.len() {
            row.glyphs[index + 1].pos.x - egui_glyph.pos.x
        } else {
            egui_glyph.advance_width
        };
        glyphs.push(KrillaGlyph::new(
            PdfGlyphId::new(u32::from(glyph_id.0)),
            x_advance / font_size,
            0.0,
            (baseline_y - egui_glyph.pos.y) / font_size,
            0.0,
            range_start..text.len(),
            None,
        ));
    }
    if glyphs.is_empty() {
        return Ok(());
    }
    let start = Pos2::new(origin.x + first_glyph.pos.x, origin.y + first_glyph.pos.y);
    if format.italics {
        surface.push_transform(&Transform::from_row(
            1.0,
            0.0,
            -ITALIC_SHEAR,
            1.0,
            ITALIC_SHEAR * start.y,
            0.0,
        ));
    }
    surface.set_stroke(None);
    surface.set_fill(Some(solid_fill(format.color)));
    surface.draw_glyphs(
        Point::from_xy(start.x, start.y),
        &glyphs,
        pdf_font,
        &text,
        font_size,
        false,
    );
    if format.italics {
        surface.pop();
    }
    Ok(())
}

fn pdf_font(fonts: &mut FontCache, resolved: &ResolvedFont) -> Result<PdfFont, String> {
    let key = (Arc::as_ptr(&resolved.data) as usize, resolved.data.index);
    if let Some(font) = fonts.get(&key) {
        return Ok(font.clone());
    }
    let font = match &resolved.data.font {
        std::borrow::Cow::Borrowed(bytes) => {
            PdfFont::new(krilla::Data::from(*bytes), resolved.data.index)
        }
        std::borrow::Cow::Owned(bytes) => {
            PdfFont::new(krilla::Data::from(bytes.clone()), resolved.data.index)
        }
    }
    .ok_or_else(|| {
        "A Font Used By The Document Could Not Be Embedded In The PDF.".to_string()
    })?;
    fonts.insert(key, font.clone());
    Ok(font)
}

fn rgb_color(color: Color32) -> rgb::Color {
    rgb::Color::new(color.r(), color.g(), color.b())
}

fn alpha_opacity(color: Color32) -> NormalizedF32 {
    NormalizedF32::new(color.a() as f32 / 255.0).unwrap_or(NormalizedF32::ONE)
}

fn solid_fill(color: Color32) -> Fill {
    Fill {
        paint: rgb_color(color).into(),
        opacity: alpha_opacity(color),
        rule: FillRule::NonZero,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{PageSetup, Paragraph, Run, TextFormat};

    fn export_in_context(document: &Document) -> Result<Vec<u8>, String> {
        let context = egui::Context::default();
        let mut result = None;
        let _ = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |_ui| {
                result = Some(export(context, document));
            });
        });
        result.unwrap()
    }

    #[test]
    fn exports_plain_text_as_a_real_pdf() {
        let mut document = Document::default();
        document.replace_plain_text("Hello Pdf World");
        let bytes = export_in_context(&document).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        assert!(bytes.len() > 1000);
    }

    #[test]
    fn exports_styled_runs_lists_and_multiple_pages() {
        let mut document = Document::default();
        document.page_setup = PageSetup {
            width: 180.0,
            height: 90.0,
            margin_top: 10.0,
            margin_bottom: 10.0,
            margin_left: 10.0,
            margin_right: 10.0,
        };
        document.paragraphs = vec![
            Paragraph {
                runs: vec![
                    Run {
                        text: "First ".into(),
                        format: TextFormat::default(),
                    },
                    Run {
                        text: "Bold".into(),
                        format: TextFormat {
                            bold: true,
                            color: Some("FF0000".into()),
                            highlight: Some("FFFF00".into()),
                            underline: true,
                            ..Default::default()
                        },
                    },
                ],
                ..Default::default()
            },
            Paragraph {
                list: Some("bullet".into()),
                runs: vec![Run {
                    text: "Item ".repeat(60),
                    format: TextFormat::default(),
                }],
                ..Default::default()
            },
        ];
        let bytes = export_in_context(&document).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        assert!(bytes.len() > 1000);
    }

    #[test]
    fn rejects_content_without_a_displayable_glyph() {
        let mut document = Document::default();
        document.replace_plain_text("Known \u{10ffff}");
        let error = export_in_context(&document).unwrap_err();
        assert!(error.contains("Font"));
    }

    #[test]
    fn propagates_layout_errors() {
        let mut document = Document::default();
        document.page_setup.margin_left = document.page_setup.width;
        let error = export_in_context(&document).unwrap_err();
        assert!(error.contains("No Space"));
    }
}
