// Tests for EventualIndex

use indexed_file::index::Index;
use indexed_file::eventual_index::{ EventualIndex, Location, GapRange, Missing::{Bounded, Unbounded}, VirtualLocation, IndexRef };

static DATA: &str = "a\na\na\na\na\n";

fn get_index(offset: usize) -> Index {
    let mut index = Index::new();
    index.parse(DATA.as_bytes(), offset);
    index
}

fn get_eventual_index(size: usize) -> EventualIndex {
    let mut index = EventualIndex::new();
    while index.bytes() < size {
        let s = index.bytes();
        println!("Size {s}");
        index.merge(get_index(index.bytes()));
    }
    index.finalize();
    index
}

fn get_partial_eventual_index(start: usize, size: usize) -> EventualIndex {
    let mut index = EventualIndex::new();
    while index.bytes() < size {
        let s = index.bytes();
        println!("Size {s}");
        index.merge(get_index(start + index.bytes()));
    }
    index.finalize();
    index
}

#[test]
fn test_eventual_index_basic() {
    let index = get_eventual_index(100);
    assert_eq!(index.bytes(), 100);
    assert_eq!(index.lines(), 51);
}

#[test]
fn test_cursor_start() {
    let index = get_eventual_index(100);
    let cursor = index.locate(0);
    dbg!(cursor);
    match cursor {
        Location::Indexed(IndexRef{index: 0, line: 0, offset: 0}) => {},
        _ => {
            dbg!(cursor);
            panic!("Expected StartOfFile; got something else");
        }
    }
}

#[test]
fn test_cursor_mid_start() {
    let index = get_partial_eventual_index(50, 100);
    let cursor = index.locate(50);
    match cursor {
        Location::Indexed(IndexRef{index: 0, line: 0, offset: 52}) => {},
        _ => panic!("Expected Index(0, 0); got something else: {:?}", cursor),
    }
    let fault = index.locate(10);
    match fault {
        Location::Gap(GapRange { gap: Bounded(0, 50), .. } ) => {},
        _ => panic!("Expected Missing(0,50); got something else: {:?}", fault),
    }
}

#[test]
fn test_cursor_last() {
    let index = get_eventual_index(100);
    let cursor = index.locate(index.bytes()-1);
    match cursor {
        Location::Indexed(_) => {},
        _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
    }
    let fault = index.locate(index.bytes() + 1);
    match fault {
        Location::Gap(GapRange { gap: Unbounded(_), .. }) => {},
        _ => panic!("Expected MissingUnbounded; got something else: {:?}", fault),
    }
}

#[test]
fn test_cursor_forward() {
    let index = get_eventual_index(100);
    let mut cursor = index.locate(0);
    let mut count = 0;
    loop {
        // dbg!(&cursor);
        match cursor {
            Location::Indexed(_) => {},
            Location::Gap(GapRange { gap: Unbounded(_), .. }) => break,
            _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
        }
        count += 1;
        println!("Line {}  Cursor: {}", count, index.start_of_line(cursor).unwrap());
        cursor = index.next_line_index(cursor);
    }
    assert_eq!(count, index.lines());
}

#[test]
fn test_cursor_reverse() {
    let index = get_eventual_index(100);
    let mut cursor = index.locate(99);
    let mut count = 0;
    let mut prev = 100;
    loop {
        match cursor {
            Location::Virtual(VirtualLocation::Start) => break,
            Location::Indexed(_) => {},
            _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
        }
        count += 1;
        let start = index.start_of_line(cursor).unwrap();
        println!("Line {}  Cursor: {}", count, start);
        assert!(start <= prev);
        prev = start;
        cursor = index.prev_line_index(cursor);
    }
    assert_eq!(count, index.lines());
}

#[test]
fn test_cursor_reverse_gap() {
    let index = get_partial_eventual_index(50, 100);
    let mut cursor = index.locate(149);
    let mut count = 0;
    loop {
        dbg!(&cursor);
        match cursor {
            Location::Indexed(_) => {},
            Location::Gap(GapRange { gap: Bounded(0, 50), .. } ) => break,
            _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
        }
        count += 1;
        cursor = index.prev_line_index(cursor);
    }
    assert_eq!(count, index.lines());
}
