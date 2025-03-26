# Filters

## Functions
Filters do two things: they limit the lines we need to see on the screen and they tell us about the lines matching our
search that do not see.  For example, my filter may have 21 matches in the file but only 10 of them are visible.
The invisible lines are hidden by some other filter, lower in the stack. The filter is oblivious to other filters
_higher_ in the stack, however.

So we need two sets of match information for each filter. One set indexes the all the lines in the file and one indexes
only the currently visible lines at this level of the filter stack.

For discussion, let's call these two sets the `All` index and the `Visible` index. For reasons that will become
clear, the `Visible` index should be a proper subset of the `All` index. That means that even though the `All` index has
gaps when we're scanning only visible lines, we still insert matches into `All` whenever we insert them into `Visible`.

The expectation is that when some lower filter changes, we only need to invalidate the `Visible` index so it can be
rehydrated from the new base information, and then only as needed by directly iterating the next lower filter and
matching its `Visible` lines against our `All` index. But for any lines we already checked previously, we won't need
to check them again since the `All` index will have the match info for us.

There is a subtle third thing the filter must do, and that is to hydrate these two sets on-demand, efficiently. If
the user wants to see the next 10 visible lines, the filter should focus on iterating exactly those visible lines
which may be next, skipping over all the ones which are already filtered out by the earlier filters. But once we
are idle and have no immediate needs, the filter should be able to scan the rest of the file to find matches which
are not visible to fill out the `All` index.

Accordingly, the filter should have two types of iterators: `iter_visible()` and `iter_all()`.  The former only needs to
iterate the `Visible` index (building it up when there are gaps by relying on the descendent Filter's `iter_visible()`),
and the latter simply iterates the descendent filter's `iter_all()` (and so on recursively) giving our filter and its
descendants the chance to match against any "new" lines in the file that haven't yet been visited.

Somewhat importantly, the `iter_all()` iterator only needs to iterate line offsets. So if some base filter changes but the
file is already indexed, the iterators can work entirely in RAM without needed to read actual lines from the file _unless_
some filter in the chain has never seen that line.  Then the filter can choose to read the line from the file on demand
and populate it in the filter chain it sends downstream.  If that filter hasn't seen the line before, presumably the
downstream filters haven't either and may need to read it. We should only do that once. (It's possible the downstream
already saw the line since our filter may have been changed or disabled before; in that case they can simply pass along
the already-read line without scanning it.)

## Types of filters
There are two kinds of filters: filter-in, filter-out.
Those are the terms `lnav` uses, and they fit fine.  But I often think of them as `grep` and `grep -v`, or `include` and `!include`.

There is also a search type which I'll call an `anchor`.  `lnav` calls this a `search`, I think.  We can also use `highlighters` as
a search type, but it may not merit the extra baggage since we only use it for the current display.  In `felon` there can
be multiple highlighters, and we could also support multiple anchors, I suppose.  But this starts to sound too complex for a UI.

There are possibly different types of matchers. `Regex` is the typical one, but we may also want to exclude lines that
are `after(timestamp)` or `before(timestamp)`.  In `lnav` these are `hide-before` and `hide-after`.  Another possibility
is to `hide-before line-number` and `hide-after line-number`.

In implementation, the `before` and `after` filters can make some assumptions about sorting to avoid timestamping all the
lines. Instead they could look ahead/behind in batches of 100 lines, say, and only timestamp the edge. If the time there
is still in range and our last line checked is in range, then all the lines in between are, too.  This works in both
directions since the before/after filters are separate entities.

## Combinations of filters
It should be possible to construct proper algebraic expressions of these kinds of filters; eg.
`where ((includes(foo) and includes(bar)) or includes(baz)) and !includes(frob)`

But in practice we have simpler needs. We rarely need to combine `includes` with `and`. We only use `or` for those. That is,
we typically want to see "all of these lines, and all of those lines, and none of the-others."  Like this:
`where (includes(foo) or includes(bar) or includes(baz)) and !includes(frob) and !includes(flutter)`

But that easily transforms to this:
`where (includes(foo) or includes(bar) or includes(baz)) and !(includes(frob) or includes(flutter))`

So we can combine all the `includes` with `or` in one group and all the `!includes` with `or` in another (the `excludes`). Then we show all of the lines
that match any `include` which do not match any `exclude`. This is equivalent to removing all the lines that match any `!include`
and then including all the lines that match any include, with the caveat that the empty set of `excludes` matches `nothing`, while the
empty set of `includes` matches `everything`.

The handy thing about this construct is that we can reduce it to two sets of `includes` combined with `or`. These might be combined
efficiently in similar combo-matcher structures and then reduced at the end with some rather simple logic. However, since
our filters will operate on iterators, it's not so straightforward.  We'll probably need special logic for each of the
`ExcludeSet` and the `IncludeSet`.


`where !ANY(exclude_set) && ANY(include_set)`

Notice that the `before` and `after` filters fit nicely in the `excludes` bucket.

## Filters do not contain LogFiles
I could make a filter a "layer" in the stack such that it replaced the LogFile higher up.  In fact, this has been my plan.
It won't work.  I need to be able to pass LogFile to different filters to be handled discretely, so no Filter can own
the LogFile, as it turns out, unless each Filter owns each subsequent Filter. They could own a `RefCell<Rc<LogFile>>` or
something, but that's unnecessary.  It's probably better to have an interface that accepts Log or LogFile instead.

    trait Filter {
        memoize(&mut self, line: &LogLine, result: bool)  {
            // Insert line offset into matches or nomatches
        }

        eval(&mut self, line: &LogLine) -> bool {
            // Evaluate the filter and remember the result
            let eval:bool = self.apply_filter(line);
        }

        quick_next(&self, offset: usize) -> Option<usize> {
            // Evaluate offset if we already have this line memoized.
            //    If so, return location of next included line, or Gap<Location>
        }

        iter_range(&mut self, log: &mut Log) -> Iterator<> {
            type Item = LogLine;
        }
    }

#### Thinking: Skip list?
Maybe our include/exclude list should be a skiplist.  This would allow us to quickly answer the question "wh

## Filter ordering
Implementation note: When combining filters with `and` the order doesn't matter much to the user. But it may matter a lot to us.
A filter with a higher number of excluded lines is better to do first since the downstream filters will then have less work to
consider.  But we can't really know which ones have more matches until we scan the sources.  Then, however, we may be able to
make some better choices for ordering, even if we only scan a small piece of the source file to start with.

## Base example
Suppose I have a filter-in filter. Its job is to know which lines to show from the source file. For example,
let's say I have a filter-in filter that shows even numbers.

`Source:   0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64`
`Filter-A:`
`    All     = [ 1 3 5 7 9 11 13 15 17 19 21 23 25 27 29 31 33 35 37 39 41 43 45 47 49 51 53 55 57 59 61 63 ]`
`    Visible = [ 1 3 5 7 9 11 13 15 17 19 21 23 25 27 29 31 33 35 37 39 41 43 45 47 49 51 53 55 57 59 61 63 ]`

When I iterate the Filter, at first it doesn't know which lines are in the source.  So it iterates the source and discovers which
lines match and which do not. It remembers which ranges it covered in an EventualIndex, and it leaves gaps where it hasn't iterated
yet.  Like this:
`Filter-A: EventualIndex:  AllIndex {start: 0, end: 19, offsets: [1 3 5 7 9 11 13 15 17 19]}`

Now suppose I have another filter-in that includes only multiples of 3.

Filter-B:       3           9                15                21                27              33    ...
Filter-B: EventualIndex:  Index {start: 0, end: 19, offsets: [3 9 15]}

Eventually this filter should know it has 10 visible matches {3 9 15 21 27 33 39 45 51 57 63} and 11 invisible matches {0 6 12 18 24 30 36 42 48 54 60}.
It can learn the visible matches by iterating over Filter-A.  But it eventually must learn about matches that filter-A didn't show.  For that
it needs to iterate the Source, but it needn't check on lines it already included.  We can simply iterate the source and skip the numbers we already
confirmed as matched. For this we need two EventualIndexes, though.  One represents all the lines that matched from Filter-A, and the other
represents all the lines that matched from Source.

Suppose I have 3 filter-out filters.  A filters out multiples of 3, B filters out multiples of 5, and C filters out even numbers.

Source: 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 81 82 83 84 85 86 87 88 89 90 91 92 93 94 95 96 97 98 99 100
Filter A:

## Pre-defined filters
In the future I imagine being able to pre-define some filters, like
`NOISE: filter-out flutter|fluttask|shmem`
`SPACE: gc.main|metrics|triage_svc|crawler|rollup|...`

These could quickly be turned on/off or referred to in match expressions (instead of a REGEX).  As on/off features, they
may have have clickable checkboxes on-screen all the time.
