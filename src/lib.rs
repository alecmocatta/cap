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
#![cfg_attr(feature = "nightly", feature(allocator_api))]
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

#[cfg(feature = "nightly")]
use std::alloc::{Alloc, AllocErr, CannotReallocInPlace};
use std::{
	alloc::{GlobalAlloc, Layout}, ptr, sync::atomic::{AtomicUsize, Ordering}
};

/// A struct that wraps another allocator and limits the number of bytes that can be allocated.
#[derive(Debug)]
pub struct Cap<H> {
	allocator: H,
	remaining: AtomicUsize,
	limit: AtomicUsize,
}

impl<H> Cap<H> {
	/// fn
	pub const fn new(allocator: H, limit: usize) -> Self {
		Self {
			allocator,
			remaining: AtomicUsize::new(limit),
			limit: AtomicUsize::new(limit),
		}
	}
	/// remaining
	pub fn remaining(&self) -> usize {
		self.remaining.load(Ordering::Relaxed)
	}
	/// limit
	pub fn limit(&self) -> usize {
		self.limit.load(Ordering::Relaxed)
	}
	/// ab
	pub fn set_limit(&self, limit: usize) -> Result<(), ()> {
		loop {
			let limit_old = self.limit.load(Ordering::Relaxed);
			if limit < limit_old {
				if self
					.remaining
					.fetch_sub(limit_old - limit, Ordering::Relaxed)
					< limit_old - limit
				{
					let _ = self
						.remaining
						.fetch_add(limit_old - limit, Ordering::Relaxed);
					break Err(());
				}
				if self
					.limit
					.compare_and_swap(limit_old, limit, Ordering::Relaxed)
					!= limit_old
				{
					continue;
				}
			} else {
				if self
					.limit
					.compare_and_swap(limit_old, limit, Ordering::Relaxed)
					!= limit_old
				{
					continue;
				}
				let _ = self
					.remaining
					.fetch_add(limit - limit_old, Ordering::Relaxed);
			}
			break Ok(());
		}
	}
}

unsafe impl<H> GlobalAlloc for Cap<H>
where
	H: GlobalAlloc,
{
	unsafe fn alloc(&self, l: Layout) -> *mut u8 {
		let size = l.size();
		let res = if self.remaining.fetch_sub(size, Ordering::Acquire) >= size {
			self.allocator.alloc(l)
		} else {
			ptr::null_mut()
		};
		if res.is_null() {
			let _ = self.remaining.fetch_add(size, Ordering::Release);
		}
		res
	}
	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		let size = layout.size();
		self.allocator.dealloc(ptr, layout);
		let _ = self.remaining.fetch_add(size, Ordering::Release);
	}
	unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
		let size = l.size();
		let res = if self.remaining.fetch_sub(size, Ordering::Acquire) >= size {
			self.allocator.alloc_zeroed(l)
		} else {
			ptr::null_mut()
		};
		if res.is_null() {
			let _ = self.remaining.fetch_add(size, Ordering::Release);
		}
		res
	}
	unsafe fn realloc(&self, ptr: *mut u8, old_l: Layout, new_s: usize) -> *mut u8 {
		let new_l = Layout::from_size_align_unchecked(new_s, old_l.align());
		let (old_size, new_size) = (old_l.size(), new_l.size());
		if new_size > old_size {
			let res = if self
				.remaining
				.fetch_sub(new_size - old_size, Ordering::Acquire)
				>= new_size - old_size
			{
				self.allocator.realloc(ptr, old_l, new_s)
			} else {
				ptr::null_mut()
			};
			if res.is_null() {
				let _ = self
					.remaining
					.fetch_add(new_size - old_size, Ordering::Release);
			}
			res
		} else {
			let res = self.allocator.realloc(ptr, old_l, new_s);
			if !res.is_null() {
				let _ = self
					.remaining
					.fetch_add(old_size - new_size, Ordering::Release);
			}
			res
		}
	}
}

#[cfg(feature = "nightly")]
unsafe impl<H> Alloc for Cap<H>
where
	H: Alloc,
{
	unsafe fn alloc(&mut self, l: Layout) -> Result<ptr::NonNull<u8>, AllocErr> {
		let size = self.allocator.usable_size(&l).1;
		let res = if self.remaining.fetch_sub(size, Ordering::Acquire) >= size {
			self.allocator.alloc(l)
		} else {
			Err(AllocErr)
		};
		if res.is_err() {
			let _ = self.remaining.fetch_add(size, Ordering::Release);
		}
		res
	}
	unsafe fn dealloc(&mut self, item: ptr::NonNull<u8>, l: Layout) {
		let size = self.allocator.usable_size(&l).1;
		self.allocator.dealloc(item, l);
		let _ = self.remaining.fetch_add(size, Ordering::Release);
	}
	fn usable_size(&self, layout: &Layout) -> (usize, usize) {
		self.allocator.usable_size(layout)
	}
	unsafe fn realloc(
		&mut self, ptr: ptr::NonNull<u8>, old_l: Layout, new_s: usize,
	) -> Result<ptr::NonNull<u8>, AllocErr> {
		let new_l = Layout::from_size_align_unchecked(new_s, old_l.align());
		let (old_size, new_size) = (
			self.allocator.usable_size(&old_l).1,
			self.allocator.usable_size(&new_l).1,
		);
		if new_size > old_size {
			let res = if self
				.remaining
				.fetch_sub(new_size - old_size, Ordering::Acquire)
				>= new_size - old_size
			{
				self.allocator.realloc(ptr, old_l, new_s)
			} else {
				Err(AllocErr)
			};
			if res.is_err() {
				let _ = self
					.remaining
					.fetch_add(new_size - old_size, Ordering::Release);
			}
			res
		} else {
			let res = self.allocator.realloc(ptr, old_l, new_s);
			if res.is_ok() {
				let _ = self
					.remaining
					.fetch_add(old_size - new_size, Ordering::Release);
			}
			res
		}
	}
	unsafe fn alloc_zeroed(&mut self, l: Layout) -> Result<ptr::NonNull<u8>, AllocErr> {
		let size = self.allocator.usable_size(&l).1;
		let res = if self.remaining.fetch_sub(size, Ordering::Acquire) >= size {
			self.allocator.alloc_zeroed(l)
		} else {
			Err(AllocErr)
		};
		if res.is_err() {
			let _ = self.remaining.fetch_add(size, Ordering::Release);
		}
		res
	}
	unsafe fn grow_in_place(
		&mut self, ptr: ptr::NonNull<u8>, old_l: Layout, new_s: usize,
	) -> Result<(), CannotReallocInPlace> {
		let new_l = Layout::from_size_align(new_s, old_l.align()).unwrap();
		let (old_size, new_size) = (
			self.allocator.usable_size(&old_l).1,
			self.allocator.usable_size(&new_l).1,
		);
		let res = if self
			.remaining
			.fetch_sub(new_size - old_size, Ordering::Acquire)
			>= new_size - old_size
		{
			self.allocator.grow_in_place(ptr, old_l, new_s)
		} else {
			Err(CannotReallocInPlace)
		};
		if res.is_err() {
			let _ = self
				.remaining
				.fetch_add(new_size - old_size, Ordering::Release);
		}
		res
	}
	unsafe fn shrink_in_place(
		&mut self, ptr: ptr::NonNull<u8>, old_l: Layout, new_s: usize,
	) -> Result<(), CannotReallocInPlace> {
		let new_l = Layout::from_size_align(new_s, old_l.align()).unwrap();
		let (old_size, new_size) = (
			self.allocator.usable_size(&old_l).1,
			self.allocator.usable_size(&new_l).1,
		);
		let res = self.allocator.shrink_in_place(ptr, old_l, new_s);
		if res.is_ok() {
			let _ = self
				.remaining
				.fetch_add(old_size - new_size, Ordering::Release);
		}
		res
	}
}

#[cfg(test)]
mod tests {
	use std::{alloc, thread};

	use super::Cap;

	#[global_allocator]
	static A: Cap<alloc::System> = Cap::new(alloc::System, usize::max_value());

	#[test]
	fn succeeds() {
		std::thread::sleep(std::time::Duration::from_secs(1));
		let used = A.limit() - A.remaining();
		for _ in 0..100 {
			let threads = (0..100)
				.map(|_| {
					thread::spawn(|| {
						for i in 0..1000 {
							let _ = (0..i).collect::<Vec<u32>>();
							let _ = (0..i).flat_map(std::iter::once).collect::<Vec<u32>>();
						}
					})
				})
				.collect::<Vec<_>>();
			threads
				.into_iter()
				.for_each(|thread| thread.join().unwrap());
			let used2 = A.limit() - A.remaining();
			assert_eq!(used, used2);
		}
	}
}
