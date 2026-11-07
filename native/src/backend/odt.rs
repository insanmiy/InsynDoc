use std::io::{Cursor, Write};

use crate::document::{Alignment, Document, Paragraph, Run};

const MIMETYPE: &str = "application/vnd.oasis.opendocument.text";
const OFFICE_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:office:1.0";
const TEXT_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";
const STYLE_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:style:1.0";
const FO_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0";
const META_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:meta:1.0";
const MANIFEST_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:manifest:1.0";
const DC_NS: &str = "http://purl.org/dc/elements/1.1/";

pub fn export(document: &Document, title: &str) -> Result<Vec<u8>, String> {
    super::rtf::validate_document(document)?;
    let content = content_xml(document)?;
    if content.len() > super::downloads::MAX_DOWNLOAD_BYTES {
        return Err("The Download Exceeds The 100 MB Limit.".into());
    }
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let stored = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    archive
        .start_file("mimetype", stored)
        .map_err(|error| format!("Could Not Create The Document Archive. {error}"))?;
    archive
        .write_all(MIMETYPE.as_bytes())
        .map_err(|error| format!("Could Not Write The Document Archive. {error}"))?;
    for (name, xml) in [
        ("META-INF/manifest.xml", manifest_xml().into_bytes()),
        ("content.xml", content.into_bytes()),
        ("styles.xml", styles_xml().into_bytes()),
        ("meta.xml", meta_xml(title).into_bytes()),
    ] {
        archive
            .start_file(name, deflated)
            .map_err(|error| format!("Could Not Create The Document Archive. {error}"))?;
        archive
            .write_all(&xml)
            .map_err(|error| format!("Could Not Write The Document Archive. {error}"))?;
    }
    let bytes = archive
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|error| format!("Could Not Finish The Document Archive. {error}"))?;
    if bytes.len() > super::downloads::MAX_DOWNLOAD_BYTES {
        return Err("The Download Exceeds The 100 MB Limit.".into());
    }
    Ok(bytes)
}

fn content_xml(document: &Document) -> Result<String, String> {
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-content xmlns:office=\"{OFFICE_NS}\" xmlns:text=\"{TEXT_NS}\" xmlns:style=\"{STYLE_NS}\" xmlns:fo=\"{FO_NS}\" office:version=\"1.2\"><office:body><office:text>"
    );
    let mut active_list: Option<&str> = None;
    for paragraph in &document.paragraphs {
        let list = match paragraph.list.as_deref() {
            None => None,
            Some("bullet") => Some("bullet"),
            Some("number") => Some("number"),
            _ => return Err("ODT Export Cannot Preserve This List Type.".into()),
        };
        if active_list != list {
            if active_list.is_some() {
                xml.push_str("</text:list>");
            }
            if let Some(kind) = list {
                let style = if kind == "bullet" {
                    "InsynDocBullet"
                } else {
                    "InsynDocNumber"
                };
                xml.push_str(&format!("<text:list text:style-name=\"{style}\">"));
            }
            active_list = list;
        }
        if list.is_some() {
            xml.push_str("<text:list-item>");
        }
        xml.push_str(&paragraph_xml(paragraph));
        if list.is_some() {
            xml.push_str("</text:list-item>");
        }
    }
    if active_list.is_some() {
        xml.push_str("</text:list>");
    }
    xml.push_str("</office:text></office:body></office:document-content>");
    Ok(xml)
}

fn paragraph_xml(paragraph: &Paragraph) -> String {
    let level = paragraph
        .style
        .as_deref()
        .and_then(|style| style.strip_prefix("Heading"))
        .and_then(|level| level.parse::<u8>().ok())
        .filter(|level| (1..=6).contains(level));
    let mut xml = String::new();
    if let Some(level) = level {
        xml.push_str(&format!("<text:h text:outline-level=\"{level}\""));
    } else {
        xml.push_str("<text:p");
    }
    xml.push_str(&paragraph_attributes(paragraph));
    xml.push('>');
    for run in &paragraph.runs {
        xml.push_str(&span_xml(run));
    }
    if level.is_some() {
        xml.push_str("</text:h>");
    } else {
        xml.push_str("</text:p>");
    }
    xml
}

fn paragraph_attributes(paragraph: &Paragraph) -> String {
    let mut attributes = String::new();
    attributes.push_str(&format!(
        " {}=\"{}\"",
        prefixed("text", "style-name"),
        paragraph_style_name(paragraph)
    ));
    let alignment = match paragraph.alignment {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "justify",
    };
    attributes.push_str(&format!(
        " {}=\"{}\"",
        prefixed("fo", "text-align"),
        alignment
    ));
    if paragraph.indent > 0.0 {
        attributes.push_str(&format!(
            " {}=\"{:.2}cm\"",
            prefixed("fo", "margin-left"),
            paragraph.indent * 2.54 / 96.0
        ));
    }
    if paragraph.line_spacing > 0.0 {
        attributes.push_str(&format!(
            " {}=\"{:.0}%\"",
            prefixed("fo", "line-height"),
            paragraph.line_spacing * 100.0
        ));
    }
    attributes
}

fn paragraph_style_name(paragraph: &Paragraph) -> &'static str {
    if paragraph
        .style
        .as_deref()
        .and_then(|style| style.strip_prefix("Heading"))
        .and_then(|level| level.parse::<u8>().ok())
        .is_some_and(|level| (1..=6).contains(&level))
    {
        "Heading"
    } else if paragraph.list.is_some() {
        "ListParagraph"
    } else {
        "Standard"
    }
}

fn span_xml(run: &Run) -> String {
    let format = &run.format;
    let mut attributes = String::new();
    if format.bold {
        attributes.push_str(&format!(" {}=\"bold\"", prefixed("fo", "font-weight")));
    }
    if format.italic {
        attributes.push_str(&format!(" {}=\"italic\"", prefixed("fo", "font-style")));
    }
    let decorations: Vec<&str> = [
        (format.underline, "underline"),
        (format.strike, "line-through"),
    ]
    .into_iter()
    .filter_map(|(enabled, name)| enabled.then_some(name))
    .collect();
    if !decorations.is_empty() {
        attributes.push_str(&format!(
            " {}=\"{}\"",
            prefixed("fo", "text-decoration"),
            decorations.join(" ")
        ));
    }
    if let Some(color) = &format.color {
        attributes.push_str(&format!(
            " {}=\"#{}\"",
            prefixed("fo", "color"),
            color.to_ascii_uppercase()
        ));
    }
    if let Some(color) = &format.highlight {
        attributes.push_str(&format!(
            " {}=\"#{}\"",
            prefixed("fo", "background-color"),
            color.to_ascii_uppercase()
        ));
    }
    if let Some(size) = format.font_size {
        attributes.push_str(&format!(
            " {}=\"{}pt\"",
            prefixed("fo", "font-size"),
            trim_number(size)
        ));
    }
    if let Some(font) = &format.font_family {
        attributes.push_str(&format!(
            " {}=\"'{}'\"",
            prefixed("style", "font-name"),
            escape_attribute(font)
        ));
    }
    match format.script.as_deref() {
        Some("superscript") => {
            attributes.push_str(&format!(" {}=\"super\"", prefixed("fo", "vertical-align")));
        }
        Some("subscript") => {
            attributes.push_str(&format!(" {}=\"sub\"", prefixed("fo", "vertical-align")));
        }
        _ => {}
    }
    if attributes.is_empty() {
        return escape_text(&run.text);
    }
    format!("<text:span{attributes}>{}</text:span>", escape_text(&run.text))
}

fn styles_xml() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-styles xmlns:office=\"{OFFICE_NS}\" xmlns:text=\"{TEXT_NS}\" xmlns:style=\"{STYLE_NS}\" xmlns:fo=\"{FO_NS}\" xmlns:list=\"urn:oasis:names:tc:opendocument:xmlns:list:1.0\" office:version=\"1.2\"><office:styles><style:style style:name=\"Standard\" style:family=\"paragraph\"/><style:style style:name=\"Heading\" style:family=\"paragraph\" style:parent-style-name=\"Standard\" style:next-style-name=\"Standard\"/><style:style style:name=\"ListParagraph\" style:family=\"paragraph\" style:parent-style-name=\"Standard\"/></office:styles><office:automatic-styles><text:list-style style:name=\"InsynDocBullet\"><text:list-level-style-bullet text:level=\"1\" text:bullet-char=\"&#8226;\" text:space-before=\"0.25cm\"/></text:list-style><text:list-style style:name=\"InsynDocNumber\"><text:list-level-style-number text:level=\"1\" text:num-format=\"1\" text:num-suffix=\".\" text:space-before=\"0.25cm\"/></text:list-style></office:automatic-styles><office:master-styles/></office:document-styles>"
    )
}

fn meta_xml(title: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-meta xmlns:office=\"{OFFICE_NS}\" xmlns:meta=\"{META_NS}\" xmlns:dc=\"{DC_NS}\" office:version=\"1.2\"><office:meta><dc:title>{}</dc:title><meta:generator>InsynDoc</meta:generator></office:meta></office:document-meta>",
        escape_text(title)
    )
}

fn manifest_xml() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><manifest:manifest xmlns:manifest=\"{MANIFEST_NS}\" manifest:version=\"1.2\"><manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"{MIMETYPE}\"/><manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/><manifest:file-entry manifest:full-path=\"styles.xml\" manifest:media-type=\"text/xml\"/><manifest:file-entry manifest:full-path=\"meta.xml\" manifest:media-type=\"text/xml\"/></manifest:manifest>"
    )
}

fn prefixed(prefix: &str, name: &str) -> String {
    format!("{prefix}:{name}")
}

fn escape_text(text: &str) -> String {
    let mut output = String::new();
    for character in text.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(character),
        }
    }
    let mut result = String::new();
    for character in output.chars() {
        match character {
            '\n' => result.push_str("<text:line-break/>"),
            _ => result.push(character),
        }
    }
    result
}

fn escape_attribute(text: &str) -> String {
    escape_text(text)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn trim_number(value: f32) -> String {
    let mut text = format!("{value}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Paragraph, TextFormat};
    use std::io::Read;

    fn text_paragraph(text: &str) -> Paragraph {
        Paragraph {
            runs: vec![Run {
                text: text.into(),
                format: TextFormat::default(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn writes_a_valid_odt_package_with_formatted_content() {
        let mut document = Document::default();
        document.paragraphs = vec![
            Paragraph {
                alignment: Alignment::Center,
                style: Some("Heading1".into()),
                runs: vec![Run {
                    text: "Title & <Co>".into(),
                    format: TextFormat {
                        bold: true,
                        italic: true,
                        underline: true,
                        strike: true,
                        font_family: Some("Arial".into()),
                        font_size: Some(18.0),
                        color: Some("FF0000".into()),
                        highlight: Some("FFFF00".into()),
                        script: Some("superscript".into()),
                    },
                }],
                ..Default::default()
            },
            Paragraph {
                list: Some("number".into()),
                runs: vec![Run {
                    text: "First".into(),
                    format: TextFormat::default(),
                }],
                ..Default::default()
            },
            Paragraph {
                list: Some("bullet".into()),
                runs: vec![Run {
                    text: "Second".into(),
                    format: TextFormat::default(),
                }],
                ..Default::default()
            },
            text_paragraph("Body Line\nNext"),
        ];
        let bytes = export(&document, "Report & Notes").unwrap();
        assert!(bytes.starts_with(b"PK\x03\x04"));
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().unwrap().into_owned())
            .collect();
        assert_eq!(names[0], "mimetype");
        {
            let first = archive.by_index(0).unwrap();
            assert_eq!(first.compression(), zip::CompressionMethod::Stored);
        }
        let mut mimetype = String::new();
        archive.by_name("mimetype").unwrap().read_to_string(&mut mimetype).unwrap();
        assert_eq!(mimetype, MIMETYPE);
        for name in [
            "META-INF/manifest.xml",
            "content.xml",
            "styles.xml",
            "meta.xml",
        ] {
            assert!(names.iter().any(|entry| entry == name), "missing {name}");
        }
        let mut content = String::new();
        archive
            .by_name("content.xml")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        for needle in [
            "<text:h text:outline-level=\"1\"",
            "fo:text-align=\"center\"",
            "fo:font-weight=\"bold\"",
            "fo:font-style=\"italic\"",
            "fo:text-decoration=\"underline line-through\"",
            "fo:color=\"#FF0000\"",
            "fo:background-color=\"#FFFF00\"",
            "fo:font-size=\"18pt\"",
            "fo:vertical-align=\"super\"",
            "style:font-name=\"'Arial'\"",
            "Title &amp; &lt;Co&gt;",
            "<text:list text:style-name=\"InsynDocNumber\">",
            "<text:list text:style-name=\"InsynDocBullet\">",
            "<text:line-break/>",
        ] {
            assert!(content.contains(needle), "content.xml missing {needle}");
        }
        let mut meta = String::new();
        archive
            .by_name("meta.xml")
            .unwrap()
            .read_to_string(&mut meta)
            .unwrap();
        assert!(meta.contains("Report &amp; Notes"));
    }

    #[test]
    fn rejects_unsupported_content_instead_of_omitting_it() {
        use std::io::Write;
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file(
                "word/document.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive
            .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:tbl/><w:sectPr/></w:body></w:document>"#)
            .unwrap();
        let document = Document::from_bytes(&archive.finish().unwrap().into_inner()).unwrap();
        assert!(export(&document, "Test").is_err());

        let mut document = Document::default();
        document.paragraphs = vec![Paragraph {
            list: Some("custom".into()),
            runs: vec![Run {
                text: "Item".into(),
                format: TextFormat::default(),
            }],
            ..Default::default()
        }];
        assert!(export(&document, "Test").is_err());
    }
}
