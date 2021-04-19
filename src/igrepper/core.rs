use crate::igrepper::output_generator::{Len, OutputGenerator};
use crate::igrepper::state::State;
use crate::igrepper::trimming::produce_render_state;
use crate::igrepper::types::RenderState;
use std::collections::HashMap;

#[derive(Debug)]
struct CacheEntry {
    pub search_line: String,
    pub output_generator: OutputGenerator,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct CacheKey {
    search_lines: u32,
    context: u32,
    active_regex: String,
}

#[derive(Debug)]
pub struct Core {
    cache: HashMap<CacheKey, CacheEntry>,
}

fn get_cache_key(state: &State) -> CacheKey {
    CacheKey {
        search_lines: state.search_line_strings_with_case_sensitivity().len() as u32,
        context: state.current_context(),
        active_regex: state.last_valid_regex().to_string(),
    }
}

impl Core {
    pub fn new() -> Core {
        Core {
            cache: HashMap::new(),
        }
    }

    pub fn get_full_output_string(&mut self, state: &State) -> String {
        self.get_output_generator(state).full_string()
    }

    pub fn widest_line_seen_so_far(&mut self, state: &State) -> u32 {
        self.get_output_generator(state).widest_line_seen_so_far()
    }

    pub fn is_output_length_at_least(&mut self, state: &State, length: u32) -> u32 {
        let output_generator = self.get_output_generator(state);
        output_generator.request(length);
        output_generator.len_simple()
    }

    pub fn get_current_output_length(&mut self, state: &State) -> Len {
        self.get_output_generator(state).len()
    }

    pub fn get_render_state(&mut self, state: &State) -> RenderState {
        let output_generator = self.get_output_generator(state);
        return produce_render_state(
            state.regex_valid(),
            state.max_y(),
            state.max_x(),
            state.pager_y(),
            state.pager_x(),
            &state.search_line_strings_with_case_sensitivity(),
            state.current_context(),
            output_generator,
        );
    }

    fn get_output_generator(&mut self, state: &State) -> &mut OutputGenerator {
        self.populate_cache(&state);
        &mut self
            .cache
            .get_mut(&get_cache_key(&state))
            .unwrap()
            .output_generator
    }

    fn populate_cache(&mut self, state: &State) {
        let first_line = state.search_line_strings().len() == 1;
        if !first_line {
            self.populate_cache(&state.clone().revert_partial_match());
        }

        let s = state.search_line_strings().last().unwrap().clone();
        let maybe_cache_entry = self.cache.get_mut(&get_cache_key(&state));

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
            let source_lines: Vec<String> = if first_line {
                state.source_lines().to_vec()
            } else {
                self.cache
                    .get_mut(&get_cache_key(&state.clone().revert_partial_match()))
                    .unwrap()
                    .output_generator
                    .full_string_vec()
            };
            let output_generator = OutputGenerator::new(
                source_lines.clone(),
                state.last_valid_regex(),
                state.current_context(),
            );
            self.cache.insert(
                get_cache_key(&state),
                CacheEntry {
                    search_line: s,
                    output_generator,
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
            &source_lines,
            vec![SearchLine::new(String::from(""), 0, true)],
            0,
            0,
            10,
            10,
        );
        let output = core.get_render_state(&state);
        let serialized = format!("{:?}", output);
        assert_eq!(serialized, "RenderState { regex_valid: true, max_y: 10, max_x: 10, input_window_height: 3, pager_window_height: 6, output_search_lines: [\"\"], output_display_lines: [StringWithColorIndex([String(\"b\"), String(\"l\"), String(\"a\"), String(\"h\")])], status_line: \"matchedLin\" }");
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
            &source_lines,
            vec![SearchLine::new(String::from("1"), 1, true)],
            0,
            0,
            10,
            10,
        );
        let output = core.get_render_state(&state);
        let serialized = format!("{:?}", output);
        assert_eq!(serialized, "RenderState { regex_valid: true, max_y: 10, max_x: 10, input_window_height: 3, pager_window_height: 6, output_search_lines: [\"1\"], output_display_lines: [StringWithColorIndex([MatchString((\"1\", 0))]), StringWithColorIndex([String(\"2\")])], status_line: \"matchedLin\" }");
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
            &source_lines,
            vec![
                SearchLine::new(String::from("a"), 0, false),
                SearchLine::new(String::from("b"), 0, true),
            ],
            0,
            0,
            10,
            10,
        );
        let output = Core::new().get_render_state(&state);
        let serialized = format!("{:?}", output);
        assert_eq!(serialized, "RenderState { regex_valid: true, max_y: 10, max_x: 10, input_window_height: 4, pager_window_height: 5, output_search_lines: [\"(?i)a\", \"b\"], output_display_lines: [StringWithColorIndex([String(\"a\"), MatchString((\"b\", 0))]), StringWithColorIndex([String(\"A\"), MatchString((\"b\", 0))])], status_line: \"matchedLin\" }");
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

        let source_lines_list = vec![
            vec![String::from("")],
            vec![String::from("blah")],
            vec![
                String::from("one"),
                String::from("two"),
                String::from("three"),
            ],
            (0..100)
                .map(|i| String::from(format!("{}", i)))
                .collect::<Vec<String>>(),
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
        for case_sensitive in vec![true, false].iter() {
            source_lines_list.iter().for_each(|source_lines| {
                search_lines_list.iter().for_each(|search_lines| {
                    let search_lines_with_context: Vec<SearchLine> = search_lines
                        .iter()
                        .map(|l| SearchLine::new(l.clone(), 0, case_sensitive.clone()))
                        .collect();
                    let state = State::new(
                        &source_lines,
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
                        format!(
                            "Missing keys in test result: {:?}",
                            missing_keys
                                .iter()
                                .cloned()
                                .collect::<Vec<String>>()
                                .join(", ")
                        )
                    );
                    assert!(
                        unexpected_keys.is_empty(),
                        format!(
                            "Unexpected keys in test result: {:?}",
                            unexpected_keys
                                .iter()
                                .cloned()
                                .collect::<Vec<String>>()
                                .join(", ")
                        )
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
