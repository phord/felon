# Bugs

[fixed] Jumping to a position that is not indexed yet displays unindexed lines, all ~.  e.g. `50P` jumps to middle, but shows nothing if not indexed yet.
[fixed] Scroll to bottom then up scrolls extra lines if file is shorter than page size.  End + PgUp (twice) shows this bug.
[fixed] Search backwards sometimes doesn't update display or displays all-tildes
[     ] No way to turn off filter
[fixed] Search backwards with `?` always searches forwards / doesn't reverse meaning of N/n
[fixed] Highlight misaligned when TABs in the line

# MVP: Features I need daily
[fixed] Switch between chopped and wrapped lines
[fixed] Horizontal scroll
[     ] Compressed file support or LESSPIPE support
[fixed] Fix pipe support
[     ] Disable mouse mode by default
[fixed] Filter + Search burns 100% CPU because Search.has_gaps() and never reaches 100%
[fixed] New search should replace previous one (remove previous highlights)
[     ] Filter-out  ("&!")

# Todo:
* Use \n to move to next line instead of sending row positioning for every row
* F3/Shift-F3 to search
* Follow mode, as file grows, load more lines and scroll to them
* scroll in chunks larger than 1 line for faster speed.  Maybe 25% of page?  or 5 lines at a time?
* [x] highlight search results
* [x] Search
* Multi-search
* Multi-filter (filter-in, filter-out)
* Filter/search configs:
  * Enable/disable
  * color
  * Filter-in/Filter-out/Highlight
  * Edit filter
* Search preview
* Bookmarks
  * F2/Shift-F2/Ctrl-F2;  and something else for Mac users?
  * anonymous
  * named
  * persistent
* Save/restore previous session
* [x] Persistent searches (" [KA] ", "STACKTRACE")
* Scrollbar/minimap
* [x] Semantic coloring for words
* Display helpful regex errors
* Faster indexing / searching (compare to bvr)
X Search/filter history recall
  X Persistent history
* Mouse tricks
  * Highlight instances of a clicked word
  * Drag-select text
  * Paste selected text (middle-click? right-click?)
  * Copy selected text to clipboard (see notes in keyboard.rs)
* Less-compat:
  * -F quit if one screen
  * -R Show ANSI escape sequences
  * -K Quit on Ctrl-C
  * -I Ignore case in searches
  * -J status column
  * -N line numbers
  * -p pattern search
  * -V --version
  * -x --tabs tabstops
  * --mouse (--MOUSE to reverse scroll wheel direction)
  * --save-marks (saves bookmarks)
  * -<number> set horiz scroll width
  * --mouse