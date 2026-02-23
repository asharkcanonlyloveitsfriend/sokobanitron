pub fn rectangularize_with_walls_in_place(lines: &mut [Vec<u8>]) {
    let mut max_width = 0usize;
    for line in lines.iter() {
        if line.len() > max_width {
            max_width = line.len();
        }
    }
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
