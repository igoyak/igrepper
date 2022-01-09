# igrepper - The interactive grepper

Filter and explore text with instant feedback. The regex is re-evaluated at every keypress. Also useful for quickly
developing and testing regular expressions.

![](docs/basic_usage.gif)

# Installation

Either:

1. __Download binary__  
   To install without setting up a rust environment, grab the `igrepper` binary from the latest
   release: https://github.com/igoyak/igrepper/releases  
   Then make it executable with `chmod` and put it in your `PATH`.

1. __Install using cargo__  
   `cargo install igrepper`

1. __Build and run from source code__  
   `cargo run`

__Dependencies__

Ubuntu: `apt-get install xsel` for clipboard support

# Usage

Recommended `.bashrc` additions:

    alias i='igrepper'
    iman() {
      man "$1" | igrepper --context 3
    }

__Read input from pipe:__

    dmesg | i

__Read input from file:__

    i /etc/fstab

Create a `grep` command using `ctrl-g`:

![](docs/grep.gif)

See context around matches:

![](docs/context.gif)

Reload the file as it changes (similar to `tail -f`):

    i -f somefile.log

### Commands:

Edit the current regex by typing.

Movement:

| Command       | Action        |
| ------------- | ------------- |
|    `Up`/`Down`/`Left`/`Right`/`PageUp`/`PageDown` | Scroll |
|    `ctrl-u`/`ctrl-d` | Half-page scroll |

Searching:

| Command       | Action        |
| ------------- | ------------- |
|    `ctrl-n`/`ctrl-j`/`Enter` | Accept current regex, start a sub-search |
|    `ctrl-p` | Revert sub-search |
|    `ctrl-i` | Toggle case sensitivity |
|    `ctrl-v` | Toggle inverted |
|    `ctrl-r`/`ctrl-t` | Decrease/Increase context-lines |

Exporting:

| Command       | Action        |
| ------------- | ------------- |
|    `ctrl-e` | Copy current match to clipboard |
|    `ctrl-g` | Copy equivalent `grep` command to clipboard |
|    `F1`     | Pipe current match to the configured external editor |
|    (Inside vim) `F1` | Pipe current buffer to `igrepper` (add `map <F1> :silent :w !igrepper<CR>:q!<CR>` to your `.vimrc`) |

### Configuration

#### External editor

Set the environment variable `IGREPPER_EDITOR` to a command and arguments, separated by whitespace, to customize which
editor is used when pressing `F1`. The command must support reading from `STDIN`.

Example `.bashrc` configuration:

    export IGREPPER_EDITOR="vim -R -" # vim in read-only mode (default)
    export IGREPPER_EDITOR="code -" # vscode
    export IGREPPER_EDITOR="nano -v -" # nano in read-only mode

## Supported platforms

Tested on Ubuntu 20.04

## Known issues

- No unicode support
- Broken colors when using `screen`/`tmux` and `urxvt`. As a workaround, you can either:
    - Run `export TERM=rxvt-unicode-256color`
    - Add `term screen-256color` to your `.screenrc`

## Dev dependencies

Ubuntu: `apt-get install libncurses-dev`

## Release build

`cargo build --release`
`cargo publish`

