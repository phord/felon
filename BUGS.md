# Bugs
- [x] Jumping to a position that is not indexed yet displays unindexed lines, all ~.  e.g. `50P` jumps to middle, but shows nothing if not indexed yet.
- [x] Scroll to bottom then up scrolls extra lines if file is shorter than page size.  End + PgUp (twice) shows this bug.
- [x] Search backwards sometimes doesn't update display or displays all-tildes
- [x] No way to turn off filter
- [x] Search backwards with `?` always searches forwards / doesn't reverse meaning of N/n
- [x] Highlight misaligned when TABs in the line
- [x] Stream without filename streams from tty; first key hangs

# MVP: Features I need daily
- [x] Switch between chopped and wrapped lines
- [x] Horizontal scroll
- [x] Compressed file support or LESSPIPE support
  - [x] Improved compressed file, but still some bugs
  - [x] Gzip support (all-at-once decoded into RAM)
- [x] Fix pipe support
- [x] Disable mouse mode by default
- [x] Filter + Search burns 100% CPU because Search.has_gaps() and never reaches 100%
- [x] New search should replace previous one (remove previous highlights)
- [x] Filter-out  ("&!")
- [x] Replacement name for grok
- [x] Digits-parsing is broken (50p -> "scroll to percent 0")

# Todo:
- [ ] Gzip support with random-access / resume parsing
- [ ] LESSPIPE/LESSOPEN support
- [ ] re-read last line to update display if last line was partial (no LF) and new data appears
- [ ] 1BRC fast file parser contestants:
  - [ ] Fast line splitter: https://github.com/SuperioOne/algorithms/tree/master/algorithms_buffer_utils/src
  - [ ] rayon::par_lines?  https://github.com/ayebear/1brc-rayon/blob/main/src/main.rs
  - [ ] rayon: So clean  https://github.com/arthurlm/one-brc-rs/blob/main/src/main.rs
  - [ ] Hand-rolled threads  https://github.com/thebracket/one_billion_rows
- [ ] Show number on status-line as digits are typed
- [ ] Use \n to move to next line instead of sending row positioning for every row
- [ ] F3/Shift-F3 to search
- [ ] Follow mode, as file grows, load more lines and scroll to them
- [ ] scroll in chunks larger than 1 line for faster speed.  Maybe 25% of page?  or 5 lines at a time?
- [x] highlight search results
- [x] Search
- [ ] Multi-search
- [ ] Multi-filter (filter-in, filter-out)
- [ ] Filter/search configs:
  - [ ] Enable/disable
  - [ ] color
  - [ ] Filter-in/Filter-out/Highlight
  - [ ] Edit filter
- [ ] Timestamps
  - [ ] Filter based on time
  - [ ] Goto time
  - [ ] Show deltas
- [ ] Search preview
- [ ] Bookmarks
  - [ ] F2/Shift-F2/Ctrl-F2;  and something else for Mac users?
  - [ ] anonymous
  - [ ] named
  - [ ] persistent
- [ ] Save/restore previous session
- [x] Persistent searches (" [KA] ", "STACKTRACE")
- [ ] Scrollbar/minimap
- [x] Semantic coloring for words
- [ ] Display helpful regex errors
- [ ] Faster indexing / searching (compare to bvr)
- [x] Search/filter history recall
  - [x] Persistent history
#### Mouse tricks
  - [ ] Highlight instances of a clicked word
  - [ ] Drag-select text
  - [ ] Paste selected text (middle-click? right-click?)
  - [ ] Copy selected text to clipboard (see notes in keyboard.rs)

#### Less-compat:
- [ ] -F quit if one screen
- [ ] -R Show ANSI escape sequences
- [ ] -K Quit on Ctrl-C
- [ ] -I Ignore case in searches
- [ ] -J status column
- [ ] -N line numbers
- [ ] -p pattern search
- [ ] -V --version
- [ ] -x --tabs tabstops
- [ ] --MOUSE to reverse scroll wheel direction
- [ ] --save-marks (saves bookmarks)
- [ ] -<number> set horiz scroll width
- [x] --mouse

#### Speedups
  - [ ] Use Cow<String> where possible for returned lines?
  - [ ] Use big buffer-chunks for regex scanning
  - [ ] Avoid scanning for line numbers until needed
  - [ ] Multithreaded scanning
