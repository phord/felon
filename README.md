### igrok is a work in progress.  It's my development project.  It is not functional yet.
### Do not clone this expecting anything useful to happen.

grok - the Log Grokker tool

grok is an interactive replacement for `zegrep | less`.  It intends to be a replacement for [lnav](https://lnav.org/) which heavily inspired some of the features.

----

Following are some design notes and ideas useful only to me, perhaps.

Originally grok was intended to be a faster word-based file indexer, capable of know everywhere a word existed in a file all at once.
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
    o Remove coloring
        . Simplify presentation / testing / portability
        . Reapply semantic coloring in sub-line indexing
    o Show index status on statusline
    o Identify weird character widths (unicode, tabs, ctrl-chars)
        . Expand control chars (<^A>, <^H>, TAB to Spaces, etc.)
        . Count others for proper display-width matching
    o Add gzip support
    o Search
    o Dynamic Config
        . Allows runtime modification, like '-S', '-N', etc.
    o Implement -N (line numbers) with dynamic updating and/or relative indexing (e.g. "end-10000")
    o Custom line stylers
    o TUI framework for cleaner painting
    o Wait for events to settle/resolve before painting
    o If no alt-screen and lines < display_height, don't fill whole screen?  (see less for something like git pager usage)
        . Something like -K support to exit when no paging needed, too.

For UI-driven workflow, is this workable?
    Expr                Find lines that contain:
    ================    ==================================
    foo
        bar             foo and bar

    foo
    bar                 foo or bar

    foo
        bar
        -baz            both foo and bar, but not baz

    -baz
        foo
        bar             foo or bar but not baz

    foo
    bar
        -baz            foo or (bar but not baz)

    foo
        -bar
            -baz        foo but not bar unless also baz (sketchy!)

    foo bar             the exact text "foo bar"


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

https://github.com/arijit79/minus      https://crates.io/crates/minus
    A pager library to use to develop pagers
    Seems quite buggy, but it could be better to collaborate here than to build my own.

    Configurable kys, options.
    Supports ANSI data, sort of.  (broken for horizontal scrolling)
    Regex searching / incremental searching
    Designed for async usage


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


https://codeberg.org/rini/pomprt
    A readline-like prompt

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