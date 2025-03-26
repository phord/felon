FELON

NAME
        felon - the Log Grokker tool

felon is an interactive replacement for `zegrep | less`.  It intends to be a replacement for [lnav](https://lnav.org/) which heavily inspired some of the features.

SYNOPSIS
        felon [ -S | --chop-long-lines ] [ -X | --no-alternate-screen ] [filename [filename ...]]

DESCRIPTION
        Felon is a pager similar to less, but sometimes faster and with more features. Felon is intended to be faster than less
        when handling compressed files and when searching or filtering the lines. Felon implements many of the same commands as
        less as a convenience.  But it doesn't implement all of them, and some of them may work differently.

COMMANDS
        In the following descriptions, ^X means control+X.  SPACE means the spacebar.  ENTER means the carriage return.

        Many commands can accept a numeric argument, N. Type the number first, then the command.  For example, 100g will
        go to line 100 from the start of the file.

        SPACE or ^V or f or ^F or z
                Scroll forward N lines, default one window. z is sticky; with z, N becomes the new window size.

        b or ^B or ESC-v or w
                Scroll backward N lines, default one window. w is sticky.

        ENTER or ^N or e or ^E or j or ^J
                Scroll forward N lines, default 1.

        y or ^Y or ^P or k or ^K
                Scroll backward N lines, default 1.

        d or ^D
                Scroll forward N lines, default one half of the screen size. d is sticky.

        u or ^U
                Scroll backward N lines, default one half of the screen size. u is sticky.

        r or R or ^R or ^L
                Repaint the screen.

        g or <
                Go to line N in the file, default 1 (beginning of file).  (Warning: this may be slow if N is large.)

        G or >
                Go to line Nth line from the end of the file, default 1 (end of file).
                Note: this differs from less' behavior. In less, NG goes to the Nth line from the start, same as Ng.

        p or % Go to a position N percent into the file.  N should be between 0 and 100, and may contain a decimal point.

        P      Go to the line containing byte offset N in the file.

        /pattern  Search forward for the Nth line containing the regex pattern.  N defaults to 1.  The search starts at the first displayed
                  line on the screen.

        ?pattern  Search backwards for the Nth line containing the pattern.  N defaults to 1.  The search starts at the last displayed line
                  on the screen.

        n      Repeat previous search, for N-th line matching the last pattern.

        N      Repeat previous search, but in the reverse direction.

        &pattern
                Display only lines which match the pattern; lines which do not match the pattern are not displayed.
                Multiple & commands may be entered, in which case all lines matching any of the inclusive patterns will be displayed, while
                all lines matching any of the exclusive patterns will be hidden.

                !
                        Make this an exclusive pattern. That is, hide lines matching this pattern instead of showing them.

        q or Q
                Exits felon.

----

Following are some design notes and ideas useful only to me, perhaps.

Originally felon was intended to be a faster word-based file indexer, capable of knowing everywhere a word existed in a file all at once.
The expectation was that this could be faster than regular expression searching after the fact. The expectation was incorrect, at least in my implementation.
It's not a new idea; it dates back to the 1940's, and an [intern at Google used it](https://swtch.com/~rsc/regexp/regexp4.html) in some modern tools there for a while.
Anyway, it turned out to be not terribly fast, in the end, and regex seems plenty fast enough.  Continuing with that for now.

Design:

FileReader - reads lines of text from the file
    Can be adapted to read from compressed files with memoized decompressor states for faster random access

Iterators
    Index-on-demand
    Line offsets
    Line and data
    Subline and data

TextBuffer - buffer to hold lines of text
    Supports indexed lines so external classes can read subsets by reference
    Can backfill from FileReader if a line is missing
    Some kind of LRU feature to drop old lines and limit memory usage

TextList - ordered set of lines referenced in a TextBuffer

SearchPhrase - a way to match text; e.g. regex, substring, time range, etc.

SearchExpr - a search expression made of one or more SearchPhrases and conjunctions
    Can be combined with other SearchExpr to make up more complex expressions

Features/Ideas:
    - Recognize/parse timestamps to support time-based filters, goto, timedelta, etc.
    - Able to show context lines (eg. -A5 -C3)
    - Highlight matches in colors (with different colors for different expressions)
    - Spool data from stdin
    - Automatically recognize log-line patterns; allows to quickly say "show all of these lines"
        - Needs to be smart enough to ignore prefix (timestamp, pid, etc.)
    - Auto-expand grouped lines (with C markers)


----

High-level todos:
    - [ ] Remove coloring
        - Simplify presentation / testing / portability
        - Reapply semantic coloring in sub-line indexing
    - [x] Show index status on statusline
    - [ ] Identify weird character widths (unicode, tabs, ctrl-chars)
        - Expand control chars (<^A>, <^H>, TAB to Spaces, etc.)
        - Count others for proper display-width matching
    - [x] Add gzip support
    - [x] Search
    - [ ] Dynamic Config
        - Allows runtime modification, like '-S', '-N', etc.
    - [ ] Implement -N (line numbers) with dynamic updating and/or relative indexing (e.g. "end-10000")
    - [ ] Custom line stylers
    - [ ] TUI framework for cleaner painting
    - [ ] Wait for events to settle/resolve before painting
    - [ ] If no alt-screen and lines < display_height, don't fill whole screen?  (see less for something like git pager usage)
        - Something like -K support to exit when no paging needed, too.

Commands I need:
    Toggle filters / colors temporarily
    Change search highlight colors
    Toggle highlights



Display filters:
    SHOW - Show all lines that match
    HIDE - Hide all lines that match
    MARK - Highlight all matches

Need to be able to disable / enable filters individually and collectively.

Every line knows its timestamp for `goto`, `delta`, and merging, but don't parse the time until it's needed.

`less` already supports filtering built-in, btw.
    https://unix.stackexchange.com/questions/179238/grep-inside-less

Syntax highlighter:  https://github.com/trishume/syntect

Related published crates:
https://crates.io/crates/streampager   https://github.com/markbt/streampager
    Pager for streams and "large files"
    Has some less-compatible keys
    Development has stalled

https://github.com/arijit79/minus      https://crates.io/crates/minus
    A pager library to use to develop pagers
    Seems quite buggy, but it could be better to collaborate here than to build my own.

    Configurable kys, options.
    Supports ANSI data, sort of.  (broken for horizontal scrolling)
    Regex searching / incremental searching
    Designed for async usage
    Mouse support


https://github.com/Avarel/bvr          https://crates.io/crates/bvr
    Designed to be fast for large files
    Still under heavy development.
    Kinda buggy.  Uses lots of crates.

    "Beaver" - chews through log files
    Filters!  This could be interesting.

https://github.com/bensadeh/tailspin   https://crates.io/crates/tailspin
    A log file highlighter

    Could be useful for matching regex language.

https://github.com/Thomasdezeeuw/a10?tab=readme-ov-file
    An io_uring wrapper that provides file and stdin abstraction, but requires a modern Linux build.  Not portable.

OMG - the RipGrep CLI helper tools crate includes a struct that decompresses any file via shell helpers (a la LESSPIPE).
https://docs.rs/grep-cli/0.1.2/grep_cli/struct.DecompressionReader.html


Nameless is a file handler that tries to make stdin, files, gzipped files, HTTPS urls, etc. all look the same.
https://docs.rs/crate/nameless/latest

https://docs.rs/clap/latest/clap/
Command line parser that can also be used for an internal REPL.

More REPLs:
    https://docs.rs/thag_rs/latest/thag_rs/
    https://github.com/jedrzejboczar/easy-repl

    Make a REPL from clap and reedline:
    https://docs.rs/reedline-repl-rs/latest/reedline_repl_rs/



https://codeberg.org/rini/pomprt
    A readline-like prompt
https://github.com/kkawakam/rustyline
    Another readline prompt. supports command history.
    Does not support async.
https://docs.rs/r3bl_terminal_async/latest/r3bl_terminal_async/
    Another one, derived from RustyLine-async, but evolved greatly
    Overkill?

https://github.com/unicode-rs/unicode-width
https://github.com/Aetf/unicode-truncate

https://github.com/jameslanska/unicode-display-width/
https://github.com/jameslanska/unicode-display-width/blob/main/docs/alternatives.md
    Not recommended for terminal measurement.  :-(

assert_eq!(width("🔥🗡🍩👩🏻‍🚀⏰💃🏼🔦👍🏻"), 15);
assert_eq!(width("🦀"), 2);
assert_eq!(width("👨‍👩‍👧‍👧"), 2);
assert_eq!(width("👩‍🔬"), 2);
assert_eq!(width("sane text"), 9);
assert_eq!(width("Ẓ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅ"), 9);
assert_eq!(width("슬라바 우크라이나"), 17);

Some unicode that is torturous to measure:
    Ẓ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅ
For example, all this is on one line:
    Ẓ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅẒ̌á̲l͔̝̞̄̑͌g̖̘̘̔̔͢͞͝o̪̔T̢̙̫̈̍͞e̬͈͕͌̏͑x̺̍ṭ̓̓ͅ
