extern crate ncurses;

use crate::igrepper::constants::*;
use crate::igrepper::types::{RenderState, StringWithColorIndex, StringWithColorIndexOrBreakLine};
use ncurses::{
    box_, chtype, getmaxyx, mvaddstr, mvwaddstr, mvwhline, newwin, stdscr, wattroff, wattron,
    wbkgd, wrefresh, A_BOLD, A_REVERSE, COLOR_PAIR,
};

pub fn clear_screen() {
    let mut y: i32 = 0;
    let mut x: i32 = 0;
    getmaxyx(stdscr(), &mut y, &mut x);
    let input_window = newwin(y as i32, x as i32, 0, 0);
    wbkgd(
        input_window,
        ' ' as chtype | COLOR_PAIR(COLOR_PAIR_DEFAULT) as chtype,
    );
    wrefresh(input_window);
}

/// Renders the result to the screen using ncurses
pub fn render(render_state: RenderState) {
    if render_state.max_x < 5 || render_state.max_y < 5 {
        mvaddstr(0, 0, "Window too small");
        return;
    }

    // Input window

    let input_window = newwin(
        render_state.input_window_height as i32,
        render_state.max_x as i32,
        0,
        0,
    );
    wbkgd(
        input_window,
        ' ' as chtype | COLOR_PAIR(COLOR_PAIR_DEFAULT) as chtype,
    );
    wattron(input_window, COLOR_PAIR(COLOR_PAIR_BORDER));
    box_(input_window, 0, 0);
    wattroff(input_window, COLOR_PAIR(COLOR_PAIR_BORDER));

    for (i, search_line) in render_state.output_search_lines.iter().enumerate() {
        let mut line: &str = search_line.line.as_str();
        let mut x_start = 1i32;
        if search_line.inverse {
            wattron(input_window, COLOR_PAIR(COLOR_PAIR_BORDER));
            mvwaddstr(input_window, i as i32 + 1, x_start, "!");
            wattroff(input_window, COLOR_PAIR(COLOR_PAIR_BORDER));
            x_start += 1;
        }

        if line.starts_with(CASE_INSENSITIVE_PREFIX) {
            wattron(input_window, COLOR_PAIR(COLOR_PAIR_BORDER));
            mvwaddstr(input_window, i as i32 + 1, x_start, CASE_INSENSITIVE_PREFIX);
            wattroff(input_window, COLOR_PAIR(COLOR_PAIR_BORDER));
            x_start += CASE_INSENSITIVE_PREFIX.len() as i32;
            line = &line[4..line.len()];
        }
        if i == render_state.output_search_lines.len() - 1 {
            if render_state.regex_valid {
                wattron(input_window, A_BOLD());
            } else {
                wattron(input_window, COLOR_PAIR(COLOR_PAIR_RED));
            }
        } else {
            wattron(input_window, COLOR_PAIR(COLOR_PAIR_INACTIVE_INPUT));
        }
        if search_line.inverse {
            wattron(input_window, A_REVERSE());
        }

        mvwaddstr(input_window, i as i32 + 1, x_start, line);
        if i == render_state.output_search_lines.len() - 1 {
            if render_state.regex_valid {
                wattroff(input_window, A_BOLD());
            } else {
                wattroff(input_window, COLOR_PAIR(COLOR_PAIR_RED));
            }
        } else {
            wattroff(input_window, COLOR_PAIR(COLOR_PAIR_INACTIVE_INPUT));
        }
        if search_line.inverse {
            wattroff(input_window, A_REVERSE());
        }
    }
    wrefresh(input_window);

    // Pager window

    let pager_window = newwin(
        render_state.pager_window_height as i32,
        render_state.max_x as i32,
        render_state.input_window_height as i32,
        0,
    );
    wbkgd(
        pager_window,
        ' ' as chtype | COLOR_PAIR(COLOR_PAIR_DEFAULT) as chtype,
    );
    wattron(pager_window, COLOR_PAIR(COLOR_PAIR_BORDER));
    box_(pager_window, 0, 0);
    wattroff(pager_window, COLOR_PAIR(COLOR_PAIR_BORDER));
    for (i, line) in render_state.output_display_lines.iter().enumerate() {
        let mut xpos: i32 = 1;
        match line {
            StringWithColorIndexOrBreakLine::StringWithColorIndex(real_line) => {
                for line_part in real_line {
                    match line_part {
                        StringWithColorIndex::String(s) => {
                            mvwaddstr(pager_window, i as i32 + 1, xpos, &s);
                            xpos += s.len() as i32;
                        }
                        StringWithColorIndex::MatchString(s) => {
                            wattron(input_window, A_BOLD());
                            wattron(pager_window, COLOR_PAIR(s.1 as i16 + 1));
                            mvwaddstr(pager_window, i as i32 + 1, xpos, &s.0);
                            wattroff(pager_window, COLOR_PAIR(s.1 as i16 + 1));
                            wattroff(input_window, A_BOLD());
                            xpos += s.0.len() as i32;
                        }
                    }
                }
            }
            StringWithColorIndexOrBreakLine::BreakLine => {
                wattron(pager_window, COLOR_PAIR(COLOR_PAIR_BORDER));
                let line_char: chtype = ('-' as u32).into();
                mvwhline(
                    pager_window,
                    i as i32 + 1,
                    1,
                    line_char,
                    (render_state.max_x - 2) as i32,
                );
                wattroff(pager_window, COLOR_PAIR(COLOR_PAIR_BORDER));
            }
        }
    }
    wrefresh(pager_window);
    let status_window = newwin(
        1,
        render_state.max_x as i32,
        (render_state.input_window_height + render_state.pager_window_height) as i32,
        0,
    );
    wbkgd(
        status_window,
        ' ' as chtype | COLOR_PAIR(COLOR_PAIR_DEFAULT) as chtype,
    );
    mvwaddstr(status_window, 0, 0, &*render_state.status_line);
    wrefresh(status_window);
}
