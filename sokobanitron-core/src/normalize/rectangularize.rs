pub fn rectangularize_with_walls_in_place(lines: &mut [Vec<u8>]) {
    let max_width = lines.iter().map(Vec::len).max().unwrap_or(0);
    for line in lines.iter_mut() {
        if line.len() < max_width {
            line.resize(max_width, b'#');
        }
    }
}

pub fn rectangularize_with_walls(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut rows: Vec<Vec<u8>> = lines.into_iter().map(|s| s.into_bytes()).collect();
    rectangularize_with_walls_in_place(&mut rows);
    rows
        .into_iter()
        .map(|row| String::from_utf8(row).expect("grid must contain valid ASCII"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_place_rectangularizes_shorter_rows() {
        let mut rows = vec![
            b"###".to_vec(),
            b"##".to_vec(),
            b"#".to_vec(),
        ];
        rectangularize_with_walls_in_place(&mut rows);
        assert_eq!(rows[0], b"###".to_vec());
        assert_eq!(rows[1], b"###".to_vec());
        assert_eq!(rows[2], b"###".to_vec());
    }

    #[test]
    fn rectangularize_string_api() {
        let input = vec!["###".to_string(), "##".to_string(), "#".to_string()];
        let out = rectangularize_with_walls(input);
        assert_eq!(out, vec!["###", "###", "###"]);
    }

    #[test]
    fn empty_input_is_noop() {
        let input: Vec<String> = vec![];
        let out = rectangularize_with_walls(input);
        assert!(out.is_empty());
    }

    #[test]
    fn already_rectangular_is_unchanged() {
        let input = vec!["###".to_string(), "###".to_string()];
        let out = rectangularize_with_walls(input.clone());
        assert_eq!(out, input);
    }

    #[test]
    fn uneven_grid_with_internal_spaces() {
        let input = vec!["# #".to_string(), "#".to_string()];
        let out = rectangularize_with_walls(input);
        assert_eq!(out, vec!["# #", "###"]);
    }
}
