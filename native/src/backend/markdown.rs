use std::io::{Cursor, Read};

use crate::document::{Alignment, Document, Run};

const WORD_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

pub fn export(document: &Document) -> Result<String, String> {
    let bytes = document.to_bytes().map_err(|error| error.to_string())?;
    validate_content(&bytes)?;
    for paragraph in &document.paragraphs {
        if paragraph.alignment != Alignment::Left
            || paragraph.indent != 0.0
            || paragraph.line_spacing != 0.0
            || paragraph.runs.iter().any(|run| {
                run.format.underline
                    || run.format.font_family.is_some()
                    || run.format.font_size.is_some()
                    || run.format.color.is_some()
                    || run.format.highlight.is_some()
                    || run.format.script.is_some()
                    || ((run.format.bold || run.format.italic || run.format.strike)
                        && (run.text.contains('\n') || run.text.trim() != run.text))
            })
        {
            return Err(unsupported_content());
        }
    }
    let mut numbered_count = 0;
    let paragraphs: Vec<String> = document
        .paragraphs
        .iter()
        .map(|paragraph| {
            let mut line_start = true;
            let text: String = paragraph
                .runs
                .iter()
                .map(|run| inline(run, &mut line_start))
                .collect();
            if paragraph.list.as_deref() != Some("number")
                || paragraph.style.as_deref().and_then(heading_level).is_some()
            {
                numbered_count = 0;
            }
            if let Some(level) = paragraph.style.as_deref().and_then(heading_level) {
                return format!("{} {text}", "#".repeat(level));
            }
            match paragraph.list.as_deref() {
                Some("bullet") => format!("- {text}"),
                Some("number") => {
                    numbered_count += 1;
                    format!("{numbered_count}. {text}")
                }
                _ => text,
            }
        })
        .collect();
    Ok(format!("{}\n", paragraphs.join("\n\n")))
}

fn heading_level(style: &str) -> Option<usize> {
    style
        .strip_prefix("Heading")?
        .parse::<usize>()
        .ok()
        .filter(|level| (1..=6).contains(level))
}

fn inline(run: &Run, line_start: &mut bool) -> String {
    if run.text.is_empty() {
        return String::new();
    }
    let mut text = String::new();
    for character in run.text.chars() {
        if *line_start && character == ' ' {
            text.push_str("&#32;");
            continue;
        }
        if *line_start && character == '\t' {
            text.push_str("&#9;");
            continue;
        }
        if character.is_ascii_punctuation() {
            text.push('\\');
        }
        text.push(character);
        *line_start = character == '\n';
    }
    if run.format.bold {
        text = format!("**{text}**");
    }
    if run.format.italic {
        text = format!("*{text}*");
    }
    if run.format.strike {
        text = format!("~~{text}~~");
    }
    text
}

fn validate_content(bytes: &[u8]) -> Result<(), String> {
    let unsupported = unsupported_content;
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| error.to_string())?
        .read_to_string(&mut xml)
        .map_err(|error| error.to_string())?;
    let parsed = roxmltree::Document::parse(&xml).map_err(|error| error.to_string())?;
    let body = parsed
        .descendants()
        .find(|node| node.has_tag_name((WORD_NAMESPACE, "body")))
        .ok_or_else(unsupported)?;
    for block in body.children().filter(|node| node.is_element()) {
        if block.has_tag_name((WORD_NAMESPACE, "sectPr")) {
            continue;
        }
        if !block.has_tag_name((WORD_NAMESPACE, "p")) {
            return Err(unsupported());
        }
        for child in block.children().filter(|node| node.is_element()) {
            if child.has_tag_name((WORD_NAMESPACE, "pPr")) {
                for property in child.children().filter(|node| node.is_element()) {
                    if property.tag_name().namespace() != Some(WORD_NAMESPACE)
                        || !matches!(property.tag_name().name(), "pStyle" | "numPr" | "jc")
                    {
                        return Err(unsupported());
                    }
                    if property.has_tag_name((WORD_NAMESPACE, "jc"))
                        && !matches!(
                            property.attribute((WORD_NAMESPACE, "val")),
                            Some("left" | "start")
                        )
                    {
                        return Err(unsupported());
                    }
                    if property.has_tag_name((WORD_NAMESPACE, "pStyle")) {
                        let style = property.attribute((WORD_NAMESPACE, "val")).unwrap_or("");
                        if style != "Normal" && heading_level(style).is_none() {
                            return Err(unsupported());
                        }
                    }
                    if property.has_tag_name((WORD_NAMESPACE, "numPr")) {
                        let id = property
                            .children()
                            .find(|node| node.has_tag_name((WORD_NAMESPACE, "numId")))
                            .and_then(|node| node.attribute((WORD_NAMESPACE, "val")));
                        let level = property
                            .children()
                            .find(|node| node.has_tag_name((WORD_NAMESPACE, "ilvl")))
                            .and_then(|node| node.attribute((WORD_NAMESPACE, "val")))
                            .unwrap_or("0");
                        if !matches!(id, Some("2147483000" | "2147483001")) || level != "0" {
                            return Err(unsupported());
                        }
                    }
                }
                continue;
            }
            if !child.has_tag_name((WORD_NAMESPACE, "r")) {
                return Err(unsupported());
            }
            if child
                .children()
                .filter(|node| node.is_element())
                .any(|node| {
                    node.tag_name().namespace() != Some(WORD_NAMESPACE)
                        || !matches!(node.tag_name().name(), "rPr" | "t" | "tab" | "br" | "cr")
                })
            {
                return Err(unsupported());
            }
            for properties in child
                .children()
                .filter(|node| node.has_tag_name((WORD_NAMESPACE, "rPr")))
            {
                if properties
                    .children()
                    .filter(|node| node.is_element())
                    .any(|node| {
                        node.tag_name().namespace() != Some(WORD_NAMESPACE)
                            || !matches!(node.tag_name().name(), "b" | "i" | "strike")
                    })
                {
                    return Err(unsupported());
                }
            }
        }
    }
    Ok(())
}

fn unsupported_content() -> String {
    "Markdown Export Cannot Yet Preserve Some Content Or Formatting In This Document.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Paragraph, TextFormat};

    fn text_paragraph(text: &str, list: Option<&str>) -> Paragraph {
        Paragraph {
            runs: vec![Run {
                text: text.into(),
                format: TextFormat::default(),
            }],
            list: list.map(str::to_owned),
            ..Default::default()
        }
    }

    #[test]
    fn escapes_literal_block_markers_and_tildes() {
        let mut document = Document::default();
        document.paragraphs = vec![text_paragraph(
            "# Heading\n- Item\n1. Number\n~~~\n    Indented",
            None,
        )];
        assert_eq!(
            export(&document).unwrap(),
            "\\# Heading\n\\- Item\n1\\. Number\n\\~\\~\\~\n&#32;&#32;&#32;&#32;Indented\n"
        );
    }

    #[test]
    fn restarts_numbering_after_a_different_paragraph() {
        let mut document = Document::default();
        document.paragraphs = vec![
            text_paragraph("First", Some("number")),
            text_paragraph("Second", Some("number")),
            text_paragraph("Break", None),
            text_paragraph("Restart", Some("number")),
        ];
        assert_eq!(
            export(&document).unwrap(),
            "1. First\n\n2. Second\n\nBreak\n\n1. Restart\n"
        );
    }

    #[test]
    fn refuses_formatting_that_cannot_be_preserved() {
        let mut document = Document::default();
        for format in [
            TextFormat {
                underline: true,
                ..Default::default()
            },
            TextFormat {
                font_family: Some("Arial".into()),
                ..Default::default()
            },
            TextFormat {
                font_size: Some(14.0),
                ..Default::default()
            },
            TextFormat {
                color: Some("FF0000".into()),
                ..Default::default()
            },
            TextFormat {
                highlight: Some("FFFF00".into()),
                ..Default::default()
            },
            TextFormat {
                script: Some("superscript".into()),
                ..Default::default()
            },
        ] {
            document.paragraphs = vec![Paragraph {
                runs: vec![Run {
                    text: "Text".into(),
                    format,
                }],
                ..Default::default()
            }];
            assert!(export(&document).is_err());
        }
        document.paragraphs = vec![Paragraph {
            alignment: Alignment::Center,
            ..Default::default()
        }];
        assert!(export(&document).is_err());
    }

    #[test]
    fn exports_formatted_headings_lists_and_escaped_unicode() {
        let mut document = Document::default();
        document.paragraphs = vec![
            Paragraph {
                style: Some("Heading1".into()),
                runs: vec![Run {
                    text: "Café [Notes]".into(),
                    format: TextFormat {
                        bold: true,
                        italic: true,
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
            Paragraph {
                list: Some("bullet".into()),
                runs: vec![Run {
                    text: "Item".into(),
                    format: TextFormat::default(),
                }],
                ..Default::default()
            },
            Paragraph {
                list: Some("number".into()),
                runs: vec![Run {
                    text: "Done".into(),
                    format: TextFormat {
                        strike: true,
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
        ];
        assert_eq!(
            export(&document).unwrap(),
            "# ***Café \\[Notes\\]***\n\n- Item\n\n1. ~~Done~~\n"
        );
    }

    #[test]
    fn refuses_unsupported_table_instead_of_omitting_it() {
        use std::io::Write;
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file(
                "word/document.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:tbl/><w:sectPr/></w:body></w:document>"#).unwrap();
        let document = Document::from_bytes(&archive.finish().unwrap().into_inner()).unwrap();
        assert!(export(&document).is_err());
    }

    #[test]
    fn refuses_unsupported_links_and_imported_numbering() {
        use std::io::Write;
        for content in [
            r#"<w:p><w:hyperlink><w:r><w:t>Link</w:t></w:r></w:hyperlink></w:p>"#,
            r#"<w:p><w:pPr><w:numPr><w:numId w:val="3"/></w:numPr></w:pPr><w:r><w:t>Item</w:t></w:r></w:p>"#,
        ] {
            let xml = format!(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{content}</w:body></w:document>"#
            );
            let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
            archive
                .start_file(
                    "word/document.xml",
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            archive.write_all(xml.as_bytes()).unwrap();
            let document = Document::from_bytes(&archive.finish().unwrap().into_inner()).unwrap();
            assert!(export(&document).is_err());
        }
    }
}
