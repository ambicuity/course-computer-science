//! bump.rs — drop-in single-file Bump allocator for Rust.
//!
//! Usage:
//!   let bump = Bump::with_capacity(64 * 1024);
//!   let n: &mut Node = bump.alloc(Node::new());
//!   ...
//!   // bump drops at scope end; everything inside it goes with it
//!
//! Production code should prefer `bumpalo` from crates.io; this module is the
//! 50-line educational version.

use std::cell::Cell;
use std::ptr;

pub struct Bump {
    buf: Box<[u8]>,
    offset: Cell<usize>,
}

impl Bump {
    pub fn with_capacity(n: usize) -> Self {
        Bump {
            buf: vec![0u8; n].into_boxed_slice(),
            offset: Cell::new(0),
        }
    }

    pub fn alloc<T>(&self, value: T) -> &mut T {
        let layout = std::alloc::Layout::new::<T>();
        let off = self.offset.get();
        let aligned = (off + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        if end > self.buf.len() {
            panic!("Bump out of space");
        }
        self.offset.set(end);
        unsafe {
            let ptr = self.buf.as_ptr().add(aligned) as *mut T;
            ptr::write(ptr, value);
            &mut *ptr
        }
    }

    pub fn alloc_slice_copy<T: Copy>(&self, items: &[T]) -> &mut [T] {
        let layout = std::alloc::Layout::array::<T>(items.len()).unwrap();
        let off = self.offset.get();
        let aligned = (off + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        if end > self.buf.len() {
            panic!("Bump out of space");
        }
        self.offset.set(end);
        unsafe {
            let ptr = self.buf.as_ptr().add(aligned) as *mut T;
            std::ptr::copy_nonoverlapping(items.as_ptr(), ptr, items.len());
            std::slice::from_raw_parts_mut(ptr, items.len())
        }
    }

    pub fn capacity(&self) -> usize { self.buf.len() }
    pub fn bytes_used(&self) -> usize { self.offset.get() }
    pub fn reset(&mut self) { self.offset.set(0); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_basic() {
        let bump = Bump::with_capacity(1024);
        let a: &mut i32 = bump.alloc(42);
        assert_eq!(*a, 42);
        *a = 100;
        assert_eq!(*a, 100);
    }

    #[test]
    fn alloc_slice() {
        let bump = Bump::with_capacity(1024);
        let s: &mut [i32] = bump.alloc_slice_copy(&[1, 2, 3, 4, 5]);
        s[0] = 99;
        assert_eq!(s, &[99, 2, 3, 4, 5]);
    }
}
