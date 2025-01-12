# Bugs

* Jumping to a position that is not indexed yet displays unindexed lines, all ~.  e.g. `50P` jumps to middle, but shows nothing if not indexed yet.
* Scroll to bottom then up scrolls extra lines if file is shorter than page size.  End + PgUp (twice) shows this bug.

# Todo:
* scroll in chunks larger than 1 line for faster speed.  Maybe 25% of page?  or 5 lines at a time?
*
* highlight search results
* Search
* Multi-search
* Multi-filter (filter-in, filter-out)
* Filter/search configs:
  * Enable/disable
  * color
  * Filter-in/Filter-out/Highlight
* Search preview
* Bookmarks
* Save/restore previous session
* Persistent searches (" [KA] ", "STACKTRACE")
* Scrollbar/minimap
* Semantic coloring for words


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