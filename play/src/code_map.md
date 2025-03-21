Code map: indexed_file

LogSource:
    Polymorphic wrapper of LogFile (literally just a type = Box<LogFile>)

    LogFile:
        Helper trait that augments BufRead + Seek
        Common helper functions to navigate a BufRead + Seek for line reading

    LogBase:
        A trait to facilitate conversion from LogFile -> LogSource and vice versa

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

SaneIndex:
    Replacement for EventualIndex that builds its index in-place as it is iterated.
    Less modular than EventualIndex, but hopefully more sane.

    SaneIndexer:
        TBD: Combines LogSource and SaneIndex; includes multiple Index filters that are updated as-we-go.
        Indexes: includes, excludes, searches.
        Supports background scanning/updating with timeout.
        Can be iterated with SaneLines.
        Will be used by SubLineIterator to navigate and keep indexes up-to-date.


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
    Combines LogSource + SaneIndexer
    Primarily an interface to open different file types as an IndexedLog
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


Rust std::lib ideas:
    So we have a source of log lines which is already a BufReader.  And we have some filters
    we would like to apply to this collection of lines.  Cool.

    let doc = file.lines().filter(my_filter).take(screen_size).collect();

    This would be nice and portable for everyone, but it lacks many features I need.

    1. lines() returns an Iterator, but doesn't support DoubleEndedIterator::rev().
    2. filter() doesn't understand that my_filter has memoized line offsets for efficiency.
       There's no way to ask lines() to return only the lines already found in my_filter,
       except by moving my_filter into the iterator producer in the first place.
    3. It doesn't support background updating, either. But maybe that needs a separate
       interface anyway. The idea here is to allow iterate to return "checkpoints"
       periodically when there is no line found for some period of time. For example,
       imagine we open a large file and then search for "foo". It takes 20 seconds to
       search the huge file and find the one instance of "foo". We don't want to block
       while we're searching, so we'd like to say "file.search(200);" to force the search
       to return after 200ms if it hasn't found the target yet. If it takes too long, the
       user can try a different search (maybe) or quit altogether.

    Another option is to use async and run the searches in threads.  The UI would always be
    listening to lines on the channel, maybe.  But it still wouldn't make it easy to interrupt
    an active operation.
