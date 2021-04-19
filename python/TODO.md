# TODO 

## Bugs

- input containing tabs is highlighted incorrectly and can overflow the curses window

## Performance

- Profile, define acceptable breaking point
- Try memoization

## Testing

- non-interactive mode for unit tests

## Features

- proper line editing: /usr/lib/python3.6/curses/textpad.py
- proper paging when selecting match or modifying regex
- group support, lookbehind/lookahead
- save match to file?
- pipe match to cmd?
- final content to clipboard
- regex to clipboard
- displaymode "show_only_unique",show each unique match on a separate line
- context: like grep -C
- vim's `smartcase`: ignore case only if no capital in regex?

## Compatibility

- Determine dependencies, python version
- Test on macos

# WISHLIST / IDEAS

## Usability

- Helper screen to show keyboard shortcuts


