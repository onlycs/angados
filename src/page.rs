use core::mem;

use bitfield_struct::bitfield;
use bitflags::bitflags;

unsafe extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

static mut ALLOC_START: usize = 0;
const PAGE_ORDER: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_ORDER;

/// Align, i.e. set to a multiple of a power of 2
const fn align_up(value: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    (value + o) & !o
}

bitflags! {
    #[derive(Debug)]
    struct PageFlags: u8 {
        const TAKEN = 0b01;
        const LAST = 0b10;
    }
}

impl PageFlags {
    const fn into_bits(self) -> u8 {
        self.bits()
    }
}

#[bitfield(u8)]
struct Page {
    #[bits(2, from = PageFlags::from_bits_truncate)]
    flags: PageFlags,
    #[bits(6)]
    _pad: u8,
}

impl Page {
    /// Checks if last (0b10) flag is set
    fn is_last(&self) -> bool {
        self.flags().contains(PageFlags::LAST)
    }

    /// Checks if taken (0b01) flag is set
    fn is_taken(&self) -> bool {
        self.flags().contains(PageFlags::TAKEN)
    }

    /// Clears all flags (sets to 0b00)
    fn clear(&mut self) {
        self.set_flags(PageFlags::empty());
    }

    /// Sets flags to a specific value
    fn reset(&mut self, flags: PageFlags) {
        self.set_flags(flags);
    }
}

pub fn init() {
    unsafe {
        let num_pages = HEAP_SIZE / PAGE_SIZE;
        let ptr = HEAP_START as *mut Page;

        // clear all pages
        for i in 0..num_pages {
            (*ptr.add(i)).clear();
        }

        // determine where usable memory starts
        ALLOC_START = align_up(HEAP_START + num_pages * mem::size_of::<Page>(), PAGE_ORDER);
    }
}

pub fn alloc(pages: usize) -> *mut u8 {
    assert!(pages > 0, "Must allocate at least one page of memory");

    unsafe {
        let num_pages = HEAP_SIZE / PAGE_SIZE;
        let ptr = HEAP_START as *mut Page;

        let mut found_free = 0;

        // search for a free chunk that has enough pages
        for page in 0..num_pages - pages {
            if !(*ptr.add(page)).is_taken() {
                found_free += 1;
            } else {
                found_free = 0;
            }

            if found_free < pages {
                continue;
            }

            // page is currently the last page
            let start_page = page + 1 - pages;

            // once found, make sure each page is marked non-free
            for allocate in start_page..page {
                (*ptr.add(allocate)).reset(PageFlags::TAKEN);
            }

            // mark the last page as taken and last
            (*ptr.add(page)).reset(PageFlags::TAKEN | PageFlags::LAST);

            // convert to a pointer. ALLOC_START is the start of the usable memory
            // (before that is for pages)
            return (ALLOC_START + PAGE_SIZE * start_page) as *mut u8;
        }
    }

    panic!("Out of memory");
}

pub fn free(ptr: *mut u8) {
    assert!(!ptr.is_null(), "Tried to deallocate a null pointer");

    unsafe {
        // convert from pointer to page index
        let addr = HEAP_START + (ptr as usize - ALLOC_START) / PAGE_SIZE;

        // catch in case we try to write outside of page area
        assert!(
            addr >= HEAP_START && addr < ALLOC_START,
            "Tried to deallocate an address which was out of range"
        );

        // keep clearing until we hit last bit
        let mut p = addr as *mut Page;
        while (*p).is_taken() && !(*p).is_last() {
            (*p).clear();
            p = p.add(1);
        }

        // do a check
        assert!(
            (*p).is_last(),
            "Possible double-free or double-allocation found: not taken bit found before last bit"
        );

        // clear last bit
        (*p).clear();
    }
}

pub fn zalloc(pages: usize) -> *mut u8 {
    let ptr = alloc(pages);

    if ptr.is_null() {
        return ptr;
    }

    let size = (pages * PAGE_SIZE) / mem::size_of::<u64>();
    let big = ptr as *mut u64;

    for i in 0..size {
        unsafe {
            *big.add(i) = 0;
        }
    }

    ptr
}
