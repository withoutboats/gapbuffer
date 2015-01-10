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

use core::default::Default;
use core::fmt;
use core::mem;
use core::num::{Int,UnsignedInt};
use core::ptr;
use core::raw::Slice as RawSlice;

use std::cmp;
use std::ops::Deref;
use std::iter::FromIterator;
use std::cmp::Ordering;
use std::ops::{Index, IndexMut};
use std::ops;

use alloc::heap;

static INITIAL_CAPACITY: usize = 8us; // 2^3
static MINIMUM_CAPACITY: usize = 2us;

pub struct GapBuffer<T> {
    /// A GapBuffer is a dynamic array which implements methods to shift the empty portion of the
    /// array around so that modifications can occur at any point in the array. It is optimized for
    /// data structures in which insertions and deletions tend to occur in sequence within the same
    /// area of the array, such as a buffer for a text editor.
    //
    // head is the point at which the gap begins, where pushes occur, the first element of the array
    //  which is treated as empty.
    // tail is the index in the underlying array at which the gap ends, where the element that
    //  logically follows the element before head is located.
    // cap is the maximum capacity of the array.
    // ptr is the pointer to the first element of the array.
    head: usize,
    tail: usize,
    cap: usize,
    ptr: *mut T
}

//get_idx returns the actual index from a logical index (so that it skips the gap)
fn get_idx(i: usize, leng: usize, head: usize) -> usize { if i < head { i } else { i + leng } }


impl<T> GapBuffer<T> {
    /// Turn ptr into a slice
    #[inline]
    fn buffer_as_slice(&self) -> &[T] {
        unsafe { mem::transmute(RawSlice { data: self.ptr as *const T, len: self.cap }) }
    }

    /// Moves an element out of the buffer
    #[inline]
    unsafe fn buffer_read(&mut self, off: usize) -> T {
        ptr::read(self.ptr.offset(off as isize) as *const T)
    }

    /// Writes an element into the buffer, moving it.
    #[inline]
    unsafe fn buffer_write(&mut self, off: usize, t: T) {
        ptr::write(self.ptr.offset(off as isize), t);
    }

    /// Returns true iff the buffer is at capacity
    #[inline]
    fn is_full(&self) -> bool { self.cap - self.len() == 1 }

    #[inline]
    fn get_idx(&self, i: usize) -> usize { get_idx(i, self.tail - self.head, self.head)}

    /// Copies a contiguous block of memory len long from src to dst
    #[inline]
    fn copy(&self, dst: usize, src: usize, len: usize) {
        unsafe {
            debug_assert!(dst + len <= self.cap, "dst={} src={} len={} cap={}", dst, src, len,
                          self.cap);
            debug_assert!(src + len <= self.cap, "dst={} src={} len={} cap={}", dst, src, len,
                          self.cap);
            ptr::copy_memory(
                self.ptr.offset(dst as isize),
                self.ptr.offset(src as isize) as *const T,
                len);
        }
    }

    ///Shift the gap in the GapBuffer.
    //     V         H         E
    //[o o o o o o o . . . . . o o o o]
    //
    //     H         E
    //[o o . . . . . o o o o o o o o o]
    //               M M M M M
    //
    //         H         E       V
    //[o o o o . . . . . o o o o o o o]
    //
    //                 H         E
    //[o o o o o o o o . . . . . o o o]
    //         M M M M
    fn shift(&mut self, i: usize) {

        if i < self.head { self.copy(self.tail - self.head + i, i, self.head - i); }
        else { self.copy(self.head, self.tail, i - self.head); }

        let gapsize = self.tail - self.head;

        self.head = i;
        self.tail = self.head + gapsize;

    }


}

impl<T> GapBuffer<T> {

    ///Constructs an empty GapBuffer.
    pub fn new() -> GapBuffer<T> {
        GapBuffer::with_capacity(INITIAL_CAPACITY)
    }

    ///Constructs a GapBuffer with a given initial capacity.
    pub fn with_capacity(n: usize) -> GapBuffer<T> {
        let cap = cmp::max(n + 1, MINIMUM_CAPACITY).next_power_of_two();
        let size = cap.checked_mul(mem::size_of::<T>())
                      .expect("capacity overflow");

        let ptr = if mem::size_of::<T>() != 0 {
            unsafe {
                let ptr = heap::allocate(size, mem::min_align_of::<T>())  as *mut T;;
                if ptr.is_null() { ::alloc::oom() }
                ptr
            }
        } else {
            heap::EMPTY as *mut T
        };

        GapBuffer {
            head: 0,
            tail: cap,
            cap: cap,
            ptr: ptr
        }
    }

    ///Get a reference to the element at the index.
    pub fn get(&self, i: usize) -> Option<&T> {
        if i < self.len() {
            let idx = self.get_idx(i);
            unsafe { Some(&*self.ptr.offset(idx as isize)) }
        } else {
            None
        }
    }

    ///Get a mutable reference to the element at the index.
    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        if i < self.len() {
            let idx = self.get_idx(i);
            unsafe { Some(&mut *self.ptr.offset(idx as isize)) }
        } else {
            None
        }
    }

    ///Swap the elements at the index.
    pub fn swap(&mut self, i: usize, j: usize) {
        assert!(i < self.len());
        assert!(j < self.len());
        let ri = self.get_idx(i);
        let rj = self.get_idx(j);
        unsafe {
            ptr::swap(self.ptr.offset(ri as isize), self.ptr.offset(rj as isize))
        }
    }

    ///Get the capacity of the GapBuffer without expanding.
    #[inline]
    pub fn capacity(&self) -> usize { self.cap - 1 }

    ///Reserve at least this much additional space for the GapBuffer.
    pub fn reserve(&mut self, additional: usize) {
        let new_len = self.len() + additional;
        assert!(new_len + 1 > self.len(), "capacity overflow");
        if new_len > self.capacity() {
            let count = (new_len + 1).next_power_of_two();
            assert!(count >= new_len + 1);

            if mem::size_of::<T>() != 0 {
                let old = self.cap * mem::size_of::<T>();
                let new = count.checked_mul(mem::size_of::<T>())
                               .expect("capacity overflow");
                unsafe {
                    self.ptr = heap::reallocate(self.ptr as *mut u8,
                                                old,
                                                new,
                                                mem::min_align_of::<T>()) as *mut T;
                    if self.ptr.is_null() { ::alloc::oom() }
                }
            }

            // Move the second segment of the GapBuffer

            let oldcap = self.cap;
            let oldtail = self.tail;
            self.cap = count;
            self.tail = self.cap - oldcap + oldtail;

            self.copy(self.tail, oldtail, oldcap - oldtail);

            debug_assert!(self.head < self.cap);
            debug_assert!(self.tail <= self.cap);
            debug_assert!(self.cap.count_ones() == 1);
        }
    }

    ///Get an iterator of this GapBuffer.
    pub fn iter(&self) -> Items<T> {
        Items {
            head: self.len(),
            tail: 0us,
            ghead: self.head,
            gtail: self.tail,
            buff: self.buffer_as_slice()
        }
    }

    ///Get the length of the GapBuffer.
    pub fn len(&self) -> usize { self.cap - self.tail + self.head }

    ///Is the GapBuffer empty?
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    ///Clear the GapBuffer. NOTE: datais not removed, just made inaccessible.
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = self.cap;
    }

    //Insert a new T at a given index (the gap will be shifted to that index).
    pub fn insert(&mut self, i: usize, t: T) {
        assert!(i <= self.len(), "index out of range");
        if self.is_full() {
            self.reserve(1);
            debug_assert!(!self.is_full());
        }
        if i != self.head { self.shift(i); }
        let head = self.head;
        self.head += 1;
        unsafe { self.buffer_write(head, t) };

    }

    //Remvoe from a given index (the gap will be shifted to that index).
    pub fn remove(&mut self, i: usize) -> Option<T> {
        assert!(i < self.len(), "index out of range");
        if i+1 != self.head { self.shift(i+1) }
        self.head = self.head - 1;
        let head = self.head;
        unsafe { Some(self.buffer_read(head)) }
    }

    #[inline]
    pub fn as_mut_slice<'a>(&'a mut self) -> &'a mut [T] {
        unsafe {
            mem::transmute(RawSlice {
                data: self.ptr as *const T,
                len: self.len(),
            })
        }
    }
}

//AsSlice
impl<T: Clone> AsSlice<T> for GapBuffer<T> {
    fn as_slice<'a>(&'a self) -> &'a [T]{
        unsafe {
            if self.head < self.len() {
                let data = heap::allocate(self.len(), mem::min_align_of::<T>())  as *mut T;
                for (i, t) in self.iter().enumerate() {
                    ptr::write(data.offset(i as isize), t.clone());
                }
                mem::transmute(RawSlice {
                    data: data as *const T,
                    len: self.len(),
                })
            } else {
                mem::transmute(RawSlice {
                    data: self.ptr as *const T,
                    len: self.len(),
                })
            }
        }
    }
}

//Clone
impl<T: Clone> Clone for GapBuffer<T> {
    fn clone(&self) -> GapBuffer<T> {
        self.iter().map(|t| t.clone()).collect()
    }
}

//Default
impl<T> Default for GapBuffer<T> {
    #[inline]
    fn default() -> GapBuffer<T> { GapBuffer::new() }
}

//Eq & PartialEq
impl <A, B> PartialEq<GapBuffer<B>> for GapBuffer<A> where A: PartialEq<B> {
    #[inline]
    fn eq(&self, other: &GapBuffer<B>) -> bool { PartialEq::eq(&**self, &**other) }
    #[inline]
    fn ne(&self, other: &GapBuffer<B>) -> bool { PartialEq::ne(&**self, &**other) }
}

impl<A: Eq> Eq for GapBuffer<A> {}

impl<T> Deref for GapBuffer<T> {
    type Target = [T];

    fn deref<'b>(&'b self) -> &'b [T] {
        self.as_slice()
    }
}

//Ord & PartialOrd
impl<A: PartialOrd> PartialOrd for GapBuffer<A> {
    #[inline]
    fn partial_cmp(&self, other: &GapBuffer<A>) -> Option<Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl<A: Ord> Ord for GapBuffer<A> {
    #[inline]
    fn cmp(&self, other: &GapBuffer<A>) -> Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

//FromIterator
impl<A> FromIterator<A> for GapBuffer<A> {
    fn from_iter<I: Iterator<Item=A>>(iterator: I) -> GapBuffer<A> {
        let (lower, _) = iterator.size_hint();
        let mut zip = GapBuffer::with_capacity(lower);
        zip.extend(iterator);
        zip
    }
}

//Extend
impl<A> Extend<A> for GapBuffer<A> {
    fn extend<T: Iterator<Item=A>>(&mut self, mut iterator: T) {
        let mut head = 0;
        for elem in iterator {
            self.insert(head, elem);
            head += 1;
        }
    }
}

//Show
impl<T: fmt::Show> fmt::Show for GapBuffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<T> Index<usize> for GapBuffer<T> {
    type Output = T;

    #[inline]
    fn index<'a>(&'a self, index: &usize) -> &'a T {
        &self.as_slice()[*index]
    }
}

impl<T: Clone> IndexMut<usize> for GapBuffer<T> {
    type Output = T;

    #[inline]
    fn index_mut<'a>(&'a mut self, index: &usize) -> &'a mut T {
        &mut self.as_mut_slice()[*index]
    }
}


impl<T> ops::Index<ops::Range<usize>> for GapBuffer<T> {
    type Output = [T];
    #[inline]
    fn index(&self, index: &ops::Range<usize>) -> &[T] {
        self.as_slice().index(index)
    }
}
impl<T> ops::Index<ops::RangeTo<usize>> for GapBuffer<T> {
    type Output = [T];
    #[inline]
    fn index(&self, index: &ops::RangeTo<usize>) -> &[T] {
        self.as_slice().index(index)
    }
}
impl<T> ops::Index<ops::RangeFrom<usize>> for GapBuffer<T> {
    type Output = [T];
    #[inline]
    fn index(&self, index: &ops::RangeFrom<usize>) -> &[T] {
        self.as_slice().index(index)
    }
}
impl<T> ops::Index<ops::FullRange> for GapBuffer<T> {
    type Output = [T];
    #[inline]
    fn index(&self, _index: &ops::FullRange) -> &[T] {
        self.as_slice()
    }
}

impl<T> ops::IndexMut<ops::Range<usize>> for GapBuffer<T> {
    type Output = [T];
    #[inline]
    fn index_mut(&mut self, index: &ops::Range<usize>) -> &mut [T] {
        self.as_mut_slice().index_mut(index)
    }
}
impl<T> ops::IndexMut<ops::RangeTo<usize>> for GapBuffer<T> {
    type Output = [T];
    #[inline]
    fn index_mut(&mut self, index: &ops::RangeTo<usize>) -> &mut [T] {
        self.as_mut_slice().index_mut(index)
    }
}
impl<T> ops::IndexMut<ops::RangeFrom<usize>> for GapBuffer<T> {
    type Output = [T];
    #[inline]
    fn index_mut(&mut self, index: &ops::RangeFrom<usize>) -> &mut [T] {
        self.as_mut_slice().index_mut(index)
    }
}
impl<T> ops::IndexMut<ops::FullRange> for GapBuffer<T> {
    type Output = [T];
    #[inline]
    fn index_mut(&mut self, _index: &ops::FullRange) -> &mut [T] {
        self.as_mut_slice()
    }
}


//### Iterator implementation. #####################################################################
pub struct Items<'a, T:'a> {
    buff: &'a [T],
    tail: usize,
    head: usize,
    ghead: usize,
    gtail: usize,
}

impl<'a, T: Clone> Iterator for Items<'a, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.tail + self.gtail - self.ghead == self.buff.len() { return None };
        let tail = get_idx(self.tail, self.gtail - self.ghead, self.ghead);
        self.tail += 1;
        unsafe { Some((*self.buff.get_unchecked(tail)).clone()) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.buff.len() - self.gtail + self.ghead;
        (len, Some(len))
    }
}

impl<'a, T: Clone> DoubleEndedIterator for Items<'a, T> {
    fn next_back(&mut self) -> Option<T> {
        let head = get_idx(self.head , self.gtail - self.ghead, self.ghead);
        self.head -= 1;
        if head - 1 != self.head { None }
        else { unsafe { Some((*self.buff.get_unchecked(head)).clone()) } }
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

        let mut test1: GapBuffer<usize> = GapBuffer::new();
        let mut test2: GapBuffer<usize> = GapBuffer::new();

        for x in range(0, 10) {
            test1.insert(x,x);
        }

        for x in range(0,10) {
            assert!(test1.remove(0) == Some(x), "Remove failed at {} (forward)", x);
        }

        test2.extend(range(0, 5));
        test2.remove(0);
        for (i, x) in test2.iter().enumerate() {
            assert!(x == i + 1, "Remove test2 failed. Index {} is {}", x, i);
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

    #[test]
    fn test_slice_after_remove() {
        let mut buffer: GapBuffer<usize> = GapBuffer::new();
        buffer.extend(range(0,5));
        buffer.remove(0);

        assert!(buffer[] == [1, 2, 3, 4],  "Slice after removed failed.");
        assert!(buffer[0] == 1, "buffer[0] = {}", buffer[0]);
    }

}
