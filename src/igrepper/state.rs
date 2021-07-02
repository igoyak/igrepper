use super::regex::{Error, Regex};
use crate::igrepper::constants::CASE_INSENSITIVE_PREFIX;
use crate::igrepper::trimming::{content_width, pager_content_height, pager_window_height};
use std::cmp;

#[derive(Debug, Clone)]
pub struct State<'a> {
    source_lines: &'a Vec<String>,
    search_lines: Vec<SearchLine>,
    last_valid_regex: Regex,
    pager_x: u32,
    pager_y: u32,
    max_y: u32,
    max_x: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SearchLine {
    pub line: String,
    pub context: u32,
    pub case_sensitive: bool,
    pub inverse: bool,
}

impl SearchLine {
    pub fn new(line: String, context: u32, case_sensitive: bool, inverse: bool) -> SearchLine {
        SearchLine {
            line,
            context,
            case_sensitive,
            inverse,
        }
    }

    pub fn line_with_sensitivity_prefix(&self) -> String {
        if self.case_sensitive {
            self.line.clone()
        } else {
            format!("{}{}", CASE_INSENSITIVE_PREFIX, self.line)
        }
    }
    pub fn construct_regex(&self) -> Result<Regex, Error> {
        Regex::new(self.line_with_sensitivity_prefix().as_str())
    }
}

fn default_regex() -> Regex {
    Regex::new(CASE_INSENSITIVE_PREFIX).unwrap()
}

impl<'a> State<'a> {
    pub fn new(
        source_lines: &'_ Vec<String>,
        search_lines: Vec<SearchLine>,
        pager_x: u32,
        pager_y: u32,
        max_y: u32,
        max_x: u32,
    ) -> State {
        assert_ne!(
            0,
            search_lines.len(),
            "The vector 'search_lines' in State must be non-empty"
        );
        search_lines[0..search_lines.len() - 1]
            .iter()
            .for_each(|l| {
                let regex_valid = l.construct_regex().is_ok();
                assert!(
                    regex_valid,
                    "All except the last line in 'search_lines' need to be valid regexes"
                );
            });
        let regex: Regex = search_lines
            .last()
            .unwrap()
            .construct_regex()
            .unwrap_or(default_regex());
        State::new_with_regex(
            source_lines,
            search_lines,
            regex,
            pager_x,
            pager_y,
            max_y,
            max_x,
        )
    }

    fn new_with_regex(
        source_lines: &'_ Vec<String>,
        search_lines: Vec<SearchLine>,
        last_valid_regex: Regex,
        pager_x: u32,
        pager_y: u32,
        max_y: u32,
        max_x: u32,
    ) -> State {
        State {
            source_lines,
            search_lines,
            last_valid_regex,
            pager_x,
            pager_y,
            max_y,
            max_x,
        }
    }
    pub fn max_y(&self) -> u32 {
        self.max_y
    }
    pub fn max_x(&self) -> u32 {
        self.max_x
    }
    pub fn pager_y(&self) -> u32 {
        self.pager_y
    }
    pub fn pager_x(&self) -> u32 {
        self.pager_x
    }
    pub fn current_context(&self) -> u32 {
        self.search_lines.last().unwrap().context
    }
    pub fn inverted(&self) -> bool {
        self.search_lines.last().unwrap().inverse
    }
    pub fn search_lines(&self) -> Vec<SearchLine> {
        self.search_lines.clone()
    }
    pub fn search_line_strings(&self) -> Vec<String> {
        self.search_lines
            .iter()
            .cloned()
            .map(|s| s.line)
            .collect::<Vec<String>>()
    }

    pub fn source_lines(&self) -> &Vec<String> {
        self.source_lines
    }

    pub fn regex_valid(&self) -> bool {
        return match self.regex() {
            Ok(_) => true,
            Err(_) => false,
        };
    }

    pub fn regex(&self) -> Result<Regex, Error> {
        self.search_lines.last().unwrap().construct_regex()
    }

    pub fn last_valid_regex(&self) -> Regex {
        self.last_valid_regex.clone()
    }

    pub fn last_search_line_empty(&self) -> bool {
        self.search_lines.last().unwrap().line.is_empty()
    }

    pub fn pop_search_char(self) -> State<'a> {
        let mut search_lines = self.search_lines.clone();
        let last_search_line = search_lines.last_mut().unwrap();
        last_search_line.line.pop();
        let regex = last_search_line
            .construct_regex()
            .unwrap_or(self.last_valid_regex);
        State::new_with_regex(
            self.source_lines,
            search_lines,
            regex,
            self.pager_x,
            self.pager_y,
            self.max_y,
            self.max_x,
        )
    }
    pub fn push_search_char(self, new_char: char) -> State<'a> {
        let mut search_lines = self.search_lines.clone();
        let last_search_line = search_lines.last_mut().unwrap();
        last_search_line.line.push(new_char);
        let regex = last_search_line
            .construct_regex()
            .unwrap_or(self.last_valid_regex);
        State::new_with_regex(
            self.source_lines,
            search_lines,
            regex,
            self.pager_x,
            self.pager_y,
            self.max_y,
            self.max_x,
        )
    }

    pub fn accept_partial_match(self) -> State<'a> {
        if self.search_lines.last().unwrap().line != "" && self.regex_valid() {
            let mut search_lines = self.search_lines.clone();
            search_lines.push(SearchLine {
                line: String::from(""),
                context: search_lines.last().unwrap().context,
                case_sensitive: search_lines.last().unwrap().case_sensitive,
                inverse: search_lines.last().unwrap().inverse,
            });
            return State::new_with_regex(
                self.source_lines,
                search_lines,
                default_regex(),
                self.pager_x,
                self.pager_y,
                self.max_y,
                self.max_x,
            );
        }
        self
    }
    pub fn revert_partial_match(self) -> State<'a> {
        if self.search_lines.len() > 1 {
            let mut search_lines = self.search_lines.clone();
            search_lines.pop();
            let regex: Regex = search_lines.last().unwrap().construct_regex().unwrap(); // previous lines should be valid regexes
            return State::new_with_regex(
                self.source_lines,
                search_lines,
                regex,
                self.pager_x,
                self.pager_y,
                self.max_y,
                self.max_x,
            );
        }
        self
    }
    pub fn set_max_yx(self, max_y: u32, max_x: u32) -> State<'a> {
        State::new_with_regex(
            self.source_lines,
            self.search_lines,
            self.last_valid_regex,
            self.pager_x,
            self.pager_y,
            max_y,
            max_x,
        )
    }
    pub fn modify_context(self, context_diff: i32) -> State<'a> {
        let mut lines = self.search_lines.clone();
        let last_line = lines.pop().unwrap();
        let mut context = last_line.context;
        if context_diff > 0 {
            context = context.saturating_add(context_diff as u32);
        } else {
            context = context.saturating_sub(-context_diff as u32);
        }
        lines.push(SearchLine::new(
            last_line.line,
            context,
            last_line.case_sensitive,
            last_line.inverse,
        ));

        State::new_with_regex(
            self.source_lines,
            lines,
            self.last_valid_regex,
            self.pager_x,
            self.pager_y,
            self.max_y,
            self.max_x,
        )
    }
    /// Moves the pager horizontally
    /// Clamps the new pager position to only allow valid values
    pub fn page_x(self, amount: i32, longest_line_length: u32) -> State<'a> {
        let pager_x = if amount >= 0 {
            cmp::min(
                self.pager_x.saturating_add(amount as u32),
                longest_line_length.saturating_sub(content_width(self.max_x)),
            )
        } else {
            self.pager_x.saturating_sub(amount.wrapping_abs() as u32)
        };
        State::new_with_regex(
            self.source_lines,
            self.search_lines,
            self.last_valid_regex,
            pager_x,
            self.pager_y,
            self.max_y,
            self.max_x,
        )
    }

    /// Moves the pager vertically
    /// Clamps the new pager position to only allow valid values
    pub fn page_y(self, amount: i32, output_line_count: u32) -> State<'a> {
        let pager_y: u32;
        if amount >= 0 {
            let pager_y_max = output_line_count.saturating_sub(pager_content_height(
                pager_window_height(self.max_y, self.search_lines.len() as u32),
            ));
            pager_y = cmp::min(pager_y_max, self.pager_y.saturating_add(amount as u32));
        } else {
            pager_y = self.pager_y.saturating_sub(amount.wrapping_abs() as u32);
        }
        State::new_with_regex(
            self.source_lines,
            self.search_lines.clone(),
            self.last_valid_regex,
            self.pager_x,
            pager_y,
            self.max_y,
            self.max_x,
        )
    }

    pub fn toggle_case_sensitivity(self) -> State<'a> {
        let mut search_lines = self.search_lines.clone();
        let mut last_search_line = search_lines.last_mut().unwrap();
        last_search_line.case_sensitive = !last_search_line.case_sensitive;
        let regex = last_search_line
            .construct_regex()
            .unwrap_or(self.last_valid_regex);
        State::new_with_regex(
            self.source_lines,
            search_lines,
            regex,
            self.pager_x,
            self.pager_y,
            self.max_y,
            self.max_x,
        )
    }

    pub fn toggle_inverted(self) -> State<'a> {
        let mut search_lines = self.search_lines.clone();
        let mut last_search_line = search_lines.last_mut().unwrap();
        last_search_line.inverse = !last_search_line.inverse;
        let regex = last_search_line
            .construct_regex()
            .unwrap_or(self.last_valid_regex);
        State::new_with_regex(
            self.source_lines,
            search_lines,
            regex,
            self.pager_x,
            self.pager_y,
            self.max_y,
            self.max_x,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    #[should_panic(expected = "The vector 'search_lines' in State must be non-empty")]
    fn panic_on_empty_search_lines() {
        State::new(&vec![], vec![], 0, 0, 0, 0);
    }

    #[test]
    #[should_panic(
        expected = "All except the last line in 'search_lines' need to be valid regexes"
    )]
    fn panic_on_invalid_initial_regex() {
        State::new(
            &vec![],
            vec![
                SearchLine::new(String::from("\\"), 0, false, false),
                SearchLine::new(String::from(""), 0, false, false),
            ],
            0,
            0,
            0,
            0,
        );
    }

    fn get_state(source_lines: &Vec<String>) -> State {
        State::new(
            source_lines,
            vec![
                SearchLine::new(String::from("abc"), 0, false, false),
                SearchLine::new(String::from("d"), 0, false, false),
            ],
            0,
            0,
            10,
            10,
        )
    }

    fn get_source_lines() -> Vec<String> {
        vec![
            String::from("one"),
            String::from("two"),
            String::from("three"),
        ]
    }

    #[test]
    fn push_char() {
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).push_search_char('e');
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"de\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)de, pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
    }

    #[test]
    fn push_char_create_invalid_regex() {
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).push_search_char('\\');
        assert_eq!(format!("{:?}", state.last_valid_regex()), "(?i)d");
    }

    #[test]
    fn pop_char() {
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).pop_search_char();

        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i), pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
        let state = state.pop_search_char();
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i), pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
    }

    #[test]
    fn accepting_match() {
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).accept_partial_match();
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i), pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
    }

    #[test]
    fn accepting_match_should_reject_invalid_regex() {
        let source_lines = get_source_lines();
        let state = get_state(&source_lines)
            .push_search_char('\\')
            .accept_partial_match();
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\\\\\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
    }

    #[test]
    fn reverting_match() {
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).revert_partial_match();
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)abc, pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
    }

    #[test]
    fn incrementing_context() {
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).modify_context(1);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 1, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
        let state = state.modify_context(2);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 3, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
    }

    #[test]
    fn decrementing_context() {
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).modify_context(-1);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
    }

    #[test]
    fn page_y() {
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).page_y(1, 10);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 0, pager_y: 1, max_y: 10, max_x: 10 }");
        let state = state.page_y(100, 10);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 0, pager_y: 7, max_y: 10, max_x: 10 }");
        let state = state.page_y(-100, 10);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
    }

    #[test]
    fn page_x() {
        let longest_line_length = 15;
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).page_x(1, longest_line_length);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 1, pager_y: 0, max_y: 10, max_x: 10 }");
        let state = state.page_x(100, longest_line_length);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 7, pager_y: 0, max_y: 10, max_x: 10 }");
        let state = state.page_x(-100, longest_line_length);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 0, pager_y: 0, max_y: 10, max_x: 10 }");
    }

    #[test]
    fn toggle_inverted_match() {
        let longest_line_length = 15;
        let source_lines = get_source_lines();
        let state = get_state(&source_lines).page_x(1, longest_line_length);
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: false }], last_valid_regex: (?i)d, pager_x: 1, pager_y: 0, max_y: 10, max_x: 10 }");
        let state = state.toggle_inverted();
        assert_eq!(format!("{:?}", state), "State { source_lines: [\"one\", \"two\", \"three\"], search_lines: [SearchLine { line: \"abc\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"d\", context: 0, case_sensitive: false, inverse: true }], last_valid_regex: (?i)d, pager_x: 1, pager_y: 0, max_y: 10, max_x: 10 }");
    }
}
