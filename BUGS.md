# Bugs

[fixed] Jumping to a position that is not indexed yet displays unindexed lines, all ~.  e.g. `50P` jumps to middle, but shows nothing if not indexed yet.
[     ] Scroll to bottom then up scrolls extra lines if file is shorter than page size.  End + PgUp (twice) shows this bug.
[     ] Search backwards sometimes doesn't update display or displays all-tildes
[     ] No way to turn off filter
[     ] Search backwards with `?` always searches forwards / doesn't reverse meaning of N/n

# MVP: Features I need daily
[fixed] Switch between chopped and wrapped lines
[fixed] Horizontal scroll
[     ] Compressed file support or LESSPIPE support

# Todo:
* F3/Shift-F3 to search
* scroll in chunks larger than 1 line for faster speed.  Maybe 25% of page?  or 5 lines at a time?
* [x] highlight search results
* [x] Search
* Multi-search
* Multi-filter (filter-in, filter-out)
* Filter/search configs:
  * Enable/disable
  * color
  * Filter-in/Filter-out/Highlight
* Search preview
* Bookmarks
* Save/restore previous session
* [x] Persistent searches (" [KA] ", "STACKTRACE")
* Scrollbar/minimap
* [x] Semantic coloring for words
* Display helpful regex errors
* Faster indexing / searching (compare to bvr)

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
  * -<number> set horiz scroll width
  * --mouse
  *