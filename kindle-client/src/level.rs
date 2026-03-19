use sokobanitron_core::normalize_to_walkable_region_lines;
use std::fs;
use std::path::{Path, PathBuf};

const PORTRAIT_LEVEL_VISUAL: &str = "\
_@_#\n\
_#_#\n\
___#\n\
#_.#\n\
#_.#\n\
#_.#\n\
__##\n\
_$__\n\
_$$_\n\
____";

pub fn portrait_level_ascii() -> String {
    PORTRAIT_LEVEL_VISUAL
        .chars()
        .map(|ch| if ch == '_' { ' ' } else { ch })
        .collect()
}

pub fn load_kindle_levels() -> Vec<String> {
    let parsed = first_slc_contents()
        .map(|xml| parse_slc_levels(&xml))
        .unwrap_or_default()
        .into_iter()
        .map(|ascii| normalize_and_orient_level(&ascii))
        .filter(|ascii| !ascii.trim().is_empty())
        .collect::<Vec<_>>();
    if parsed.is_empty() {
        vec![portrait_level_ascii()]
    } else {
        parsed
    }
}

fn first_slc_contents() -> Option<String> {
    first_slc_path().and_then(|path| fs::read_to_string(path).ok())
}

fn first_slc_path() -> Option<PathBuf> {
    for dir in slc_search_dirs() {
        if let Some(path) = first_slc_in_dir(&dir) {
            return Some(path);
        }
    }
    None
}

fn slc_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let parent = parent.to_path_buf();
        if !dirs.iter().any(|d| d == &parent) {
            dirs.push(parent);
        }
    }
    dirs
}

fn first_slc_in_dir(dir: &Path) -> Option<PathBuf> {
    let mut entries = fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("slc"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries.into_iter().next()
}

fn normalize_and_orient_level(ascii: &str) -> String {
    let mut lines = ascii.lines().map(ToString::to_string).collect::<Vec<_>>();
    lines = normalize_to_walkable_region_lines(lines);
    if is_wider_than_tall(&lines) {
        lines = rotate_clockwise_lines(&lines);
    }
    lines.join("\n")
}

fn parse_slc_levels(xml: &str) -> Vec<String> {
    let mut levels = Vec::new();
    let mut in_level = false;
    let mut lines: Vec<String> = Vec::new();

    for raw_line in xml.lines() {
        let line = raw_line.trim();
        if line.starts_with("<Level ") {
            in_level = true;
            lines.clear();
            continue;
        }
        if line == "</Level>" {
            if in_level && !lines.is_empty() {
                levels.push(lines.join("\n"));
            }
            in_level = false;
            lines.clear();
            continue;
        }
        if !in_level {
            continue;
        }

        if let Some(content) = extract_tag_content(line, "L") {
            lines.push(decode_xml_entities(content));
        }
    }

    levels
}

fn extract_tag_content<'a>(line: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if !line.starts_with(&open) || !line.ends_with(&close) {
        return None;
    }
    Some(&line[open.len()..line.len().saturating_sub(close.len())])
}

fn decode_xml_entities(input: &str) -> String {
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn is_wider_than_tall(lines: &[String]) -> bool {
    let h = lines.len();
    if h == 0 {
        return false;
    }
    let w = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    w > h
}

fn rotate_clockwise_lines(lines: &[String]) -> Vec<String> {
    let h = lines.len();
    if h == 0 {
        return Vec::new();
    }
    let w = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    if w == 0 {
        return Vec::new();
    }

    let mut grid = vec![vec![' '; w]; h];
    for (r, line) in lines.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            grid[r][c] = ch;
        }
    }

    let mut out = vec![String::with_capacity(h); w];
    for c in 0..w {
        let mut row = String::with_capacity(h);
        for r in (0..h).rev() {
            row.push(grid[r][c]);
        }
        out[c] = row;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{first_slc_in_dir, is_wider_than_tall, parse_slc_levels, rotate_clockwise_lines};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_level_lines_from_slc_xml() {
        let xml = r#"
<?xml version="1.0"?>
<SokobanLevels>
  <LevelCollection>
    <Level Id="1" Width="3" Height="2">
      <L>###</L>
      <L>#@.</L>
    </Level>
  </LevelCollection>
</SokobanLevels>
"#;
        let levels = parse_slc_levels(xml);
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0], "###\n#@.");
    }

    #[test]
    fn rotates_wide_level_to_portrait() {
        let lines = vec!["####".to_string(), "#@ $#".to_string()];
        assert!(is_wider_than_tall(&lines));
        let rotated = rotate_clockwise_lines(&lines);
        assert_eq!(rotated.len(), 5);
        assert_eq!(rotated[0].chars().count(), 2);
    }

    #[test]
    fn picks_first_slc_file_lexicographically() {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "kindle-level-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("zeta.slc"), "<SokobanLevels/>").expect("write zeta");
        fs::write(dir.join("alpha.slc"), "<SokobanLevels/>").expect("write alpha");
        fs::write(dir.join("note.txt"), "ignore").expect("write txt");

        let picked = first_slc_in_dir(&dir).expect("should find slc");
        let expected = PathBuf::from(&dir).join("alpha.slc");
        assert_eq!(picked, expected);

        fs::remove_dir_all(&dir).expect("cleanup");
    }
}
