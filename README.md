# igrepper - Interactive curses-based grepping tool

## Install

Either [from pypi](https://pypi.org/project/igrepper/):

    pip install igrepper

or copy `./igrepper/igrepper.py` into your `PATH`.

Recommended bash alias:

    alias i='igrepper'
    iman() { man "$1" | igrepper -c 3}

## Usage

__Pipe input text:__

    dmesg | i

__Read input from file:__

    i /path/to/file
    
__Commands__:

    ctrl-j: Select next match
    ctrl-k: Select previous match
    ctrl-y: Copy selected match to clipboard
    ctrl-g: Copy grep command to clipboard
    ctrl-n: New subsearch
    ctrl-p: Cancel subsearch
    ctrl-v: Toggle case sensitivity
    ctrl-t: Increase context lines
    ctrl-r: Decrease context lines
    Up/Down/PageUp/PageDown: scroll
    ctrl-u/ctrl-d: half-page scroll
    

    
