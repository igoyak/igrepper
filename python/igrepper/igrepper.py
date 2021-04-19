#!/usr/bin/env python3

import re
import curses
from enum import Enum
from subprocess import run, Popen, PIPE
import os
import sys
import argparse
from typing import List

from pathlib import Path
import logging


BOLD = '\033[1m'
INVERTED = '\033[7m'
RESET = '\033[0m'
AVAILABLE_COLORS = 6
FOOTER_LINE_NO = 2
WORD_REGEX = '\\S+'

CTRL_D = 4
CTRL_G = 7
CTRL_H = 8
CTRL_I = 9
CTRL_J = 10
CTRL_K = 11
CTRL_L = 12
CTRL_N = 14
CTRL_O = 15
CTRL_P = 16
CTRL_R = 18
CTRL_T = 20
CTRL_U = 21
CTRL_V = 22
CTRL_Y = 25     # suspends on mac..
CTRL_9 = 57
CTRL_8 = 263
MAC_BACKSPACE = 127

debug = False

class Logger:
    log_file = '%s.log' % (Path().absolute() / Path(__file__).stem)
    FORMAT = '%(levelname)-5s:%(asctime)-15s:%(module)s.%(funcName)-s() ~ %(message)s'
    logging.basicConfig(filename=log_file, level=logging.INFO, format=FORMAT)
    log = logging.getLogger(__name__)


def log_with_debugging(func):
    """
    Decorator to log exceptions when debug argument is passed in order to
    catch keycodes etc.
    """
    def inner(*args, **kwargs):
        try:
            # TODO e.g. a -d2 argument to change this into debug
            # logging is set to INFO by default
            Logger.log.debug(args)
            return func(*args, **kwargs)
        except:
            if debug:
                Logger.log.exception('Arguments were: %r', args[1:])
            else:
                raise
    return inner


class DisplayMode(Enum):
    show_default = 1
    show_all = 2
    show_only_match = 3
    show_only_unique = 4  # each unique on separate line


def copy_to_clipboard(string):
    print('Copied to clipboard: \n\n' + BOLD + INVERTED + string + RESET + '\n')
    p = Popen(clipboard_cmd(), stdin=PIPE)
    p.communicate(input=string.encode('utf-8'))


def clipboard_cmd() -> list:
    if sys.platform == 'darwin':
        return ['pbcopy']
    else:
        return ['xsel', '-bi']


def grep_cmd() -> list:
    """
    Compatibility helper for macOS which by default doesn't have a GNU flavoured
    grep command and thus ```grep --perl-regexp``` is not a valid option
    """
    grep_cmd = ['grep']
    if sys.platform == 'darwin':
        which_ggrep_query = str(run(['which', 'ggrep'], stdout=PIPE).stdout)
        if 'not found' in which_ggrep_query:
            grep_version_query = str(run(['grep', '--version'], stdout=PIPE).stdout)
            if 'BSD' in grep_version_query:
                print('Your grep is of BSD flavour and doesn\'t support perl type regexp')
                print('( ' + grep_version_query.splitlines()[0] + ' )')
                print('Consider installing GNU grep: brew install grep')
            else:
                print('Unknown grep flavour.. trying \'grep\'')
                print('( ' + grep_version_query.splitlines()[0] + ' )')
        else:
            grep_cmd = ['ggrep']

    return grep_cmd



class Match:
    def __init__(self, regex_match):
        self.text = regex_match.group()
        self.start = regex_match.start()
        self.end = regex_match.end()
        self.unique_id = None


class Line:
    def __init__(self, orig_line_no: int, line_text: str, line_matches: List[Match], break_line=False) -> None:
        self.orig_line_no = orig_line_no
        self.line_text = line_text
        self.line_matches = line_matches
        self.break_line = break_line


class Search:
    """
    Object created from input, keeps track of regex, matches, settings
    """

    def __init__(self, input_lines: List[str], initial_context: int = 0) -> None:
        self.valid = True
        self.lines: List[List[Match]] = []
        self._input_lines = input_lines
        self.match_count = 0
        self.number_of_matched_lines = 0
        self.unique_match_count = 0
        self.selected_match = 0
        self.unique_matches: List[str] = []
        self.previous_searches: List[Search] = []
        self.ignore_case = True
        self.regex = ''
        self.output_lines: List[Line] = []
        self.context: int = initial_context

    def is_empty(self):
        return len(self.regex) == 0

    def next_match(self):
        if self.unique_matches:
            self.selected_match = (self.selected_match + 1) % self.unique_match_count

    def prev_match(self):
        if self.unique_matches:
            self.selected_match = (self.selected_match - 1 + self.unique_match_count) % self.unique_match_count

    def update(self, regex: str):
        self.regex = regex
        if self.is_empty():
            self.output_lines = []
            for orig_line_no, line in enumerate(self._input_lines):
                self.output_lines.append(Line(orig_line_no, line, []))
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
        self.output_lines = []
        output_lines_dict = {}
        self.lines = [[Match(x) for x in rg.finditer(l)] for l in self._input_lines]
        for orig_line_number, matches_on_line in enumerate(self.lines):
            if matches_on_line:
                match_line = Line(orig_line_number, self._input_lines[orig_line_number], matches_on_line)
                output_lines_dict[orig_line_number] = match_line  # Override possible context line with a match line

                # Add context lines, including break lines
                if self.context > 0:
                    first_context_line = True
                    for context_line_no in range(orig_line_number - self.context, orig_line_number + self.context + 1):
                        if context_line_no < 0 or context_line_no >= len(self._input_lines):
                            continue
                        if context_line_no in output_lines_dict:
                            # Already exists
                            continue
                        if first_context_line and len(output_lines_dict) > 1 and context_line_no > 1 \
                                and context_line_no - 2 not in output_lines_dict:
                            # create break
                            b = Line(context_line_no - 1, '', [], break_line=True)
                            output_lines_dict[context_line_no - 1] = b
                        first_context_line = False
                        output_lines_dict[context_line_no] = Line(context_line_no, self._input_lines[context_line_no],
                                                                  [])

        self.output_lines = [output_lines_dict[x] for x in sorted(output_lines_dict.keys())]

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
    def __init__(self, input_lines: List[str], regex='', initial_context=0) -> None:
        self.win = curses.initscr()
        self.win.keypad(True)
        curses.start_color()

        curses.init_pair(1, curses.COLOR_RED, curses.COLOR_BLACK)
        curses.init_pair(2, curses.COLOR_GREEN, curses.COLOR_BLACK)
        curses.init_pair(3, curses.COLOR_YELLOW, curses.COLOR_BLACK)
        curses.init_pair(4, curses.COLOR_BLUE, curses.COLOR_BLACK)
        curses.init_pair(5, curses.COLOR_MAGENTA, curses.COLOR_BLACK)
        curses.init_pair(6, curses.COLOR_CYAN, curses.COLOR_BLACK)
        curses.init_pair(30, curses.COLOR_RED, curses.COLOR_WHITE)
        curses.init_pair(31, curses.COLOR_BLACK, curses.COLOR_WHITE)
        curses.init_pair(32, curses.COLOR_BLACK, curses.COLOR_BLACK)
        curses.use_default_colors()

        self.regex = regex
        self.input_lines = input_lines
        self.display_mode = DisplayMode.show_default
        self.pager_ypos = 0
        self.pager_xpos = 0
        self.search = Search(self.input_lines, initial_context=initial_context)
        self.quit = False

    def toggle_display_mode(self):
        if self.display_mode == DisplayMode.show_default:
            self.display_mode = DisplayMode.show_only_match
        elif self.display_mode == DisplayMode.show_only_match:
            self.display_mode = DisplayMode.show_all
        elif self.display_mode == DisplayMode.show_all:
            self.display_mode = DisplayMode.show_default

    def get_number_of_pager_lines(self):
        maxy, _ = self.win.getmaxyx()
        header_line_no = len(self.search.previous_searches) + 2
        pager_line_no = maxy - header_line_no - FOOTER_LINE_NO
        return pager_line_no

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

    @log_with_debugging
    def process_char(self, char: int):
        if char < 0:
            # in tmux this is an option when the pane gets resized
            return
        elif char in (CTRL_H, curses.KEY_BACKSPACE, MAC_BACKSPACE):
            if len(self.regex) > 0:
                self.regex = self.regex[:-1]
        elif char in (CTRL_L, curses.KEY_RESIZE):
            pass  # The curses window has been resized, so re-render it
        elif char == CTRL_N:
            if self.search.valid:
                previous_match_objects = self.search.previous_searches + [self.search]
                self.search = Search([l.line_text for l in self.search.output_lines if not l.break_line])
                self.search.previous_searches = previous_match_objects
                self.search.context = previous_match_objects[-1].context
                self.regex = ''
        elif char == CTRL_P:
            if self.search.previous_searches:
                self.search = self.search.previous_searches.pop()
                self.regex = self.search.regex
        elif char == CTRL_O:
            self.toggle_display_mode()
        elif char == CTRL_J:
            self.search.next_match()
        elif char == CTRL_K:
            self.search.prev_match()
        elif char == CTRL_G:
            self.endwin()
            grep_commands = []
            for m in self.search.previous_searches + [self.search]:
                options = '--perl-regexp '
                if m.ignore_case:
                    options += '--ignore-case '
                if m.context > 0:
                    options += '--context {} '.format(m.context)
            grep_commands.append("{} {}'{}' ".format(*grep_cmd(), options, m.regex.replace("'", "\\'")))
            to_yank = ' | '.join(grep_commands)
            copy_to_clipboard(to_yank)
            self.quit = True
        elif char == CTRL_Y:
            if not self.search.unique_matches:
                return
            self.endwin()
            to_yank = self.search.unique_matches[self.search.selected_match]
            if to_yank:
                copy_to_clipboard(to_yank)
                self.quit = True
        elif char == CTRL_V:
            self.search.ignore_case = not self.search.ignore_case
        elif char == curses.KEY_DOWN:
            self.pager_ypos += 1
        elif char == curses.KEY_NPAGE:
            self.pager_ypos = self.pager_ypos + self.get_number_of_pager_lines()
        elif char == CTRL_D:
            self.pager_ypos = self.pager_ypos + int(self.get_number_of_pager_lines() / 2)
        elif char == curses.KEY_UP:
            self.pager_ypos = max(0, self.pager_ypos - 1)
        elif char == curses.KEY_PPAGE:
            self.pager_ypos = max(0, self.pager_ypos - self.get_number_of_pager_lines())
        elif char == CTRL_U:
            self.pager_ypos = max(0, self.pager_ypos - int(self.get_number_of_pager_lines() / 2))
        elif char == curses.KEY_RIGHT:
            self.pager_xpos += 1
        elif char == curses.KEY_LEFT:
            self.pager_xpos = max(0, self.pager_xpos - 1)
        elif char == CTRL_R:
            self.search.context = max(0, self.search.context - 1)
        elif char == CTRL_T:
            self.search.context += 1
        else:
            self.regex += chr(char)

        max_pager_ypos = max(0, len(self.search.output_lines) - self.get_number_of_pager_lines())
        self.pager_ypos = min(self.pager_ypos, max_pager_ypos)

    def endwin(self):
        self.win.erase()
        self.win.refresh()
        curses.endwin()


def render(search: Search, window, pager_ypos: int, pager_xpos: int):
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
    max_pager_ypos = max(0, len(search.output_lines) - pager_line_no)
    if pager_ypos >= max_pager_ypos:
        more_lines_after = False
    output_lines = search.output_lines[pager_ypos:pager_ypos + pager_line_no]

    # Fill break lines with text to display
    for l in output_lines:
        if l.break_line:
            l.line_text = '-' * maxx

    line_text_to_print = [line.line_text[pager_xpos:pager_xpos + maxx - 1] for line in output_lines]

    # write output
    window.addstr('\n'.join(line_text_to_print))

    # highlight break lines
    for lineno, line_object in enumerate(output_lines):
        if line_object.break_line:
            color = curses.color_pair(32)
            window.chgat(lineno + header_line_no, 0 - pager_xpos, maxx, color | curses.A_BOLD)

    # highlight matches
    for lineno, line_object in enumerate(output_lines):
        for match in line_object.line_matches:
            # Only highlight if inside pager view
            def inside_pager(match_start, match_end, first_visible_x, last_visible_x):
                if match_start < first_visible_x or match_start > last_visible_x:
                    return False
                if match_end < first_visible_x or match_end > last_visible_x:
                    return False
                return True

            if inside_pager(match.start, match.end, pager_xpos, pager_xpos + maxx - 1):
                if match.unique_id == search.selected_match:
                    color = curses.color_pair(31)
                else:
                    color = curses.color_pair((match.unique_id % AVAILABLE_COLORS) + 1)
                window.chgat(lineno + header_line_no, match.start - pager_xpos, match.end - match.start,
                             color | curses.A_BOLD)

    # Footer
    footer_char, footer_color = ('v', curses.color_pair(31)) if more_lines_after else ('-', curses.color_pair(0))
    window.addstr(footer_div_ypos, 0, footer_char * maxx, footer_color)
    case_sensitivity_text = "[case insensitive], " if search.ignore_case else "[case sensitive],   "
    status_line = case_sensitivity_text + \
                  'context: {}, '.format(search.context) + \
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
    parser.add_argument("-c", "--context", action="store", type=int, default=0,
                        help="Print CONTEXT num of output context")
    parser.add_argument("-d", "--debug", action="store_true", default=False)

    args = parser.parse_args()

    global debug
    debug = args.debug

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
        f = IGrepper(input_lines, initial_regex, initial_context=args.context)
        f.run()
    finally:
        curses.endwin()


if __name__ == '__main__':
    main()
