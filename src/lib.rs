//  Copyright 2014 David Lee Aronson.
//
//  This program is free software: you can redistribute it and/or modify it under the terms of the
//  GNU Lesser General Public License as published by the Free Software Foundation, either version 3
//  of the License, or (at your option) any later version.
//
//  This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
//  without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See
//  the GNU Lesser General Public License for more details.
//
//  You should have received a copy of the GNU Lesser General Public License along with this
//  program.  If not, see <http://www.gnu.org/licenses/>.
#![feature(slicing_syntax)]
#![allow(unstable)]

extern crate core;
extern crate alloc;

use core::fmt;

use std::collections::ring_buf::RingBuf;
use std::iter::FromIterator;
use std::cmp::Ordering;
use std::ops::{Index, IndexMut};

/// A GapBuffer is a dynamic array which implements methods to shift the empty portion of the
/// array around so that modifications can occur at any point in the array. It is optimized for
/// data structures in which insertions and deletions tend to occur in sequence within the same
/// area of the array, such as a buffer for a text editor.
#[derive(Clone,Default)]
pub struct GapBuffer<T> {
    /// The start offset of the ring buffer.  This is necessary in order to prevent leftward
    /// motion from wrapping around from the conceptual front of the buffer to the back (or vice
    /// versa).
    offset: usize,
    /// The backing ring buffer.  Pushing onto the back is considered to insert a character
    /// into the leftmost empty slot in the gap, while popping from the front is considered
    /// deleting the leftmost nonempty slot after the gap.  Moving the gap right means cycling the
    /// first element to the back; moving left means cycling the last element to the front.
    buf: RingBuf<T>,
}

impl<T> GapBuffer<T> {
    ///Constructs an empty GapBuffer.
    pub fn new() -> GapBuffer<T> {
        GapBuffer {
            buf: RingBuf::new(),
            offset: 0,
        }
    }

    ///Constructs a GapBuffer with a given initial capacity.
    pub fn with_capacity(n: usize) -> GapBuffer<T> {
        GapBuffer {
            buf: RingBuf::with_capacity(n),
            offset: 0,
        }
    }

    fn get_idx(&self, i: usize) -> usize {
        if i < self.offset {
            // Left of cursor, so indexing starts at self.len() - offset.
            // Note the order: (self.len() - offset) should be evaluated first, since it is
            // guaranteed to be nonnegative, and then i should be added (it cannot exceed
            // self.len() since i < offset, hence it cannot overflow).
            (self.len() - self.offset) + i
        } else if i < self.len() {
            // At or right of cursor, subtract offset.
            i - self.offset
        } else {
            // i out of bounds--leave it that way.
            i
        }
    }


    /// Shift the gap in the gap buffer.  Note: does not perform bounds checks.
    fn shift(&mut self, i: usize) {
        // Since the caller should have checked bounds already, unwrap() in this function should
        // never fail.
        match i.cmp(&self.offset) {
            // Already at the correct position, don't do anything
            Ordering::Equal => return,
            // Need to move left
            Ordering::Less => {
                // Moving left means cycling the last element to the front.
                let mut last = self.buf.pop_back().unwrap();
                self.offset -= 1;
                while i < self.offset {
                    self.buf.push_front(last);
                    last = self.buf.pop_back().unwrap();
                    self.offset -= 1;
                }
                self.buf.push_front(last);
            },
            // Need to move right
            Ordering::Greater => {
                // Moving right means cycling the first element to the back.
                let mut first = self.buf.pop_front().unwrap();
                self.offset += 1;
                while i > self.offset {
                    self.buf.push_back(first);
                    first = self.buf.pop_front().unwrap();
                    self.offset += 1;
                }
                self.buf.push_back(first);
            }
        }
    }

    ///Get a reference to the element at the index.
    pub fn get(&self, i: usize) -> Option<&T> {
        let i = self.get_idx(i);
        self.buf.get(i)
    }

    ///Get a mutable reference to the element at the index.
    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        let i = self.get_idx(i);
        self.buf.get_mut(i)
    }

    /// Swap the elements at the index.
    /// i and j may be equal.
    ///
    /// Panics if there is no element with either index.
    pub fn swap(&mut self, i: usize, j: usize) {
        let i = self.get_idx(i);
        let j = self.get_idx(j);
        self.buf.swap(i, j);
    }

    ///Get the capacity of the GapBuffer without expanding.
    #[inline]
    pub fn capacity(&self) -> usize { self.buf.capacity() }

    /// Reserve at least this much additional space for the GapBuffer.
    /// The collection may reserve more space to avoid frequent reallocations.
    ///
    /// Panics if the new capacity overflows uint.
    pub fn reserve(&mut self, additional: usize) {
        self.buf.reserve(additional)
    }

    ///Get an iterator of this GapBuffer.
    pub fn iter(&self) -> Items<T> {
        Items {
            buff: self,
            idx: 0,
        }
    }

    ///Get the length of the GapBuffer.
    pub fn len(&self) -> usize { self.buf.len() }

    ///Is the GapBuffer empty?
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    ///Clears the buffer, removing all values.
    pub fn clear(&mut self) {
        self.offset = 0;
        self.buf.clear();
    }

    /// Insert a new T at a given index (the gap will be shifted to that index).
    ///
    /// Panics if i is greater than RingBuf's length.
    pub fn insert(&mut self, i: usize, t: T) {
        // Valid indices: [0, len]
        assert!(i <= self.len(), "index out of bounds");
        // Gap is just before index i
        self.shift(i);
        // push_back inserts into the leftmost empty slot in the gap.
        self.offset += 1;
        self.buf.push_back(t);
    }

    /// Removes and returns the element at position i from the gap buffer.  The gap will be shifted
    /// to just before the index.  Returns None if i is out of bounds.
    pub fn remove(&mut self, i: usize) -> Option<T> {
        // Valid indices: [0, len)
        if self.len() <= i {
            return None;
        }
        self.shift(i); // Gap is just before index i
        // pop_front removes from the rightmost empty slot after the gap.
        self.buf.pop_front()
    }
}

//Eq & PartialEq
impl <A, B> PartialEq<GapBuffer<B>> for GapBuffer<A> where A: PartialEq<B> {
    #[inline]
    fn eq(&self, other: &GapBuffer<B>) -> bool {
        if self.len() != other.len() { return false }
        // This isn't as efficient as it could be...
        self.iter().zip(other.iter()).all( |(x, y)| x == y )
    }
}

impl<A> Eq for GapBuffer<A> where A: Eq {}

//Ord & PartialOrd
impl<A> PartialOrd for GapBuffer<A> where A: PartialOrd {
    #[inline]
    fn partial_cmp(&self, other: &GapBuffer<A>) -> Option<Ordering> {
        match self.len().cmp(&other.len()) {
            Ordering::Equal => {
                for (x, y) in self.iter().zip(other.iter()) {
                    match x.partial_cmp(y) {
                        Some(Ordering::Equal) => continue,
                        cmp => return cmp,
                    }
                }
                Some(Ordering::Equal)
            }
            cmp => Some(cmp),
        }
    }
}

impl<A> Ord for GapBuffer<A> where A: Ord {
    #[inline]
    fn cmp(&self, other: &GapBuffer<A>) -> Ordering {
        match self.len().cmp(&other.len()) {
            Ordering::Equal => {
                for (x, y) in self.iter().zip(other.iter()) {
                    match x.cmp(y) {
                        Ordering::Equal => continue,
                        cmp => return cmp,
                    }
                }
                Ordering::Equal
            }
            cmp => cmp,
        }
    }
}

//FromIterator
impl<A> FromIterator<A> for GapBuffer<A> {
    fn from_iter<I: Iterator<Item=A>>(iterator: I) -> GapBuffer<A> {
        let buf = iterator.collect();
        GapBuffer {
            buf: buf,
            offset: 0,
        }
    }
}

//Extend
impl<A> Extend<A> for GapBuffer<A> {
    fn extend<T: Iterator<Item=A>>(&mut self, iterator: T) {
        let len = self.len();
        // push_back inserts into the leftmost empty slot in the gap.
        self.shift(len);
        // So, extending the ring buffer directly at this point will have the same effect as
        // repeated right insertions.  We don't need to modify the offset because the cursor stays
        // in place.
        self.buf.extend(iterator);
    }
}

//Show
impl<T> fmt::Show for GapBuffer<T> where T: fmt::Show {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "["));
        let mut iter = self.iter();
        if let Some(fst) = iter.next() {
            try!(write!(f, "{:?}", fst));
            for e in iter {
                try!(write!(f, ",{:?}", e));
            }
        }
        write!(f, "]")
    }
}

impl<T> Index<usize> for GapBuffer<T> {
    type Output = T;

    #[inline]
    fn index<'a>(&'a self, index: &usize) -> &'a T {
        let index = self.get_idx(*index);
        &self.buf[index]
    }
}

impl<T> IndexMut<usize> for GapBuffer<T> {
    type Output = T;

    #[inline]
    fn index_mut<'a>(&'a mut self, index: &usize) -> &'a mut T {
        let index = self.get_idx(*index);
        &mut self.buf[index]
    }
}

//### Iterator implementation. #####################################################################
// Could likely be made more efficient since we know we're inbounds...
#[derive(Clone)]
pub struct Items<'a, T: 'a> {
    buff: &'a GapBuffer<T>,
    idx: usize,
}

impl<'a, T> Iterator for Items<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        let next = self.buff.get(self.idx);
        if next.is_some() {
            self.idx += 1;
        }
        next
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.buff.len();
        (len, Some(len))
    }
}

#[cfg(test)]
mod tests {

    use GapBuffer;

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
                (Some(&x), Some(y)) => {
                    assert!(x == y, "Element at index {} is {}", y, x);
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

        let mut test1: GapBuffer<usize> = GapBuffer::new();
        let mut test2: GapBuffer<usize> = GapBuffer::new();

        for x in range(0, 10) {
            test1.insert(x,x);
        }

        for x in range(0,10) {
            assert!(test1.remove(0) == Some(x), "Remove failed at {} (forward)", x);
        }

        test2.extend(0..5);
        test2.remove(0);
        for (i, &x) in test2.iter().enumerate() {
            assert!(x == i + 1, "Remove test2 failed. Index {} is {}", x, i);
        }

    }
}
