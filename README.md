igrok - the Log Grokker tool

igrok reads a text file (possibly compressed) and indexes every word in the file.  Users can then browse
the interesting lines of the file by searching for keywords identified in it.

Needs some special expression parsing.  Something like

    Expr               Find lines that contain:
    ================   ==================================
    "+foo +bar -baz"   both foo and bar, but not baz
    "(foo bar) -baz"   foo or bar but not baz
    "foo (bar -baz)"   foo or (bar but not baz)
    "foo -(bar -baz)"  foo but not bar unless also baz
    "'foo bar'"        the exact text "foo bar"
    "foo_bar"          the exact text "foo_bar"



Design:

FileReader - reads lines of text from the file
    Can be adapted to read from compressed files with memoized decompressor states for faster random access

TextBuffer - buffer to hold lines of text
    Supports indexed lines so external classes can read subsets by reference
    Can backfill from FileReader if a line is missing
    Some kind of LRU feature to drop old lines and limit memory usage

TextList - ordered set of lines referenced in a TextBuffer

SearchPhrase - a word or glob that can match text literally
    "foo", "foo*", "foo_bar"
    Should I punt and let this be a proper regex?

SearchExpr - a search expression made of one or more SearchPhrases and conjunctions
    Can be combined with other SearchExpr to make up more complex expressions

Features/Ideas:
    - Use TUI or Termion or something else for managing the display
    - Recognize/parse timestamps to support time-based filters, goto, timedelta, etc.
    - Able to show context lines (eg. -A5 -C3)
    - Highlight matches in colors (with different colors for different expressions)
    - Spool data from stdin
    - Automatically recognize log-line patterns; allows to quickly say "show all of these lines"
        - Needs to be smart enough to ignore prefix (timestamp, pid, etc.)
    - Auto-expand grouped lines (with C markers)


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

    foo_bar             the exact text "foo_bar" (even though _ is not indexed)


Display filters:
    SHOW - Show all lines that match
    HIDE - Hide all lines that match
    MARK - Highlight all matches

Need to be able to disable / enable filters individually and collectively.

Every line knows its timestamp for `goto`, `delta`, and merging, but don't parse the time until it's needed.

Blast!  `less` already supports filtering built-in!
    https://unix.stackexchange.com/questions/179238/grep-inside-less

Syntax highlighter:  https://github.com/trishume/syntect