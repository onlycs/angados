use bitfield_struct::bitfield;
use bitflags::bitflags;

use crate::page::{self, zalloc};

bitflags! {
    #[derive(Debug)]
    pub struct PTEFlags: u8 {
        const VALID = 1 << 0;
        const READ = 1 << 1;
        const WRITE = 1 << 2;
        const EXECUTE = 1 << 3;
        const USER = 1 << 4;
        const GLOBAL = 1 << 5;
        const ACCESSED = 1 << 6;
        const DIRTY = 1 << 7;
    }
}

impl PTEFlags {
    const RWX: Self = Self::READ.union(Self::WRITE).union(Self::EXECUTE);
    const RWXUG: Self = Self::RWX.union(Self::USER).union(Self::GLOBAL);

    const fn into_bits(self) -> u8 {
        self.bits()
    }
}

#[bitfield(u64)]
struct PTE {
    #[bits(8, from = PTEFlags::from_bits_truncate)]
    flags: PTEFlags,
    #[bits(2)]
    _rsw: u8,
    #[bits(9)]
    ppn0: u64,
    #[bits(9)]
    ppn1: u64,
    #[bits(26)]
    ppn2: u64,
    #[bits(10)]
    _reserved: u16,
}

impl PTE {
    const PPN_MASK: u64 = ((1 << 44) - 1) << 10;

    unsafe fn ptr(&self) -> *mut u8 {
        let ppn = (self.ppn2() << 18) | (self.ppn1() << 9) | self.ppn0();
        (ppn << 12) as *mut u8
    }

    fn with_ptr(&mut self, ptr: *mut u8) {
        let page = ptr as u64;
        assert!(page & 0xfff == 0, "Page pointer must be page aligned");

        // the bottom 12 bits are zeroed because of the alignment (checked above)
        // ppn starts from bit 10 so we can shr by 12 and shl by 10, aka shr by 2.
        // note that, if the memory capabilities of the future rise by beyond 44 bits
        // (128 TiB max) then this will overflow into the top 10 reserved bits.
        // but for now, we can do this.
        self.0 &= !Self::PPN_MASK; // clear all ppn bits
        self.0 |= page >> 2;
    }

    fn with_ppn(&mut self, ppn: u64) {
        let ppn_offset = (ppn & (1 << 44) - 1) << 10;

        self.0 &= !Self::PPN_MASK; // clear all ppn bits
        self.0 |= ppn_offset;
    }

    fn is_leaf(&self) -> bool {
        self.flags().intersects(PTEFlags::RWX)
    }

    fn clear(&mut self) {
        self.0 = 0;
    }
}

#[bitfield(u64)]
pub struct VirtualAddress {
    #[bits(12)]
    offset: u16,
    #[bits(9)]
    vpn0: usize,
    #[bits(9)]
    vpn1: usize,
    #[bits(9)]
    vpn2: usize,
    #[bits(9)]
    vpn3: usize,
    #[bits(16)]
    _pad: u16,
}

impl VirtualAddress {
    pub fn vpn(&self) -> [usize; 4] {
        [self.vpn0(), self.vpn1(), self.vpn2(), self.vpn3()]
    }
}

#[bitfield(u64)]
pub struct PhysicalAddress {
    #[bits(12)]
    offset: u16,
    #[bits(9)]
    ppn0: u64,
    #[bits(9)]
    ppn1: u64,
    #[bits(26)]
    ppn2: u64,
    #[bits(8)]
    _pad: u8,
}

impl PhysicalAddress {
    pub fn ppn(&self) -> u64 {
        (self.ppn2() << 18) | (self.ppn1() << 9) | self.ppn0()
    }
}

pub struct Table {
    entries: [PTE; 512],
}

pub fn map(root: &mut Table, vaddr: VirtualAddress, paddr: PhysicalAddress, flags: PTEFlags) {
    assert!(
        flags.intersects(PTEFlags::RWX),
        "Flags must be either read, write, or execute"
    );

    assert!(
        !flags.intersects(!PTEFlags::RWXUG),
        "Flags can only be read, write, execute, user or global"
    );

    let vpn = vaddr.vpn();
    let ppn = paddr.ppn();

    let mut entry = &mut root.entries[vpn[3]];

    for i in (0..3).rev() {
        if !entry.flags().contains(PTEFlags::VALID) {
            let page = zalloc(1);
            entry.with_flags(PTEFlags::VALID);
            entry.with_ptr(page);
        }

        assert!(
            !entry.flags().intersects(PTEFlags::RWX),
            "PTE entry is a leaf node, cannot map further"
        );

        unsafe {
            let ptr = entry.ptr() as *mut Table;
            entry = &mut (*ptr).entries[vpn[i]];
        }
    }

    entry.with_flags(flags | PTEFlags::VALID);
    entry.with_ppn(ppn);
}

pub fn unmap(root: &mut Table) {
    fn recurse(root: &mut Table, level: usize) {
        if level == 0 {
            // this level does not have branches, so return
            return;
        }

        for entry in &mut root.entries {
            if !entry.flags().contains(PTEFlags::VALID) || entry.is_leaf() {
                continue;
            }

            let ptr = unsafe { entry.ptr() } as *mut Table;
            recurse(unsafe { &mut *ptr }, level - 1);

            // free the table itself
            page::free(ptr as *mut u8);
            entry.clear();
        }
    }

    recurse(root, 3);
}
