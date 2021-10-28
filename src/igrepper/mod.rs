use ncurses::{
    curs_set, endwin, getch, getmaxyx, init_pair, initscr, keypad, noecho, raw, refresh,
    start_color, stdscr, CURSOR_VISIBILITY, KEY_BACKSPACE, KEY_DOWN, KEY_ENTER, KEY_LEFT,
    KEY_NPAGE, KEY_PPAGE, KEY_RESIZE, KEY_RIGHT, KEY_UP,
};
use std::cmp;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::{char, thread};

extern crate ncurses;
extern crate regex;

mod constants;
mod core;
pub mod output_generator;
pub mod rendering;
pub mod state;
mod trimming;
mod types;

use crate::file_reading::SourceProducer;
use crate::igrepper::constants::*;
use crate::igrepper::core::Core;
use crate::igrepper::output_generator::Len;
use crate::igrepper::rendering::clear_screen;
use crate::igrepper::state::{SearchLine, State};
use inotify::Inotify;

pub enum Message {
    Character(i32),
    ReloadFile,
    ErrorMessage(String),
}

pub enum CharRequesterMessage {
    ReadyToReceiveChar,
    Exit,
}

fn get_screen_size() -> (u32, u32) {
    let mut y: i32 = 0;
    let mut x: i32 = 0;
    getmaxyx(stdscr(), &mut y, &mut x);
    (y as u32, x as u32)
}

pub fn igrepper(
    source_producer: SourceProducer,
    initial_context: u32,
    initial_regex: Option<&str>,
    inotify_option: Option<Inotify>,
    external_editor: Vec<String>,
) {
    // Setup ncurses
    initscr();
    raw();
    keypad(stdscr(), true);
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    start_color();
    init_pair(COLOR_PAIR_DEFAULT, 231i16, 232i16);
    init_pair(COLOR_PAIR_RED, 1i16, 232i16);
    init_pair(COLOR_PAIR_ACTIVE_INPUT, 1i16, 7i16);
    init_pair(COLOR_PAIR_INACTIVE_INPUT, 248i16, 232i16);
    init_pair(COLOR_PAIR_BORDER, 8i16, 232i16);

    for (i, c) in MATCH_COLORS.iter().enumerate() {
        init_pair(i as i16 + 1, c.clone(), 232i16);
    }

    refresh();

    let (max_y, max_x) = get_screen_size();

    let mut core = core::Core::new();
    let mut state = state::State::new(
        source_producer.get_source(),
        vec![SearchLine::new(
            String::from(initial_regex.unwrap_or("")),
            initial_context,
            false,
            false,
        )],
        0,
        0,
        max_y,
        max_x,
    );
    let (tx, rx) = mpsc::channel();
    let (char_requester_tx, char_requester_rx) = mpsc::channel();

    if let Some(mut inotify) = inotify_option {
        let inotify_tx = tx.clone();
        thread::spawn(move || {
            let mut buffer = [0; 1024];

            loop {
                let events_result = inotify.read_events_blocking(&mut buffer);
                match events_result {
                    Ok(events) => {
                        if events.count() > 0 {
                            inotify_tx.send(Message::ReloadFile).unwrap();
                        }
                    }
                    Err(e) => {
                        inotify_tx
                            .send(Message::ErrorMessage(e.to_string()))
                            .unwrap();
                    }
                }
            }
        });
    }

    thread::spawn(move || loop {
        match char_requester_rx
            .recv()
            .unwrap_or(CharRequesterMessage::Exit)
        {
            CharRequesterMessage::ReadyToReceiveChar => {
                let ch = getch();
                tx.send(Message::Character(ch)).unwrap();
            }
            CharRequesterMessage::Exit => {
                break;
            }
        }
    });

    loop {
        let render_state = core.get_render_state(&state);
        rendering::render(render_state);
        refresh();
        char_requester_tx
            .send(CharRequesterMessage::ReadyToReceiveChar)
            .unwrap();
        let message = rx.recv().unwrap();
        match message {
            Message::ReloadFile => {
                state = state.set_source_lines(source_producer.get_source());
                core.clear_cache();
            }
            Message::ErrorMessage(message) => {
                panic!("Inotify error: {}", message);
            }
            Message::Character(ch) => match ch {
                KEY_LEFT => {
                    state = {
                        let widest = core.widest_line_seen_so_far(&state);
                        state.page_x(-5, widest)
                    }
                }
                KEY_RIGHT => {
                    state = {
                        let widest = core.widest_line_seen_so_far(&state);
                        state.page_x(5, widest)
                    }
                }
                KEY_UP => state = page_y(-1, state, &mut core),
                KEY_DOWN => state = page_y(1, state, &mut core),

                3 => {
                    clear_screen();
                    endwin();
                    break;
                }
                KEY_PPAGE => {
                    state = {
                        let y = state.max_y() as i32;
                        page_y(-y, state, &mut core)
                    }
                }
                KEY_NPAGE => {
                    state = {
                        let y = state.max_y() as i32;
                        page_y(y, state, &mut core)
                    }
                }
                CTRL_U => {
                    state = {
                        let y = state.max_y() as i32;
                        page_y(-y / 2, state, &mut core)
                    }
                }
                CTRL_D => {
                    state = {
                        let y = state.max_y() as i32;
                        page_y(y / 2, state, &mut core)
                    }
                }
                CTRL_L | KEY_RESIZE => {
                    let (max_y, max_x) = get_screen_size();
                    state = state.set_max_yx(max_y, max_x);
                    refresh();
                }
                CTRL_R => {
                    state = state.modify_context(-1);
                }
                CTRL_T => {
                    state = state.modify_context(1);
                }
                CTRL_N | KEY_ENTER | 0xa => {
                    state = state.accept_partial_match();
                }
                CTRL_P => {
                    state = state.revert_partial_match();
                }
                CTRL_I => {
                    state = state.toggle_case_sensitivity();
                }
                CTRL_V => {
                    state = state.toggle_inverted();
                }
                CTRL_G => {
                    if !state.regex_valid() || state.empty_search_lines() {
                        continue;
                    }
                    clear_screen();
                    endwin();
                    copy_grep_to_clipboard(&state.search_lines());
                    break;
                }
                CTRL_E => {
                    if !state.regex_valid() {
                        continue;
                    }
                    clear_screen();
                    endwin();
                    copy_full_to_clipboard_from_string(&core.get_full_output_string(&state));
                    break;
                }
                F1 | F1_2 => {
                    if !state.regex_valid() {
                        continue;
                    }
                    clear_screen();
                    endwin();
                    pipe_to_external_editor(external_editor, &core.get_full_output_string(&state));
                    break;
                }
                CTRL_H | KEY_BACKSPACE | ALTERNATIVE_BACKSPACE => {
                    state = state.pop_search_char();
                    state = page_y(0, state, &mut core)
                }
                c => {
                    if let Some(new_char) = char::from_u32(c as u32) {
                        state = state.push_search_char(new_char);
                        state = page_y(0, state, &mut core)
                    }
                }
            },
        }
    }
}

/// Tries to page vertically, may query more output lines.
fn page_y(amount: i32, s: State, c: &mut Core) -> State {
    let wanted_ypage = cmp::max(0, s.pager_y() as i32 + amount) as u32;
    let mut output_lines_count: u32;

    let need_to_query = match c.get_current_output_length(&s) {
        Len::Is(n) => {
            output_lines_count = n;
            false
        }
        Len::AtLeast(n) => {
            output_lines_count = n;
            n < wanted_ypage
        }
    };

    if need_to_query {
        output_lines_count = c.is_output_length_at_least(&s, wanted_ypage + s.max_y());
    }
    s.page_y(amount, output_lines_count)
}

fn copy_grep_to_clipboard(search_lines: &Vec<SearchLine>) -> () {
    let grep_line = construct_grep_line(search_lines);
    copy_to_clipboard(&grep_line);
    print_copied_to_clipboard(grep_line);
}

fn construct_grep_line(search_lines: &Vec<SearchLine>) -> String {
    search_lines
        .iter()
        .filter(|l| !l.line.is_empty())
        .map(|l| {
            format!(
                "{grep}{context}{inverted} --perl-regexp '{regex}'",
                grep = grep_path(),
                context = if l.context > 0 && !l.inverse {
                    format!(" --context {}", l.context)
                } else {
                    String::from("")
                },
                regex = l.line_with_sensitivity_prefix().replace("'", "'\\''"),
                inverted = if l.inverse { " -v" } else { "" }
            )
        })
        .collect::<Vec<String>>()
        .join(" | ")
}

fn grep_path() -> String {
    return "grep".to_string();
}

fn copy_to_clipboard(string: &String) -> () {
    let mut child_process = Command::new("xsel")
        .arg("--clipboard")
        .arg("--input")
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to copy to clipboard");

    child_process
        .stdin
        .as_mut()
        .unwrap()
        .write_all(string.as_bytes())
        .unwrap();
    child_process.wait().unwrap();
}

fn pipe_to_external_editor(command_and_arguments: Vec<String>, string: &String) {
    let command_path = command_and_arguments.first().unwrap();
    let command_arguments = &command_and_arguments[1..command_and_arguments.len()];

    let mut command = Command::new(command_path);
    let mut command = &mut command;
    for arg in command_arguments {
        command = command.arg(arg);
    }
    let error_message = format!(
        "Failed to pipe to external editor '{}'",
        command_and_arguments.join(" ")
    );
    let mut child_process = command.stdin(Stdio::piped()).spawn().expect(&error_message);

    child_process
        .stdin
        .as_mut()
        .expect(&error_message)
        .write_all(string.as_bytes())
        .expect(&error_message);
    child_process.wait().expect(&error_message);
}

fn copy_full_to_clipboard_from_string(string_to_copy: &String) -> () {
    copy_to_clipboard(&string_to_copy);
    print_copied_to_clipboard(string_to_copy.clone());
}

fn print_copied_to_clipboard(string: String) {
    macro_rules! copied_to_clipboard {
        () => {
            "Copied to clipboard: \n\n"
        };
    }
    macro_rules! bold {
        () => {
            "\x1b[0;1m"
        };
    }
    macro_rules! inverted {
        () => {
            "\x1b[0;7m"
        };
    }
    macro_rules! reset {
        () => {
            "\x1b[0;0m"
        };
    }
    macro_rules! variable {
        () => {
            "{}"
        };
    }
    macro_rules! newline {
        () => {
            "\n"
        };
    }

    println!(
        concat!(
            copied_to_clipboard!(),
            bold!(),
            inverted!(),
            variable!(),
            reset!(),
            newline!()
        ),
        string
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn construct_grep_one_line() {
        let search_lines: Vec<SearchLine> =
            vec![SearchLine::new("foo".to_string(), 0, false, false)];
        assert_eq!(
            construct_grep_line(&search_lines),
            "grep --perl-regexp \'(?i)foo\'"
        );
    }

    #[test]
    fn construct_grep_case_sensitive() {
        let search_lines: Vec<SearchLine> =
            vec![SearchLine::new("foo".to_string(), 0, true, false)];
        assert_eq!(
            construct_grep_line(&search_lines),
            "grep --perl-regexp \'foo\'"
        );
    }

    #[test]
    fn construct_grep_inverted() {
        let search_lines: Vec<SearchLine> =
            vec![SearchLine::new("foo".to_string(), 0, false, true)];
        assert_eq!(
            construct_grep_line(&search_lines),
            "grep -v --perl-regexp \'(?i)foo\'"
        );
    }

    #[test]
    fn construct_grep_sensitive_and_inverted() {
        let search_lines: Vec<SearchLine> = vec![SearchLine::new("foo".to_string(), 0, true, true)];
        assert_eq!(
            construct_grep_line(&search_lines),
            "grep -v --perl-regexp \'foo\'"
        );
    }

    #[test]
    fn construct_grep_context() {
        let search_lines: Vec<SearchLine> =
            vec![SearchLine::new("foo".to_string(), 2, false, false)];
        assert_eq!(
            construct_grep_line(&search_lines),
            "grep --context 2 --perl-regexp \'(?i)foo\'"
        );
    }

    #[test]
    fn construct_grep_context_is_ignored_when_inverted() {
        let search_lines: Vec<SearchLine> =
            vec![SearchLine::new("foo".to_string(), 2, false, true)];
        assert_eq!(
            construct_grep_line(&search_lines),
            "grep -v --perl-regexp \'(?i)foo\'"
        );
    }

    #[test]
    fn construct_grep_multiple_lines() {
        let search_lines: Vec<SearchLine> = vec![
            SearchLine::new("foo".to_string(), 0, false, false),
            SearchLine::new("bar".to_string(), 1, true, false),
        ];
        assert_eq!(
            construct_grep_line(&search_lines),
            "grep --perl-regexp \'(?i)foo\' | grep --context 1 --perl-regexp \'bar\'"
        );
    }

    #[test]
    fn construct_grep_with_single_quote() {
        let search_lines: Vec<SearchLine> =
            vec![SearchLine::new("isn't".to_string(), 0, false, false)];
        assert_eq!(
            construct_grep_line(&search_lines),
            "grep --perl-regexp \'(?i)isn\'\\\'\'t\'"
        );
    }
}
