import pytest
import unittest
from unittest import mock
from unittest.mock import Mock, patch
import subprocess
from pprint import pprint
import json
import curses

from unittest.mock import MagicMock

from igrepper.igrepper import Search, DisplayMode, render, IGrepper
from igrepper.igrepper import CTRL_I, CTRL_Y, CTRL_G, CTRL_K, CTRL_J, CTRL_P, CTRL_N, CTRL_H, CTRL_O


def get_sample(filename):
    with open('samples/{}'.format(filename)) as f:
        return ''.join(f.readlines()).split('\n')


def grep(filename, regex):
    try:
        return subprocess.check_output(['grep', '-P', regex, './samples/{}'.format(filename)]) \
            .decode('utf-8') \
            .split('\n')
    except ZeroDivisionError:
        return ''


def slog(m):
    import subprocess
    subprocess.run(["notify-send", str(m)])


mock_color_pair = MagicMock(side_effect=lambda x: x)


@mock.patch("curses.initscr", mock.MagicMock())
@mock.patch("curses.start_color", mock.MagicMock())
@mock.patch("curses.init_pair", mock.MagicMock())
@mock.patch("curses.use_default_colors", mock.MagicMock())
@mock.patch("curses.color_pair", mock_color_pair)
def test_igrepper():
    wrote_output = False
    tty_inputs = [
        ['i', '.', CTRL_J, CTRL_J, CTRL_K, CTRL_H, CTRL_N, '\\', 'w', '+'],
        ['\\', 'd', CTRL_H, curses.KEY_BACKSPACE],
        [curses.KEY_DOWN, curses.KEY_DOWN, '.', '\\', 'd', CTRL_N, 'p', CTRL_I, '.', CTRL_P],
        ['l', CTRL_H]
    ]
    tty_inputs = [[ord(y) if type(y) == str else y for y in x] for x in tty_inputs]

    sample_names = ['simple', 'longlines']
    for sample_name in sample_names:
        for tty_input_no, tty_input in enumerate(tty_inputs):
            input_lines = get_sample(sample_name)
            output = run(input_lines, tty_input)
            try:
                with open('expected/{}_{}'.format(sample_name, tty_input_no)) as f:
                    expected = json.load(f)
                    if expected != output:
                        with open('diffs/expected_{}_{}'.format(sample_name, tty_input_no), 'w') as f:
                            f.write(str('\n'.join([str(_) for _ in expected])))
                        with open('diffs/actual_{}_{}'.format(sample_name, tty_input_no), 'w') as f:
                            f.write(str('\n'.join([str(_) for _ in output])))
                    assert expected == output
            except FileNotFoundError:
                wrote_output = True
                with open('expected/{}_{}'.format(sample_name, tty_input_no), 'w') as f:
                    json.dump(output, f)
    if wrote_output:
        print('Output files written, rerun test')
        assert False
    return


def run(input_lines, tty_input):
    f = IGrepper(input_lines, '')
    f.win = MagicMock(return_value=3)
    f.win.refresh = MagicMock()
    f.win.getmaxyx = MagicMock(return_value=(30, 20))
    written = []

    def mock_getch(param=None):
        x = 0
        l = tty_input

        def inner():
            # sleep(0.1)
            nonlocal x
            if x >= len(l):
                f.quit = True
                return ord('d')
            ret = l[x]
            x += 1
            return ret

        return inner

    f.win.getch = Mock(side_effect=mock_getch())

    def mock_erase():
        written.append([])

    f.win.erase = Mock(side_effect=mock_erase)
    written = []

    def mock_addstr(a, b=None, c=None, d=None):
        written[-1].append([a, b, c, d])

    f.win.addstr = Mock(side_effect=mock_addstr)

    f.run()
    # pprint(written)
    return written
