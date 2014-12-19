#![feature(slicing_syntax)]

extern crate gapbuffer;

use gapbuffer::GapBuffer;

#[test]
fn test_init() {
//Test declaration & initialization
    let test: GapBuffer<uint> = GapBuffer::with_capacity(100);
    assert!(test.capacity() >= 100, "buffer initialized to {} capacity", test.capacity())
    assert!(test.len() == 0, "Buffer initialized to {} length", test.len())

}

#[test]
fn test_insert() {
    let mut test: GapBuffer<uint> = GapBuffer::new();

    //Test insertion to end.
    for x in range(0, 100) {
        if x % 2 == 0 { test.insert(x/2, x); }
    }
    assert!(test.len() == 50, "After even insertions, buffer length is {}", test.len())

    //Test insertion in the middle.
    for x in range(0, 100) {
        if x % 2 == 1 { test.insert(x, x); }
    }
    assert!(test.len() == 100, "After odd insertions, buffer length is {}", test.len())
}

#[test]
fn test_iter() {
//Test iteration.
    let mut test: GapBuffer<uint> = GapBuffer::new();

    for x in range(0, 100) {
        test.insert(x,x);
    }

    let mut iterator = test.iter();
    let mut index = range(0,100);
    loop {
        match (iterator.next(), index.next()) {
            (Some(x), Some(y)) => {
                assert!(Some(x) == Some(&y), "(backward iter) Element at index {} is {}", y, x);
            }
            (None, _) | (_, None) => { break }
        }
    }
    loop {
        match (iterator.next_back(), index.next_back()) {
            (Some(x), Some(y)) => {
                assert!(Some(x) == Some(&y), "(backward iter) Element at index {} is {}", y, x);
            }
            (None, _) | (_, None) => { break }
        }
    }

}

#[test]
fn test_index() {
//Test indexing.
    let mut test: GapBuffer<uint> = GapBuffer::new();

    for x in range(0, 100) {
        test.insert(x,x);
    }

    for x in range(0,100) {
        assert!(test[x] == x, "Index {} failed", x);
    }

}

#[test]
fn test_remove() {
//Test removal.

    let mut test1: GapBuffer<uint> = GapBuffer::new();
    let mut test2: GapBuffer<uint> = GapBuffer::new();

    for x in range(0, 100) {
        test1.insert(x,x);
        test2.insert(x,x);
    }

    for x in range(0,100) {
        assert!(test1.remove(0) == Some(x), "Remove failed at {} (forward)", x);
    }

}

#[test]
fn test_slice() {

    let mut test = GapBuffer::new();

    for x in range(0, 5) {
        test.insert(x,x)
    }

    let mut slice = test[].iter();
    let mut index = range(0, 5);
    loop {
        match (slice.next(), index.next()) {
            (Some(x), Some(y)) => { assert!(x == &y, "Slice failed in []"); }
            (None, _) | (_, None) => { break }
        }
    }

    slice = test[3..].iter();
    index = range(3, 5);
    loop {
        match (slice.next(), index.next()) {
            (Some(x), Some(y)) => { assert!(x == &y, "Slice failed in [3..]"); }
            (None, _) | (_, None) => { break }
        }
    }

    slice = test[..3].iter();
    index = range(0, 3);
    loop {
        match (slice.next(), index.next()) {
            (Some(x), Some(y)) => { assert!(x == &y, "Slice failed in [..3]"); }
            (None, _) | (_, None) => { break }
        }
    }

    slice = test[1..4].iter();
    index = range(1, 4);
    loop {
        match (slice.next(), index.next()) {
            (Some(x), Some(y)) => { assert!(x == &y, "Slice failed in [1..4]"); }
            (None, _) | (_, None) => { break }
        }
    }

}
