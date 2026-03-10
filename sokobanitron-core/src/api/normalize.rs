use crate::normalize::pipeline;

pub fn normalize_to_walkable_region_lines(lines: Vec<String>) -> Vec<String> {
    pipeline::normalize_to_walkable_region_lines(lines)
}

pub fn normalize_to_walkable_region(lines: Vec<String>) -> Vec<String> {
    pipeline::normalize_to_walkable_region(lines)
}
