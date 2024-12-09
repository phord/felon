A quick issue tracker

- Isolate features into logical, publishable crates

- Switch LogLine to hold Vec<u8> instead of String
  - [x] Sanitize all output before splitting it
  - Move unicode to future roadmap
    - ** This contradicts BufRead::read_line and ::lines() which return String
         Note: It's not possible to find newlines in a utf-8 file without considering unicode.
         Disabling Strings may mean only listing ASCII files

- Fix IndexedLog.next() to be cooperatively concurrent
  - Maybe we can show a spinner on the screen while we wait for lines to appear
- Custom readline replacement
  - Supports cooperative concurrency
  - Constrains editor to fit on status line
- Custom popup list for filter editor operations:
  - Disable/Enable
  - Color selection
  - Persistence (save filters for future sessions)
- Easy less-compat features (see notes in LESS.md)
- Dynamic scrollbar
  - Use reverse-highlight on right edge of display
  - Mouse-mode only
  - Display updates dynamically as we scroll, when possible
- Update filters/search in the background until no more gaps
  - Multithreaded updates if allowed
- Status-line display of filter-update progress
- Visual/Mouse mode
  - Drag-select text
  - Drag scrollbar
  - Copy to clipboard (how?)
  - Single-click to select/unselect a word
    - Auto highlight all matching words
    - Selected text auto-fills search/filter prompt
- Timestamps
  - Hide/show delta column
  - goto-time command
- Commandline with user commands for every keyboard action
  - Activate with ':'
  - MUST HAVE: autocompletion
  - goto-percent, goto-offset, goto-line, goto-time, etc.
  - highlight
  - filter-in, filter-out, filter-clear, filter-enable, filter-disable
  - up, down, wrap, color, etc.
- Custom syntax highlighting
  - Designate timestamp region
  - Specify line grouping
  - Identify severity lines (crit, err, warn, info)


# Brainstorming about interfaces and crates
    Implement interfaces for target file types
     - actual files
     - streams
       - special for stdin
     - compressed files
       - zstd
       - gzip

      - The wrappers implement a core set of traits needed for grok.
        - Traits
          - Read
          - Seek
          - BufRead
          - IndexedLog
            - len
            - next
            - read_line
            - count_lines
            - future: status (percent indexed, relative position?)
        - Stream
            - wait
            - wait_for_end

        - Wrappers
          - actual files
            - Stream
          - streams (incl stdin)
            - Seek, BufRead, Stream
          - compressed files
            - Seek, BufRead, Stream

      - Additional
        - LogIterator (DoubleEndedIterator for (offset, line))


    Crates
      - Stream
        - Adds Seek, BufRead for streams
        - Normalizes stdin