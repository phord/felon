# Interesting less keyboard commands:

    Most commands accept a numeric argument. The number is typed *before* the command is entered. e.g., to jump to 50%
      position in the file, type 50p. To jump to byte 10000000, type 10000000P.

    digits - build a number as an argument for the next command

    p - go to percentage point in file
    P - go to byte offset in file
    g < ESC-< - go to line N (not prompted; default 1)
    G > ESC-> - go to line N (not prompted; default end of file)

    r ^R or ^L - repaint the screen
    R - refresh and reload the file (in case the file changed)

    PgDn SPACE ^V ^F f z -- move down one page or N lines (if N was given first); z is sticky (saves the page size)
    PgUp b ^B ESC-v w - scroll back one page (opposite of SPACE); w is sticky
    Note: Number before PgDn indicates lines to scroll, not pages.  Wild!

    ENTER ^N e ^E j ^J J - move down N (default 1) lines
    y ^Y ^P k ^K K Y - scroll up N lines (opposite of j)
    J K and Y scroll past end/begin of screen. All others stop at file edges

    d ^D - scroll forward half a screen or N lines; N is sticky; becomes new default for d/u
    u ^U - scroll up half a screen or N lines; N is sticky; becomes new default for d/u

    F - go to end of file and try to read more data


    { } [ ] ( ) - find matching brace/bracket/parenthesis

    m <x> - bookmark first line on screen with letter given (x is any alpha, upper or lower)
    M <x> - bookmark last line on screen with letter given
    ' <x> - go to bookmark with letter given (and position as it was marked, at top or bottom)
    ^X^X <n> - got to bookmark

    / ? - search; can have N argument for Nth match
    Search modifiers:
        ! ^N - lines that don't match
        * ^E - search multiple files, even ones not loaded yet
        @ ^F - Search from the beginning (or end for reverse search)
        ^K - Highlight matches on screen but don't scroll
        ^R - turn off regex
        ^W - turns on wrap mode (search wraps to top/bottom)

    n N - repeat last search (forward or backward)
    ESC-n ESC-N - repeat last search across file boundaries
    ESC-u - toggle search highlighting
    ESC-U - clear the search

    & filters stack up; each new filter adds on to the one before. Lines must match all filters.
    & with no argument clears the filter stack

    q Q :q :Q ZZ - quit less

    v - edit the current file
    ! cmd - run a shell command; if no cmd, opens a shell
    !! repeats the last shell command
    | <mark> <cmd> - pipe the data between the screen and the mark to a shell command

    s filename - save current file; only works for pipes, not files
