use std::io::{Cursor, Write};

use crate::document::{Alignment, Document, Run};

pub fn export(document: &Document, title: &str) -> Result<Vec<u8>, String> {
    super::rtf::validate_document(document)?;
    let mut html = format!("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><title>{}</title><style>body{{font-family:Arial,sans-serif;font-size:11pt;line-height:1.5;color:#111;overflow-wrap:anywhere}}main{{white-space:pre-wrap}}img{{max-width:100%;height:auto}}</style></head><body><main>", escape_html(title));
    let mut active_list = None;
    for paragraph in &document.paragraphs {
        let list = match paragraph.list.as_deref() {
            Some("bullet") => Some("ul"),
            Some("number") => Some("ol"),
            None => None,
            _ => return Err("HTML Export Cannot Preserve This List Type.".into()),
        };
        if active_list != list {
            if let Some(previous) = active_list {
                html.push_str(&format!("</{previous}>"));
            }
            if let Some(next) = list {
                html.push_str(&format!("<{next}>"));
            }
            active_list = list;
        }
        let heading = paragraph
            .style
            .as_deref()
            .and_then(|style| style.strip_prefix("Heading"))
            .and_then(|level| level.parse::<u8>().ok())
            .filter(|level| (1..=6).contains(level));
        let tag = if list.is_some() {
            "li".to_owned()
        } else if let Some(level) = heading {
            format!("h{level}")
        } else {
            "p".to_owned()
        };
        let alignment = match paragraph.alignment {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
            Alignment::Justify => "justify",
        };
        let mut style = format!("text-align:{alignment};");
        if paragraph.indent > 0.0 {
            style.push_str(&format!("margin-left:{}px;", paragraph.indent));
        }
        if paragraph.line_spacing > 0.0 {
            style.push_str(&format!("line-height:{};", paragraph.line_spacing));
        }
        html.push_str(&format!("<{tag} style=\"{style}\">"));
        for run in &paragraph.runs {
            html.push_str(&inline(run));
        }
        if paragraph.runs.iter().all(|run| run.text.is_empty()) {
            html.push_str("<br>");
        }
        html.push_str(&format!("</{tag}>"));
    }
    if let Some(previous) = active_list {
        html.push_str(&format!("</{previous}>"));
    }
    html.push_str("</main></body></html>");
    if html.len() > super::downloads::MAX_DOWNLOAD_BYTES {
        return Err("The Download Exceeds The 100 MB Limit.".into());
    }
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file(
            "index.html",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .map_err(|error| format!("Could Not Create The Web Page Archive. {error}"))?;
    archive
        .write_all(html.as_bytes())
        .map_err(|error| format!("Could Not Write The Web Page Archive. {error}"))?;
    archive
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|error| format!("Could Not Finish The Web Page Archive. {error}"))
}

pub(super) fn escape_html(text: &str) -> String {
    let mut result = String::new();
    for character in text.chars() {
        match character {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(character),
        }
    }
    result
}

pub(super) fn inline(run: &Run) -> String {
    let mut text = escape_html(&run.text)
        .replace('\n', "<br>")
        .replace('\t', "&#9;");
    let mut style = String::new();
    if let Some(color) = &run.format.color {
        style.push_str(&format!("color:#{color};"));
    }
    if let Some(color) = &run.format.highlight {
        style.push_str(&format!("background-color:#{color};"));
    }
    if let Some(size) = run.format.font_size {
        style.push_str(&format!("font-size:{size}pt;"));
    }
    if let Some(font) = &run.format.font_family {
        let encoded: String = font
            .chars()
            .map(|character| format!("\\{:x} ", character as u32))
            .collect();
        style.push_str(&format!("font-family:'{encoded}';"));
    }
    if !style.is_empty() {
        text = format!("<span style=\"{}\">{text}</span>", escape_html(&style));
    }
    if run.format.strike {
        text = format!("<s>{text}</s>");
    }
    if run.format.underline {
        text = format!("<u>{text}</u>");
    }
    if run.format.italic {
        text = format!("<em>{text}</em>");
    }
    if run.format.bold {
        text = format!("<strong>{text}</strong>");
    }
    match run.format.script.as_deref() {
        Some("superscript") => format!("<sup>{text}</sup>"),
        Some("subscript") => format!("<sub>{text}</sub>"),
        _ => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Paragraph, TextFormat};
    use std::io::Read;

    #[test]
    fn exports_a_zip_with_escaped_html_and_formatted_content() {
        let mut document = Document::default();
        document.paragraphs = vec![Paragraph {
            alignment: Alignment::Center,
            style: Some("Heading1".into()),
            runs: vec![Run {
                text: "<script>& café\nNext".into(),
                format: TextFormat {
                    bold: true,
                    italic: true,
                    underline: true,
                    strike: true,
                    font_family: Some("A' } color".into()),
                    font_size: Some(14.0),
                    color: Some("FF0000".into()),
                    highlight: Some("00FF00".into()),
                    script: Some("superscript".into()),
                },
            }],
            ..Default::default()
        }];
        let bytes = export(&document, "<Title & Notes>").unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert_eq!(archive.len(), 1);
        let mut html = String::new();
        archive
            .by_name("index.html")
            .unwrap()
            .read_to_string(&mut html)
            .unwrap();
        assert!(html.contains("<title>&lt;Title &amp; Notes&gt;</title>"));
        assert!(html.contains("<h1 style=\"text-align:center;\">"));
        assert!(html.contains("<sup><strong><em><u><s>"));
        assert!(html.contains("&lt;script&gt;&amp; café<br>Next"));
        assert!(!html.contains("<script>"));
        assert!(!html.contains("font-family:'A' } color"));
        assert!(html.contains("font-size:14pt;"));
    }

    #[test]
    fn emits_lists_and_rejects_unrepresented_tables() {
        let mut document = Document::default();
        document.paragraphs = vec![Paragraph {
            list: Some("bullet".into()),
            runs: vec![Run {
                text: "Item".into(),
                format: TextFormat::default(),
            }],
            ..Default::default()
        }];
        let bytes = export(&document, "Test").unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut html = String::new();
        archive
            .by_name("index.html")
            .unwrap()
            .read_to_string(&mut html)
            .unwrap();
        assert!(html.contains("<ul><li style=\"text-align:left;\">Item</li></ul>"));

        let mut source = zip::ZipWriter::new(Cursor::new(Vec::new()));
        source
            .start_file(
                "word/document.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        source.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:tbl/></w:body></w:document>"#).unwrap();
        let document = Document::from_bytes(&source.finish().unwrap().into_inner()).unwrap();
        assert!(export(&document, "Test").is_err());
    }
}
