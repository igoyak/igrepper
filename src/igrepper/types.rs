use crate::igrepper::state::SearchLine;

#[derive(Debug, Clone)]
pub struct RenderState {
    pub regex_valid: bool,
    pub max_y: u32,
    pub max_x: u32,
    pub input_window_height: u32,
    pub pager_window_height: u32,
    pub output_search_lines: Vec<SearchLine>,
    pub output_display_lines: Vec<StringWithColorIndexOrBreakLine>,
    pub status_line: String,
}

#[derive(Debug, Clone)]
pub enum StringWithColorIndexOrBreakLine {
    StringWithColorIndex(Vec<StringWithColorIndex>),
    BreakLine,
}

#[derive(Debug, Clone)]
pub enum StringWithColorIndex {
    MatchString((String, u32)), // u32 = color index
    String(String),
}

#[derive(Debug, Clone)]
pub enum Line {
    LineWithMatches(LineWithMatches),
    BreakLine,
}

#[derive(Debug, Clone)]
pub struct LineWithMatches {
    pub line: String,
    pub matches: Vec<MatchPosition>,
}

#[derive(Debug, Clone)]
pub struct MatchPosition {
    pub start: u32,
    pub end: u32,
}
