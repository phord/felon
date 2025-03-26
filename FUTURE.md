Following features from less are not implemented.

        J      Like j, but continues to scroll beyond the end of the file.

        K or Y Like k, but continues to scroll beyond the beginning of the file.

        ESC-) or RIGHTARROW
                Scroll horizontally right N characters, default half the screen width (see the -# option).  If a number N is specified, it becomes the default for future RIGHTARROW and LEFTARROW commands.  While the text is scrolled, it acts as though the -S option (chop lines) were in effect.

        ESC-( or LEFTARROW
                Scroll horizontally left N characters, default half the screen width (see the -# option).  If a number N is specified, it becomes the default for future RIGHTARROW and LEFTARROW commands.

        ESC-} or ^RIGHTARROW
                Scroll horizontally right to show the end of the longest displayed line.

        ESC-{ or ^LEFTARROW
                Scroll horizontally left back to the first column.

        R      Repaint the screen, discarding any buffered input.  That is, reload the current file.  Useful if the file is changing while it is being viewed.

        F      Scroll forward, and keep trying to read when the end of file is reached.  Normally this command would be used when already at the end of the file.  It is a way to monitor the tail of a file which is growing while it is being viewed.  (The behavior is similar to the "tail -f" command.)  To stop waiting for more data, enter the interrupt character (usually ^C).  On some
                systems you can also use ^X.

        ESC-F  Like F, but as soon as a line is found which matches the last search pattern, the terminal bell is rung and forward scrolling stops.

        ESC-<
        ESC->

            ESC-G  Same as G, except if no number N is specified and the input is standard input, goes to the last line which is currently buffered.

        {      If a left curly bracket appears in the top line displayed on the screen, the { command will go to the matching
                right curly bracket.  The matching right curly bracket is positioned on the bottom line of the screen.  If
                there is more than one left curly bracket on the top line, a number N may be used to specify the N-th bracket
                on the line.

        }      If a right curly bracket appears in the bottom line displayed on the screen, the } command will go to the
                matching left curly bracket.  The matching left curly bracket is positioned on the top line of the screen.
                If there is more than one right curly bracket on the top line, a number N may be used to specify the N-th
                bracket on the line.

        (      Like {, but applies to parentheses rather than curly brackets.

        )      Like }, but applies to parentheses rather than curly brackets.

        [      Like {, but applies to square brackets rather than curly brackets.

        ]      Like }, but applies to square brackets rather than curly brackets.

        ESC-^F Followed by two characters, acts like {, but uses the two characters as open and close brackets, respectively.
                For example, "ESC ^F < >" could be used to go forward to the > which matches the < in the top displayed line.

        ESC-^B Followed by two characters, acts like }, but uses the two characters as open and close brackets, respectively.
                For example, "ESC ^B < >" could be used to go backward to the < which matches the > in the bottom displayed line.

        m      Followed by any lowercase or uppercase letter, marks the first displayed line with that letter.  If the status
                column is enabled via the -J option, the status column shows the marked line.
            PROMPT: set mark:

        M      Acts like m, except the last displayed line is marked rather than the first displayed line.

        '      (Single quote.)  Followed by any lowercase or uppercase letter, returns to the position which was previously marked
                with that letter.  Followed by another single quote, returns to the position at which the last "large" movement
                command was executed.  Followed by a ^ or $, jumps to the beginning or end of the file respectively.  Marks are
                preserved when a new  file  is examined, so the ' command can be used to switch between input files.
            PROMPT: goto mark:

        ^X^X   Same as single quote.

        ESC-m  Followed by any lowercase or uppercase letter, clears the mark identified by that letter.
            PROMPT: clear mark:

                If pattern is empty (if
                you type & immediately followed by ENTER), any filtering is turned off, and all lines are displayed.  While filtering is in
                effect, an ampersand is displayed at the beginning of the prompt, as a reminder that some lines in the file may be hidden.

        = or ^G or :f
                Prints some information about the file being viewed, including its name and the line number and byte offset of the
                bottom line being displayed.  If possible, it also prints the length of the file, the number of lines in the file
                and the percent of the file above the last displayed line.

        -       Followed  by  one  of the command line option letters (see OPTIONS below), this will change the setting of that
                option and print a message describing the new setting.  If a ^P (CONTROL-P) is entered immediately after the dash,
                the setting of the option is changed but no message is printed.  If the option letter has a numeric value (such
                as -b or -h), or a string value (such as -P or -t), a new value may be entered after the option letter.  If no new
                value is entered, a message describing the current setting is printed and nothing is changed.

        --     Like the - command, but takes a long option name (see OPTIONS below) rather than a single option letter.  You must
                press ENTER or RETURN after typing the option name.  A ^P immediately after the second dash suppresses printing of
                a message describing the new setting, as in the - command.

        -+     Followed by one of the command line option letters this will reset the option to its default setting and print
                a message describing the new setting.  (The "-+X" command does the same thing as "-+X" on the command line.)
                This does not work for string-valued options.

        --+    Like the -+ command, but takes a long option name rather than a single option letter.

        -!     Followed by one of the command line option letters, this will reset the option to the "opposite" of its default
                setting and print a message describing the new setting.  This does not work for numeric or string-valued options.

        --!    Like the -! command, but takes a long option name rather than a single option letter.

        _      (Underscore.)  Followed by one of the command line option letters, this will print a message describing the current
                setting of that option.  The setting of the option is not changed.

        __     (Double underscore.)  Like the _ (underscore) command, but takes a long option name rather than a single option
                letter.  You must press ENTER or RETURN after typing the option name.

        +cmd   Causes the specified cmd to be executed each time a new file is examined.  For example, +G causes less to initially
                display each file starting at the end rather than the beginning.

        V      Prints the version number of felon being run.

        or :q or :Q or ZZ

       v      Invokes an editor to edit the current file being viewed.  The editor is taken from the environment variable VISUAL
            if defined, or EDITOR if VISUAL is not defined, or defaults to "vi" if neither VISUAL nor EDITOR is defined.  See
            also the discussion of LESSEDIT under the section on PROMPTS below.

       ! shell-command
              Invokes a shell to run the shell-command given.  A percent sign (%) in the command is replaced by the name of the
              current file.  A pound sign (#) is replaced by the name of the previously examined file.  "!!" repeats the last shell
              command.  "!" with no shell command simply invokes a shell.  On Unix systems, the shell is taken from the environment
              variable  SHELL,  or defaults to "sh".  On MS-DOS and OS/2 systems, the shell is the normal command processor.

       | <m> shell-command
              <m> represents any mark letter.  Pipes a section of the input file to the given shell command.  The section of the
              file to be piped is between the position marked by the letter and the current screen.  The entire current screen is
              included, regardless of whether the marked position is before or after the current screen.  <m> may also be ^ or $ to
              indicate beginning or end of file respectively.  If <m> is . or newline, the current screen is piped.

       s filename
              Save the input to a file.  This only works if the input is a pipe, not an ordinary file.

Chord characters:
    ESC, -, _, :, ^X, Z
