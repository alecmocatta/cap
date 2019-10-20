//! A template Rust library crate.
//!
//! **[Crates.io](https://crates.io/crates/cap) │ [Repo](https://github.com/alecmocatta/cap)**
//!
//! This is template for Rust libraries, comprising a [`hello_world()`] function.
//!
//! # Example
//!
//! ```
//! //use template_rust::hello_world;
//!
//! //hello_world();
//! // prints: Hello, world!
//! ```
//!
//! # Note
//!
//! Caveat emptor.

#![doc(html_root_url = "https://docs.rs/cap/0.1.0")]
#![feature(allocator_api)]
#![feature(core_intrinsics)]
#![warn(
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	trivial_casts,
	trivial_numeric_casts,
	unused_import_braces,
	unused_qualifications,
	unused_results,
	clippy::pedantic
)] // from https://github.com/rust-unofficial/patterns/blob/master/anti_patterns/deny-warnings.md
#![allow()]

use std::{alloc, intrinsics, ptr, sync::atomic};

#[global_allocator]
static A: Alloc<alloc::System> = Alloc::new(alloc::System);

pub struct Alloc<H>(H, atomic::AtomicIsize);

impl<H> Alloc<H> {
	pub const fn new(h: H) -> Alloc<H> {
		Alloc(h, atomic::AtomicIsize::new(isize::min_value()))
	}
	fn init(&self) {
		let mut x = self.1.load(atomic::Ordering::Relaxed);
		if likely(x != isize::min_value() && x != isize::min_value() + 1) {
			return;
		}
		if x == isize::min_value() {
			x = self.1.compare_and_swap(
				isize::min_value(),
				isize::min_value() + 1,
				atomic::Ordering::Relaxed,
			);
			if x == isize::min_value() {
				let zero = self.1.swap(20_000_000, atomic::Ordering::Relaxed);
				if zero != isize::min_value() + 1 {
					unsafe { intrinsics::abort() }
				}; //assert_eq!(zero, 0);
				return;
			}
		}
		if x == isize::min_value() + 1 {
			while self.1.load(atomic::Ordering::Relaxed) == isize::min_value() + 1 {
				atomic::spin_loop_hint();
			}
		}
	}
}

unsafe impl<H> alloc::GlobalAlloc for Alloc<H>
where
	H: alloc::GlobalAlloc,
{
	unsafe fn alloc(&self, l: alloc::Layout) -> *mut u8 {
		self.init();
		let size = l.size() as isize;
		let res = if self.1.fetch_sub(size, atomic::Ordering::Acquire) >= size {
			self.0.alloc(l)
		} else {
			ptr::null_mut()
		};
		if res.is_null() {
			self.1.fetch_add(size, atomic::Ordering::Release);
		}
		res
	}
	unsafe fn dealloc(&self, ptr: *mut u8, layout: alloc::Layout) {
		let size = layout.size() as isize;
		self.0.dealloc(ptr, layout);
		self.1.fetch_add(size, atomic::Ordering::Release);
	}
	unsafe fn alloc_zeroed(&self, l: alloc::Layout) -> *mut u8 {
		self.init();
		let size = l.size() as isize;
		let res = if self.1.fetch_sub(size, atomic::Ordering::Acquire) >= size {
			self.0.alloc_zeroed(l)
		} else {
			ptr::null_mut()
		};
		if res.is_null() {
			self.1.fetch_add(size, atomic::Ordering::Release);
		}
		res
	}
	unsafe fn realloc(&self, ptr: *mut u8, old_l: alloc::Layout, new_s: usize) -> *mut u8 {
		let new_l = alloc::Layout::from_size_align_unchecked(new_s, old_l.align());
		let (old_size, new_size) = (old_l.size() as isize, new_l.size() as isize);
		if new_size > old_size {
			let res = if self
				.1
				.fetch_sub(new_size - old_size, atomic::Ordering::Acquire)
				>= new_size - old_size
			{
				self.0.realloc(ptr, old_l, new_s)
			} else {
				ptr::null_mut()
			};
			if !res.is_null() {
				self.1
					.fetch_add(new_size - old_size, atomic::Ordering::Release);
			}
			res
		} else {
			let res = self.0.realloc(ptr, old_l, new_s);
			if !res.is_null() {
				self.1
					.fetch_add(old_size - new_size, atomic::Ordering::Release);
			}
			res
		}
	}
}

unsafe impl<H> alloc::Alloc for Alloc<H>
where
	H: alloc::Alloc,
{
	unsafe fn alloc(&mut self, l: alloc::Layout) -> Result<ptr::NonNull<u8>, alloc::AllocErr> {
		self.init();
		let size = self.0.usable_size(&l).1 as isize;
		let res = if self.1.fetch_sub(size, atomic::Ordering::Acquire) >= size {
			self.0.alloc(l)
		} else {
			Err(alloc::AllocErr)
		};
		if res.is_err() {
			self.1.fetch_add(size, atomic::Ordering::Release);
		}
		res
	}
	unsafe fn dealloc(&mut self, item: ptr::NonNull<u8>, l: alloc::Layout) {
		let size = self.0.usable_size(&l).1 as isize;
		self.0.dealloc(item, l);
		self.1.fetch_add(size, atomic::Ordering::Release);
	}
	fn usable_size(&self, layout: &alloc::Layout) -> (usize, usize) {
		self.0.usable_size(layout)
	}
	unsafe fn realloc(
		&mut self, ptr: ptr::NonNull<u8>, old_l: alloc::Layout, new_s: usize,
	) -> Result<ptr::NonNull<u8>, alloc::AllocErr> {
		let new_l = alloc::Layout::from_size_align_unchecked(new_s, old_l.align());
		let (old_size, new_size) = (
			self.0.usable_size(&old_l).1 as isize,
			self.0.usable_size(&new_l).1 as isize,
		);
		if new_size > old_size {
			let res = if self
				.1
				.fetch_sub(new_size - old_size, atomic::Ordering::Acquire)
				>= new_size - old_size
			{
				self.0.realloc(ptr, old_l, new_s)
			} else {
				Err(alloc::AllocErr)
			};
			if res.is_err() {
				self.1
					.fetch_add(new_size - old_size, atomic::Ordering::Release);
			}
			res
		} else {
			let res = self.0.realloc(ptr, old_l, new_s);
			if res.is_ok() {
				self.1
					.fetch_add(old_size - new_size, atomic::Ordering::Release);
			}
			res
		}
	}
	unsafe fn alloc_zeroed(
		&mut self, l: alloc::Layout,
	) -> Result<ptr::NonNull<u8>, alloc::AllocErr> {
		self.init();
		let size = self.0.usable_size(&l).1 as isize;
		let res = if self.1.fetch_sub(size, atomic::Ordering::Acquire) >= size {
			self.0.alloc_zeroed(l)
		} else {
			Err(alloc::AllocErr)
		};
		if res.is_err() {
			self.1.fetch_add(size, atomic::Ordering::Release);
		}
		res
	}
	unsafe fn grow_in_place(
		&mut self, ptr: ptr::NonNull<u8>, old_l: alloc::Layout, new_s: usize,
	) -> Result<(), alloc::CannotReallocInPlace> {
		let new_l = alloc::Layout::from_size_align(new_s, old_l.align()).unwrap();
		let (old_size, new_size) = (
			self.0.usable_size(&old_l).1 as isize,
			self.0.usable_size(&new_l).1 as isize,
		);
		if new_size < old_size {
			intrinsics::abort()
		};
		let res = if self
			.1
			.fetch_sub(new_size - old_size, atomic::Ordering::Acquire)
			>= new_size - old_size
		{
			self.0.grow_in_place(ptr, old_l, new_s)
		} else {
			Err(alloc::CannotReallocInPlace)
		};
		if res.is_err() {
			self.1
				.fetch_add(new_size - old_size, atomic::Ordering::Release);
		}
		res
	}
	unsafe fn shrink_in_place(
		&mut self, ptr: ptr::NonNull<u8>, old_l: alloc::Layout, new_s: usize,
	) -> Result<(), alloc::CannotReallocInPlace> {
		let new_l = alloc::Layout::from_size_align(new_s, old_l.align()).unwrap();
		let (old_size, new_size) = (
			self.0.usable_size(&old_l).1 as isize,
			self.0.usable_size(&new_l).1 as isize,
		);
		if new_size > old_size {
			intrinsics::abort()
		};
		let res = self.0.shrink_in_place(ptr, old_l, new_s);
		if res.is_ok() {
			self.1
				.fetch_add(old_size - new_size, atomic::Ordering::Release);
		}
		res
	}
}

fn likely(b: bool) -> bool {
	// unsafe { intrinsics::likely(b) }
	b
}

#[cfg(test)]
mod tests {
	use super::A;
	use std::sync::atomic;

	#[test]
	fn succeeds() {
		// let env = std::env::args().next();
		println!(
			"Hello, world! {:?}",
			20_000_000 - A.1.load(atomic::Ordering::Relaxed)
		);
		let x = vec![0u8; 19_000_000];
		println!(
			"Hello, world! {:?}",
			20_000_000 - A.1.load(atomic::Ordering::Relaxed)
		);
		println!("Hello, world! {:?}", A.1);
	}
}
