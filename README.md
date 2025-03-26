    FELON

    NAME
        felon - the Log Grokker tool

    felon is an interactive replacement for `zegrep | less`.  It intends to be a replacement
    for [lnav](https://lnav.org/) which heavily inspired some of the features.

    SYNOPSIS
        felon [ -S | --chop-long-lines ] [ -X | --no-alternate-screen ] [filename [filename ...]]

    DESCRIPTION
        Felon is a pager similar to less, but sometimes faster and with more features. Felon is
        intended to be faster than less when handling compressed files and when searching or
        filtering the lines. Felon implements many of the same commands as less as a convenience.
        But it doesn't implement all of them, and some of them may work differently.

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

        ?pattern  Search backwards for the Nth line containing the pattern.  N defaults to 1.  The search starts at the first displayed line
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
