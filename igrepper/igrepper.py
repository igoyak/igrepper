#!/usr/bin/env python3

import re
import curses
from enum import Enum
from subprocess import Popen, PIPE
import os
import sys
import argparse

BOLD = '\033[1m'
INVERTED = '\033[7m'
RESET = '\033[0m'
AVAILABLE_COLORS = 6

FOOTER_LINE_NO = 2

WORD_REGEX = '\\S+'

CTRL_G = 7
CTRL_H = 8
CTRL_I = 9
CTRL_K = 11
CTRL_N = 14
CTRL_O = 15
CTRL_P = 16
CTRL_J = 10
CTRL_Y = 25


class DisplayMode(Enum):
    show_default = 1
    show_all = 2
    show_only_match = 3
    show_only_unique = 4  # each unique on separate line


def copy_to_clipboard(string):
    p = Popen(['xsel', '-bi'], stdin=PIPE)
    p.communicate(input=string.encode('utf-8'))


class Match:
    def __init__(self, regex_match):
        self.text = regex_match.group()
        self.start = regex_match.start()
        self.end = regex_match.end()
        self.unique_id = None


class Search:
    """
    Object created from input, keeps track of regex, matches, settings
    """

    def __init__(self, input_lines):
        self.valid = True
        self.lines = []
        self._input_lines = input_lines
        self.match_count = 0
        self.number_of_matched_lines = 0
        self.unique_match_count = 0
        self.selected_match = 0
        self.unique_matches = []
        self.previous_searches = []
        self.ignore_case = True
        self.regex = ''
        self.matched_lines = []
        self.line_highlights = []

    def is_empty(self):
        return len(self.regex) == 0

    def next_match(self):
        if self.unique_matches:
            self.selected_match = (self.selected_match + 1) % self.unique_match_count

    def prev_match(self):
        if self.unique_matches:
            self.selected_match = (self.selected_match - 1 + self.unique_match_count) % self.unique_match_count

    def output_lines(self):
        if self.is_empty():
            return self._input_lines
        if not self.valid:
            raise Exception()
        matched_lines = []
        for lineno, line in enumerate(self.lines):
            if line:
                matched_lines.append(self._input_lines[lineno])
        return matched_lines

    def update(self, regex):
        self.regex = regex
        if self.is_empty():
            self.matched_lines = self._input_lines
            self.line_highlights = []
            return
        try:
            if self.ignore_case:
                rg = re.compile(self.regex, re.IGNORECASE)
            else:
                rg = re.compile(self.regex)
            self.valid = True
        except re.error:
            self.valid = False
            return
        # lines: list of lines. Each line is a list of matches
        self.lines = [[Match(x) for x in rg.finditer(line)] for line in self._input_lines]
        matched_lines_dict = {}  # map input line numbers to matched lines
        line_highlights_dict = {}
        for line_number, line in enumerate(self.lines):
            if line:
                matched_lines_dict[line_number] = self._input_lines[line_number]
                line_highlights_dict[line_number] = line

        self.matched_lines = [matched_lines_dict[x] for x in sorted(matched_lines_dict.keys())]
        self.line_highlights = [line_highlights_dict[x] for x in sorted(line_highlights_dict.keys())]
        matches = [match for line in self.lines for match in line]
        nonempty_matches = [match for match in matches if match]
        self.unique_matches = []
        for match in nonempty_matches:
            if match.text in self.unique_matches:
                match.unique_id = self.unique_matches.index(match.text)
            else:
                self.unique_matches.append(match.text)
                match.unique_id = len(self.unique_matches) - 1

        self.match_count = len(nonempty_matches)
        self.unique_match_count = len(self.unique_matches)
        self.number_of_matched_lines = len([x for x in self.lines if x])


class IGrepper:
    def __init__(self, input_data, regex=''):
        self.win = curses.initscr()
        self.win.keypad(1)
        curses.start_color()

        curses.init_pair(1, curses.COLOR_RED, curses.COLOR_BLACK)
        curses.init_pair(2, curses.COLOR_GREEN, curses.COLOR_BLACK)
        curses.init_pair(3, curses.COLOR_YELLOW, curses.COLOR_BLACK)
        curses.init_pair(4, curses.COLOR_BLUE, curses.COLOR_BLACK)
        curses.init_pair(5, curses.COLOR_MAGENTA, curses.COLOR_BLACK)
        curses.init_pair(6, curses.COLOR_CYAN, curses.COLOR_BLACK)
        curses.init_pair(30, curses.COLOR_RED, curses.COLOR_WHITE)
        curses.init_pair(31, curses.COLOR_BLACK, curses.COLOR_WHITE)
        curses.use_default_colors()

        self.regex = regex
        self.input_lines = input_data
        self.display_mode = DisplayMode.show_default
        self.pager_ypos = 0
        self.pager_xpos = 0
        self.search = Search(self.input_lines)
        self.quit = False

    def toggle_display_mode(self):
        if self.display_mode == DisplayMode.show_default:
            self.display_mode = DisplayMode.show_only_match
        elif self.display_mode == DisplayMode.show_only_match:
            self.display_mode = DisplayMode.show_all
        elif self.display_mode == DisplayMode.show_all:
            self.display_mode = DisplayMode.show_default

    def run(self):
        while not self.quit:
            self.search.update(self.regex)
            self.win.erase()
            render(search=self.search,
                   window=self.win,
                   pager_ypos=self.pager_ypos,
                   pager_xpos=self.pager_xpos)
            self.win.refresh()
            char = self.win.getch()
            self.process_char(char)

    def process_char(self, char):
        if char in (CTRL_H, curses.KEY_BACKSPACE):  # backspace, ctrl-h
            if len(self.regex) > 0:
                self.regex = self.regex[:-1]
        elif char == curses.KEY_RESIZE:
            pass  # The curses window has been resized, so re-render it
        elif char == CTRL_N:  # ctrl-n
            if self.search.valid:
                previous_match_objects = self.search.previous_searches + [self.search]
                self.search = Search(self.search.output_lines())
                self.search.previous_searches = previous_match_objects
                self.regex = ''
        elif char == CTRL_P:  # ctrl-p
            if self.search.previous_searches:
                self.search = self.search.previous_searches.pop()
                self.regex = self.search.regex
        elif char == CTRL_O:
            self.toggle_display_mode()
        elif char == CTRL_J:  # ctrl-j
            self.search.next_match()
        elif char == CTRL_K:  # ctrl-k
            self.search.prev_match()
        elif char == CTRL_G:  # ctrl-g
            self.win.erase()
            self.win.refresh()
            curses.endwin()
            grep_commands = []
            for m in self.search.previous_searches + [self.search]:
                grep_cmd = "grep {}'{}'".format('-i ' if m.ignore_case else '', m.regex.replace("'", "\\'"))
                grep_commands.append(grep_cmd)
            to_yank = ' | '.join(grep_commands)
            copy_to_clipboard(to_yank)
            print('Copied to clipboard: ' + BOLD + INVERTED + to_yank + RESET)
            self.quit = True
        elif char == CTRL_Y:  # ctrl-y
            if not self.search.unique_matches:
                return
            self.win.erase()
            self.win.refresh()
            curses.endwin()
            to_yank = self.search.unique_matches[self.search.selected_match]
            if to_yank:
                copy_to_clipboard(to_yank)
                print('Copied to clipboard: ' + BOLD + INVERTED + to_yank + RESET)
                self.quit = True
        elif char == CTRL_I:  # ctrl-i
            self.search.ignore_case = not self.search.ignore_case
        elif char == curses.KEY_DOWN:
            self.pager_ypos += 1
        elif char == curses.KEY_NPAGE:
            self.pager_ypos = self.pager_ypos + curses.LINES  # TODO scroll correct amount
        elif char == curses.KEY_UP:
            self.pager_ypos = max(0, self.pager_ypos - 1)
        elif char == curses.KEY_PPAGE:
            self.pager_ypos = max(0, self.pager_ypos - curses.LINES)  # TODO scroll correct amount
        elif char == curses.KEY_RIGHT:
            self.pager_xpos += 1
        elif char == curses.KEY_LEFT:
            self.pager_xpos = max(0, self.pager_xpos - 1)
        else:
            self.regex += chr(char)

        maxy, _ = self.win.getmaxyx()
        header_line_no = len(self.search.previous_searches) + 2
        pager_line_no = maxy - header_line_no - FOOTER_LINE_NO
        max_pager_ypos = max(0, len(self.search.matched_lines) - pager_line_no)
        self.pager_ypos = min(self.pager_ypos, max_pager_ypos)


def render(search, window, pager_ypos, pager_xpos):
    more_lines_after = True
    maxy, maxx = window.getmaxyx()
    footer_div_ypos = maxy - 2
    status_line_ypos = maxy - 1
    header_line_no = len(search.previous_searches) + 2
    pager_line_no = maxy - header_line_no - FOOTER_LINE_NO
    if pager_line_no < 1 or maxx < 5:
        try:
            window.addstr('window too small'[:maxx])
        except:
            pass
        return

    for m in search.previous_searches:
        window.addstr('{}\n'.format(m.regex))

    regex_color = curses.color_pair(0) if search.valid else curses.color_pair(30)
    window.addstr('{}\n'.format(search.regex), regex_color)
    header_char, header_color = ('^', curses.color_pair(31)) if pager_ypos > 0 else ('-', curses.color_pair(0))
    window.addstr(header_char * maxx, header_color)

    # trim output to visible by pager
    matched_lines = search.matched_lines
    line_highlights = search.line_highlights
    max_pager_ypos = max(0, len(matched_lines) - pager_line_no)
    if pager_ypos >= max_pager_ypos:
        more_lines_after = False
    line_highlights = line_highlights[pager_ypos:pager_ypos + pager_line_no]
    lines_to_print_tmp = matched_lines[pager_ypos:pager_ypos + pager_line_no]
    lines_to_print = [line[pager_xpos:pager_xpos + maxx - 1] for line in lines_to_print_tmp]

    # write output
    window.addstr('\n'.join(lines_to_print))

    # highlight output
    for lineno, line_match in enumerate(line_highlights):
        for match in line_match:
            if match.start < pager_xpos + maxx and match.end > pager_xpos:
                if match.unique_id == search.selected_match:
                    color = curses.color_pair(31)
                else:
                    color = curses.color_pair((match.unique_id % AVAILABLE_COLORS) + 1)
                window.chgat(lineno + header_line_no, match.start, match.end - match.start,
                             color | curses.A_BOLD)
    # Footer
    footer_char, footer_color = ('v', curses.color_pair(31)) if more_lines_after else ('-', curses.color_pair(0))
    window.addstr(footer_div_ypos, 0, footer_char * maxx, footer_color)
    case_sensitivity_text = "[case insensitive], " if search.ignore_case else "[case sensitive],   "
    status_line = case_sensitivity_text + \
                  'lines: {}, '.format(search.number_of_matched_lines) + \
                  'matches: {}, '.format(search.match_count) + \
                  'unique: {},              '.format(search.unique_match_count) + \
                  'pag_y: {}, pag_x: {} '.format(pager_ypos, pager_xpos)
    status_line = status_line[:maxx - 1]
    window.addstr(status_line_ypos, 0, status_line)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("file", nargs='?')
    group = parser.add_mutually_exclusive_group()

    group.add_argument("-e", "--regexp", action="store", default='', help="Regular expression to preload")
    group.add_argument("-w", "--word", action="store_true", help="Preload the regular expression '\\S+'")
    # parser.add_argument("-c", "--context", action="store", type=int, help="Print CONTEXT num of output context")

    args = parser.parse_args()

    if sys.stdin.isatty():
        if not args.file:
            print('Data can only be passed by STDIN if no file parameter is specified', file=sys.stderr)
            exit(1)
        with open(args.file) as f:
            input_lines = f.read().split('\n')
    else:
        if args.file:
            print('Data can only be passed by STDIN if no file parameter is specified', file=sys.stderr)
            exit(1)
        # Hack to read stdin from pipe and then from tty
        input_lines = ''.join(sys.stdin.read()).split('\n')
        os.close(sys.stdin.fileno())
        sys.__stdin__ = sys.stdin = open('/dev/tty')

    initial_regex = args.regexp
    if args.word:
        initial_regex = WORD_REGEX
    try:
        f = IGrepper(input_lines, initial_regex)
        f.run()
    finally:
        curses.endwin()


if __name__ == '__main__':
    main()
