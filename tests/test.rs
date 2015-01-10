// Copyright 2014 David Lee Aronson.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the
// GNU Lesser General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See
// the GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License along with this
// program. If not, see <http://www.gnu.org/licenses/>.

#![feature(slicing_syntax)]

extern crate gapbuffer;

use gapbuffer::GapBuffer;

#[test]
fn test_init() {
//Test declaration & initialization
    let test: GapBuffer<usize> = GapBuffer::with_capacity(100);
    assert!(test.capacity() >= 100, "buffer initialized to {} capacity", test.capacity());
    assert!(test.len() == 0, "Buffer initialized to {} length", test.len());

}

#[test]
fn test_insert() {
    let mut test: GapBuffer<usize> = GapBuffer::new();

    //Test insertion to end.
    for x in range(0, 100) {
        if x % 2 == 0 { test.insert(x/2, x); }
    }
    assert!(test.len() == 50, "After even insertions, buffer length is {}", test.len());

    //Test insertion in the middle.
    for x in range(0, 100) {
        if x % 2 == 1 { test.insert(x, x); }
    }
    assert!(test.len() == 100, "After odd insertions, buffer length is {}", test.len());
}

#[test]
fn test_iter() {
//Test iteration.
    let mut test: GapBuffer<usize> = GapBuffer::new();

    for x in range(0, 100) {
        test.insert(x,x);
    }

    let mut iterator = test.iter();
    let mut index = range(0,100);
    loop {
        match (iterator.next(), index.next()) {
            (Some(x), Some(y)) => {
                assert!(Some(x) == Some(y), "(backward iter) Element at index {} is {}", y, x);
            }
            (None, _) | (_, None) => { break }
        }
    }
    loop {
        match (iterator.next_back(), index.next_back()) {
            (Some(x), Some(y)) => {
                assert!(Some(x) == Some(y), "(backward iter) Element at index {} is {}", y, x);
            }
            (None, _) | (_, None) => { break }
        }
    }

}

#[test]
fn test_index() {
//Test indexing.
    let mut test: GapBuffer<usize> = GapBuffer::new();

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

    let mut test: GapBuffer<usize> = GapBuffer::new();

    for x in range(0, 100) {
        test.insert(x,x);
    }

    for x in range(0,100) {
        assert!(test.remove(0) == Some(x), "Remove failed at {} (forward)", x);
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
