use crate::igrepper::types::{Line, LineWithMatches, MatchPosition};
use regex::Regex;
use std::cmp;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Len {
    Is(u32),
    AtLeast(u32),
}

/// Struct representing the result of a regex search.
/// It generates the output lazily.
#[derive(Debug)]
pub struct OutputGenerator {
    source_lines: Vec<String>,
    regex: Regex,
    context: u32,
    result: Vec<Line>,
    lines_processed: u32,
    widest_line_seen: u32,
    lines_with_match_ranges_dict: HashMap<usize, Line, RandomState>,
}

impl OutputGenerator {
    pub fn new(source_lines: Vec<String>, regex: Regex, context: u32) -> OutputGenerator {
        OutputGenerator {
            source_lines,
            regex,
            context,
            lines_with_match_ranges_dict: HashMap::new(),
            lines_processed: 0,
            widest_line_seen: 0,
            result: vec![],
        }
    }

    /// Returns the length of the currently processed output.
    pub fn len_simple(&self) -> u32 {
        self.result.len() as u32
    }

    /// Returns either:
    /// - The length of the output if fully processed
    /// - The length of the currently processed output otherwise
    pub fn len(&self) -> Len {
        if self.lines_processed == self.source_lines.len() as u32 {
            Len::Is(self.result.len() as u32)
        } else {
            Len::AtLeast(self.result.len() as u32)
        }
    }

    pub fn full_vec(&mut self) -> &Vec<Line> {
        self.request(u32::max_value());
        &self.result
    }

    pub fn full_string_vec(&mut self) -> Vec<String> {
        self.full_vec()
            .iter()
            .filter_map(|line| match line {
                Line::LineWithMatches(l) => Some(l.line.clone()),
                _ => None,
            })
            .collect::<Vec<String>>()
    }

    pub fn full_string(&mut self) -> String {
        self.full_string_vec().join("\n")
    }

    pub fn slice(&mut self, start: u32, end: u32) -> &[Line] {
        return &self.result[start as usize..end as usize];
    }

    /// Maps the internal dictionary state into a Vec.
    fn map_to_vec(&mut self) -> () {
        let mut line_numbers: Vec<usize> = self
            .lines_with_match_ranges_dict
            .keys()
            .cloned()
            .collect::<Vec<usize>>();
        line_numbers.sort();
        self.result.clear();
        for line_num in line_numbers {
            self.result.push(
                self.lines_with_match_ranges_dict
                    .get(&line_num)
                    .unwrap()
                    .clone(),
            );
        }
    }

    pub fn widest_line_seen_so_far(&self) -> u32 {
        self.widest_line_seen
    }

    /// Requests a number of output lines from the generator.
    /// Returns the number of lines calculated, either the same as requested, or less
    /// in case the end of the output was reached.
    pub fn request(&mut self, requested: u32) -> Len {
        let request_chunk_size = 1000;
        let end = requested
            .saturating_sub(requested % request_chunk_size)
            .saturating_add(request_chunk_size);
        while self.lines_with_match_ranges_dict.len() < end as usize
            && self.lines_processed < self.source_lines.len() as u32
        {
            let line = &self.source_lines[self.lines_processed as usize];
            self.widest_line_seen = cmp::max(self.widest_line_seen, line.len() as u32);

            let line_match_ranges: Vec<MatchPosition> = self
                .regex
                .find_iter(&line)
                .map(|match_on_line| MatchPosition {
                    start: match_on_line.start() as u32,
                    end: match_on_line.end() as u32,
                })
                .collect();

            if line_match_ranges.len() > 0 {
                self.lines_with_match_ranges_dict.insert(
                    self.lines_processed as usize,
                    Line::LineWithMatches(LineWithMatches {
                        line: String::from(line),
                        matches: line_match_ranges,
                    }),
                );
                // Add context lines
                if self.context > 0 {
                    let first_context_line_num =
                        self.lines_processed.saturating_sub(self.context) as usize;
                    let last_context_line_num = cmp::min(
                        self.source_lines.len(),
                        self.lines_processed as usize + self.context as usize + 1,
                    );
                    // Add break-line
                    let break_line_num = first_context_line_num.saturating_sub(1);
                    if self.lines_with_match_ranges_dict.len() > 1
                        && !self
                            .lines_with_match_ranges_dict
                            .contains_key(&break_line_num)
                    {
                        self.lines_with_match_ranges_dict
                            .insert(break_line_num, Line::BreakLine);
                    }

                    for context_line_num in first_context_line_num..last_context_line_num {
                        if self
                            .lines_with_match_ranges_dict
                            .contains_key(&context_line_num)
                        {
                            continue;
                        }
                        self.lines_with_match_ranges_dict.insert(
                            context_line_num,
                            Line::LineWithMatches(LineWithMatches {
                                line: String::from(&self.source_lines[context_line_num]),
                                matches: vec![],
                            }),
                        );
                    }
                }
            }
            self.lines_processed += 1;
        }
        self.map_to_vec();
        self.len()
    }
}
