
## From man less:

   -R or --RAW-CONTROL-CHARS
   Like -r, but only ANSI "color" escape sequences and OSC 8 hyperlink sequences are output in "raw" form.  Unlike -r, the screen appearance is maintained correctly, provided that there are
   no  escape  sequences  in the file other than these types of escape sequences.  Color escape sequences are only supported when the color is changed within one line, not across lines.  In
   other words, the beginning of each line is assumed to be normal (non-colored), regardless of any escape sequences in previous lines.  For the purpose of keeping track of  screen  appear‐
   ance, these escape sequences are assumed to not move the cursor.

   OSC 8 hyperlinks are sequences of the form:

       ESC ] 8 ; ... \7

   The terminating sequence may be either a BEL character (\7) or the two-character sequence "ESC \".

   ANSI color escape sequences are sequences of the form:

       ESC [ ... m

   where  the  "..."  is  zero or more color specification characters.  You can make less think that characters other than "m" can end ANSI color escape sequences by setting the environment
   variable LESSANSIENDCHARS to the list of characters which can end a color escape sequence.  And you can make less think that characters other than the standard ones  may  appear  between
   the ESC and the m by setting the environment variable LESSANSIMIDCHARS to the list of characters which can appear.

=====

 A line has several representations, and the one we use has functional consequences. Consider a line with embedded ANSI
 control sequences.  We define these representations of the text:

 A. The original line:         "Some \x07 noisy \x1b[31mred\x1b[0m text.\x09With tabs."
 B. The sanitized text:        "Some ^G noisy red text.    With tabs."
      OR, if not in -R mode:   "Some ^G noisy ESC[31mredESC[0m text.     With tabs."
 C. The logical text:          "Some _ noisy red text._With tabs."                    <-- where underscores represent single bytes
      OR, if not in -R mode:   "Some _ noisy _[31mred_[0m text._With tabs."

 Note that we display B but we search C.

 So when we try to search this text, what can we match?
     search("red")                Works in B and C
     search("noisy red")          Works in -R mode
     search("text..With tabs")    Should work, but only because we actually search C.
     search("text.\s")            Should match the whole tab.
     search("text.^IWith")        Same.

Observations:
  - We need to produce the 'logical' line whenever we are searching for something (FilteredLog).
  - When -R mode is off, logical == original.  So, -R runs slower, and should be used sparingly.
  - If there are no ANSI sequences in the line, logical == original.  Discovering this is still expensive, though.
  - Implementing -R mode may be a waste of time for Grok.  We don't want to use it for most log-file searching.
  - We can speed this up somewhat by caching whether any codes were present in Waypoint.

=====

Some code I thought I'd use for ANSI parsing:

    #[allow(dead_code)]
    enum AnsiSequences {
        Esc,    // prev was Esc
        Csi,    // inside Control Sequence Introducer (Esc [)
        Osc,    // inside Operating System Command (Esc ])
        Dcs,    // inside Device Control String (Esc P)
        None,   // not inside any sequence
    }


But it might be easier/faster to use `Regex::replace()` with a Regex that matches ANSI control sequences.

====

## Advanced mode: Getting the caret position to find the width of the printed unicode

## Minimap
Sixel graphics in the terminal is a thing:

    convert /home/phord/phord-x1y6/phord/oldlaptop/mnt/snapshot-20210114/phord/Downloads/avatar.png sixel:-

https://nick-black.com/dankwiki/index.php/Sixel#Detecting_support