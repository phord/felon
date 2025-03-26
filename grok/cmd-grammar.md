### Command Grammar for all Felon commands

Felon has match expressions, styles, modifiers

filter out "shmem.res"
filter in --disabled --group GC "gc.main"
filter list
filter delete
filter clear
filter enable
filter disable
filter on
filter off


    match add "^(?<timestamp>[A-Z][a-z]{2} {1,2}\\d{1,2} \\d{2}:\\d{2}:\\d{2}\\.\\d{3}) (?<pid>[0-9A-F]{12}) (?<crumb>[A-Z])      (?<indent> *)(?<module>\\w+(?:(?:\\.|::)\\w+)*)(?: \\[(?<instance>\\w+(?:(?:\\.|::)\\w+)*)\\]){0,1} (?<body>.*)$"
match add
match list
match delete
match clear
match enable
match disable

    match add example "(?x)
                      ^(...\ [\ 1-3]\d\ [0-2]\d:[0-5]\d:\d{2}\.\d{3})\    # date & time
                       ([A-F0-9]{12})\                                    # PID
                       ([A-Z])\                                           # crumb"

Styles can be colors, inverse, #ffffff, #fff, #fff/#000, /Green, etc.

style set timestamp Green
style set pid Semantic
style set module Semantic
style set numbers Semantic

style set [label] [style]
style list
style clear
style delete [label]
style enable
style disable
style search style, style, style, ...

bookmark set [label]
bookmark goto [label]
bookmark next
bookmark prev
bookmark list
bookmark clear
bookmark toggle
bookmark delete

keymap add [F3] search-next
keymap list
keymap show [F3]
keymap delete [F3]

set-level crumb error "E"
set-level crumb critical "K"
set-level crumb critical "A"

time format timestamp "%b %d %H:%M:%S.%f"
time filter before hh:mm:ss
time filter after hh:mm:ss

show timedelta
show numbers
show offset
show filename
show match
    # More?


highlight-colors Red, Yellow, Green

default-colors Rgb(140,140,140)-on-Rgb(0)
default-colors "#c0c0c0"-on-"#000000"

# Filters are not colored by default, but they can be
filter-color None

# Preview colors are used to show the match-so-far as the user is typing the expression
preview-color White-on-Red

# Color of the status line elements
status-color Inverse



set wrap chop
set number on



match add " {3}(?<module>[A-Za-z0-9_.]+) (?<module>\[([a-z0-9_.]+)\]){0,1}"

match add "Space calculate job finished, ((?<capacity_bytes>\d{7,18}) [[:word:]]+[, ]+)+"

# Can we use the evalexpr crate?
replace capacity_bytes expr("a=1.0 * capacity_bytes; a /= 1024 * 1024 * 1024 * 1024; capacity_bytes, str_from(a) + "TB\"")

match add control "[[:control:]]"
replace control expr("a=ord(control); a += 64; "^" + str_from(a)")
highlight control Inverse

# Match any 0x{hex} number, any 16-digit all-uppercase hex number at word delimiters, or any decimal number which is not part of a word suffix.
# TODO: Also match UUIDS and include units when attached, like `123GB`
match add numbers "(\b0x[[:xdigit:]]+\b|\b[0-9A-F]{16}\b|(?:[[:digit:]]+\.)*[[:digit:]]+)"

style "module" bold,semantic
style numbers semantic,italic
style timestamp Green
style default White,onBlack


linemode chop # break, wrap, etc.

keymap "F3" search-next
keymap "Shift+F3" search-prev
keymap "Tab" edit-filters
keymap "^L" toggle-filters

show line-numbers
show timestamp
show delta
show filename
hide pid
show offset
show scrollbar

# Misc commands:
# goto-line
# goto-time
# goto-offset
# goto-percent
