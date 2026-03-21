use std::alloc::{self, Layout};
use std::mem;
use std::ptr::{self, NonNull};

/// A fixed-size, circular data structure. Values can be inserted indefinitely, overwriting the
/// earliest entries when initial capacity is exhausted.
/// ```
/// # extern crate extra_collections;
/// # use extra_collections::RingBuf;
/// # fn main() {
/// let mut buf = RingBuf::new(8);
/// for i in 0..8 {
///     buf.push(i);
/// }
///
/// let expected = vec![0, 1, 2, 3, 4, 5, 6, 7];
/// let actual: Vec<u8> = buf.iter().copied().collect();
/// assert_eq!(actual, expected);
///
/// buf.push(8);
/// let expected = vec![1, 2, 3, 4, 5, 6, 7, 8];
/// let actual: Vec<u8> = buf.iter().copied().collect();
/// assert_eq!(actual, expected);
/// # }
/// ```
pub struct RingBuf<T> {
    /// The number of elements the buffer can hold
    cap: usize,
    /// The current number of elements stored in the buffer
    len: usize,
    /// The current start of the buffer
    head: usize,
    /// The current end of the buffer
    tail: usize,
    /// The data on the heap
    ptr: NonNull<T>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for RingBuf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: Clone> Clone for RingBuf<T> {
    fn clone(&self) -> Self {
        let layout = Layout::array::<T>(self.cap).expect("Allocation too large");
        let ptr = unsafe { alloc::alloc(layout) };
        let ptr = match NonNull::new(ptr as *mut T) {
            Some(p) => p,
            None => alloc::handle_alloc_error(layout),
        };
        let mut new = Self {
            cap: self.cap,
            len: self.len,
            head: self.head,
            tail: self.tail,
            ptr,
        };

        for value in self.iter() {
            new.push(value.clone());
        }

        new
    }
}

impl<T> Drop for RingBuf<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
        let layout = Layout::array::<T>(self.cap).unwrap();
        unsafe {
            alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

impl<T> RingBuf<T> {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        assert!(mem::size_of::<T>() != 0, "Not ready to handle ZSTs");
        let layout = Layout::array::<T>(capacity).expect("Allocation too large");
        let ptr = unsafe { alloc::alloc(layout) };
        let ptr = match NonNull::new(ptr as *mut T) {
            Some(p) => p,
            None => alloc::handle_alloc_error(layout),
        };
        Self {
            cap: capacity,
            len: 0,
            head: 0,
            tail: 0,
            ptr,
        }
    }

    /// Converts the index into the internal representation to retrieve the correct data
    #[inline]
    fn index_conv(&self, index: usize) -> Option<usize> {
        if index > self.len() {
            return None;
        }
        let out = (self.head + index) % self.cap;

        Some(out)
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        unsafe {
            ptr::write(self.ptr.as_ptr().add(self.tail), value);
        }

        self.inc_tail();

        if self.len < self.cap {
            self.len += 1;
        } else {
            self.inc_head();
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.dec_tail();
            self.len -= 1;
            unsafe { Some(ptr::read(self.ptr.as_ptr().add(self.tail))) }
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        let index = self.index_conv(index)?;
        unsafe { Some(&*self.ptr.as_ptr().add(index)) }
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let index = self.index_conv(index)?;
        unsafe { Some(&mut *self.ptr.as_ptr().add(index)) }
    }

    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.get(0)
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.get(self.tail - 1)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn inc_head(&mut self) {
        self.head = (self.head + 1) % self.cap;
    }

    #[inline]
    fn inc_tail(&mut self) {
        self.tail = (self.tail + 1) % self.cap;
    }

    #[inline]
    fn dec_tail(&mut self) {
        if self.tail == 0 {
            self.tail = self.cap - 1;
        } else {
            self.tail -= 1;
        }
    }

    #[inline]
    pub fn iter<'a>(&'a self) -> RingBufFiniteIter<'a, T> {
        RingBufFiniteIter { buf: self, cur: 0 }
    }

    #[inline]
    pub fn iter_mut<'a>(&'a mut self) -> RingBufFiniteIterMut<'a, T> {
        RingBufFiniteIterMut { buf: self, cur: 0 }
    }

    /// Order of elements and starting point are preserved at the cost of an allocation
    #[inline]
    pub fn into_new_slice(self) -> Box<[T]> {
        let vec: Vec<T> = self.into_iter().collect();

        vec.into_boxed_slice()
    }

    /// Order of elements is preserved, but not the starting point.
    /// Does not allocate
    #[inline]
    pub fn into_slice(self) -> Box<[T]> {
        let slice = unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) };
        unsafe { Box::from_raw(slice) }
    }

    /// Order of elements is preserved, but not the starting point
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> std::ops::Index<usize> for RingBuf<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let index = self.index_conv(index).unwrap_or_else(|| {
            panic!(
                "index out of bound: index was {} but length was {}",
                index,
                self.len()
            )
        });
        unsafe { &*self.ptr.as_ptr().add(index) }
    }
}

impl<T> std::ops::IndexMut<usize> for RingBuf<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let index = self.index_conv(index).unwrap_or_else(|| {
            panic!(
                "index out of bound: index was {} but length was {}",
                index,
                self.len()
            )
        });
        unsafe { &mut *self.ptr.as_ptr().add(index) }
    }
}

impl<T> std::iter::IntoIterator for RingBuf<T> {
    type Item = T;
    type IntoIter = RingBufFiniteIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        RingBufFiniteIntoIter { buf: self, cur: 0 }
    }
}

pub struct RingBufFiniteIterMut<'a, T> {
    buf: &'a mut RingBuf<T>,
    cur: usize,
}

impl<'a, T> Iterator for RingBufFiniteIterMut<'a, T> {
    type Item = &'a mut T;

    #[inline]
    fn next<'s>(&'s mut self) -> Option<Self::Item> {
        if self.cur < self.buf.len() {
            // SAFETY:
            //     The mutable reference may live as long as the structure itself
            //     So long as only one mutable reference may ever be obtained from the iteration
            //     process
            //     As long as self.cur never goes backwards, it is impossible for the same element
            //     to be returned multiple times
            let item: &'a mut T = unsafe { std::mem::transmute(self.buf.get_mut(self.cur)?) };
            self.cur += 1;
            return Some(item);
        } else {
            None
        }
    }
}

pub struct RingBufFiniteIter<'a, T> {
    buf: &'a RingBuf<T>,
    cur: usize,
}

impl<'a, T> Iterator for RingBufFiniteIter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.buf.len() {
            let item = self.buf.get(self.cur)?;
            self.cur += 1;
            return Some(item);
        } else {
            None
        }
    }
}

pub struct RingBufFiniteIntoIter<T> {
    buf: RingBuf<T>,
    cur: usize,
}

impl<T> Iterator for RingBufFiniteIntoIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.buf.len() {
            let index = self.buf.index_conv(self.cur)?;
            // SAFETY:
            //     self.cur must never decrease, ensuring the the now-uninitialized memory is never
            //     read
            let item: T = unsafe { ptr::read(self.buf.ptr.as_ptr().add(index)) };
            self.cur += 1;

            Some(item)
        } else {
            None
        }
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn push() {
        let mut buf: RingBuf<u8> = RingBuf::new(8);
        buf.push(1);
        assert!(buf.first().is_some());
    }

    #[test]
    fn pop() {
        let mut buf: RingBuf<u8> = RingBuf::new(8);
        buf.push(1);
        let out = buf.pop();
        assert!(out.is_some());
    }

    #[test]
    #[should_panic]
    fn pop_empty() {
        let mut buf: RingBuf<u8> = RingBuf::new(8);
        buf.pop().unwrap();
    }

    #[test]
    fn push_at_capacity() {
        let mut buf: RingBuf<u8> = RingBuf::new(8);
        for _ in 0..8 {
            buf.push(1);
        }
        assert!(buf.len() == 8);
        assert!(buf.tail == 0);

        buf.push(1);
        assert!(buf.len() == 8);
        assert!(buf.tail == 1);
        assert!(buf.head == 1);
    }

    #[test]
    fn pop_at_capacity() {
        let mut buf: RingBuf<u8> = RingBuf::new(8);
        for _ in 0..9 {
            buf.push(1);
        }

        buf.pop().unwrap();
        assert!(buf.len() == 7);
        assert!(buf.tail == 0);
        assert!(buf.head == 1);
    }

    #[test]
    fn iter() {
        let mut buf: RingBuf<u8> = RingBuf::new(8);
        for i in 0..8 {
            buf.push(i);
        }

        let expected = vec![0, 1, 2, 3, 4, 5, 6, 7];
        assert_eq!(expected.len(), 8);
        let actual: Vec<u8> = buf.iter().copied().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn iter_at_capacity() {
        let mut buf: RingBuf<u8> = RingBuf::new(8);
        for i in 0..9 {
            buf.push(i);
        }

        let expected = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(expected.len(), 8);
        let actual: Vec<u8> = buf.iter().copied().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn clone() {
        let mut buf: RingBuf<String> = RingBuf::new(5);
        for i in 0..8 {
            buf.push(format!("{}", i));
        }

        let expected: Vec<String> = buf.iter().cloned().collect();
        let actual: Vec<String> = buf.clone().iter().cloned().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn into_iter() {
        let mut buf: RingBuf<u8> = RingBuf::new(8);
        for i in 0..8 {
            buf.push(i);
        }

        let expected: Vec<u8> = buf.iter().copied().collect();
        let iter: RingBufFiniteIntoIter<u8> = buf.into_iter();
        let actual: Vec<u8> = iter.collect();
        assert_eq!(actual, expected);
    }
}
