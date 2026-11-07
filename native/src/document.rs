use std::fmt;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::path::Path;

const WORD_NAMESPACE: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const MAX_PACKAGE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextFormat {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub color: Option<String>,
    pub highlight: Option<String>,
    pub script: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub text: String,
    pub format: TextFormat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabStop {
    pub position: f32,
    pub alignment: String,
    pub leader: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Paragraph {
    pub runs: Vec<Run>,
    pub alignment: Alignment,
    pub style: Option<String>,
    pub indent: f32,
    pub line_spacing: f32,
    pub space_before: f32,
    pub space_after: f32,
    pub list: Option<String>,
    pub bidi: bool,
    pub indent_right: f32,
    pub special: Option<String>,
    pub special_amount: f32,
    pub line_rule: Option<String>,
    pub keep_next: bool,
    pub keep_lines: bool,
    pub widow_control: bool,
    pub page_break_before: bool,
    pub contextual_spacing: bool,
    pub tabs: Vec<TabStop>,
}

impl Default for Paragraph {
    fn default() -> Self {
        Self {
            runs: Vec::new(),
            alignment: Alignment::default(),
            style: None,
            indent: 0.0,
            line_spacing: 0.0,
            space_before: 0.0,
            space_after: 0.0,
            list: None,
            bidi: false,
            indent_right: 0.0,
            special: None,
            special_amount: 0.0,
            line_rule: None,
            keep_next: false,
            keep_lines: false,
            widow_control: true,
            page_break_before: false,
            contextual_spacing: false,
            tabs: Vec::new(),
        }
    }
}

impl Paragraph {
    pub fn plain_text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum BodyElement {
    Paragraph { original: Paragraph, xml: String },
    Preserved(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PageSetup {
    pub width: f32,
    pub height: f32,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            width: 816.0,
            height: 1056.0,
            margin_top: 96.0,
            margin_right: 96.0,
            margin_bottom: 96.0,
            margin_left: 96.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub paragraphs: Vec<Paragraph>,
    pub page_setup: PageSetup,
    original_page_setup: PageSetup,
    package: Vec<(String, Vec<u8>)>,
    body: Vec<BodyElement>,
    document_prefix: String,
    document_suffix: String,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            paragraphs: vec![Paragraph::default()],
            page_setup: PageSetup::default(),
            original_page_setup: PageSetup::default(),
            package: vec![
                ("[Content_Types].xml".into(), br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#.to_vec()),
                ("_rels/.rels".into(), br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_vec()),
            ],
            body: Vec::new(),
            document_prefix: format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><w:document xmlns:w=\"{WORD_NAMESPACE}\"><w:body>"),
            document_suffix: "</w:body></w:document>".into(),
        }
    }
}

#[derive(Debug)]
pub struct DocumentError(String);

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DocumentError {}

impl Document {
    pub fn open(path: &Path) -> Result<Self, DocumentError> {
        let bytes = crate::backend::files::read_document(path).map_err(DocumentError)?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DocumentError> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|error| DocumentError(format!("Could Not Read The DOCX Package. {error}")))?;
        let mut package = Vec::new();
        let mut total_bytes: u64 = 0;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| DocumentError(format!("Could Not Read A DOCX Part. {error}")))?;
            if entry.is_dir() {
                continue;
            }
            total_bytes = total_bytes.saturating_add(entry.size());
            if total_bytes > MAX_PACKAGE_BYTES {
                return Err(DocumentError(
                    "The Expanded Document Exceeds The 128 MB Limit.".into(),
                ));
            }
            let name = entry
                .name()
                .map_err(|error| {
                    DocumentError(format!("Could Not Read A DOCX Part Name. {error}"))
                })?
                .into_owned();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(|error| {
                DocumentError(format!("Could Not Decompress A DOCX Part. {error}"))
            })?;
            package.push((name, data));
        }
        let source = package
            .iter()
            .find(|(name, _)| name == "word/document.xml")
            .ok_or_else(|| DocumentError("The DOCX Package Has No Main Document.".into()))?;
        let xml = std::str::from_utf8(&source.1)
            .map_err(|error| DocumentError(format!("The Document XML Is Not UTF-8. {error}")))?;
        let parsed = roxmltree::Document::parse(xml)
            .map_err(|error| DocumentError(format!("Could Not Parse The Document XML. {error}")))?;
        let body_node = parsed
            .descendants()
            .find(|node| node.has_tag_name((WORD_NAMESPACE, "body")))
            .ok_or_else(|| DocumentError("The DOCX Package Has No Document Body.".into()))?;
        let body_range = body_node.range();
        let open_end = xml[body_range.clone()]
            .find('>')
            .map(|offset| body_range.start + offset + 1)
            .ok_or_else(|| DocumentError("The Document Body Is Invalid.".into()))?;
        let close_start = xml[body_range.clone()]
            .rfind("</")
            .map(|offset| body_range.start + offset)
            .unwrap_or(open_end);
        let self_closing = xml[body_range.start..open_end].ends_with("/>");
        let numbering = parse_numbering(&package);
        let mut paragraphs = Vec::new();
        let mut body = Vec::new();
        for node in body_node.children().filter(|node| node.is_element()) {
            let raw = xml[node.range()].to_string();
            if node.has_tag_name((WORD_NAMESPACE, "p")) {
                let mut paragraph = parse_paragraph(node);
                if paragraph.list.is_none() {
                    paragraph.list = paragraph_list(node, &numbering);
                }
                body.push(BodyElement::Paragraph {
                    original: paragraph.clone(),
                    xml: raw,
                });
                paragraphs.push(paragraph);
            } else {
                body.push(BodyElement::Preserved(raw));
            }
        }
        if paragraphs.is_empty() {
            let paragraph = Paragraph::default();
            let position = body.iter().position(|element| matches!(element, BodyElement::Preserved(xml) if xml.contains("sectPr"))).unwrap_or(body.len());
            body.insert(
                position,
                BodyElement::Paragraph {
                    original: paragraph.clone(),
                    xml: paragraph_xml(&paragraph),
                },
            );
            paragraphs.push(paragraph);
        }
        // Generated paragraphs use the standard w prefix even if the source uses another prefix.
        let mut document_prefix = if self_closing {
            format!("{}>", &xml[..open_end - 2])
        } else {
            xml[..open_end].to_string()
        };
        if body_node.lookup_namespace_uri(Some("w")) != Some(WORD_NAMESPACE) {
            let insertion = document_prefix.rfind('>').unwrap_or(document_prefix.len());
            document_prefix.insert_str(insertion, &format!(" xmlns:w=\"{WORD_NAMESPACE}\""));
        }
        let document_suffix = if self_closing {
            let body_name = xml[body_range.start + 1..open_end]
                .split([' ', '\t', '\r', '\n', '/', '>'])
                .next()
                .unwrap_or("w:body");
            format!("</{body_name}>{}", &xml[body_range.end..])
        } else {
            xml[close_start..].to_string()
        };
        let page_setup = parse_page_setup(&parsed);
        Ok(Self {
            page_setup: page_setup.clone(),
            original_page_setup: page_setup,
            paragraphs,
            package,
            body,
            document_prefix,
            document_suffix,
        })
    }

    pub fn save(&mut self, path: &Path) -> Result<(), DocumentError> {
        let bytes = self.to_bytes()?;
        crate::backend::files::write_document(path, &bytes).map_err(DocumentError)
    }

    pub fn has_unrepresented_body_content(&self) -> bool {
        self.body.iter().any(|element| {
            let BodyElement::Preserved(xml) = element else {
                return false;
            };
            let source = format!("{}{}{}", self.document_prefix, xml, self.document_suffix);
            let Ok(parsed) = roxmltree::Document::parse(&source) else {
                return true;
            };
            let Some(body) = parsed.descendants().find(|node| node.has_tag_name((WORD_NAMESPACE, "body"))) else {
                return true;
            };
            !body.children().filter(|node| node.is_element())
                .all(|node| node.has_tag_name((WORD_NAMESPACE, "sectPr")))
        })
    }

    pub fn can_replace_all_content(&self) -> Result<(), String> {
        if self.has_unrepresented_body_content() {
            return Err("The Document Contains Content That Cannot Yet Be Replaced Safely.".into());
        }
        for element in &self.body {
            if let BodyElement::Paragraph { xml, .. } = element {
                let source = format!("{}{}{}", self.document_prefix, xml, self.document_suffix);
                if !can_rewrite_paragraph(&source)
                    || has_unmodeled_run_properties(&source)
                    || has_unmodeled_paragraph_properties(&source)
                {
                    return Err("The Document Contains Content That Cannot Yet Be Replaced Safely.".into());
                }
            }
        }
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, DocumentError> {
        let source_paragraphs = self
            .body
            .iter()
            .filter(|element| matches!(element, BodyElement::Paragraph { .. }))
            .count();
        if source_paragraphs > 0 && source_paragraphs != self.paragraphs.len() {
            return Err(DocumentError("The Document Structure Changed Without Preserving Its Original Content. Undo The Structural Edit Before Saving.".into()));
        }
        let mut document_xml = self.document_prefix.clone();
        let mut paragraph_index = 0;
        for element in &self.body {
            match element {
                BodyElement::Paragraph { original, xml } => {
                    if let Some(paragraph) = self.paragraphs.get(paragraph_index) {
                        if paragraph == original {
                            document_xml.push_str(xml);
                        } else {
                            let source =
                                format!("{}{}{}", self.document_prefix, xml, self.document_suffix);
                            if !can_rewrite_paragraph(&source) {
                                return Err(DocumentError("This Paragraph Contains Content The Native Editor Cannot Yet Rewrite Safely. Undo Its Changes Before Saving.".into()));
                            }
                            if has_unmodeled_run_properties(&source) {
                                return Err(DocumentError("This Paragraph Contains Run Formatting That Cannot Yet Be Preserved Safely. Undo Its Changes Before Saving.".into()));
                            }
                            document_xml
                                .push_str(&paragraph_xml_with_numbering(paragraph, Some(&source), paragraph.list == original.list));
                        }
                    }
                    paragraph_index += 1;
                }
                BodyElement::Preserved(xml) => {
                    // Section properties must remain the final body element.
                    if xml.contains("sectPr") {
                        while let Some(paragraph) = self.paragraphs.get(paragraph_index) {
                            document_xml.push_str(&paragraph_xml(paragraph));
                            paragraph_index += 1;
                        }
                    }
                    document_xml.push_str(xml);
                }
            }
        }
        while let Some(paragraph) = self.paragraphs.get(paragraph_index) {
            document_xml.push_str(&paragraph_xml(paragraph));
            paragraph_index += 1;
        }
        document_xml.push_str(&self.document_suffix);
        if self.page_setup != self.original_page_setup {
            document_xml = update_page_setup(&document_xml, &self.page_setup)?;
        }

        roxmltree::Document::parse(&document_xml).map_err(|error| {
            DocumentError(format!(
                "The Edited Document Contains Invalid XML Content. Remove Unsupported Control Characters And Retry. {error}"
            ))
        })?;
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let package = self.package_with_numbering()?;
        for (name, data) in &package {
            if name == "word/document.xml" {
                continue;
            }
            writer
                .start_file(name, options)
                .map_err(|error| DocumentError(format!("Could Not Create A DOCX Part. {error}")))?;
            writer
                .write_all(data)
                .map_err(|error| DocumentError(format!("Could Not Write A DOCX Part. {error}")))?;
        }
        writer
            .start_file("word/document.xml", options)
            .map_err(|error| {
                DocumentError(format!("Could Not Create The Document XML. {error}"))
            })?;
        writer
            .write_all(document_xml.as_bytes())
            .map_err(|error| DocumentError(format!("Could Not Write The Document XML. {error}")))?;
        writer
            .finish()
            .map(|cursor| cursor.into_inner())
            .map_err(|error| DocumentError(format!("Could Not Finish The DOCX Package. {error}")))
    }

    pub fn split_paragraph(
        &mut self,
        index: usize,
        first: Paragraph,
        second: Paragraph,
    ) -> Result<(), String> {
        if index >= self.paragraphs.len() {
            return Err("The Paragraph Is No Longer Available.".into());
        }
        let location = self
            .body
            .iter()
            .enumerate()
            .filter(|(_, element)| matches!(element, BodyElement::Paragraph { .. }))
            .nth(index)
            .map(|(index, _)| index);
        if let Some(location) = location {
            let BodyElement::Paragraph { xml, original } = &self.body[location] else {
                unreachable!()
            };
            let source = format!("{}{}{}", self.document_prefix, xml, self.document_suffix);
            if !can_rewrite_paragraph(&source)
                || has_unmodeled_run_properties(&source)
                || has_unmodeled_paragraph_properties(&source)
            {
                return Err(
                    "This Paragraph Contains Content That Cannot Yet Be Split Safely.".into(),
                );
            }
            let second_xml = paragraph_xml_with_numbering(&second, Some(&source), original.list == second.list);
            self.body.insert(
                location + 1,
                BodyElement::Paragraph {
                    original: second.clone(),
                    xml: second_xml,
                },
            );
        } else if !self.body.is_empty() {
            return Err("The Document Structure Cannot Be Changed Safely.".into());
        }
        self.paragraphs[index] = first;
        self.paragraphs.insert(index + 1, second);
        Ok(())
    }

    pub fn merge_paragraphs(
        &mut self,
        first_index: usize,
        merged: Paragraph,
    ) -> Result<(), String> {
        if first_index + 1 >= self.paragraphs.len() {
            return Err("The Paragraph Is No Longer Available.".into());
        }
        let locations: Vec<usize> = self
            .body
            .iter()
            .enumerate()
            .filter_map(|(index, element)| {
                matches!(element, BodyElement::Paragraph { .. }).then_some(index)
            })
            .collect();
        if !self.body.is_empty() {
            let (Some(&first), Some(&second)) =
                (locations.get(first_index), locations.get(first_index + 1))
            else {
                return Err("The Document Structure Cannot Be Changed Safely.".into());
            };
            if second != first + 1 {
                return Err(
                    "Paragraphs Separated By Other Document Content Cannot Be Merged.".into(),
                );
            }
            for location in [first, second] {
                let BodyElement::Paragraph { xml, .. } = &self.body[location] else {
                    unreachable!()
                };
                let source = format!("{}{}{}", self.document_prefix, xml, self.document_suffix);
                if !can_rewrite_paragraph(&source)
                    || has_unmodeled_run_properties(&source)
                    || has_unmodeled_paragraph_properties(&source)
                {
                    return Err(
                        "These Paragraphs Contain Content That Cannot Yet Be Merged Safely.".into(),
                    );
                }
            }
            self.body.remove(second);
        }
        self.paragraphs[first_index] = merged;
        self.paragraphs.remove(first_index + 1);
        Ok(())
    }

    fn package_with_numbering(&self) -> Result<Vec<(String, Vec<u8>)>, DocumentError> {
        let mut package = self.package.clone();
        let mut index = 0;
        let mut needs_numbering = false;
        for element in &self.body {
            if let BodyElement::Paragraph { original, .. } = element {
                if let Some(paragraph) = self.paragraphs.get(index) {
                    needs_numbering |= paragraph.list.is_some() && paragraph.list != original.list;
                }
                index += 1;
            }
        }
        needs_numbering |= self.paragraphs.iter().skip(index).any(|paragraph| paragraph.list.is_some());
        if !needs_numbering {
            return Ok(package);
        }
        let part = "word/numbering.xml";
        let existing = package
            .iter()
            .find(|(name, _)| name == part)
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned());
        let mut numbering = existing
            .unwrap_or_else(|| format!("<w:numbering xmlns:w=\"{WORD_NAMESPACE}\"></w:numbering>"));
        numbering = ensure_word_namespace(&numbering)?;
        let parsed = roxmltree::Document::parse(&numbering)
            .map_err(|error| DocumentError(format!("Could Not Read List Definitions. {error}")))?;
        let root = parsed.root_element();
        let mut abstract_additions = String::new();
        let mut number_additions = String::new();
        for (identifier, kind, label) in [
            (2147483000_u32, "bullet", "\u{2022}"),
            (2147483001, "decimal", "%1."),
        ] {
            let identifier_text = identifier.to_string();
            if let Some(number) = root.children().find(|node| {
                node.attribute((WORD_NAMESPACE, "numId")) == Some(identifier_text.as_str())
            }) {
                let abstract_identifier =
                    child(number, "abstractNumId").and_then(|node| attribute(node, "val"));
                let compatible = abstract_identifier
                    .as_deref()
                    .and_then(|identifier| {
                        root.children().find(|node| {
                            node.attribute((WORD_NAMESPACE, "abstractNumId")) == Some(identifier)
                        })
                    })
                    .and_then(|node| child(node, "lvl"))
                    .and_then(|node| child(node, "numFmt"))
                    .and_then(|node| attribute(node, "val"))
                    .is_some_and(|format| format == kind);
                if compatible {
                    continue;
                }
                return Err(DocumentError("This Document Uses A Conflicting Native List Identifier. Existing Lists Have Not Been Changed.".into()));
            }
            if root.children().any(|node| {
                node.attribute((WORD_NAMESPACE, "abstractNumId")) == Some(identifier_text.as_str())
            }) {
                return Err(DocumentError("This Document Uses A Reserved Native List Identifier. Save A Copy Without Changing Lists.".into()));
            }
            abstract_additions.push_str(&format!("<w:abstractNum w:abstractNumId=\"{identifier}\"><w:multiLevelType w:val=\"singleLevel\"/><w:lvl w:ilvl=\"0\"><w:start w:val=\"1\"/><w:numFmt w:val=\"{kind}\"/><w:lvlText w:val=\"{label}\"/><w:lvlJc w:val=\"left\"/><w:pPr><w:tabs><w:tab w:val=\"num\" w:pos=\"720\"/></w:tabs><w:ind w:left=\"720\" w:hanging=\"360\"/></w:pPr></w:lvl></w:abstractNum>"));
            number_additions.push_str(&format!(
                "<w:num w:numId=\"{identifier}\"><w:abstractNumId w:val=\"{identifier}\"/></w:num>"
            ));
        }
        let first_number = root
            .children()
            .find(|node| node.has_tag_name((WORD_NAMESPACE, "num")))
            .map(|node| node.range().start);
        if let Some(insertion) = first_number {
            numbering.insert_str(insertion, &abstract_additions);
        } else {
            numbering = append_xml_children(&numbering, &abstract_additions)?;
        }
        numbering = append_xml_children(&numbering, &number_additions)?;
        upsert_part(&mut package, part, numbering.into_bytes());
        let rel_part = "word/_rels/document.xml.rels";
        let mut relationships = package.iter().find(|(name, _)| name == rel_part).map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned()).unwrap_or_else(|| "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"></Relationships>".into());
        if !relationships.contains("relationships/numbering") {
            let parsed = roxmltree::Document::parse(&relationships)
                .map_err(|error| DocumentError(error.to_string()))?;
            let mut identifier = 1;
            while parsed.root_element().children().any(|node| {
                node.attribute("Id") == Some(format!("rIdNativeList{identifier}").as_str())
            }) {
                identifier += 1;
            }
            relationships = append_xml_children(&relationships, &format!("<Relationship Id=\"rIdNativeList{identifier}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/>"))?;
            upsert_part(&mut package, rel_part, relationships.into_bytes());
        }
        if let Some((_, bytes)) = package
            .iter_mut()
            .find(|(name, _)| name == "[Content_Types].xml")
        {
            let mut content_types = String::from_utf8_lossy(bytes).into_owned();
            if !content_types.contains("/word/numbering.xml") {
                content_types = append_xml_children(&content_types, "<Override PartName=\"/word/numbering.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml\"/>")?;
                *bytes = content_types.into_bytes();
            }
        }
        Ok(package)
    }

    pub fn plain_text(&self) -> String {
        self.paragraphs
            .iter()
            .map(Paragraph::plain_text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn replace_plain_text(&mut self, text: &str) {
        self.paragraphs = text
            .split('\n')
            .map(|line| Paragraph {
                runs: vec![Run {
                    text: line.into(),
                    format: TextFormat::default(),
                }],
                ..Paragraph::default()
            })
            .collect();
    }
}

fn preserved_attributes(node: Option<roxmltree::Node<'_, '_>>, excluded: &[&str]) -> String {
    node.map(|node| {
        node.attributes()
            .filter(|attribute| !excluded.contains(&attribute.name()))
            .map(|attribute| {
                let prefix = if attribute.namespace() == Some(WORD_NAMESPACE) {
                    "w:"
                } else {
                    ""
                };
                format!(
                    " {prefix}{}=\"{}\"",
                    attribute.name(),
                    escape_xml(attribute.value())
                )
            })
            .collect()
    })
    .unwrap_or_default()
}

fn ensure_word_namespace(xml: &str) -> Result<String, DocumentError> {
    let parsed =
        roxmltree::Document::parse(xml).map_err(|error| DocumentError(error.to_string()))?;
    let root = parsed.root_element();
    if root.lookup_namespace_uri(Some("w")) == Some(WORD_NAMESPACE) {
        return Ok(xml.to_owned());
    }
    let mut result = xml.to_owned();
    let opening_end = root.range().start
        + xml[root.range()]
            .find('>')
            .ok_or_else(|| DocumentError("The XML Root Is Invalid.".into()))?;
    let insertion = if xml.as_bytes()[opening_end - 1] == b'/' {
        opening_end - 1
    } else {
        opening_end
    };
    result.insert_str(insertion, &format!(" xmlns:w=\"{WORD_NAMESPACE}\""));
    Ok(result)
}

fn append_xml_children(xml: &str, children: &str) -> Result<String, DocumentError> {
    let parsed =
        roxmltree::Document::parse(xml).map_err(|error| DocumentError(error.to_string()))?;
    let root = parsed.root_element();
    let range = root.range();
    let source = &xml[range.clone()];
    let mut result = xml.to_string();
    if source.ends_with("/>") {
        let name = source[1..]
            .split([' ', '\t', '\r', '\n', '/', '>'])
            .next()
            .unwrap_or("root");
        result.replace_range(range.end - 2..range.end, &format!(">{children}</{name}>"));
    } else {
        let insertion = xml[..range.end]
            .rfind("</")
            .ok_or_else(|| DocumentError("The XML Root Is Invalid.".into()))?;
        result.insert_str(insertion, children);
    }
    Ok(result)
}

fn upsert_part(package: &mut Vec<(String, Vec<u8>)>, name: &str, bytes: Vec<u8>) {
    if let Some((_, existing)) = package.iter_mut().find(|(part, _)| part == name) {
        *existing = bytes;
    } else {
        package.push((name.into(), bytes));
    }
}

fn parse_page_setup(parsed: &roxmltree::Document<'_>) -> PageSetup {
    let mut setup = PageSetup::default();
    let Some(section) = parsed
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NAMESPACE, "sectPr")))
        .last()
    else {
        return setup;
    };
    if let Some(size) = child(section, "pgSz") {
        if let Some(value) = attribute(size, "w").and_then(|value| value.parse::<f32>().ok()) {
            setup.width = value / 15.0;
        }
        if let Some(value) = attribute(size, "h").and_then(|value| value.parse::<f32>().ok()) {
            setup.height = value / 15.0;
        }
    }
    if let Some(margin) = child(section, "pgMar") {
        for (name, target) in [
            ("top", &mut setup.margin_top),
            ("right", &mut setup.margin_right),
            ("bottom", &mut setup.margin_bottom),
            ("left", &mut setup.margin_left),
        ] {
            if let Some(value) = attribute(margin, name).and_then(|value| value.parse::<f32>().ok())
            {
                *target = value / 15.0;
            }
        }
    }
    setup
}

fn update_page_setup(xml: &str, setup: &PageSetup) -> Result<String, DocumentError> {
    if ![
        setup.width,
        setup.height,
        setup.margin_top,
        setup.margin_right,
        setup.margin_bottom,
        setup.margin_left,
    ]
    .iter()
    .all(|value| value.is_finite() && *value >= 0.0)
        || setup.width < setup.margin_left + setup.margin_right + 96.0
        || setup.height < setup.margin_top + setup.margin_bottom + 96.0
    {
        return Err(DocumentError(
            "Page Margins Must Leave At Least One Inch Of Editable Space.".into(),
        ));
    }
    let parsed =
        roxmltree::Document::parse(xml).map_err(|error| DocumentError(error.to_string()))?;
    let section = parsed
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NAMESPACE, "sectPr")))
        .last();
    let mut replacement = format!("<w:sectPr><w:pgSz w:w=\"{}\" w:h=\"{}\"/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/>", (setup.width*15.0).round() as u32, (setup.height*15.0).round() as u32, (setup.margin_top*15.0).round() as u32, (setup.margin_right*15.0).round() as u32, (setup.margin_bottom*15.0).round() as u32, (setup.margin_left*15.0).round() as u32);
    if let Some(section) = section {
        for property in section.children().filter(|node| node.is_element()) {
            if !matches!(property.tag_name().name(), "pgSz" | "pgMar") {
                replacement.push_str(&xml[property.range()]);
            }
        }
    }
    replacement.push_str("</w:sectPr>");
    let mut result = xml.to_string();
    if let Some(section) = section {
        result.replace_range(section.range(), &replacement);
    } else {
        let body = parsed
            .descendants()
            .find(|node| node.has_tag_name((WORD_NAMESPACE, "body")))
            .ok_or_else(|| DocumentError("Document Body Is Missing.".into()))?;
        let insertion = xml[..body.range().end]
            .rfind("</")
            .ok_or_else(|| DocumentError("Document Body Is Invalid.".into()))?;
        result.insert_str(insertion, &replacement);
    }
    Ok(result)
}

fn child<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children()
        .find(|child| child.has_tag_name((WORD_NAMESPACE, name)))
}

fn attribute(node: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    node.attribute((WORD_NAMESPACE, name)).map(str::to_string)
}

fn enabled(node: roxmltree::Node<'_, '_>, name: &str) -> bool {
    child(node, name).is_some_and(|value| {
        !matches!(
            value.attribute((WORD_NAMESPACE, "val")),
            Some("0" | "false" | "off" | "none")
        )
    })
}

fn parse_numbering(package: &[(String, Vec<u8>)]) -> HashMap<(String, String), String> {
    let mut result = HashMap::new();
    let Some((_, bytes)) = package.iter().find(|(name, _)| name == "word/numbering.xml") else {
        return result;
    };
    let Ok(xml) = std::str::from_utf8(bytes) else { return result; };
    let Ok(parsed) = roxmltree::Document::parse(xml) else { return result; };
    let root = parsed.root_element();
    for number in root.children().filter(|node| node.has_tag_name((WORD_NAMESPACE, "num"))) {
        let Some(id) = attribute(number, "numId") else { continue; };
        let Some(abstract_id) = child(number, "abstractNumId").and_then(|node| attribute(node, "val")) else { continue; };
        let Some(definition) = root.children().find(|node| node.has_tag_name((WORD_NAMESPACE, "abstractNum")) && attribute(*node, "abstractNumId").as_deref() == Some(abstract_id.as_str())) else { continue; };
        for level in definition.children().filter(|node| node.has_tag_name((WORD_NAMESPACE, "lvl"))) {
            let Some(level_id) = attribute(level, "ilvl") else { continue; };
            let Some(format) = child(level, "numFmt").and_then(|node| attribute(node, "val")) else { continue; };
            let kind = match format.as_str() { "bullet" => "bullet", "decimal" => "number", _ => continue };
            if number.children().any(|node| node.has_tag_name((WORD_NAMESPACE, "lvlOverride")) && attribute(node, "ilvl").as_deref() == Some(level_id.as_str())) {
                continue;
            }
            result.insert((id.clone(), level_id), kind.into());
        }
    }
    result
}

fn paragraph_list(node: roxmltree::Node<'_, '_>, numbering: &HashMap<(String, String), String>) -> Option<String> {
    let properties = child(node, "pPr").and_then(|node| child(node, "numPr"))?;
    let id = child(properties, "numId").and_then(|node| attribute(node, "val"))?;
    let level = child(properties, "ilvl").and_then(|node| attribute(node, "val")).unwrap_or_else(|| "0".into());
    numbering.get(&(id, level)).cloned()
}

fn parse_paragraph(node: roxmltree::Node<'_, '_>) -> Paragraph {
    let mut paragraph = Paragraph::default();
    if let Some(properties) = child(node, "pPr") {
        paragraph.style = child(properties, "pStyle").and_then(|value| attribute(value, "val"));
        paragraph.indent = child(properties, "ind")
            .and_then(|value| attribute(value, "left"))
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.0)
            / 15.0;
        paragraph.line_spacing = child(properties, "spacing")
            .and_then(|value| attribute(value, "line"))
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.0)
            / 240.0;
        if let Some(ind) = child(properties, "ind") {
            paragraph.indent_right = attribute(ind, "right")
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(0.0)
                / 15.0;
            if let Some(amount) = attribute(ind, "firstLine")
                .and_then(|value| value.parse::<f32>().ok())
            {
                paragraph.special = Some("firstLine".into());
                paragraph.special_amount = amount / 15.0;
            } else if let Some(amount) = attribute(ind, "hanging")
                .and_then(|value| value.parse::<f32>().ok())
            {
                paragraph.special = Some("hanging".into());
                paragraph.special_amount = amount / 15.0;
            }
        }
        if let Some(spacing) = child(properties, "spacing") {
            paragraph.space_before = attribute(spacing, "before")
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(0.0)
                / 20.0;
            paragraph.space_after = attribute(spacing, "after")
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(0.0)
                / 20.0;
            let rule = attribute(spacing, "lineRule");
            let line = attribute(spacing, "line")
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(0.0);
            if matches!(rule.as_deref(), Some("atLeast" | "exact")) && line > 0.0 {
                paragraph.line_rule = rule;
                paragraph.line_spacing = line / 20.0;
            } else {
                paragraph.line_spacing = line / 240.0;
            }
        }
        paragraph.bidi = child(properties, "bidi").is_some();
        paragraph.keep_next = child(properties, "keepNext").is_some();
        paragraph.keep_lines = child(properties, "keepLines").is_some();
        paragraph.page_break_before = child(properties, "pageBreakBefore").is_some();
        paragraph.contextual_spacing = child(properties, "contextualSpacing").is_some();
        if let Some(widow) = child(properties, "widowControl") {
            paragraph.widow_control = attribute(widow, "val")
                .map(|value| !matches!(value.as_str(), "0" | "false" | "off" | "none"))
                .unwrap_or(true);
        }
        if let Some(tabs) = child(properties, "tabs") {
            paragraph.tabs = tabs
                .children()
                .filter(|value| value.has_tag_name((WORD_NAMESPACE, "tab")))
                .map(|tab| TabStop {
                    position: attribute(tab, "pos")
                        .and_then(|value| value.parse::<f32>().ok())
                        .unwrap_or(0.0)
                        / 15.0,
                    alignment: attribute(tab, "val").unwrap_or_else(|| "left".into()),
                    leader: attribute(tab, "leader").unwrap_or_else(|| "none".into()),
                })
                .collect();
        }
        paragraph.list = child(properties, "numPr")
            .and_then(|value| child(value, "numId"))
            .and_then(|value| attribute(value, "val"))
            .and_then(|identifier| match identifier.as_str() {
                "2147483000" => Some("bullet".into()),
                "2147483001" => Some("number".into()),
                _ => None,
            });
        paragraph.alignment = match child(properties, "jc")
            .and_then(|value| attribute(value, "val"))
            .as_deref()
        {
            Some("center") => Alignment::Center,
            Some("right" | "end") => Alignment::Right,
            Some("both" | "distribute") => Alignment::Justify,
            _ => Alignment::Left,
        };
    }
    for run_node in node
        .descendants()
        .filter(|value| value.has_tag_name((WORD_NAMESPACE, "r")))
    {
        let mut format = TextFormat::default();
        if let Some(properties) = child(run_node, "rPr") {
            format.bold = enabled(properties, "b");
            format.italic = enabled(properties, "i");
            format.underline = enabled(properties, "u");
            format.strike = enabled(properties, "strike");
            format.font_family = child(properties, "rFonts")
                .and_then(|value| attribute(value, "ascii").or_else(|| attribute(value, "hAnsi")));
            format.font_size = child(properties, "sz")
                .and_then(|value| attribute(value, "val"))
                .and_then(|value| value.parse::<f32>().ok())
                .map(|size| size / 2.0);
            format.color = child(properties, "color").and_then(|value| attribute(value, "val"));
            format.highlight = child(properties, "shd")
                .and_then(|value| attribute(value, "fill"))
                .or_else(|| {
                    child(properties, "highlight")
                        .and_then(|value| attribute(value, "val"))
                        .map(|name| {
                            if name == "yellow" {
                                "FFFF00".into()
                            } else {
                                name
                            }
                        })
                });
            format.script =
                child(properties, "vertAlign").and_then(|value| attribute(value, "val"));
        }
        let mut text = String::new();
        for content in run_node.children().filter(|value| value.is_element()) {
            match content.tag_name().name() {
                "t" if content.tag_name().namespace() == Some(WORD_NAMESPACE) => {
                    text.push_str(content.text().unwrap_or(""))
                }
                "tab" => text.push('\t'),
                "br" | "cr" => text.push('\n'),
                _ => {}
            }
        }
        paragraph.runs.push(Run { text, format });
    }
    paragraph
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn paragraph_xml(paragraph: &Paragraph) -> String {
    paragraph_xml_with_source(paragraph, None)
}

fn has_unmodeled_paragraph_properties(xml: &str) -> bool {
    let wrapped = wrap_source(xml);
    let Ok(parsed) = roxmltree::Document::parse(&wrapped) else {
        return true;
    };
    let Some(paragraph) = parsed.descendants().find(|node| node.has_tag_name((WORD_NAMESPACE, "p"))) else {
        return true;
    };
    if paragraph.attributes().len() != 0 {
        return true;
    }
    let Some(properties) = child(paragraph, "pPr") else {
        return false;
    };
    if properties.attributes().len() != 0 {
        return true;
    }
    properties.children().filter(|node| node.is_element()).any(|node| {
        if node.tag_name().namespace() != Some(WORD_NAMESPACE) {
            return true;
        }
        let allowed = match node.tag_name().name() {
            "pStyle" | "jc" => &["val"][..],
            "ind" => &["left", "start"][..],
            "spacing" => &["line", "lineRule"][..],
            "numPr" => {
                return node.attributes().len() != 0
                    || node.children().filter(|child| child.is_element()).any(|child| {
                        child.tag_name().namespace() != Some(WORD_NAMESPACE)
                            || !matches!(child.tag_name().name(), "ilvl" | "numId")
                            || child.attributes().any(|attribute| {
                                attribute.namespace() != Some(WORD_NAMESPACE) || attribute.name() != "val"
                            })
                            || child.children().any(|child| child.is_element())
                    });
            }
            _ => return true,
        };
        node.attributes().any(|attribute| {
            attribute.namespace() != Some(WORD_NAMESPACE) || !allowed.contains(&attribute.name())
        }) || node.children().any(|child| child.is_element())
    })
}

fn has_unmodeled_run_properties(xml: &str) -> bool {
    let wrapped = wrap_source(xml);
    let Ok(parsed) = roxmltree::Document::parse(&wrapped) else {
        return true;
    };
    let Some(paragraph) = parsed
        .descendants()
        .find(|node| node.has_tag_name((WORD_NAMESPACE, "p")))
    else {
        return true;
    };
    paragraph
        .children()
        .filter(|node| node.has_tag_name((WORD_NAMESPACE, "r")))
        .any(|run| {
            if run.attributes().len() != 0 {
                return true;
            }
            let Some(properties) = child(run, "rPr") else {
                return false;
            };
            if properties.attributes().len() != 0 {
                return true;
            }
            properties
                .children()
                .filter(|node| node.is_element())
                .any(|property| {
                    if property.tag_name().namespace() != Some(WORD_NAMESPACE) {
                        return true;
                    }
                    let allowed: &[&str] = match property.tag_name().name() {
                        "b" | "i" | "u" | "strike" | "sz" | "color" | "highlight" | "vertAlign" => {
                            &["val"]
                        }
                        "rFonts" => &["ascii", "hAnsi"],
                        "shd" => &["val", "fill"],
                        _ => return true,
                    };
                    if matches!(property.tag_name().name(), "b" | "i" | "u" | "strike")
                        && matches!(
                            property.attribute((WORD_NAMESPACE, "val")),
                            Some("0" | "false" | "off" | "none")
                        )
                    {
                        return true;
                    }
                    if property.tag_name().name() == "rFonts"
                        && property.attribute((WORD_NAMESPACE, "ascii"))
                            != property.attribute((WORD_NAMESPACE, "hAnsi"))
                    {
                        return true;
                    }
                    property.attributes().any(|attribute| {
                        attribute.namespace() != Some(WORD_NAMESPACE)
                            || !allowed.contains(&attribute.name())
                    }) || property.children().any(|node| node.is_element())
                })
        })
}

fn can_rewrite_paragraph(xml: &str) -> bool {
    let wrapped = wrap_source(xml);
    let Ok(parsed) = roxmltree::Document::parse(&wrapped) else {
        return false;
    };
    let Some(paragraph) = parsed
        .descendants()
        .find(|node| node.has_tag_name((WORD_NAMESPACE, "p")))
    else {
        return false;
    };
    if paragraph.attributes().any(|attribute| {
        attribute.namespace().is_some_and(|namespace| namespace != WORD_NAMESPACE)
    }) || child(paragraph, "pPr").is_some_and(|properties| {
        properties.attributes().any(|attribute| {
            attribute.namespace().is_some_and(|namespace| namespace != WORD_NAMESPACE)
        })
    }) {
        return false;
    }
    paragraph
        .children()
        .filter(|node| node.is_element())
        .all(|node| {
            if node.has_tag_name((WORD_NAMESPACE, "pPr")) {
                return true;
            }
            if !node.has_tag_name((WORD_NAMESPACE, "r")) {
                return false;
            }
            node.children()
                .filter(|node| node.is_element())
                .all(|content| {
                    content.tag_name().namespace() == Some(WORD_NAMESPACE)
                        && matches!(content.tag_name().name(), "rPr" | "t" | "tab" | "br" | "cr")
                        && (content.tag_name().name() != "br"
                            || content
                                .attribute((WORD_NAMESPACE, "type"))
                                .is_none_or(|kind| kind == "textWrapping"))
                })
        })
}

fn wrap_source(xml: &str) -> String {
    if xml.starts_with("<?xml") || xml.contains(":document") {
        return xml.to_string();
    }
    format!("<root xmlns:w=\"{WORD_NAMESPACE}\">{xml}</root>")
}

fn paragraph_xml_with_source(paragraph: &Paragraph, source: Option<&str>) -> String {
    paragraph_xml_with_numbering(paragraph, source, paragraph.list.is_none())
}

fn paragraph_xml_with_numbering(paragraph: &Paragraph, source: Option<&str>, preserve_numbering: bool) -> String {
    let wrapped = source.map(wrap_source);
    let parsed = wrapped
        .as_deref()
        .and_then(|xml| roxmltree::Document::parse(xml).ok());
    let original = parsed.as_ref().and_then(|document| {
        document
            .descendants()
            .find(|node| node.has_tag_name((WORD_NAMESPACE, "p")))
    });
    let alignment = match paragraph.alignment {
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
        Alignment::Justify => "both",
    };
    let paragraph_attributes = preserved_attributes(original, &[]);
    let property_attributes = preserved_attributes(original.and_then(|node| child(node, "pPr")), &[]);
    let mut xml = format!("<w:p{paragraph_attributes}><w:pPr{property_attributes}>");
    if let Some(style) = &paragraph.style {
        xml.push_str(&format!("<w:pStyle w:val=\"{}\"/>", escape_xml(style)));
    }
    if paragraph.keep_next {
        xml.push_str("<w:keepNext/>");
    }
    if paragraph.keep_lines {
        xml.push_str("<w:keepLines/>");
    }
    if paragraph.page_break_before {
        xml.push_str("<w:pageBreakBefore/>");
    }
    if !paragraph.widow_control {
        xml.push_str("<w:widowControl w:val=\"0\"/>");
    }
    if let Some(kind) = paragraph.list.as_ref().filter(|_| !preserve_numbering) {
        let identifier = if kind == "bullet" {
            2147483000_u32
        } else {
            2147483001
        };
        xml.push_str(&format!(
            "<w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"{identifier}\"/></w:numPr>"
        ));
    }
    if !paragraph.tabs.is_empty() {
        xml.push_str("<w:tabs>");
        for tab in &paragraph.tabs {
            xml.push_str(&format!(
                "<w:tab w:val=\"{}\" w:pos=\"{}\" w:leader=\"{}\"/>",
                escape_xml(&tab.alignment),
                (tab.position * 15.0).round() as i32,
                escape_xml(&tab.leader)
            ));
        }
        xml.push_str("</w:tabs>");
    }
    if paragraph.bidi {
        xml.push_str("<w:bidi/>");
    }
    let original_properties = original.and_then(|node| child(node, "pPr"));
    if paragraph.line_spacing > 0.0
        || paragraph.space_before > 0.0
        || paragraph.space_after > 0.0
    {
        let extras = preserved_attributes(
            original_properties.and_then(|node| child(node, "spacing")),
            &["line", "lineRule", "before", "after"],
        );
        let line = if let Some(rule) = paragraph.line_rule.as_deref() {
            format!(
                " w:line=\"{}\" w:lineRule=\"{rule}\"",
                (paragraph.line_spacing * 20.0).round() as u32
            )
        } else if paragraph.line_spacing > 0.0 {
            format!(
                " w:line=\"{}\" w:lineRule=\"auto\"",
                (paragraph.line_spacing * 240.0).round() as u32
            )
        } else {
            String::new()
        };
        let before = if paragraph.space_before > 0.0 {
            format!(" w:before=\"{}\"", (paragraph.space_before * 20.0).round() as u32)
        } else {
            String::new()
        };
        let after = if paragraph.space_after > 0.0 {
            format!(" w:after=\"{}\"", (paragraph.space_after * 20.0).round() as u32)
        } else {
            String::new()
        };
        xml.push_str(&format!("<w:spacing{line}{before}{after}{extras}/>"));
    }
    let original_indent = original_properties.and_then(|node| child(node, "ind"));
    if paragraph.indent != 0.0
        || paragraph.indent_right != 0.0
        || paragraph.special.is_some()
        || original_indent.is_some()
    {
        let extras = preserved_attributes(
            original_indent,
            &["left", "start", "right", "end", "firstLine", "hanging"],
        );
        let right = if paragraph.indent_right != 0.0 {
            format!(
                " w:right=\"{}\"",
                (paragraph.indent_right * 15.0).round() as i32
            )
        } else {
            String::new()
        };
        let special = match paragraph.special.as_deref() {
            Some("firstLine") => format!(
                " w:firstLine=\"{}\"",
                (paragraph.special_amount * 15.0).round() as i32
            ),
            Some("hanging") => format!(
                " w:hanging=\"{}\"",
                (paragraph.special_amount * 15.0).round() as i32
            ),
            _ => String::new(),
        };
        xml.push_str(&format!(
            "<w:ind w:left=\"{}\"{right}{special}{extras}/>",
            (paragraph.indent * 15.0).round() as i32
        ));
    }
    if paragraph.contextual_spacing {
        xml.push_str("<w:contextualSpacing/>");
    }
    xml.push_str(&format!("<w:jc w:val=\"{alignment}\"/>"));
    if let Some(properties) = original.and_then(|node| child(node, "pPr")) {
        for property in properties.children().filter(|node| node.is_element()) {
            if !matches!(
                property.tag_name().name(),
                "jc" | "pStyle"
                    | "ind"
                    | "spacing"
                    | "bidi"
                    | "keepNext"
                    | "keepLines"
                    | "widowControl"
                    | "pageBreakBefore"
                    | "contextualSpacing"
                    | "tabs"
            ) && !(property.tag_name().name() == "numPr" && !preserve_numbering)
            {
                xml.push_str(&wrapped.as_deref().unwrap_or("")[property.range()]);
            }
        }
    }
    xml.push_str("</w:pPr>");
    let original_runs: Vec<_> = original
        .map(|node| {
            node.children()
                .filter(|value| value.has_tag_name((WORD_NAMESPACE, "r")))
                .collect()
        })
        .unwrap_or_default();
    for (index, run) in paragraph.runs.iter().enumerate() {
        xml.push_str("<w:r><w:rPr>");
        if let Some(properties) = original_runs
            .get(index)
            .and_then(|node| child(*node, "rPr"))
        {
            for property in properties.children().filter(|node| node.is_element()) {
                if !matches!(
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
                ) {
                    xml.push_str(&wrapped.as_deref().unwrap_or("")[property.range()]);
                }
            }
        }
        for (active, tag) in [
            (run.format.bold, "b"),
            (run.format.italic, "i"),
            (run.format.underline, "u"),
            (run.format.strike, "strike"),
        ] {
            if active {
                if tag == "u" {
                    xml.push_str("<w:u w:val=\"single\"/>");
                } else {
                    xml.push_str(&format!("<w:{tag}/>"));
                }
            }
        }
        if let Some(font) = &run.format.font_family {
            xml.push_str(&format!(
                "<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\"/>",
                escape_xml(font),
                escape_xml(font)
            ));
        }
        if let Some(size) = run
            .format
            .font_size
            .filter(|size| size.is_finite() && *size > 0.0)
        {
            xml.push_str(&format!(
                "<w:sz w:val=\"{}\"/>",
                (size * 2.0).round() as u32
            ));
        }
        if let Some(color) = &run.format.color {
            xml.push_str(&format!("<w:color w:val=\"{}\"/>", escape_xml(color)));
        }
        if let Some(highlight) = &run.format.highlight {
            xml.push_str(&format!(
                "<w:shd w:val=\"clear\" w:fill=\"{}\"/>",
                escape_xml(highlight)
            ));
        }
        if let Some(script) = &run.format.script {
            xml.push_str(&format!("<w:vertAlign w:val=\"{}\"/>", escape_xml(script)));
        }
        xml.push_str("</w:rPr>");
        for segment in run.text.split_inclusive(['\n', '\t']) {
            let text = segment.trim_end_matches(['\n', '\t']);
            xml.push_str(&format!(
                "<w:t xml:space=\"preserve\">{}</w:t>",
                escape_xml(text)
            ));
            if segment.ends_with('\n') {
                xml.push_str("<w:br/>");
            } else if segment.ends_with('\t') {
                xml.push_str("<w:tab/>");
            }
        }
        xml.push_str("</w:r>");
    }
    xml.push_str("</w:p>");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded_body(body: &str) -> Document {
        let xml = format!(
            "<w:document xmlns:w=\"{WORD_NAMESPACE}\"><w:body>{body}</w:body></w:document>"
        );
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "word/document.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
        Document::from_bytes(&writer.finish().unwrap().into_inner()).unwrap()
    }

    fn package_xml(bytes: &[u8]) -> String {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        xml
    }

    #[test]
    fn empty_document_body_has_an_editable_paragraph() {
        let document = loaded_body("");
        assert_eq!(document.paragraphs.len(), 1);
        assert!(!document.has_unrepresented_body_content());
        let reopened = Document::from_bytes(&document.to_bytes().unwrap()).unwrap();
        assert_eq!(reopened.paragraphs.len(), 1);
    }

    #[test]
    fn detects_unrepresented_content_without_confusing_section_properties() {
        assert!(!loaded_body("<w:p/><w:sectPr/>").has_unrepresented_body_content());
        assert!(loaded_body("<w:p/><w:tbl/><w:sectPr/>").has_unrepresented_body_content());
        assert!(loaded_body("<w:p><w:bookmarkStart w:id=\"0\" w:name=\"test\"/></w:p>").can_replace_all_content().is_err());
        assert!(loaded_body("<w:p/><w:tbl/>").can_replace_all_content().is_err());
        assert!(loaded_body("<w:p><w:r><w:t>Text</w:t></w:r></w:p><w:sectPr/>").can_replace_all_content().is_ok());
    }

    #[test]
    fn structural_edits_reject_unmodeled_paragraph_properties() {
        let mut document = loaded_body("<w:p><w:r><w:t>First</w:t></w:r></w:p><w:p><w:pPr><w:sectPr/></w:pPr><w:r><w:t>Second</w:t></w:r></w:p>");
        let original = document.clone();
        let paragraph = document.paragraphs[1].clone();
        assert!(document.split_paragraph(1, paragraph.clone(), paragraph).is_err());
        assert!(document.merge_paragraphs(0, document.paragraphs[0].clone()).is_err());
        assert_eq!(document, original);
        assert!(document.can_replace_all_content().is_err());
        let mut formatted = loaded_body("<w:p/><w:p><w:pPr><w:ind w:left=\"720\" w:hanging=\"360\"/></w:pPr></w:p>");
        assert!(formatted.merge_paragraphs(0, Paragraph::default()).is_err());
        let mut first_section = loaded_body("<w:p><w:pPr><w:sectPr/></w:pPr></w:p><w:p/>");
        assert!(first_section.merge_paragraphs(0, Paragraph::default()).is_err());
    }

    #[test]
    fn text_edits_preserve_paragraph_attributes_and_reject_unknown_namespaces() {
        let mut document = loaded_body("<w:p w:rsidR=\"1234\"><w:pPr w:rsidP=\"5678\"/><w:r><w:t>Before</w:t></w:r></w:p>");
        document.paragraphs[0].runs[0].text = "After".into();
        let xml = package_xml(&document.to_bytes().unwrap());
        assert!(xml.contains("w:rsidR=\"1234\""));
        assert!(xml.contains("w:rsidP=\"5678\""));

        let mut foreign = loaded_body("<w:p xmlns:x=\"urn:test\" x:custom=\"keep\"><w:r><w:t>Before</w:t></w:r></w:p>");
        foreign.paragraphs[0].runs[0].text = "After".into();
        assert!(foreign.to_bytes().is_err());
    }

    #[test]
    fn imported_numbered_list_keeps_its_numbering_when_text_changes() {
        let mut source = loaded_body("<w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"42\"/></w:numPr></w:pPr><w:r><w:t>Item</w:t></w:r></w:p>");
        source.package.push(("word/numbering.xml".into(), format!("<w:numbering xmlns:w=\"{WORD_NAMESPACE}\"><w:abstractNum w:abstractNumId=\"9\"><w:lvl w:ilvl=\"0\"><w:numFmt w:val=\"decimal\"/></w:lvl></w:abstractNum><w:num w:numId=\"42\"><w:abstractNumId w:val=\"9\"/></w:num></w:numbering>").into_bytes()));
        let mut document = Document::from_bytes(&source.to_bytes().unwrap()).unwrap();
        assert_eq!(document.paragraphs[0].list.as_deref(), Some("number"));
        let mut split = document.clone();
        split.paragraphs[0].list = Some("bullet".into());
        let first = split.paragraphs[0].clone();
        let second = first.clone();
        split.split_paragraph(0, first, second).unwrap();
        let split = Document::from_bytes(&split.to_bytes().unwrap()).unwrap();
        assert!(split.paragraphs.iter().all(|paragraph| paragraph.list.as_deref() == Some("bullet")));
        document.paragraphs[0].runs[0].text = "Edited".into();
        let bytes = document.to_bytes().unwrap();
        let xml = package_xml(&bytes);
        assert!(xml.contains("w:numId w:val=\"42\""));
        assert!(!xml.contains("2147483001"));
        let mut reopened = Document::from_bytes(&bytes).unwrap();
        assert_eq!(reopened.paragraphs[0].list.as_deref(), Some("number"));
        assert_eq!(reopened.paragraphs[0].plain_text(), "Edited");
        reopened.paragraphs[0].list = None;
        let removed = package_xml(&reopened.to_bytes().unwrap());
        assert!(!removed.contains("<w:numPr>"));
    }

    #[test]
    fn split_before_a_table_preserves_order_and_later_drawing() {
        let mut document=loaded_body("<w:p><w:r><w:t>Before</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p><w:r><w:t>After</w:t><w:drawing/></w:r></w:p>");
        let mut first = document.paragraphs[0].clone();
        let mut second = first.clone();
        first.runs[0].text = "Be".into();
        second.runs[0].text = "fore".into();
        document.split_paragraph(0, first, second).unwrap();
        let xml = package_xml(&document.to_bytes().unwrap());
        assert!(xml.find(">Be<").unwrap() < xml.find(">fore<").unwrap());
        assert!(xml.find(">fore<").unwrap() < xml.find("<w:tbl>").unwrap());
        assert!(xml.find("<w:tbl>").unwrap() < xml.find(">After<").unwrap());
        assert!(xml.contains("<w:drawing/>"));
    }

    #[test]
    fn merge_keeps_untouched_later_drawing_and_refuses_to_cross_tables() {
        let mut document=loaded_body("<w:p><w:r><w:t>One</w:t></w:r></w:p><w:p><w:r><w:t>Two</w:t></w:r></w:p><w:p><w:r><w:t>Three</w:t><w:drawing/></w:r></w:p>");
        let mut merged = document.paragraphs[0].clone();
        merged.runs.extend(document.paragraphs[1].runs.clone());
        document.merge_paragraphs(0, merged).unwrap();
        let xml = package_xml(&document.to_bytes().unwrap());
        assert!(xml.contains("<w:drawing/>"));
        assert_eq!(
            Document::from_bytes(&document.to_bytes().unwrap())
                .unwrap()
                .plain_text(),
            "OneTwo\nThree"
        );
        let mut separated = loaded_body("<w:p/><w:tbl/><w:p/>");
        let before = separated.clone();
        assert!(separated.merge_paragraphs(0, Paragraph::default()).is_err());
        assert_eq!(separated, before);
    }

    #[test]
    fn refuses_untracked_structure_and_unmodeled_run_properties() {
        let mut document=loaded_body("<w:p><w:r><w:t>First</w:t></w:r></w:p><w:p><w:r><w:t>Second</w:t><w:drawing/></w:r></w:p>");
        document.paragraphs.remove(0);
        assert!(document.to_bytes().is_err());
        let mut formatted=loaded_body("<w:p><w:r><w:t>Hello World</w:t></w:r><w:r><w:rPr><w:rStyle w:val=\"Emphasis\"/></w:rPr><w:t>Other</w:t></w:r></w:p>");
        formatted.paragraphs[0].runs.insert(
            1,
            Run {
                text: "Bold".into(),
                format: TextFormat {
                    bold: true,
                    ..Default::default()
                },
            },
        );
        assert!(formatted
            .to_bytes()
            .unwrap_err()
            .to_string()
            .contains("Run Formatting"));
    }

    #[test]
    fn new_list_and_page_properties_survive_docx_roundtrip() {
        let mut document = Document::default();
        document.replace_plain_text("Formatted List");
        document.paragraphs[0].list = Some("bullet".into());
        document.paragraphs[0].indent = 48.0;
        document.paragraphs[0].line_spacing = 1.5;
        document.paragraphs[0].runs[0].format.highlight = Some("FFFF00".into());
        document.paragraphs[0].runs[0].format.script = Some("superscript".into());
        document.page_setup.width = 1122.0;
        document.page_setup.height = 793.0;
        let bytes = document.to_bytes().unwrap();
        let reopened = Document::from_bytes(&bytes).unwrap();
        assert_eq!(document.paragraphs, reopened.paragraphs);
        assert_eq!(document.page_setup, reopened.page_setup);
        let numbering = reopened
            .package
            .iter()
            .find(|(name, _)| name == "word/numbering.xml")
            .unwrap();
        let numbering_xml = std::str::from_utf8(&numbering.1).unwrap();
        let parsed = roxmltree::Document::parse(numbering_xml).unwrap();
        let label = parsed
            .descendants()
            .find(|node| node.has_tag_name((WORD_NAMESPACE, "lvlText")))
            .unwrap();
        assert_eq!(attribute(label, "val").unwrap(), "\u{2022}");
        assert!(reopened
            .package
            .iter()
            .any(|(name, bytes)| name == "word/_rels/document.xml.rels"
                && String::from_utf8_lossy(bytes).contains("relationships/numbering")));
        let mut changed = reopened;
        changed.paragraphs[0].list = None;
        let removed = Document::from_bytes(&changed.to_bytes().unwrap()).unwrap();
        assert_eq!(removed.paragraphs[0].list, None);
    }

    #[test]
    fn new_list_definitions_expand_empty_xml_parts_and_keep_schema_order() {
        let mut document = Document::default();
        document.paragraphs[0].list = Some("number".into());
        document.package.push((
            "word/numbering.xml".into(),
            format!("<w:numbering xmlns:w=\"{WORD_NAMESPACE}\"/>").into_bytes(),
        ));
        document.package.push(("word/_rels/document.xml.rels".into(), b"<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>".to_vec()));
        let reopened = Document::from_bytes(&document.to_bytes().unwrap()).unwrap();
        let numbering = reopened
            .package
            .iter()
            .find(|(name, _)| name == "word/numbering.xml")
            .unwrap();
        let xml = std::str::from_utf8(&numbering.1).unwrap();
        assert!(xml.rfind("</w:abstractNum>").unwrap() < xml.find("<w:num ").unwrap());
        assert_eq!(reopened.paragraphs[0].list.as_deref(), Some("number"));
    }

    #[test]
    fn paragraph_format_changes_keep_other_spacing_and_indent_attributes() {
        let source = "<w:p><w:pPr><w:ind w:left=\"720\" w:right=\"360\"/><w:spacing w:line=\"240\" w:after=\"120\"/></w:pPr><w:r><w:t>Text</w:t></w:r></w:p>";
        let wrapped = wrap_source(source);
        let parsed = roxmltree::Document::parse(&wrapped).unwrap();
        let mut paragraph = parse_paragraph(
            parsed
                .descendants()
                .find(|node| node.has_tag_name((WORD_NAMESPACE, "p")))
                .unwrap(),
        );
        paragraph.line_spacing = 1.5;
        paragraph.indent = 0.0;
        let edited = paragraph_xml_with_source(&paragraph, Some(source));
        assert!(edited.contains("w:after=\"120\""));
        assert!(edited.contains("w:right=\"360\""));
        assert!(edited.contains("w:left=\"0\""));
        assert!(edited.contains("w:line=\"360\""));
    }

    #[test]
    fn round_trip_preserves_text_and_formatting() {
        let mut document = Document::default();
        document.paragraphs[0] = Paragraph {
            runs: vec![Run {
                text: " A & B < C\tD\nNext ".into(),
                format: TextFormat {
                    bold: true,
                    italic: true,
                    underline: true,
                    strike: true,
                    font_family: Some("Times New Roman".into()),
                    font_size: Some(12.5),
                    color: Some("AABBCC".into()),
                    ..TextFormat::default()
                },
            }],
            alignment: Alignment::Center,
            style: Some("Heading1".into()),
            ..Paragraph::default()
        };
        let reopened = Document::from_bytes(&document.to_bytes().unwrap()).unwrap();
        assert_eq!(reopened.paragraphs, document.paragraphs);
    }

    #[test]
    fn preserves_unknown_parts_and_body_elements_when_text_changes() {
        let mut document = Document::default();
        document
            .package
            .push(("word/media/image1.png".into(), vec![0, 1, 255, 2]));
        document.body.push(BodyElement::Preserved("<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table Content</w:t></w:r></w:p></w:tc></w:tr></w:tbl>".into()));
        document.body.push(BodyElement::Preserved(
            "<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/></w:sectPr>".into(),
        ));
        document.replace_plain_text("Changed Text");
        let reopened = Document::from_bytes(&document.to_bytes().unwrap()).unwrap();
        assert_eq!(reopened.plain_text(), "Changed Text");
        assert!(reopened
            .package
            .contains(&("word/media/image1.png".into(), vec![0, 1, 255, 2])));
        assert!(reopened.body.iter().any(|element| matches!(element, BodyElement::Preserved(xml) if xml.contains("Table Content"))));
        let again = Document::from_bytes(&reopened.to_bytes().unwrap()).unwrap();
        assert_eq!(reopened.paragraphs, again.paragraphs);
    }

    #[test]
    fn rejects_invalid_packages() {
        assert!(Document::from_bytes(b"Invalid Package").is_err());
    }

    #[test]
    fn rejects_invalid_xml_characters_before_saving() {
        let mut document = Document::default();
        document.replace_plain_text("Text\u{1}With Invalid Control Character");
        assert!(document.to_bytes().is_err());
    }

    #[test]
    fn edits_documents_with_nonstandard_namespace_prefixes() {
        let mut document = Document::default();
        document.document_prefix = format!("<q:document xmlns:q=\"{WORD_NAMESPACE}\"><q:body>");
        document.document_suffix = "</q:body></q:document>".into();
        document.body.push(BodyElement::Paragraph {
            original: Paragraph::default(),
            xml: "<q:p><q:r><q:t>Original</q:t></q:r></q:p>".into(),
        });
        let bytes = document.to_bytes().unwrap();
        let mut reopened = Document::from_bytes(&bytes).unwrap();
        reopened.replace_plain_text("Edited");
        let saved = Document::from_bytes(&reopened.to_bytes().unwrap()).unwrap();
        assert_eq!(saved.plain_text(), "Edited");
    }

    #[test]
    fn refuses_to_destroy_unsupported_inline_content() {
        let mut document = Document::default();
        let xml = "<w:p><w:r><w:t>Original</w:t><w:drawing/></w:r></w:p>";
        document.body.push(BodyElement::Paragraph {
            original: Paragraph::default(),
            xml: xml.into(),
        });
        document.replace_plain_text("Changed");
        assert!(document
            .to_bytes()
            .unwrap_err()
            .to_string()
            .contains("Cannot Yet Rewrite Safely"));
    }

    #[test]
    fn preserves_spacing_and_unknown_run_properties_during_edit() {
        let original = "<w:p><w:pPr><w:spacing w:after=\"240\"/></w:pPr><w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t>Original</w:t></w:r></w:p>";
        let wrapped = wrap_source(original);
        let parsed = roxmltree::Document::parse(&wrapped).unwrap();
        let mut paragraph = parse_paragraph(
            parsed
                .descendants()
                .find(|node| node.has_tag_name((WORD_NAMESPACE, "p")))
                .unwrap(),
        );
        paragraph.runs[0].text = "Edited".into();
        let saved = paragraph_xml_with_source(&paragraph, Some(original));
        assert!(saved.contains("<w:spacing w:after=\"240\"/>"));
        assert!(saved.contains("<w:shd w:val=\"clear\" w:fill=\"FFFF00\"/>"));
        assert!(saved.contains("Edited"));
    }
}
