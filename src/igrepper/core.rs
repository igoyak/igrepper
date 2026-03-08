use crate::igrepper::output_generator::{Len, OutputGenerator, SourceLines};
use crate::igrepper::state::{SearchLine, State};
use crate::igrepper::trimming::produce_render_state;
use crate::igrepper::types::RenderState;
use std::collections::HashMap;

#[derive(Debug)]
struct CacheEntry {
    pub search_line: String,
    pub output_generator: OutputGenerator,
    /// If this generator's source is buffered from a parent, stores the
    /// parent's cache key so Core can drain matching lines before request().
    pub parent_key: Option<CacheKey>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
struct CacheKey {
    search_lines: Vec<SearchLine>,
    context: u32,
    inverted: bool,
    active_regex: String,
}

#[derive(Debug)]
pub struct Core {
    cache: HashMap<CacheKey, CacheEntry>,
}

fn get_cache_key(state: &State) -> CacheKey {
    CacheKey {
        search_lines: state.search_lines(),
        context: state.current_context(),
        inverted: state.inverted(),
        active_regex: state.last_valid_regex().to_string(),
    }
}

impl Core {
    pub fn new() -> Core {
        Core {
            cache: HashMap::new(),
        }
    }

    pub fn clear_cache(&mut self) {
        self.cache = HashMap::new();
    }

    pub fn get_full_output_string(&mut self, state: &State) -> String {
        let key = get_cache_key(state);
        self.populate_cache(state);
        self.drain_and_request_full(&key);
        self.cache
            .get_mut(&key)
            .unwrap()
            .output_generator
            .full_string()
    }

    pub fn widest_line_seen_so_far(&mut self, state: &State) -> u32 {
        let key = get_cache_key(state);
        self.populate_cache(state);
        self.cache
            .get(&key)
            .unwrap()
            .output_generator
            .widest_line_seen_so_far()
    }

    pub fn is_output_length_at_least(&mut self, state: &State, length: u32) -> u32 {
        let key = get_cache_key(state);
        self.populate_cache(state);
        self.drain_parent_into_child(&key, length);
        let output_generator = &mut self.cache.get_mut(&key).unwrap().output_generator;
        output_generator.request(length);
        output_generator.len_simple()
    }

    pub fn get_current_output_length(&mut self, state: &State) -> Len {
        let key = get_cache_key(state);
        self.populate_cache(state);
        self.cache.get(&key).unwrap().output_generator.len()
    }

    pub fn get_render_state(&mut self, state: &State) -> RenderState {
        let key = get_cache_key(state);
        self.populate_cache(state);
        let lines_needed = state.pager_y() + state.max_y() + 10;
        self.drain_parent_into_child(&key, lines_needed);
        let output_generator = &mut self.cache.get_mut(&key).unwrap().output_generator;
        produce_render_state(
            state.regex_valid(),
            state.max_y(),
            state.max_x(),
            state.pager_y(),
            state.pager_x(),
            &state.search_lines(),
            state.current_context(),
            output_generator,
        )
    }

    /// Drains matching lines from a parent OutputGenerator into the child's
    /// buffered source. Requests enough lines from the parent to cover the
    /// child's needs, then copies new matching lines into the child's buffer.
    fn drain_parent_into_child(&mut self, child_key: &CacheKey, child_requested: u32) {
        let entry = self.cache.get(child_key).unwrap();
        let parent_key = match &entry.parent_key {
            Some(k) => k.clone(),
            None => return,
        };

        let child_needs = {
            let child = &entry.output_generator;
            let request_chunk_size: u32 = 1000;
            let end = child_requested
                .saturating_sub(child_requested % request_chunk_size)
                .saturating_add(request_chunk_size);
            end.saturating_add(child.context()).saturating_add(1) as usize
        };
        let already_buffered = entry.output_generator.source_buffered_len();

        if already_buffered >= child_needs {
            return;
        }

        // First, recursively ensure the parent is also drained from its parent.
        self.drain_parent_into_child(&parent_key, child_needs as u32);

        let parent = &mut self.cache.get_mut(&parent_key).unwrap().output_generator;
        parent.request(child_needs as u32);
        let parent_matching_count = parent.matching_line_count();
        let parent_exhausted = parent.is_fully_processed();
        let new_lines: Vec<String> = if already_buffered < parent_matching_count {
            parent.matching_lines()[already_buffered..parent_matching_count].to_vec()
        } else {
            vec![]
        };

        let child = &mut self.cache.get_mut(child_key).unwrap().output_generator;
        child
            .source_lines_mut()
            .extend_buffer(&new_lines, parent_exhausted);
    }

    /// Fully drains parent into child (used for full_string/full_vec).
    fn drain_and_request_full(&mut self, child_key: &CacheKey) {
        let entry = self.cache.get(child_key).unwrap();
        if entry.parent_key.is_none() {
            return;
        }
        let parent_key = entry.parent_key.clone().unwrap();
        let already_buffered = entry.output_generator.source_buffered_len();

        self.drain_and_request_full(&parent_key);

        let parent = &mut self.cache.get_mut(&parent_key).unwrap().output_generator;
        parent.request(u32::MAX);
        let parent_matching_count = parent.matching_line_count();
        let parent_exhausted = parent.is_fully_processed();
        let new_lines: Vec<String> = if already_buffered < parent_matching_count {
            parent.matching_lines()[already_buffered..parent_matching_count].to_vec()
        } else {
            vec![]
        };

        let child = &mut self.cache.get_mut(child_key).unwrap().output_generator;
        child
            .source_lines_mut()
            .extend_buffer(&new_lines, parent_exhausted);
    }

    fn populate_cache(&mut self, state: &State) {
        let first_line = state.search_line_strings().len() == 1;
        if !first_line {
            self.populate_cache(&state.clone().revert_partial_match());
        }

        let s = state.search_line_strings().last().unwrap().clone();
        let maybe_cache_entry = self.cache.get_mut(&get_cache_key(state));

        let mut cache_ok = true;
        if let Some(cache_entry) = maybe_cache_entry {
            if cache_entry.search_line == s {
                // cache ok for i
            } else {
                cache_ok = false;
            }
        } else {
            cache_ok = false;
        }
        if !cache_ok {
            let (source_lines, parent_key) = if first_line {
                (SourceLines::Raw(state.source_lines_arc()), None)
            } else {
                let reverted_key = get_cache_key(&state.clone().revert_partial_match());
                (SourceLines::new_buffered(), Some(reverted_key))
            };
            let output_generator = OutputGenerator::new(
                source_lines,
                state.last_valid_regex(),
                state.last_search_line_empty(),
                state.current_context(),
                state.inverted(),
            );
            self.cache.insert(
                get_cache_key(state),
                CacheEntry {
                    search_line: s,
                    output_generator,
                    parent_key,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::igrepper::state::SearchLine;
    use std::collections::HashSet;

    use std::fs::File;
    use std::io::{Read, Write};
    use std::{fs, io};

    extern crate serde_json;

    const SNAPSHOT_DIRECTORY: &str = "tests/snapshots";
    const SNAPSHOT_DIFF_DIRECTORY: &str = "tests/snapshots_diff";

    #[test]
    fn test_one() {
        let source_lines = vec![String::from("blah")];
        let mut core = Core::new();
        let state = State::new(
            source_lines,
            vec![SearchLine::new(String::from(""), 0, true, false)],
            0,
            0,
            10,
            10,
        );
        let output = core.get_render_state(&state);
        let serialized = format!("{:?}", output);
        assert_eq!(serialized, "RenderState { regex_valid: true, max_y: 10, max_x: 10, input_window_height: 3, pager_window_height: 6, output_search_lines: [SearchLine { line: \"\", context: 0, case_sensitive: true, inverse: false }], output_display_lines: [StringWithColorIndex([String(\"b\"), String(\"l\"), String(\"a\"), String(\"h\")])], status_line: \"matchedLin\" }");
    }

    #[test]
    fn test_context() {
        let source_lines = vec![
            String::from("1"),
            String::from("2"),
            String::from("3"),
            String::from("4"),
        ];
        let mut core = Core::new();
        let state = State::new(
            source_lines,
            vec![SearchLine::new(String::from("1"), 1, true, false)],
            0,
            0,
            10,
            10,
        );
        let output = core.get_render_state(&state);
        let serialized = format!("{:?}", output);
        assert_eq!(serialized, "RenderState { regex_valid: true, max_y: 10, max_x: 10, input_window_height: 3, pager_window_height: 6, output_search_lines: [SearchLine { line: \"1\", context: 1, case_sensitive: true, inverse: false }], output_display_lines: [StringWithColorIndex([MatchString((\"1\", 0))]), StringWithColorIndex([String(\"2\")])], status_line: \"matchedLin\" }");
    }

    #[test]
    fn test_case_sensitivity() {
        let source_lines = vec![
            String::from("ab"),
            String::from("Ab"),
            String::from("aB"),
            String::from("BB"),
        ];
        let state = State::new(
            source_lines,
            vec![
                SearchLine::new(String::from("a"), 0, false, false),
                SearchLine::new(String::from("b"), 0, true, false),
            ],
            0,
            0,
            10,
            10,
        );
        let output = Core::new().get_render_state(&state);
        let serialized = format!("{:?}", output);
        assert_eq!(serialized, "RenderState { regex_valid: true, max_y: 10, max_x: 10, input_window_height: 4, pager_window_height: 5, output_search_lines: [SearchLine { line: \"(?i)a\", context: 0, case_sensitive: false, inverse: false }, SearchLine { line: \"b\", context: 0, case_sensitive: true, inverse: false }], output_display_lines: [StringWithColorIndex([String(\"a\"), MatchString((\"b\", 0))]), StringWithColorIndex([String(\"A\"), MatchString((\"b\", 0))])], status_line: \"matchedLin\" }");
    }

    #[test]
    fn test_inverse_searching() {
        let source_lines = vec![
            String::from("ab"),
            String::from("Ab"),
            String::from("aB"),
            String::from("BB"),
            String::from("c"),
        ];
        let state = State::new(
            source_lines,
            vec![
                SearchLine::new(String::from("a"), 0, false, true),
                SearchLine::new(String::from("b"), 0, true, true),
            ],
            0,
            0,
            10,
            10,
        );
        let output = Core::new().get_render_state(&state);
        let serialized = format!("{:?}", output);
        assert_eq!(serialized, "RenderState { regex_valid: true, max_y: 10, max_x: 10, input_window_height: 4, pager_window_height: 5, output_search_lines: [SearchLine { line: \"(?i)a\", context: 0, case_sensitive: false, inverse: true }, SearchLine { line: \"b\", context: 0, case_sensitive: true, inverse: true }], output_display_lines: [StringWithColorIndex([String(\"BB\")]), StringWithColorIndex([String(\"c\")])], status_line: \"matchedLin\" }");
    }

    #[test]
    fn snapshot_tests() {
        fs::create_dir_all(SNAPSHOT_DIRECTORY).unwrap();
        fs::create_dir_all(SNAPSHOT_DIFF_DIRECTORY).unwrap();
        let paths = fs::read_dir(SNAPSHOT_DIFF_DIRECTORY).unwrap();
        for path in paths {
            let dir_entry = path.unwrap();
            if dir_entry
                .file_name()
                .into_string()
                .unwrap()
                .ends_with(".snapshot.json")
            {
                fs::remove_file(dir_entry.path()).unwrap();
            }
        }

        let source_lines_list = [
            vec![String::from("")],
            vec![String::from("blah")],
            vec![
                String::from("one"),
                String::from("two"),
                String::from("three"),
            ],
            (0..100).map(|i| format!("{}", i)).collect::<Vec<String>>(),
        ];
        let search_lines_list = vec![
            vec![String::from("")],
            vec![String::from(".")],
            vec![String::from("b")],
            vec![String::from("t")],
            vec![String::from("t...")],
            vec![String::from("t"), String::from("o")],
            vec![String::from("2")],
            vec![String::from("2"), String::from("3")],
            vec![String::from("\\")], // invalid regex
        ];
        let mut test_results: HashMap<String, String> = HashMap::new();
        for case_sensitive in [true, false].iter() {
            source_lines_list.iter().for_each(|source_lines| {
                search_lines_list.iter().for_each(|search_lines| {
                    let search_lines_with_context: Vec<SearchLine> = search_lines
                        .iter()
                        .map(|l| SearchLine::new(l.clone(), 0, *case_sensitive, false))
                        .collect();
                    let state = State::new(
                        source_lines.clone(),
                        search_lines_with_context.clone(),
                        0,
                        0,
                        10,
                        10,
                    );
                    let output = Core::new().get_render_state(&state);
                    let serialized = format!("{:?}", output);
                    test_results.insert(format!("{:?}", state), serialized);
                });
            });
        }
        let test_name = "core";
        let expected = read_map_from_disk(test_name);
        match expected {
            Ok(e) => {
                if e != test_results {
                    write_to_disk(test_name, &test_results).unwrap();
                    let expected_keys: HashSet<String> = e.keys().cloned().collect();
                    let actual_keys: HashSet<String> = test_results.keys().cloned().collect();
                    let missing_keys: HashSet<String> =
                        expected_keys.difference(&actual_keys).cloned().collect();
                    let unexpected_keys: HashSet<String> =
                        actual_keys.difference(&expected_keys).cloned().collect();
                    assert!(
                        missing_keys.is_empty(),
                        "Missing keys in test result: {:?}",
                        missing_keys
                            .iter()
                            .cloned()
                            .collect::<Vec<String>>()
                            .join(", ")
                    );
                    assert!(
                        unexpected_keys.is_empty(),
                        "Unexpected keys in test result: {:?}",
                        unexpected_keys
                            .iter()
                            .cloned()
                            .collect::<Vec<String>>()
                            .join(", ")
                    );
                    expected_keys.iter().for_each(|k| {
                        assert_eq!(
                            e.get(k),
                            test_results.get(k),
                            "Snapshot mismatch for key '{}'",
                            k
                        )
                    })
                }
            }
            Err(e) => {
                println!("Error reading existing snapshot, writing new: {:?}", e);
                write_to_disk(test_name, &test_results).unwrap();
                assert!(false, "No snapshot found");
            }
        }
    }

    fn read_map_from_disk(test_name: &str) -> Result<HashMap<String, String>, io::Error> {
        let mut file = File::open(format!(
            "{}/{}.snapshot.json",
            SNAPSHOT_DIRECTORY, test_name
        ))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let deserialized: HashMap<String, String> = serde_json::from_str(&contents).unwrap();
        Ok(deserialized)
    }

    fn write_to_disk(
        test_name: &str,
        test_results: &HashMap<String, String>,
    ) -> Result<(), io::Error> {
        let mut file = File::create(format!(
            "{}/{}.snapshot.json",
            SNAPSHOT_DIFF_DIRECTORY, test_name
        ))?;
        file.write_all(
            serde_json::to_string_pretty(&test_results)
                .unwrap()
                .as_ref(),
        )?;
        Ok(())
    }
}
