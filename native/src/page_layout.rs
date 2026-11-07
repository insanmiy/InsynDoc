use std::ops::Range;
use std::sync::Arc;

use egui::{Align, Color32, FontFamily, FontId, Galley, Pos2, Stroke};

use crate::document::{Alignment, Document, Paragraph, TextFormat};

// Coordinates Use Unscaled Editor Points, With The Origin At The Page's Top Left.
pub struct DocumentLayout {
    pub pages: Vec<PageLayout>,
    pub page_size: egui::Vec2,
}

#[derive(Default)]
pub struct PageLayout {
    pub fragments: Vec<ParagraphFragment>,
}

pub struct ParagraphFragment {
    pub paragraph_index: usize,
    pub rows: Range<usize>,
    pub galley: Arc<Galley>,
    pub origin: Pos2,
    pub marker: Option<String>,
    pub marker_origin: Pos2,
    pub glyph_fonts: Arc<Vec<Vec<Option<ResolvedFont>>>>,
}

#[derive(Clone, Debug)]
pub struct ResolvedFont {
    pub name: String,
    pub data: Arc<egui::FontData>,
}

pub fn layout_document(ctx: &egui::Context, document: &Document) -> Result<DocumentLayout, String> {
    let setup = &document.page_setup;
    let geometry = [
        setup.width,
        setup.height,
        setup.margin_top,
        setup.margin_right,
        setup.margin_bottom,
        setup.margin_left,
    ];
    if geometry
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("The Page Dimensions And Margins Must Be Finite And Nonnegative.".into());
    }
    let width = setup.width - setup.margin_left - setup.margin_right;
    let height = setup.height - setup.margin_top - setup.margin_bottom;
    if width <= 0.0 || height <= 0.0 {
        return Err("The Page Margins Leave No Space For Document Content.".into());
    }

    let mut layout = DocumentLayout {
        pages: vec![PageLayout::default()],
        page_size: egui::vec2(setup.width, setup.height),
    };
    let mut page_y = setup.margin_top;
    let page_bottom = setup.height - setup.margin_bottom;
    let mut numbered_count = 0;
    for (index, paragraph) in document.paragraphs.iter().enumerate() {
        if !paragraph.indent.is_finite()
            || !paragraph.line_spacing.is_finite()
            || !paragraph.space_before.is_finite()
            || !paragraph.space_after.is_finite()
        {
            return Err("The Paragraph Layout Contains An Invalid Value.".into());
        }
        page_y = (page_y + paragraph.space_before.max(0.0)).min(page_bottom);
        let marker = match paragraph.list.as_deref() {
            Some("bullet") => {
                numbered_count = 0;
                Some("\u{2022}".to_string())
            }
            Some("number") => {
                numbered_count += 1;
                Some(format!("{numbered_count}."))
            }
            _ => {
                numbered_count = 0;
                None
            }
        };
        let indent = paragraph.indent.max(0.0);
        let marker_width = if marker.is_some() { 24.0 } else { 0.0 };
        let text_width = width - indent - marker_width;
        if text_width <= 0.0 {
            return Err("The Paragraph Indent Leaves No Space For Text.".into());
        }
        let galley = paragraph_galley(ctx, paragraph, text_width)?;
        let glyph_fonts = resolve_glyph_fonts(ctx, &galley);
        let text_left = setup.margin_left + indent + marker_width;
        let text_anchor = match paragraph.alignment {
            Alignment::Center => text_left + text_width * 0.5,
            Alignment::Right => text_left + text_width,
            _ => text_left,
        };
        let mut row = 0;
        while row < galley.rows.len() {
            let first_row = row;
            let row_top = galley.rows[row].rect.top();
            let mut fragment_height = 0.0;
            while row < galley.rows.len() {
                let candidate_height = galley.rows[row].rect.bottom() - row_top;
                if galley.rows[row].rect.height() > height {
                    return Err("A Line Of Text Is Taller Than The Available Page Area.".into());
                }
                if page_y + candidate_height > page_bottom {
                    break;
                }
                fragment_height = candidate_height;
                row += 1;
            }
            if row == first_row {
                layout.pages.push(PageLayout::default());
                page_y = setup.margin_top;
                continue;
            }
            layout
                .pages
                .last_mut()
                .unwrap()
                .fragments
                .push(ParagraphFragment {
                    paragraph_index: index,
                    rows: first_row..row,
                    galley: galley.clone(),
                    origin: egui::pos2(text_anchor, page_y - row_top),
                    marker: if first_row == 0 { marker.clone() } else { None },
                    marker_origin: egui::pos2(setup.margin_left + indent, page_y),
                    glyph_fonts: glyph_fonts.clone(),
                });
            page_y += fragment_height;
            if row < galley.rows.len() {
                layout.pages.push(PageLayout::default());
                page_y = setup.margin_top;
            }
        }
        page_y = (page_y + paragraph.space_after.max(0.0) + 8.0).min(page_bottom);
    }
    Ok(layout)
}

pub(crate) fn resolve_glyph_fonts(
    ctx: &egui::Context,
    galley: &Galley,
) -> Arc<Vec<Vec<Option<ResolvedFont>>>> {
    ctx.fonts(|fonts| {
        let mut locked = fonts.lock();
        let font_data = locked.fonts.definitions().font_data.clone();
        let rows = galley.rows.iter().map(|row| {
            row.glyphs.iter().map(|glyph| {
                let section = galley.job.sections.get(glyph.section_index as usize)?;
                let font = locked.fonts.font(&section.format.font_id);
                let name = font.characters().get(&glyph.chr)?.first()?;
                let data = font_data.get(name)?.clone();
                Some(ResolvedFont { name: name.clone(), data })
            }).collect()
        }).collect();
        Arc::new(rows)
    })
}

fn paragraph_galley(
    ctx: &egui::Context,
    paragraph: &Paragraph,
    width: f32,
) -> Result<Arc<Galley>, String> {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    job.halign = match paragraph.alignment {
        Alignment::Center => Align::Center,
        Alignment::Right => Align::RIGHT,
        _ => Align::LEFT,
    };
    job.justify = paragraph.alignment == Alignment::Justify;
    for run in &paragraph.runs {
        job.append(
            &run.text,
            0.0,
            text_format(ctx, &run.format, paragraph.line_spacing)?,
        );
    }
    if job.text.is_empty() {
        let format = paragraph
            .runs
            .first()
            .map(|run| &run.format)
            .cloned()
            .unwrap_or_default();
        job.append("", 0.0, text_format(ctx, &format, paragraph.line_spacing)?);
    }
    Ok(ctx.fonts(|fonts| fonts.layout_job(job)))
}

pub(crate) fn text_format(
    ctx: &egui::Context,
    format: &TextFormat,
    line_spacing: f32,
) -> Result<egui::TextFormat, String> {
    let size = format.font_size.unwrap_or(11.0);
    if !size.is_finite() || size <= 0.0 || line_spacing < 0.0 {
        return Err("The Text Size Or Line Spacing Is Invalid.".into());
    }
    let name = format.font_family.as_deref().unwrap_or("Calibri");
    let family_name = match (format.bold, format.italic) {
        (true, true) => format!("{name} Bold Italic"),
        (true, false) => format!("{name} Bold"),
        (false, true) => format!("{name} Italic"),
        _ => name.to_string(),
    };
    let family = ctx
        .fonts(|fonts| {
            fonts.families().into_iter().find(
                |family| matches!(family, FontFamily::Name(name) if name.as_ref() == family_name),
            )
        })
        .unwrap_or(FontFamily::Proportional);
    let dedicated_italic =
        format.italic && matches!(&family, FontFamily::Name(name) if name.as_ref() == family_name);
    let color = parse_color(format.color.as_deref()).unwrap_or(Color32::from_rgb(32, 32, 32));
    let background = parse_color(format.highlight.as_deref()).unwrap_or(Color32::TRANSPARENT);
    let font_size = size * 4.0 / 3.0 * if format.script.is_some() { 0.7 } else { 1.0 };
    Ok(egui::TextFormat {
        font_id: FontId::new(font_size, family),
        color,
        background,
        valign: if format.script.as_deref() == Some("subscript") {
            Align::BOTTOM
        } else {
            Align::TOP
        },
        italics: format.italic && !dedicated_italic,
        underline: if format.underline {
            Stroke::new(1.0_f32, color)
        } else {
            Stroke::NONE
        },
        strikethrough: if format.strike {
            Stroke::new(1.0_f32, color)
        } else {
            Stroke::NONE
        },
        line_height: (line_spacing > 0.0).then_some(font_size * line_spacing),
        ..Default::default()
    })
}

fn parse_color(value: Option<&str>) -> Option<Color32> {
    let value = u32::from_str_radix(value?.trim_start_matches('#'), 16).ok()?;
    Some(Color32::from_rgb(
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{PageSetup, Run};

    fn with_layout(document: &Document, check: impl FnOnce(DocumentLayout)) {
        let context = egui::Context::default();
        let mut layout = None;
        let _ = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                layout = Some(layout_document(ui.ctx(), document).unwrap());
            });
        });
        check(layout.unwrap());
    }

    #[test]
    fn wraps_and_paginates_rows_without_losing_text() {
        let mut document = Document::default();
        document.page_setup = PageSetup {
            width: 180.0,
            height: 90.0,
            margin_top: 10.0,
            margin_bottom: 10.0,
            margin_left: 10.0,
            margin_right: 10.0,
        };
        document.replace_plain_text(&"Words To Wrap Across The Page ".repeat(30));
        with_layout(&document, |layout| {
            assert!(layout.pages.len() > 1);
            let mut previous_row = 0;
            let mut final_row = 0;
            for page in &layout.pages {
                assert!(!page.fragments.is_empty());
                for fragment in &page.fragments {
                    assert_eq!(fragment.paragraph_index, 0);
                    assert_eq!(fragment.rows.start, previous_row);
                    for row in &fragment.galley.rows[fragment.rows.clone()] {
                        assert!(fragment.origin.y + row.rect.top() >= 9.99);
                        assert!(fragment.origin.y + row.rect.bottom() <= 80.01);
                    }
                    previous_row = fragment.rows.end;
                    final_row = fragment.galley.rows.len();
                }
            }
            assert_eq!(previous_row, final_row);
            assert_eq!(layout.page_size, egui::vec2(180.0, 90.0));
        });
    }

    #[test]
    fn retains_run_styles_alignment_and_list_markers() {
        let mut document = Document::default();
        document.paragraphs = vec![Paragraph {
            alignment: Alignment::Right,
            list: Some("bullet".into()),
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
                        ..Default::default()
                    },
                },
            ],
            ..Default::default()
        }];
        with_layout(&document, |layout| {
            let fragment = &layout.pages[0].fragments[0];
            assert_eq!(fragment.marker.as_deref(), Some("\u{2022}"));
            assert_eq!(fragment.galley.job.text, "First Bold");
            assert_eq!(fragment.galley.job.sections.len(), 2);
            assert_eq!(fragment.galley.job.sections[1].format.color, Color32::RED);
            assert_eq!(fragment.galley.job.halign, Align::RIGHT);
            assert_eq!(
                fragment.origin.x,
                document.page_setup.width - document.page_setup.margin_right
            );
            for (row, glyph_fonts) in fragment.galley.rows.iter().zip(fragment.glyph_fonts.iter()) {
                assert_eq!(row.glyphs.len(), glyph_fonts.len());
                assert!(glyph_fonts.iter().all(Option::is_some));
            }
            let resolved = fragment.glyph_fonts[0][0].as_ref().unwrap();
            assert!(!resolved.name.is_empty());
            assert!(!resolved.data.font.is_empty());
        });
    }

    #[test]
    fn unavailable_glyph_is_not_misattributed_to_a_font() {
        let mut document = Document::default();
        document.replace_plain_text("Known \u{10ffff}");
        with_layout(&document, |layout| {
            let fragment = &layout.pages[0].fragments[0];
            let row = &fragment.galley.rows[0];
            let index = row.glyphs.iter().position(|glyph| glyph.chr == '\u{10ffff}').unwrap();
            assert!(fragment.glyph_fonts[0][index].is_none());
        });
    }

    #[test]
    fn rejects_invalid_page_geometry() {
        let mut document = Document::default();
        document.page_setup.margin_left = document.page_setup.width;
        let context = egui::Context::default();
        let mut error = None;
        let _ = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                error = layout_document(ui.ctx(), &document).err();
            });
        });
        assert!(error.unwrap().contains("No Space"));
    }

    #[test]
    fn empty_paragraphs_occupy_lines_and_list_markers_do_not_repeat() {
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
            Paragraph::default(),
            Paragraph {
                list: Some("number".into()),
                runs: vec![Run {
                    text: "Long Item ".repeat(50),
                    format: TextFormat::default(),
                }],
                ..Default::default()
            },
        ];
        with_layout(&document, |layout| {
            let fragments: Vec<_> = layout
                .pages
                .iter()
                .flat_map(|page| &page.fragments)
                .collect();
            let empty = fragments
                .iter()
                .find(|fragment| fragment.paragraph_index == 0)
                .unwrap();
            assert!(!empty.rows.is_empty());
            let list_fragments: Vec<_> = fragments
                .iter()
                .filter(|fragment| fragment.paragraph_index == 1)
                .collect();
            assert!(list_fragments.len() > 1);
            assert_eq!(list_fragments[0].marker.as_deref(), Some("1."));
            assert!(list_fragments
                .iter()
                .skip(1)
                .all(|fragment| fragment.marker.is_none()));
        });
    }

    #[test]
    fn rejects_a_line_taller_than_the_content_area() {
        let mut document = Document::default();
        document.page_setup.height = 198.0;
        let context = egui::Context::default();
        let mut error = None;
        let _ = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                error = layout_document(ui.ctx(), &document).err();
            });
        });
        assert!(error.unwrap().contains("Taller"));
    }
}
