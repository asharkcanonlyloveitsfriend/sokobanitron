pub fn normalize_leading_indentation_with_walls_in_place(lines: &mut [Vec<u8>]) {
    for line in lines {
        if let Some(first_wall) = line.iter().position(|&b| b == b'#') {
            for ch in &mut line[..first_wall] {
                *ch = b'#';
            }
        }
    }
}

pub fn normalize_leading_indentation_with_walls(lines: Vec<String>) -> Vec<String> {
    let mut rows: Vec<Vec<u8>> = lines.into_iter().map(|s| s.into_bytes()).collect();
    normalize_leading_indentation_with_walls_in_place(&mut rows);
    rows
        .into_iter()
        .map(|row| String::from_utf8(row).expect("grid must contain valid ASCII"))
        .collect()
}
