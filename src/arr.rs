use core::alloc::Layout;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use std::alloc;

/// An helper struct for dealing with regular, fixed-size arrays. Enables easier procedural
/// initialization.
///
/// May be allocated on the stack or the heap.
pub struct Arr<T, const C: usize> {
    len: usize,
    cap: usize,
    data_ptr: NonNull<MaybeUninit<T>>,
    init_ptr: NonNull<bool>,
}

impl<T, const C: usize> Arr<T, C> {
    #[inline]
    pub fn new() -> Self {
        Self::new_on_stack()
    }

    #[inline]
    pub fn new_on_stack() -> Self {
        let mut data: [MaybeUninit<T>; C] = std::array::from_fn(|_| MaybeUninit::uninit());
        let mut init: [bool; C] = std::array::from_fn(|_| false);
        let Some(data_ptr) = NonNull::new(data.as_mut_ptr()) else {
            // ptr is guaranteed to be non-null
            unreachable!();
        };
        let Some(init_ptr) = NonNull::new(init.as_mut_ptr()) else {
            // ptr is guaranteed to be non-null
            unreachable!();
        };
        Self {
            len: 0,
            cap: C,
            data_ptr,
            init_ptr,
        }
    }

    #[inline]
    pub fn new_on_heap() -> Self {
        let layout = Layout::array::<T>(C).expect("Allocation too large");
        let data_ptr = unsafe { alloc::alloc(layout) };
        let data_ptr = match NonNull::new(data_ptr as *mut MaybeUninit<T>) {
            Some(p) => p,
            None => alloc::handle_alloc_error(layout),
        };

        let layout = Layout::array::<bool>(C).expect("Allocation too large");
        let init_ptr = unsafe { alloc::alloc(layout) };
        let init_ptr = match NonNull::new(init_ptr as *mut bool) {
            Some(p) => p,
            None => alloc::handle_alloc_error(layout),
        };

        Self {
            len: 0,
            cap: C,
            data_ptr,
            init_ptr,
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        let is_init = unsafe { *self.init_ptr.as_ptr().add(index) };
        if is_init {
            let val = unsafe { &*self.data_ptr.as_ptr().add(index) };
            let val = unsafe { val.assume_init_ref() };
            Some(val)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let is_init = unsafe { *self.init_ptr.as_ptr().add(index) };
        if is_init {
            let val = unsafe { &mut *self.data_ptr.as_ptr().add(index) };
            let val = unsafe { val.assume_init_mut() };
            Some(val)
        } else {
            None
        }
    }

    #[inline]
    pub fn init(&mut self, index: usize, value: T) {
        unsafe {
            self.init_ptr.as_ptr().add(index).write(true);
            self.data_ptr.as_ptr().add(index).write(MaybeUninit::new(value));
        }
    }

    #[inline]
    pub fn into_boxed_slice(self) -> Box<[Option<T>]> {
        todo!()
    }

    #[inline]
    pub fn as_raw_slice(&self) -> &[MaybeUninit<T>] {
        todo!()
    }

    #[inline]
    pub fn as_raw_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        todo!()
    }

}

impl<T, const C: usize> Default for Arr<T, C> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const C: usize> std::ops::Index<usize> for Arr<T, C> {
    type Output = T;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap_or_else(|| {
            panic!(
                "attempted to access uninitialized memory at index {}",
                index
            )
        })
    }
}

impl<T, const C: usize> std::ops::IndexMut<usize> for Arr<T, C> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap_or_else(|| {
            eprintln!("attempted to access uninitialized memory at index {}", index);
            panic!(
                "try calling `init()` before attempting to use mutable indexing"
            )
        })
    }
}
