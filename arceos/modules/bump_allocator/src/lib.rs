#![no_std]
#![allow(unused)]

use core::num;
use core::ptr::NonNull;
use core::alloc::Layout;
use allocator::AllocError;

use allocator::{BaseAllocator, ByteAllocator, PageAllocator, AllocResult, BuddyByteAllocator};
use bitmap_allocator::BitAlloc;


type BitAllocUsed = bitmap_allocator::BitAlloc1M;

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    base: usize,
    right_base: usize,

    total_pages: usize,
    used_pages: usize,
    page_inner: BitAllocUsed,

    split_page: usize,
    byte_inner: BuddyByteAllocator,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            base: 0,
            right_base: 0,

            total_pages: 0,
            used_pages: 0,
            page_inner: BitAllocUsed::DEFAULT,

            split_page: 0,
            byte_inner: BuddyByteAllocator::new(),
        }
    }
}

#[inline]
const fn align_down(pos: usize, align: usize) -> usize {
    pos & !(align - 1)
}

#[inline]
const fn align_up(pos: usize, align: usize) -> usize {
    (pos + align - 1) & !(align - 1)
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        assert!(PAGE_SIZE.is_power_of_two());

        let end = align_down(start + size, PAGE_SIZE);
        let start = align_up(start, PAGE_SIZE);
        
        self.base = start;
        self.right_base = end;
        self.total_pages = (end - start) / PAGE_SIZE;

        self.page_inner.insert(0..self.total_pages);
    
        self.split_page = self.total_pages - 1;
        self.byte_inner.init(self.base, PAGE_SIZE);
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        // not allowed, memory should be only init once.
        Err(AllocError::NoMemory)
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        loop {
            if let Ok(ptr) = self.byte_inner.alloc(layout) {
                return Ok(ptr);
            } else {
                let old_size = self.byte_inner.total_bytes();

                let expand_size = old_size
                    .max(layout.size())
                    .next_power_of_two()
                    .max(PAGE_SIZE);
                let expand_pages = expand_size / PAGE_SIZE;
                
                if expand_pages > self.split_page {
                    return Err(AllocError::NoMemory);
                }

                if (self.split_page - expand_pages..self.split_page).any(|idx| !self.page_inner.test(idx)) {
                    return Err(AllocError::NoMemory);
                }

                self.byte_inner.add_memory(self.base + (self.total_pages - self.split_page) * PAGE_SIZE, expand_size);
                self.split_page -= expand_pages;
            }
        }
    }

    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        self.byte_inner.dealloc(pos, layout)
    }

    fn total_bytes(&self) -> usize {
        self.byte_inner.total_bytes()
    }

    fn used_bytes(&self) -> usize {
        self.byte_inner.used_bytes()
    }

    fn available_bytes(&self) -> usize {
        self.byte_inner.available_bytes()
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        if align_pow2 % PAGE_SIZE != 0 {
            return Err(AllocError::InvalidParam);
        }
        let align_pow2 = align_pow2 / PAGE_SIZE;
        if !align_pow2.is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }
        let align_log2 = align_pow2.trailing_zeros() as usize;
        match num_pages.cmp(&1) {
            core::cmp::Ordering::Equal => {
                if let Some(idx) = self.page_inner.alloc() {
                    if idx >= self.split_page {
                        self.page_inner.dealloc(idx);
                        return Err(AllocError::NoMemory);
                    }
                    return Ok(self.right_base - (idx + 1) * PAGE_SIZE);
                }
                return Err(AllocError::NoMemory);
                
            },
            core::cmp::Ordering::Greater => {
                if let Some(idx) = self.page_inner.alloc_contiguous(num_pages, align_log2) {
                    if idx + num_pages - 1 >= self.split_page {
                        self.page_inner.dealloc(idx);
                        return Err(AllocError::NoMemory);
                    }
                    return Ok(self.right_base - (idx + 1) * PAGE_SIZE);
                }
                return Err(AllocError::NoMemory);
            },
            _ => Err(AllocError::InvalidParam),
        }
        .inspect(|_| self.used_pages += num_pages)
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        self.used_pages -= num_pages;
        self.page_inner.dealloc((self.right_base - pos) / PAGE_SIZE - 1)
    }

    fn available_pages(&self) -> usize {
        self.total_pages - self.used_pages
    }

    fn total_pages(&self) -> usize {
        self.total_pages
    }

    fn used_pages(&self) -> usize {
        self.used_pages
    }
}
