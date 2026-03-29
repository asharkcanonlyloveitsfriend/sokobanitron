use quick_xml::Reader;
use quick_xml::events::Event;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub(crate) struct ParsedLevelSet {
    pub(crate) title: String,
    pub(crate) levels: Vec<ParsedLevel>,
}

#[derive(Debug)]
pub(crate) struct ParsedLevel {
    pub(crate) grid: String,
}

pub(crate) fn parse_slc_file(path: &Path) -> io::Result<ParsedLevelSet> {
    let xml = fs::read_to_string(path)?;
    parse_slc_xml(&xml)
}

pub(crate) fn parse_slc_xml(xml: &str) -> io::Result<ParsedLevelSet> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut title = String::new();
    let mut levels = Vec::new();
    let mut current_tag: Option<Vec<u8>> = None;
    let mut current_level_lines = Vec::new();
    let mut in_level = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = event.name().as_ref().to_vec();
                if name.as_slice() == b"Level" {
                    in_level = true;
                    current_level_lines.clear();
                }
                current_tag = Some(name);
            }
            Ok(Event::End(event)) => {
                let name = event.name().as_ref().to_vec();
                if name.as_slice() == b"Level" {
                    if !current_level_lines.is_empty() {
                        levels.push(ParsedLevel {
                            grid: current_level_lines.join("\n"),
                        });
                    }
                    current_level_lines.clear();
                    in_level = false;
                }
                current_tag = None;
            }
            Ok(Event::Text(text)) => {
                let decoded = text.decode().map_err(xml_error)?.into_owned();
                match current_tag.as_deref() {
                    Some(b"Title") if !in_level && title.is_empty() => title = decoded,
                    Some(b"L") if in_level => current_level_lines.push(decoded),
                    _ => {}
                }
            }
            Ok(Event::CData(text)) => {
                let decoded = text.decode().map_err(xml_error)?.into_owned();
                if current_tag.as_deref() == Some(b"L") && in_level {
                    current_level_lines.push(decoded);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => return Err(xml_error(err)),
        }
    }

    Ok(ParsedLevelSet { title, levels })
}

pub(crate) fn fallback_title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.replace('_', " ").trim().to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "Imported Level Set".to_string())
}

fn xml_error(err: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("parse slc: {err}"))
}
