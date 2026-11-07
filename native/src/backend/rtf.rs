use std::io::{Cursor, Read};

use crate::document::{Alignment, Document, Run};

const WORD_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn validate_and_collect(document: &Document) -> Result<(Vec<String>, Vec<String>), String> {
    let bytes = document.to_bytes().map_err(|error| error.to_string())?;
    validate_content(&bytes, document)?;
    let mut fonts: Vec<String> = Vec::new();
    let mut colors: Vec<String> = Vec::new();
    for paragraph in &document.paragraphs {
        if !paragraph.indent.is_finite()
            || paragraph.indent < 0.0
            || !paragraph.line_spacing.is_finite()
            || paragraph.line_spacing < 0.0
        {
            return Err(unsupported());
        }
        for run in &paragraph.runs {
            if let Some(font) = &run.format.font_family {
                if font.is_empty()
                    || font
                        .chars()
                        .any(|character| character.is_control() || character == ';')
                {
                    return Err(unsupported());
                }
                if !fonts.contains(font) {
                    fonts.push(font.clone());
                }
            }
            if let Some(size) = run.format.font_size {
                if !size.is_finite() || size <= 0.0 || size > 16383.0 {
                    return Err(unsupported());
                }
            }
            if !matches!(
                run.format.script.as_deref(),
                None | Some("superscript" | "subscript")
            ) {
                return Err(unsupported());
            }
            for color in [&run.format.color, &run.format.highlight]
                .into_iter()
                .flatten()
            {
                if color.len() != 6 || !color.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(unsupported());
                }
                let color = color.to_ascii_uppercase();
                if !colors.contains(&color) {
                    colors.push(color);
                }
            }
        }
    }
    let page = &document.page_setup;
    let dimensions = [
        page.width,
        page.height,
        page.margin_top,
        page.margin_right,
        page.margin_bottom,
        page.margin_left,
    ];
    if dimensions
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0 || *value > 31680.0 / 15.0)
        || page.width == 0.0
        || page.height == 0.0
    {
        return Err(unsupported());
    }
    Ok((fonts, colors))
}

pub(super) fn validate_document(document: &Document) -> Result<(), String> {
    validate_and_collect(document).map(|_| ())
}

pub fn export(document: &Document) -> Result<String, String> {
    let (fonts, colors) = validate_and_collect(document)?;
    let page = &document.page_setup;
    let mut output = String::from("{\\rtf1\\ansi\\uc1\\deff0");
    output.push_str("{\\fonttbl{\\f0\\fnil Arial;}");
    for (index, font) in fonts.iter().enumerate() {
        output.push_str(&format!(
            "{{\\f{}\\fnil {};}}",
            index + 1,
            escape_text(font)
        ));
    }
    output.push('}');
    output.push_str("{\\colortbl;");
    for color in &colors {
        let red = u8::from_str_radix(&color[0..2], 16).map_err(|_| unsupported())?;
        let green = u8::from_str_radix(&color[2..4], 16).map_err(|_| unsupported())?;
        let blue = u8::from_str_radix(&color[4..6], 16).map_err(|_| unsupported())?;
        output.push_str(&format!("\\red{red}\\green{green}\\blue{blue};"));
    }
    output.push('}');
    output.push_str(&format!(
        "\\paperw{}\\paperh{}\\margt{}\\margr{}\\margb{}\\margl{}\n",
        twips(page.width),
        twips(page.height),
        twips(page.margin_top),
        twips(page.margin_right),
        twips(page.margin_bottom),
        twips(page.margin_left)
    ));
    let mut numbered_count = 0;
    for paragraph in &document.paragraphs {
        output.push_str("{\\pard");
        output.push_str(match paragraph.alignment {
            Alignment::Left => "\\ql",
            Alignment::Center => "\\qc",
            Alignment::Right => "\\qr",
            Alignment::Justify => "\\qj",
        });
        if paragraph.indent != 0.0 {
            output.push_str(&format!("\\li{}", twips(paragraph.indent)));
        }
        if paragraph.line_spacing > 0.0 {
            output.push_str(&format!(
                "\\sl{}\\slmult1",
                (paragraph.line_spacing * 240.0).round() as i32
            ));
        }
        if let Some(level) = paragraph.style.as_deref().and_then(heading_level) {
            output.push_str(&format!("\\b\\fs{}", (44_i32 - level as i32 * 4).max(24)));
        } else {
            output.push_str("\\fs22");
        }
        output.push(' ');
        match paragraph.list.as_deref() {
            Some("bullet") => {
                numbered_count = 0;
                output.push_str("\\u8226?\\tab ");
            }
            Some("number") => {
                numbered_count += 1;
                output.push_str(&format!("{numbered_count}.\\tab "));
            }
            None => numbered_count = 0,
            _ => return Err(unsupported()),
        }
        for run in &paragraph.runs {
            output.push_str(&format_run(run, &fonts, &colors));
        }
        output.push_str("\\par}\n");
    }
    output.push('}');
    Ok(output)
}

fn twips(pixels: f32) -> i32 {
    (pixels * 15.0).round() as i32
}

fn heading_level(style: &str) -> Option<usize> {
    style
        .strip_prefix("Heading")?
        .parse::<usize>()
        .ok()
        .filter(|level| (1..=6).contains(level))
}

fn escape_text(text: &str) -> String {
    let mut output = String::new();
    for character in text.chars() {
        match character {
            '\\' | '{' | '}' => {
                output.push('\\');
                output.push(character);
            }
            '\n' => output.push_str("\\line "),
            '\t' => output.push_str("\\tab "),
            character if (' '..='~').contains(&character) => output.push(character),
            character => {
                let mut units = [0u16; 2];
                for unit in character.encode_utf16(&mut units) {
                    output.push_str(&format!("\\u{}?", *unit as i16));
                }
            }
        }
    }
    output
}

fn format_run(run: &Run, fonts: &[String], colors: &[String]) -> String {
    let format = &run.format;
    let mut result = String::from("{");
    for (enabled, control) in [
        (format.bold, "\\b"),
        (format.italic, "\\i"),
        (format.underline, "\\ul"),
        (format.strike, "\\strike"),
    ] {
        if enabled {
            result.push_str(control);
        }
    }
    if let Some(font) = &format.font_family {
        result.push_str(&format!(
            "\\f{}",
            fonts.iter().position(|entry| entry == font).unwrap() + 1
        ));
    }
    if let Some(size) = format.font_size {
        result.push_str(&format!("\\fs{}", (size * 2.0).round() as i32));
    }
    if let Some(color) = &format.color {
        result.push_str(&format!(
            "\\cf{}",
            colors
                .iter()
                .position(|entry| entry.eq_ignore_ascii_case(color))
                .unwrap()
                + 1
        ));
    }
    if let Some(color) = &format.highlight {
        result.push_str(&format!(
            "\\highlight{}",
            colors
                .iter()
                .position(|entry| entry.eq_ignore_ascii_case(color))
                .unwrap()
                + 1
        ));
    }
    match format.script.as_deref() {
        Some("superscript") => result.push_str("\\super"),
        Some("subscript") => result.push_str("\\sub"),
        _ => {}
    }
    result.push(' ');
    result.push_str(&escape_text(&run.text));
    result.push('}');
    result
}

fn unsupported() -> String {
    "This Export Cannot Yet Preserve Some Content Or Formatting In This Document.".into()
}

fn validate_content(bytes: &[u8], document: &Document) -> Result<(), String> {
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
    let mut paragraph_index = 0;
    let mut list_ids: Vec<(String, String)> = Vec::new();
    for block in body.children().filter(|node| node.is_element()) {
        if block.has_tag_name((WORD_NAMESPACE, "sectPr")) {
            if block
                .children()
                .filter(|node| node.is_element())
                .any(|node| {
                    node.tag_name().namespace() != Some(WORD_NAMESPACE)
                        || !matches!(node.tag_name().name(), "pgSz" | "pgMar")
                })
            {
                return Err(unsupported());
            }
            if block
                .children()
                .filter(|node| node.has_tag_name((WORD_NAMESPACE, "pgMar")))
                .any(|node| {
                    node.attribute((WORD_NAMESPACE, "gutter"))
                        .is_some_and(|value| value != "0")
                })
            {
                return Err(unsupported());
            }
            continue;
        }
        if !block.has_tag_name((WORD_NAMESPACE, "p")) {
            return Err(unsupported());
        }
        let paragraph = document
            .paragraphs
            .get(paragraph_index)
            .ok_or_else(unsupported)?;
        paragraph_index += 1;
        for child in block.children().filter(|node| node.is_element()) {
            if child.has_tag_name((WORD_NAMESPACE, "pPr")) {
                for property in child.children().filter(|node| node.is_element()) {
                    if property.tag_name().namespace() != Some(WORD_NAMESPACE)
                        || !matches!(
                            property.tag_name().name(),
                            "pStyle" | "numPr" | "jc" | "ind" | "spacing"
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
                        let level = property
                            .children()
                            .find(|node| node.has_tag_name((WORD_NAMESPACE, "ilvl")))
                            .and_then(|node| node.attribute((WORD_NAMESPACE, "val")))
                            .unwrap_or("0");
                        if level != "0"
                            || !matches!(paragraph.list.as_deref(), Some("bullet" | "number"))
                        {
                            return Err(unsupported());
                        }
                        let id = property
                            .children()
                            .find(|node| node.has_tag_name((WORD_NAMESPACE, "numId")))
                            .and_then(|node| node.attribute((WORD_NAMESPACE, "val")))
                            .ok_or_else(unsupported)?;
                        if !matches!(id, "2147483000" | "2147483001") {
                            return Err(unsupported());
                        }
                        let entry = (
                            id.to_owned(),
                            paragraph.list.clone().ok_or_else(unsupported)?,
                        );
                        if !list_ids.contains(&entry) {
                            list_ids.push(entry);
                        }
                    }
                    if property.has_tag_name((WORD_NAMESPACE, "jc"))
                        && !matches!(
                            property.attribute((WORD_NAMESPACE, "val")),
                            Some("left" | "start" | "center" | "right" | "end" | "both")
                        )
                    {
                        return Err(unsupported());
                    }
                    if property.has_tag_name((WORD_NAMESPACE, "ind"))
                        && property.attributes().any(|attribute| {
                            attribute.namespace() != Some(WORD_NAMESPACE)
                                || attribute.name() != "left"
                        })
                    {
                        return Err(unsupported());
                    }
                    if property.has_tag_name((WORD_NAMESPACE, "ind"))
                        && property
                            .attribute((WORD_NAMESPACE, "left"))
                            .is_some_and(|value| value.parse::<i32>().is_err())
                    {
                        return Err(unsupported());
                    }
                    if property.has_tag_name((WORD_NAMESPACE, "spacing"))
                        && property.attributes().any(|attribute| {
                            attribute.namespace() != Some(WORD_NAMESPACE)
                                || !matches!(attribute.name(), "line" | "lineRule")
                        })
                    {
                        return Err(unsupported());
                    }
                    if property.has_tag_name((WORD_NAMESPACE, "spacing"))
                        && property
                            .attribute((WORD_NAMESPACE, "lineRule"))
                            .is_some_and(|value| value != "auto")
                    {
                        return Err(unsupported());
                    }
                    if property.has_tag_name((WORD_NAMESPACE, "spacing"))
                        && property
                            .attribute((WORD_NAMESPACE, "line"))
                            .is_some_and(|value| value.parse::<u32>().is_err())
                    {
                        return Err(unsupported());
                    }
                }
                continue;
            }
            if !child.has_tag_name((WORD_NAMESPACE, "r")) {
                return Err(unsupported());
            }
            for node in child.children().filter(|node| node.is_element()) {
                if node.tag_name().namespace() != Some(WORD_NAMESPACE)
                    || !matches!(node.tag_name().name(), "rPr" | "t" | "tab" | "br" | "cr")
                {
                    return Err(unsupported());
                }
                if node.has_tag_name((WORD_NAMESPACE, "br"))
                    && node
                        .attribute((WORD_NAMESPACE, "type"))
                        .is_some_and(|value| value != "textWrapping")
                {
                    return Err(unsupported());
                }
                if node.has_tag_name((WORD_NAMESPACE, "rPr")) {
                    for property in node.children().filter(|node| node.is_element()) {
                        if property.tag_name().namespace() != Some(WORD_NAMESPACE)
                            || !matches!(
                                property.tag_name().name(),
                                "b" | "i"
                                    | "u"
                                    | "strike"
                                    | "rFonts"
                                    | "sz"
                                    | "color"
                                    | "shd"
                                    | "highlight"
                                    | "vertAlign"
                            )
                        {
                            return Err(unsupported());
                        }
                        if property.has_tag_name((WORD_NAMESPACE, "u"))
                            && property
                                .attribute((WORD_NAMESPACE, "val"))
                                .is_some_and(|value| !matches!(value, "single" | "none"))
                        {
                            return Err(unsupported());
                        }
                        if property.has_tag_name((WORD_NAMESPACE, "rFonts")) {
                            if property.attributes().any(|attribute| {
                                attribute.namespace() != Some(WORD_NAMESPACE)
                                    || !matches!(attribute.name(), "ascii" | "hAnsi")
                            }) || matches!((property.attribute((WORD_NAMESPACE, "ascii")), property.attribute((WORD_NAMESPACE, "hAnsi"))), (Some(first), Some(second)) if first != second)
                            {
                                return Err(unsupported());
                            }
                        } else if property.has_tag_name((WORD_NAMESPACE, "shd")) {
                            if property.attributes().any(|attribute| {
                                attribute.namespace() != Some(WORD_NAMESPACE)
                                    || !matches!(attribute.name(), "val" | "color" | "fill")
                            }) || property
                                .attribute((WORD_NAMESPACE, "val"))
                                .is_some_and(|value| value != "clear")
                            {
                                return Err(unsupported());
                            }
                        } else if property.attributes().any(|attribute| {
                            attribute.namespace() != Some(WORD_NAMESPACE)
                                || attribute.name() != "val"
                        }) {
                            return Err(unsupported());
                        }
                    }
                }
            }
        }
    }
    if paragraph_index != document.paragraphs.len() {
        return Err(unsupported());
    }
    validate_styles(&mut archive, document)?;
    validate_numbering(&mut archive, &list_ids)?;
    Ok(())
}

fn validate_styles(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    document: &Document,
) -> Result<(), String> {
    let mut xml = String::new();
    match archive.by_name("word/styles.xml") {
        Ok(mut file) => {
            file.read_to_string(&mut xml)
                .map_err(|error| error.to_string())?;
        }
        Err(zip::result::ZipError::FileNotFound) => return Ok(()),
        Err(error) => return Err(error.to_string()),
    }
    let parsed = roxmltree::Document::parse(&xml).map_err(|error| error.to_string())?;
    if parsed
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NAMESPACE, "docDefaults")))
        .flat_map(|node| node.descendants())
        .any(|node| {
            (node.has_tag_name((WORD_NAMESPACE, "rPr"))
                || node.has_tag_name((WORD_NAMESPACE, "pPr")))
                && node.children().any(|child| child.is_element())
        })
    {
        return Err(unsupported());
    }
    for style in parsed
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NAMESPACE, "style")))
    {
        let Some(id) = style.attribute((WORD_NAMESPACE, "styleId")) else {
            continue;
        };
        let used = document
            .paragraphs
            .iter()
            .any(|paragraph| paragraph.style.as_deref().unwrap_or("Normal") == id);
        if used
            && style
                .children()
                .filter(|node| node.is_element())
                .any(|node| {
                    node.has_tag_name((WORD_NAMESPACE, "basedOn"))
                        || ((node.has_tag_name((WORD_NAMESPACE, "rPr"))
                            || node.has_tag_name((WORD_NAMESPACE, "pPr")))
                            && node.children().any(|child| child.is_element()))
                })
        {
            return Err(unsupported());
        }
    }
    Ok(())
}

fn validate_numbering(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    list_ids: &[(String, String)],
) -> Result<(), String> {
    if list_ids.is_empty() {
        return Ok(());
    }
    let mut xml = String::new();
    archive
        .by_name("word/numbering.xml")
        .map_err(|_| unsupported())?
        .read_to_string(&mut xml)
        .map_err(|error| error.to_string())?;
    let parsed = roxmltree::Document::parse(&xml).map_err(|error| error.to_string())?;
    for (id, kind) in list_ids {
        let number = parsed
            .descendants()
            .find(|node| {
                node.has_tag_name((WORD_NAMESPACE, "num"))
                    && node.attribute((WORD_NAMESPACE, "numId")) == Some(id.as_str())
            })
            .ok_or_else(unsupported)?;
        if number
            .children()
            .filter(|node| node.is_element())
            .any(|node| !node.has_tag_name((WORD_NAMESPACE, "abstractNumId")))
        {
            return Err(unsupported());
        }
        let abstract_id = number
            .children()
            .find(|node| node.has_tag_name((WORD_NAMESPACE, "abstractNumId")))
            .and_then(|node| node.attribute((WORD_NAMESPACE, "val")))
            .ok_or_else(unsupported)?;
        if abstract_id != id {
            return Err(unsupported());
        }
        let abstract_number = parsed
            .descendants()
            .find(|node| {
                node.has_tag_name((WORD_NAMESPACE, "abstractNum"))
                    && node.attribute((WORD_NAMESPACE, "abstractNumId")) == Some(abstract_id)
            })
            .ok_or_else(unsupported)?;
        if abstract_number
            .children()
            .filter(|node| node.is_element())
            .any(|node| {
                node.tag_name().namespace() != Some(WORD_NAMESPACE)
                    || !matches!(node.tag_name().name(), "multiLevelType" | "lvl")
            })
        {
            return Err(unsupported());
        }
        let levels: Vec<_> = abstract_number
            .children()
            .filter(|node| node.has_tag_name((WORD_NAMESPACE, "lvl")))
            .collect();
        if levels.len() != 1 || levels[0].attribute((WORD_NAMESPACE, "ilvl")) != Some("0") {
            return Err(unsupported());
        }
        if levels[0]
            .children()
            .filter(|node| node.is_element())
            .any(|node| {
                node.tag_name().namespace() != Some(WORD_NAMESPACE)
                    || !matches!(
                        node.tag_name().name(),
                        "start" | "numFmt" | "lvlText" | "lvlJc" | "pPr"
                    )
            })
        {
            return Err(unsupported());
        }
        let value = |name| {
            levels[0]
                .children()
                .find(|node| node.has_tag_name((WORD_NAMESPACE, name)))
                .and_then(|node| node.attribute((WORD_NAMESPACE, "val")))
        };
        let expected = if kind == "bullet" {
            ("bullet", "•")
        } else {
            ("decimal", "%1.")
        };
        if value("start") != Some("1")
            || value("numFmt") != Some(expected.0)
            || value("lvlText") != Some(expected.1)
        {
            return Err(unsupported());
        }
        if value("lvlJc") != Some("left") {
            return Err(unsupported());
        }
        let properties = levels[0]
            .children()
            .find(|node| node.has_tag_name((WORD_NAMESPACE, "pPr")))
            .ok_or_else(unsupported)?;
        if properties
            .children()
            .filter(|node| node.is_element())
            .any(|node| {
                node.tag_name().namespace() != Some(WORD_NAMESPACE)
                    || !matches!(node.tag_name().name(), "tabs" | "ind")
            })
        {
            return Err(unsupported());
        }
        let indent = properties
            .children()
            .find(|node| node.has_tag_name((WORD_NAMESPACE, "ind")))
            .ok_or_else(unsupported)?;
        if indent.attribute((WORD_NAMESPACE, "left")) != Some("720")
            || indent.attribute((WORD_NAMESPACE, "hanging")) != Some("360")
            || indent.attributes().count() != 2
        {
            return Err(unsupported());
        }
        let tabs = properties
            .children()
            .find(|node| node.has_tag_name((WORD_NAMESPACE, "tabs")))
            .ok_or_else(unsupported)?;
        let tabs: Vec<_> = tabs.children().filter(|node| node.is_element()).collect();
        if tabs.len() != 1
            || !tabs[0].has_tag_name((WORD_NAMESPACE, "tab"))
            || tabs[0].attribute((WORD_NAMESPACE, "val")) != Some("num")
            || tabs[0].attribute((WORD_NAMESPACE, "pos")) != Some("720")
            || tabs[0].attributes().count() != 2
        {
            return Err(unsupported());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Paragraph, TextFormat};
    use std::io::Write;

    #[test]
    fn escapes_unicode_surrogates_control_characters_and_rtf_syntax() {
        assert_eq!(
            escape_text("Café 😀 {x}\\\n\t"),
            "Caf\\u233? \\u-10179?\\u-8704? \\{x\\}\\\\\\line \\tab "
        );
    }

    #[test]
    fn exports_representable_formatting_and_page_settings() {
        let mut document = Document::default();
        document.paragraphs = vec![Paragraph {
            alignment: Alignment::Center,
            indent: 48.0,
            line_spacing: 1.5,
            runs: vec![Run {
                text: "Text".into(),
                format: TextFormat {
                    bold: true,
                    italic: true,
                    underline: true,
                    strike: true,
                    font_family: Some("Arial".into()),
                    font_size: Some(12.0),
                    color: Some("FF0000".into()),
                    highlight: Some("00FF00".into()),
                    script: Some("superscript".into()),
                },
            }],
            ..Default::default()
        }];
        let rtf = export(&document).unwrap();
        assert!(rtf.starts_with("{\\rtf1\\ansi\\uc1"));
        assert!(rtf.contains("\\paperw12240\\paperh15840"));
        assert!(rtf.contains("\\red255\\green0\\blue0;\\red0\\green255\\blue0;"));
        assert!(rtf.contains("\\pard\\qc\\li720\\sl360\\slmult1"));
        assert!(rtf.contains("\\b\\i\\ul\\strike\\f1\\fs24\\cf1\\highlight2\\super Text"));
    }

    #[test]
    fn refuses_tables_and_unmodeled_run_properties() {
        for body in [
            r#"<w:tbl/>"#,
            r#"<w:p><w:r><w:rPr><w:shadow/></w:rPr><w:t>Text</w:t></w:r></w:p>"#,
            r#"<w:p><w:hyperlink><w:r><w:t>Link</w:t></w:r></w:hyperlink></w:p>"#,
        ] {
            let xml = format!(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
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

    #[test]
    fn refuses_unmodeled_heading_style_and_custom_list_numbering() {
        let document_xml = |body: &str| {
            format!(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
            )
        };
        let fixture = |body: &str, extra_name: &str, extra: &str| {
            let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
            let options = zip::write::SimpleFileOptions::default();
            archive.start_file("word/document.xml", options).unwrap();
            archive.write_all(document_xml(body).as_bytes()).unwrap();
            archive.start_file(extra_name, options).unwrap();
            archive.write_all(extra.as_bytes()).unwrap();
            Document::from_bytes(&archive.finish().unwrap().into_inner()).unwrap()
        };
        let heading = fixture(
            r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>"#,
            "word/styles.xml",
            r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:styleId="Heading1"><w:rPr><w:color w:val="FF0000"/></w:rPr></w:style></w:styles>"#,
        );
        assert!(export(&heading).is_err());
        let list = fixture(
            r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2147483001"/></w:numPr></w:pPr><w:r><w:t>Item</w:t></w:r></w:p>"#,
            "word/numbering.xml",
            r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="2147483001"><w:lvl w:ilvl="0"><w:start w:val="4"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1)"/></w:lvl></w:abstractNum><w:num w:numId="2147483001"><w:abstractNumId w:val="2147483001"/></w:num></w:numbering>"#,
        );
        assert!(export(&list).is_err());
    }
}
