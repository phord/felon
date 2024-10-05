A quick issue tracker

- Isolate features into logical, publishable crates

- Switch LogLine to hold Vec<u8> instead of String
  - Sanitize all output before splitting it
  - Move unicode to future roadmap
    - ** This contradicts BufRead::read_line and ::lines() which return String
    -    Note: It's not possible to find newlines in a utf-8 the file without considering unicode.
    -    Disabling Strings may mean only listing ASCII files


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
      - StreamCache
        - len
        - wait
        - wait_to_end

    - Wrappers
      - actual files
        - StreamCache
      - streams (incl stdin)
        - Seek, BufRead, StreamCache
      - compressed files
        - Seek, BufRead, StreamCache

  - Additional
    - LogIterator (DoubleEndedIterator for (offset, line))


Crates
 - StreamCache
   - Adds Seek, BufRead for streams
   - Normalizes stdin