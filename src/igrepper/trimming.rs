use crate::igrepper::constants::*;
use crate::igrepper::output_generator::{Len, OutputGenerator};
use crate::igrepper::types::{
    Line, LineWithMatches, RenderState, StringWithColorIndex, StringWithColorIndexOrBreakLine,
};
use std::cmp;
use std::collections::HashMap;

/// Returns a state that can be rendered to the screen
///
///          <──────────── content_width ────────────>
///         <──────────────── max_x ──────────────────>
///       ^ ┌─────────────────────────────────────────┐
///       │ │▓▓▓▓▓▓▓▓▓▓▓▓                             │  <- input window, search lines
///       │ │▓▓▓▓▓▓▓                                  │
///       │ └─────────────────────────────────────────┘
///       │ ┌─────────────────────────────────────────┐
/// max_y │ │░░░░░░░░░░░░░░░░░░░░░                    │  <- pager window, output lines
///       │ │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░         │
///       │ │░░░░░░░░░                                │
///       │ │░░░░░░░░░░░░░                            │
///       │ └─────────────────────────────────────────┘
///       v ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓                        <- status line
///
pub fn produce_render_state(
    regex_valid: bool,
    max_y: u32,
    max_x: u32,
    pager_y: u32,
    pager_x: u32,
    search_lines: &Vec<String>,
    context: u32,
    result_generator: &mut OutputGenerator,
) -> RenderState {
    let input_window_height = input_window_height(max_y, search_lines.len() as u32);
    let pager_content_height =
        pager_content_height(pager_window_height(max_y, search_lines.len() as u32));

    let output_display_lines = output_lines_display_format(
        pager_y,
        pager_x,
        content_width(max_x),
        pager_content_height,
        result_generator,
    );

    let matched_lines = match result_generator.len() {
        Len::Is(n) => format!("={}", n),
        Len::AtLeast(n) => format!(">{}", n),
    };
    let status_line = format!(
        "matchedLines{} pageY: {}, pageX: {}, context: {}",
        matched_lines, pager_y, pager_x, context
    );

    RenderState {
        regex_valid,
        max_y,
        max_x,
        input_window_height,
        pager_window_height: pager_window_height(max_y, search_lines.len() as u32),
        output_search_lines: search_lines_display_format(
            input_window_height,
            search_lines,
            content_width(max_x),
        ),
        output_display_lines,
        status_line: status_line
            .chars()
            .into_iter()
            .take(max_x as usize)
            .collect(),
    }
}

/// Get the width of the application content, based
/// on the max_x of the terminal.
pub fn content_width(max_x: u32) -> u32 {
    let borders_width = 2;
    max_x.saturating_sub(borders_width)
}

/// Using the screen dimensions and pager position, returns output
/// that is visible.
fn output_lines_display_format(
    pager_y: u32,
    pager_x: u32,
    content_width: u32,
    pager_content_height: u32,
    result_generator: &mut OutputGenerator,
) -> Vec<StringWithColorIndexOrBreakLine> {
    const REQUEST_BUFFER_SIZE: u32 = 10; // Request a little more than actually needed.
    result_generator.request(pager_y + pager_content_height + REQUEST_BUFFER_SIZE);
    let line_count_at_least = match result_generator.len() {
        Len::Is(n) => n,
        Len::AtLeast(n) => n,
    };
    let first_line_no = cmp::min(line_count_at_least, pager_y);
    let last_line_no = cmp::min(
        pager_y as usize + pager_content_height as usize,
        line_count_at_least as usize,
    ) as u32;
    let visible_lines = result_generator.slice(first_line_no, last_line_no);

    // We want to colorize each unique match with the same color, so we create a closure
    // that keeps track of the matches seen so far.
    let mut matches_to_colors = HashMap::new();
    let mut color_number: u32 = 0;
    let mut get_color = |string: &str| -> u32 {
        let s = String::from(string);
        if !matches_to_colors.contains_key(&s) {
            matches_to_colors.insert(s, color_number);
            if color_number < MAX_MATCH_COLORS as u32 - 1 {
                color_number += 1;
            }
        }
        matches_to_colors[string]
    };

    visible_lines
        .iter()
        .map(|line| match line {
            Line::BreakLine => StringWithColorIndexOrBreakLine::BreakLine,
            Line::LineWithMatches(l) => {
                trim_and_colorize_line(l, pager_x, content_width, &mut get_color)
            }
        })
        .collect::<Vec<StringWithColorIndexOrBreakLine>>()
}

/// Trims a single output line to fit the screen.
/// Includes color information for each character.
///
/// A line may contain multiple matches and non-matches, which
/// may lie fully within, partially outside or fully outside the
/// current screen.
///
///   mm__mmmmm__mm___mm
///        ^        ^
///        └────────┘
///       content width
///
fn trim_and_colorize_line<F: FnMut(&str) -> u32>(
    line_with_match_ranges: &LineWithMatches,
    pager_x: u32,
    content_width: u32,
    mut get_color: F,
) -> StringWithColorIndexOrBreakLine {
    let mut chars_to_drop = pager_x;
    let mut chars_to_take = content_width;
    let mut display_line: Vec<StringWithColorIndex> = vec![];
    let original_line = &line_with_match_ranges.line;
    let mut cell_width = 0;
    let mut end_of_last_match = 0u32;

    // Returns a substring that fits horizontally on screen
    let mut trim_horizontally = |mut s: String| -> String {
        // full drop
        if chars_to_drop > s.chars().count() as u32 {
            chars_to_drop -= s.chars().count() as u32;
            return String::from("");
        }
        // partial drop
        if chars_to_drop > 0 {
            s = s
                .chars()
                .into_iter()
                .skip(chars_to_drop as usize)
                .collect::<String>();
            chars_to_drop = 0;
        }
        // take
        if s.chars().count() as u32 > chars_to_take {
            let right_trimmed = s
                .chars()
                .into_iter()
                .take(chars_to_take as usize)
                .collect::<String>();
            chars_to_take = 0;
            return right_trimmed;
        }
        chars_to_take -= s.chars().count() as u32;
        return s;
    };

    for match_range in &line_with_match_ranges.matches {
        // Process string between current position and the start of the next match
        if end_of_last_match < match_range.start {
            let string_before_match = replace_tabs_with_spaces(
                cell_width as u32,
                &original_line[end_of_last_match as usize..match_range.start as usize],
            );
            cell_width += string_before_match.chars().count();
            let string_before_match = trim_horizontally(string_before_match);
            if !string_before_match.is_empty() {
                display_line.push(StringWithColorIndex::String(String::from(
                    string_before_match,
                )));
            }
        }

        // Process the current match on the line
        let string_with_match =
            &original_line[match_range.start as usize..match_range.end as usize];
        let color = get_color(string_with_match);

        let string_with_match = replace_tabs_with_spaces(cell_width as u32, string_with_match);
        cell_width += string_with_match.chars().count();

        let string_with_match = trim_horizontally(string_with_match);
        if !string_with_match.is_empty() {
            display_line.push(StringWithColorIndex::MatchString((
                String::from(string_with_match),
                color,
            )));
        }
        end_of_last_match = match_range.end;
    }
    // Process the string after the last match
    if let Some(last_match) = line_with_match_ranges.matches.last() {
        let string_after_last_match =
            replace_tabs_with_spaces(cell_width as u32, &original_line[last_match.end as usize..]);
        let string_after_last_match = trim_horizontally(string_after_last_match);
        if !string_after_last_match.is_empty() {
            display_line.push(StringWithColorIndex::String(String::from(
                string_after_last_match,
            )));
        }
    } else {
        // No matches on this line
        let full_line_without_matches = replace_tabs_with_spaces(cell_width as u32, original_line);
        let full_line_without_matches = trim_horizontally(full_line_without_matches);
        if !full_line_without_matches.is_empty() {
            display_line.push(StringWithColorIndex::String(String::from(
                full_line_without_matches,
            )));
        }
    }
    return StringWithColorIndexOrBreakLine::StringWithColorIndex(display_line);
}

/// Returns the same string where every tab is replaced with 1-4 spaces,
/// depending on the horizontal position of the tab character.
///
/// Example, with a single tab character in different places:
/// ┌────────┐
/// │    aaaa│
/// │a   aaaa│
/// │aa  aaaa│
/// │aaa aaaa│
/// └────────┘
fn replace_tabs_with_spaces(current_steps: u32, input_string: &str) -> String {
    let tabstop = 4;
    let mut steps_taken = current_steps;
    let mut output_string = String::from("");
    for s in input_string.chars() {
        if s == '\t' {
            let tab_width = tabstop - steps_taken % tabstop;
            for _ in 0..tab_width {
                output_string.push(' ');
            }
            steps_taken += tab_width;
        } else if s.len_utf8() > 1 {
            steps_taken += 1;
            output_string.push('_'); // replace unicode chars, until proper support
        } else {
            steps_taken += 1;
            output_string.push(s);
        }
    }
    output_string.clone()
}

pub fn pager_content_height(pager_window_height_no: u32) -> u32 {
    pager_window_height_no.saturating_sub(2) // 2 for borders
}

pub fn pager_window_height(max_y: u32, search_lines: u32) -> u32 {
    max_y
        .saturating_sub(input_window_height(max_y, search_lines))
        .saturating_sub(1) // 1 for status line
}

fn input_window_height(max_y: u32, search_lines: u32) -> u32 {
    let min = 2;
    let max = max_y.saturating_sub(5); // 2 + 2 borders + 1 status line
    let wanted = search_lines + 2; // 2 for borders
    cmp::min(max, cmp::max(min, wanted))
}

/// Trim search lines by width and height
fn search_lines_display_format(
    input_window_height: u32,
    search_lines: &Vec<String>,
    content_width: u32,
) -> Vec<String> {
    let lines_to_take = cmp::min(
        input_window_height.saturating_sub(2) as usize,
        search_lines.len(),
    );
    let mut output_search_lines: Vec<String> = vec![];
    for search_line in search_lines.iter().rev().take(lines_to_take).rev() {
        let last_column_no = cmp::min(search_line.len(), content_width as usize);
        let output_line = &search_line[0..last_column_no];
        output_search_lines.push(String::from(output_line));
    }
    output_search_lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn replace_tabs_with_spaces_zero_current_steps() {
        assert_eq!("    x", replace_tabs_with_spaces(0, "\tx"));
        assert_eq!("x   x", replace_tabs_with_spaces(0, "x\tx"));
        assert_eq!("xx  x", replace_tabs_with_spaces(0, "xx\tx"));
        assert_eq!("xxx x", replace_tabs_with_spaces(0, "xxx\tx"));
        assert_eq!("xxxx    x", replace_tabs_with_spaces(0, "xxxx\tx"));
    }

    #[test]
    fn replace_tabs_with_spaces_different_current_steps() {
        assert_eq!("   x", replace_tabs_with_spaces(1, "\tx"));
        assert_eq!("  x", replace_tabs_with_spaces(2, "\tx"));
        assert_eq!(" x", replace_tabs_with_spaces(3, "\tx"));
        assert_eq!("    x", replace_tabs_with_spaces(4, "\tx"));
    }
}
