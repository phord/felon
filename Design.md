# Planning

Here's how I think this will evolve:

There is a Document that manages the source data.
The document holds a merged collection of log lines / fragments.

Struct          Description                                 Iterator produces
============    =========================================   =======================================================
Logs            Generic log line sources                    LogLine, Offset
TimeStamper     Applies timestamp to log lines              LogLine, Offset, Timestamp
MergedLogs      Collection of logs merged by timestamp      LogInfo: Log lines + source info + MergedOffset
Filters         Include/Exclude filters to reduce lines     LogInfo + SkipInfo? {number of lines skipped}
Markers         Highlighter for produced log lines          DispLogInfo: LogInfo + Marker
Search          Marks lines that match search exprs         DispLogInfo: LogInfo + Marker + Anchor
Formatter       Applies Markers to line text
Framer          Fit log lines to the page width (chopping)  DispLogInfo + ClipInfo {subsection of line to display}
Navigator       Applies navigation to log lines             FragmentLogInfo + ???

Document        Thing that gets navigated and indexed       Holds: Logs, Searches, Filters, Width, etc.

Cursor          Thing that translates Doc-locations to Log-locations
                Is timestamp good enough?  Timestamp + Lines?
                Do we even need this?  Figure it out when we get there.

Iterated log line struct:
-   LogLine
-   _?? Text-only LogLine (for matching around ESC codes)_
-   Offset
-   Timestamp
-   FileIndex/FileName
-   DocOffset
-   _?? SkipInfo (GapInfo)_
-   ClipInfo: Vec<usize>
-   Markers: Vec<Marker>
-   Anchors: Vec<Marker>  **<--** _Markers that are also navigation points, memoized_

We can lookup these fields when requested, if available:
-   LineNumber: usize,
-   DocLineNumber: usize,
-   DocSizeBytes: usize,

Document:
-   MergedLogs,
-   Searches
-   FilterIns
-   FilterOuts
-   FrameWidth
-

## Multi-regex matching
There is some advantage to be had when matching multiple regexes on a string. But there is also some cost
which can be eliminated. If we know we will show a line, we could choose to match all the regexes needed
for displaying it (the formatter, timestamp, search, highlight, etc.).  But some regexes will include or
exclude the line.  So we should categorize them somewhere.

1. FilterOut - is used to exclude lines from display
2. FilterIn - is used to include lines and highlight matched locations
3. Timestamp - is needed to determine whether the line is to be displayed next
4. Highlight - is used to color parts of lines
5. Format - is used to color parts of lines

Maybe only the last two are useful to combine.  But within each type we could combine all the regexes for that
type together in one RegexSet. RegexSet can tell us if any matches exist cheaply, so it can be used efficiently
for both filterIn and filterOut.


## Roadmap:
* Framer
  * test with `--bin more` pager
* Markers and Formatter
* Search: marker generation
* Search: navigation / memoization
* Wrappers for rest of above interfaces

## MergedLogs
Lines are produced in order of (timestamp, file index, file offset). Thus the lines are sorted
by timestamp, but multiple lines with the same timestamp have a consistent ordering, and
lines within a file with the same timestamp have a stable ordering.

Placement of MergedLogs in the stack is TBD. Are there benefits to apply search filters
before or after merging? Suppose we are iterating a large compressed file from the end.
In that case it's more important to find the chunk of data near the end so we can display
it, but we don't need to wait for all of it to be searched before then.

How would we navigate the doc if searches are applied per-file?  We would need to ask each
file to tell us the next target and then choose the least of them by timestamp. Would this
be more reasonable from a Doc level?  Maybe I'm overthinking it a lot.  How often will
so many files be present that it even matters?

### Decision: Apply filters at the file-level

Filters (including/excluding/counting lines) should be applied at the file level, before merging.
**Reason:** If we filter *after* the merge, it simplifies our code a bit, but it also requires that we
compare every line in every file with others to determine the order first. Then we would apply
the filters afterwards.  If we filter out 99% of 1 million lines, we would have to first order
the million lines and then filter them all.  But if we filter first and then sort, we only need
to filter 1 million and then sort 100,000. However, in fact we only have to sort the lines we
end up displaying.  So we may filter 1M then only sort 200.  But if we merge first, we would not
be able to avoid sorting at least a lot more lines.

There may be some filters which apply to timestamps. These should be applied at the file-level, too.
Timestamp is trying to be lazy so we don't calc it on every line. For range-filters (e.g. filter-out
lines before 10pm) we can apply the filter quickly by a binary search on file data, so still every
line doesn't need to be stamped. As similar optimization can apply to sorting where, for example,
two files which do not overlap in time do not need to be compared line-by-line to determine
ordering.

## Search
The search layer will produce lines as a "pass-through" service adding Anchors.
It memoizes found location to speed up repeated and future searches. Future searches
can be managed asynchronously by running some service worker owned by the Search struct.
This worker walks through bounded iterations of the child structs to find the next
matching lines and building up the memoization information.

This layering needs some interface to allow us to quickly filter immediate results,
to concurrently build filters for complete results in the background, and to identify
regions still needing to be filled.  For example, we use normal iterator functions for
immediate results, but we should constrain the result to a given timestamp.  This
allows us to take the next matching line from one file without burning too much CPU
searching all the other files to find a match whose timestamp then needs to be checked.
Instead, the search for the other files can know they can stop when they exceed some
timestamp.  But for this we assume there is some "lead" file who already found a line
and therefore can tell us a target timestamp. This is not guaranteed, though, so instead
we also need our iterator to search in small ranges whenever it's in a gap so we can
rotate among other sources until we do find one with a match.

This seems like it might not be an improvement, but I believe it is.  Supposed we have
two 1 million line files and we apply a filter that removes 99% of the lines.  So we have
20,000 lines remaining to be displayed.  Perhaps all 20,000 are in one file. If I try
naively to search the other file for its "next" visible line, I won't find any. So I have to
search the entire file applying the filter to every line in the file. But we can do better.

1. Search up to 1,000 lines of file1 for the next matching line.
   a. Get the timestamp of the last line we checked in file1.
2. Search up to 1,000 lines of file2 for the next matching line.
   a. Get the timestamp of the last line we checked in file2.
3. There are three possibilities:
   a. We have no matching lines.  --> Repeat from step 1.
   b. We have a match from both files.  --> Yield the line whose timestamp is lower.
   c. We have a match from one file, but not the other.
      i. If the timestamp of the matched file is lower than the checked timestamp of all the other logs, yield that line.
     ii. Otherwise, keep searching the other files in 1,000 line blocks until all timestamps are larger than our matched line.  (3.c.i)



## Markers
Markers are used for highlighting text according to some rules, highlight requests, or
search requests.  Rules can be things like timestamp fields in green, semantic highlighted
module names and numbers, etc. Markers are only needed at display time and they don't get
memoized or even generated for unseen lines.

## Anchors
Anchors represent navigation points in the document.  They are places we can jump to by
searching forward and backward. Do we need anchors for time-based offsets?

## TimeStamper
Parses the log line to find timestamp information which matches some defined log format.
If no timestamp is matched, the log operates in some plaintext mode (details TBD).
The log is presumed to be in _mostly_ sorted order (by timestamp).  Out-of-order lines
are handled specially as "blocks".  That is, lines which have no timestamp or whose
timestamp is lower than their predecessor's are assumed to have the same timestamp as
the preceding line. Thus all the lines in a block are presented together in time.
Handling this correctly requires some supposition about the maximum number of out-of-order
lines to consider as a block, and it requires us to look ahead (or behind) this many lines
to process blocks consistently.

Timestamp parsing is somewhat expensive.  We can mitigate it somewhat by only calling for
it when needed.  It is only needed when two files are being merged, or when a user wants
to jump to some time offset in a single file, or wants to show the delta column (time between
lines), or something.

We can also avoid timestamping lines that later get filtered out. This requires us to filter
before we need the timestamp, though.  It implies that we filter at a low level, perhaps. At
least it must happen at some level before we merge files together.

## Framer
Handles displaying and tracking log line fragments. There are two modes to consider here:
Wrapping or Chopping. Chopping is similar to "no framing", and can be largely ignored.

In wrapping mode, each line is iterated in passthrough mode with a FragmentInfo added. Lines
which are longer than the desired width will be iterated more than once, with different
FragmentInfo for each duplicate indicate a separate section of the line as the fragment.
A fragment is a section of a line which will fill at most one page-width of text for a row.
Iterating forward is easy to reason about.  If a line is shorter than the requested width,
the fragment contains the whole line.  If it is longer, the fragment contains only the
desired width of characters, and the next line iterated contains the remaining characters,
again constrained to the width, and so on.

Iterating in reverse order simply does this process in reverse.  Shorter lines get a single,
all-inclusive FragmentInfo.  Longer lines get clipped FragmentInfo data, but the fragments
are reversed so the "last" fragment is produced first, then the next-to-last, and so on.

So what happens when we search forward to a matched string? The AnchorInfo tells us where
the match is by MergedOffset. In Chopped mode, we can simply scroll to that line. In wrapped
mode, we can scroll to the line that contains that offset. But consider that very long lines
may exceed a whole page in size (width * height), so we must scroll to the fragment within
the line if the fragments since the start of the line exceed our page height.

# Async considerations
Some work should be done synchronously to produce immediate display results, but some should
be deferred to some background process to handle asynchronously. Consider three cases:

1. Filter by some string: We can iterate the near lines of the Doc to find enough strings
   that match the filter to fill the page.  Then we can continue searching in the background
   to find the lines for the next page and for the rest of the document.

2. Line count: When we scroll to the end of the document, we may want to show the line number.
   We don't know the line number until we index the whole document. So we can show some placeholder
   until we do know the line numbers, then continue counting the lines in the background. Once
   the displayed lines are known, they can be updated on the screen.  (Bear in mind that we will
   likely never be in a situation where some of the displayed numbers are known and some are not.)

3. Search for string with filtered display: We want to find all the useful anchors in the file so we can jump
   to other found locations.  We also want to show the user how many matches were found, both
   in the displayed lines and the hidden lines.  We never need to jump to the hidden lines, but
   having the count is useful. Having the locations is also useful in case the user disables
   a filter, because we won't need to search again. But it's most useful to us to have the
   filtered search results first (our immediate need) so we may need to do two passes of searches.

Since loading lines is our most expensive operation, we should take advantage of as many concurrent
searches as possible. Therefore our async processes can follow a similar workflow to our sync ones,
where each line is presented to each processor in turn for consideration as needed.  Each processor
can keep track of which lines it has already considered so it doesn't need to do any duplicate work.

This include/exclude and eventual-index data should be wrapped into some common trait or struct
since it will be useful in many different contexts.

It may be useful to index some sections in a multi-threaded way allowing mp systems to run faster.
But it would be wasteful to decode the same sections of compressed files in separate threads
since each would be forced to decompress the same piece again when the lines are in the same frame,
for example. So some intelligence must exist to minimize this and coordinate any queued up indexer
work happening in the background.

The background processing of lines from different files can be done efficiently in completely
different threads, so perhaps it makes sense to queue up work per-file below the MergedLogs point.
But this must be done in a way that still allows the results eventually to be associated with
their offsets in the Merged document (which has different offsets to consider).

Question: Can we rely on the LogInfo:file-offset for this information?  How do we turn that
          into a cursor later on to allow us to navigate easily?

### What work is there to do in the background?
1. Line indexing
2. Search matches
3. Filter-in matches
4. Filter-out matches
(Highlight matches do **not** need to be in the background.)

One way to organize these is to store sets of all matches vs. non-matches so we can easily turn filters
on/off without having to re-scan everything later. But we can get results faster if focus on "matched"
items first; for example, if a line matches a filter-in, we can stop matching further filter-ins for that
line. Similarly for filter-outs, but we can also delay matching searches on these.  Searches can also be
delayed for lines which match no filter-ins.  However, the delayed work will need to be done eventually,
and if we have to load the lines twice, it may be more expensive than applying the searches on the first
pass.

Probably the right thing to do is to build an EventualIndexSearch struct that knows its gaps for each
one of these items. Then we can parse gaps for all as they are encountered, skipping parsing for sections
already indexed, and scanning gaps in the background that need to be filled in for any search.  One
extra concern for the searches is that they are line-based while the line-index is char-based. We may
need to load lines separately for the matchers after we index individual lines.

# Filters

I have a lot of random thoughts about filter implementations, but not all of them are good ideas.  I've placed
them in Filters.md

====
# Old notes

Filters are applied
LogFile - access to lines from files
LineIndexer - Access to indexed lines within a file
    - Loads and indexes sections of file on-demand as needed
LogFilter - applies a search filter to log lines

BundleView - collects filters and files, memoizes resulting set of lines and give access to log lines

Navigator - moves a cursor in a BundleView
Display - renders lines on console; navigates file with keyboard commands

felon -m5 shows "first 5 matches"
felon -m-5 shows "last 5 matches" (because we can do this efficiently on text files)


There are three levels of urgency for filter/highlight results:
    1. Match visible lines on the screen
    2. Match filtered-in lines in the file
    3. Match all filtered-out lines in the file

Case: Create a new immed-search on a filtered view of the file
    Find 1 to highlight matches as I type.
    If there are none there, find 2 to show examples of matches elsewhere in the file, and to skip ahead when applied.
    Find 3 to show a count of "hidden matches" on the status line

Case: Create a new filter-out
    Find 1 to highlight on-screen matches as I type.
    If there are none there, find 2 to show examples of matches elsewhere in the file.
    Find 2 + 3 to show a count of "hidden matches" next to the filter

Case: Create a new filter-in
    Find 2 + 3 to show a count of "hidden matches" next to the filter

Another Rust pager has entered the chat!
https://github.com/dandavison/delta/blob/master/manual/src/features.md


Roadmap:

- Create `cat` command to read file lines and render them in colors
  - Multiple files are merged by timestamp
- Create `grep` command to filter file lines and render them in colors with matched-region colors
- Teach LineIndexer to parse lines in separate threads again
- Create readers for gzip and zstd


====

Design idea using simple rust std:: traits.

Have a memoizing line iterator that operates on a ReadBuf.
The ReadBuf can try to read more data in as it's added past EOF.  (I think)

Can we implement a BufReader with "unlimited" capacity?
StdinLocked already implements ReadBuf.  Can we reimplement this in terms of our infinite buffer?

As a start, maybe we can use BufReader::with_capacity(10 * 1024 * 1024, f);  But this seems
inefficient, like it would lock up 10M of memory, and whatever else we decide we need.

Also, BufReader implements Seek... partially correctly for our needs, but without reading
intermediate data which is needed from a stream.
