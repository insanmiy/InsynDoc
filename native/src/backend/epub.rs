use std::io::{Cursor, Write};

use crate::document::{Alignment, Document, Paragraph};

const MIMETYPE: &str = "application/epub+zip";
const CONTAINER: &str = "META-INF/container.xml";
const OPF: &str = "OEBPS/content.opf";
const NAV: &str = "OEBPS/nav.xhtml";
const STYLES: &str = "OEBPS/styles.css";

pub fn export(document: &Document, title: &str) -> Result<Vec<u8>, String> {
    super::rtf::validate_document(document)?;
    let chapters = chapters(document)?;
    let body = chapters
        .iter()
        .map(|(_, _, paragraphs)| chapter_xhtml(title, paragraphs))
        .collect::<Result<Vec<_>, _>>()?;
    if body.iter().map(String::len).sum::<usize>() > super::downloads::MAX_DOWNLOAD_BYTES {
        return Err("The Download Exceeds The 100 MB Limit.".into());
    }
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let stored = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    archive
        .start_file("mimetype", stored)
        .map_err(|error| format!("Could Not Create The Publication Archive. {error}"))?;
    archive
        .write_all(MIMETYPE.as_bytes())
        .map_err(|error| format!("Could Not Write The Publication Archive. {error}"))?;
    let mut parts: Vec<(String, Vec<u8>)> = vec![
        (CONTAINER.into(), container_xml().into_bytes()),
        (OPF.into(), opf_xml(title, &chapters).into_bytes()),
        (NAV.into(), nav_xhtml(title, &chapters).into_bytes()),
        (STYLES.into(), styles_css().as_bytes().to_vec()),
    ];
    for (index, chapter_body) in body.into_iter().enumerate() {
        parts.push((
            format!("OEBPS/chapter-{}.xhtml", index + 1),
            chapter_body.into_bytes(),
        ));
    }
    for (name, bytes) in parts {
        archive
            .start_file(name, deflated)
            .map_err(|error| format!("Could Not Create The Publication Archive. {error}"))?;
        archive
            .write_all(&bytes)
            .map_err(|error| format!("Could Not Write The Publication Archive. {error}"))?;
    }
    let bytes = archive
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|error| format!("Could Not Finish The Publication Archive. {error}"))?;
    if bytes.len() > super::downloads::MAX_DOWNLOAD_BYTES {
        return Err("The Download Exceeds The 100 MB Limit.".into());
    }
    Ok(bytes)
}

// Splits The Document Into Chapters At Each Top Level Heading.
fn chapters(document: &Document) -> Result<Vec<(usize, String, Vec<Paragraph>)>, String> {
    let mut result: Vec<(usize, String, Vec<Paragraph>)> = Vec::new();
    for paragraph in &document.paragraphs {
        let top_level_heading = paragraph
            .style
            .as_deref()
            .and_then(heading_level)
            .is_some_and(|level| level == 1);
        let starts_new_chapter = top_level_heading
            && result
                .last()
                .is_some_and(|(_, _, items)| !items.is_empty());
        if starts_new_chapter {
            let title = paragraph.plain_text().trim().to_owned();
            result.push((
                result.len() + 1,
                if title.is_empty() {
                    format!("Chapter {}", result.len() + 1)
                } else {
                    title
                },
                vec![paragraph.clone()],
            ));
            continue;
        }
        match result.last_mut() {
            Some((_, _, items)) => items.push(paragraph.clone()),
            None => result.push((1, "Untitled".to_owned(), vec![paragraph.clone()])),
        }
    }
    if result.is_empty() {
        result.push((1, "Untitled".to_owned(), Vec::new()));
    }
    // Retitles The First Chapter From Its Opening Heading When Present.
    if let Some((_, title, items)) = result.first_mut() {
        if let Some(first) = items.first() {
            if first
                .style
                .as_deref()
                .and_then(heading_level)
                .is_some()
            {
                let text = first.plain_text().trim().to_owned();
                if !text.is_empty() {
                    *title = text;
                }
            }
        }
    }
    for (index, (number, title, _)) in result.iter_mut().enumerate() {
        *number = index + 1;
        if title.is_empty() {
            *title = format!("Chapter {}", index + 1);
        }
    }
    Ok(result)
}

fn heading_level(style: &str) -> Option<u8> {
    style
        .strip_prefix("Heading")?
        .parse::<u8>()
        .ok()
        .filter(|level| (1..=6).contains(level))
}

fn chapter_xhtml(title: &str, paragraphs: &[Paragraph]) -> Result<String, String> {
    let mut body = String::new();
    let mut active_list = None;
    for paragraph in paragraphs {
        let list = match paragraph.list.as_deref() {
            Some("bullet") => Some("ul"),
            Some("number") => Some("ol"),
            None => None,
            _ => return Err("EPUB Export Cannot Preserve This List Type.".into()),
        };
        if active_list != list {
            if let Some(previous) = active_list {
                body.push_str(&format!("</{previous}>"));
            }
            if let Some(next) = list {
                body.push_str(&format!("<{next}>"));
            }
            active_list = list;
        }
        let heading = paragraph
            .style
            .as_deref()
            .and_then(heading_level);
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
        body.push_str(&format!("<{tag} style=\"{style}\">"));
        for run in &paragraph.runs {
            body.push_str(&super::html::inline(run));
        }
        if paragraph.runs.iter().all(|run| run.text.is_empty()) {
            body.push_str("<br>");
        }
        body.push_str(&format!("</{tag}>"));
    }
    if let Some(previous) = active_list {
        body.push_str(&format!("</{previous}>"));
    }
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE html><html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"en\"><head><meta charset=\"UTF-8\"/><title>{}</title><link rel=\"stylesheet\" type=\"text/css\" href=\"styles.css\"/></head><body><main>{body}</main></body></html>",
        super::html::escape_html(title),
    ))
}

fn container_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\"><rootfiles><rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>".into()
}

fn opf_xml(title: &str, chapters: &[(usize, String, Vec<Paragraph>)]) -> String {
    let mut manifest = String::new();
    manifest.push_str("<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>");
    manifest.push_str("<item id=\"styles\" href=\"styles.css\" media-type=\"text/css\"/>");
    let mut spine = String::new();
    for (index, (number, _, _)) in chapters.iter().enumerate() {
        manifest.push_str(&format!(
            "<item id=\"chapter-{number}\" href=\"chapter-{}.xhtml\" media-type=\"application/xhtml+xml\"/>",
            index + 1
        ));
        spine.push_str(&format!("<itemref idref=\"chapter-{number}\"/>"));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"book-id\"><metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:opf=\"http://www.idpf.org/2007/opf\"><dc:identifier id=\"book-id\">urn:insyndoc:export</dc:identifier><dc:title>{}</dc:title><dc:language>en</dc:language><meta property=\"dcterms:modified\">{}</meta></metadata><manifest>{manifest}</manifest><spine>{spine}</spine></package>",
        super::html::escape_html(title),
        modified_timestamp(),
    )
}

fn nav_xhtml(title: &str, chapters: &[(usize, String, Vec<Paragraph>)]) -> String {
    let mut items = String::new();
    for (index, (_, chapter_title, _)) in chapters.iter().enumerate() {
        items.push_str(&format!(
            "<li><a href=\"chapter-{}.xhtml\">{}</a></li>",
            index + 1,
            super::html::escape_html(chapter_title)
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE html><html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"en\"><head><meta charset=\"UTF-8\"/><title>{}</title></head><body><nav epub:type=\"toc\"><h1>{}</h1><ol>{items}</ol></nav></body></html>",
        super::html::escape_html(title),
        super::html::escape_html(title),
    )
}

fn styles_css() -> &'static str {
    "body{font-family:Arial,sans-serif;font-size:11pt;line-height:1.5;color:#111;overflow-wrap:anywhere}main{white-space:pre-wrap}img{max-width:100%;height:auto}h1,h2,h3,h4,h5,h6{line-height:1.2}"
}

fn modified_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Paragraph, Run, TextFormat};
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
    fn writes_a_valid_epub3_package_with_formatted_chapters() {
        let mut document = Document::default();
        document.paragraphs = vec![
            Paragraph {
                style: Some("Heading1".into()),
                runs: vec![Run {
                    text: "First & <Chapter>".into(),
                    format: TextFormat::default(),
                }],
                ..Default::default()
            },
            Paragraph {
                alignment: Alignment::Right,
                runs: vec![Run {
                    text: "Body <Text>".into(),
                    format: TextFormat {
                        bold: true,
                        italic: true,
                        underline: true,
                        strike: true,
                        font_size: Some(14.0),
                        color: Some("FF0000".into()),
                        highlight: Some("00FF00".into()),
                        script: Some("superscript".into()),
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
                style: Some("Heading1".into()),
                runs: vec![Run {
                    text: "Second".into(),
                    format: TextFormat::default(),
                }],
                ..Default::default()
            },
            text_paragraph("Tail"),
        ];
        let bytes = export(&document, "My <Book> & Notes").unwrap();
        assert!(bytes.starts_with(b"PK\x03\x04"));
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        {
            let first = archive.by_index(0).unwrap();
            assert_eq!(first.name().unwrap(), "mimetype");
            assert_eq!(first.compression(), zip::CompressionMethod::Stored);
        }
        let mut mimetype = String::new();
        archive.by_name("mimetype").unwrap().read_to_string(&mut mimetype).unwrap();
        assert_eq!(mimetype, MIMETYPE);
        let mut container = String::new();
        archive.by_name(CONTAINER).unwrap().read_to_string(&mut container).unwrap();
        assert!(container.contains("OEBPS/content.opf"));
        let mut opf = String::new();
        archive.by_name(OPF).unwrap().read_to_string(&mut opf).unwrap();
        for needle in [
            "<package",
            "version=\"3.0\"",
            "<dc:title>My &lt;Book&gt; &amp; Notes</dc:title>",
            "dcterms:modified",
            "id=\"nav\"",
            "properties=\"nav\"",
            "id=\"chapter-1\"",
            "id=\"chapter-2\"",
            "<itemref idref=\"chapter-1\"/>",
            "<itemref idref=\"chapter-2\"/>",
        ] {
            assert!(opf.contains(needle), "content.opf missing {needle}");
        }
        let mut nav = String::new();
        archive.by_name(NAV).unwrap().read_to_string(&mut nav).unwrap();
        assert!(nav.contains("epub:type=\"toc\""));
        assert!(nav.contains("chapter-1.xhtml\">First &amp; &lt;Chapter&gt;</a>"));
        assert!(nav.contains("chapter-2.xhtml\">Second</a>"));
        let mut chapter_one = String::new();
        archive
            .by_name("OEBPS/chapter-1.xhtml")
            .unwrap()
            .read_to_string(&mut chapter_one)
            .unwrap();
        for needle in [
            "<h1 style=\"text-align:left;\">",
            "text-align:right;",
            "font-size:14pt;",
            "color:#FF0000;",
            "background-color:#00FF00;",
            "<sup><strong><em><u><s>",
            "Body &lt;Text&gt;",
            "<ul><li",
        ] {
            assert!(chapter_one.contains(needle), "chapter missing {needle}");
        }
        assert!(!chapter_one.contains("Second"));
        let mut chapter_two = String::new();
        archive
            .by_name("OEBPS/chapter-2.xhtml")
            .unwrap()
            .read_to_string(&mut chapter_two)
            .unwrap();
        assert!(chapter_two.contains("<h1 style=\"text-align:left;\">Second</h1>"));
        assert!(chapter_two.contains("Tail"));
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
