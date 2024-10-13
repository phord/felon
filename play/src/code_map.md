Code map: indexed_file

LogFile:
    Helper trait that augments BufRead + Seek
    Common helper functions to navigate a BufRead + Seek for line reading

    LogSource:
        Polymorphic wrapper of LogFile

    LogBase:
        Combines LogSource and LogFile (I don't remember why)

    Stream:
        Helper trait for async streams
        LogFile is implemented generically for all Streams

LogFile Implementations:
    TextLogFile:
        Normal text file + Stream

    CachedStreamReader:
        Wraps streams and stdin to add Seek (random access)
        Caches entire stream in memory
        Non-blocking reader
        Supports reading from Stdin, redirects and pipes

    ZstdLogFile:
        Implements memoized Seek + Read for zstd-compressed files

    CompressedFile:
        Implements generic memoized Seek + Read for compressed files
        FIXME: Currently it is specific to zstd-compressed files

    MockLogFile:
        In-memory LogFile simulant with repeating data

    CursorLogFile:
        In-memory file for unit tests

EventualIndex:
    A memoization class that iterates lines from piecewise text and remembers their locations
    Subsequent pieces merge together so the index is eventually complete.
    The text can be iterated forwards or backwards.
    Different chunks can be indexed on different threads.
    Understands relative offsets like "StartOfFile", "After(pos)", etc.

    Location:
        A cursor into an EventualIndex. Represents memoized locations, new locations (gaps),
        and virtual locations.

    Index:
        A partial index of line-ending locations for a single chunk of text.

LineIndexer:
    Combines LogFile + EventualIndex to allow iterating lines from the file forwards and backwards efficiently

LineViewMode:
    Describes how a line should be displayed: whole, wrapped, or partial

Log:
    Combines LogFile + EventualIndex
    Supports iterators that memoize the line locations as they're used.

LineIndexerIterator:
    Iterates line offsets from an Eventualindex using a DoubleEndedIterator

LineIndexerDataIterator:
    Iterates LogLines from a Log using a DoubleEndedIterator

SubLineHelper:
    Breaks a line up into chunks appropriate for LineViewMode
    Supports iterating forwards or backwards
    Supports iterating from an offset to a subsection of a line (e.g. lines are wrapped and we're iterating from a saved position)

SubLineIterator:
    Iterates log lines in different view modes: Whole, Clipped or Wrapped

LogLine:
    Information about a log line from a log file.
    Includes the line text and offset in the file.
