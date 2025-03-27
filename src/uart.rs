use core::fmt;

use crate::{print, println};

pub struct Uart {
    base: usize,
}

impl Uart {
    pub const fn new(base: usize) -> Self {
        Uart { base }
    }

    fn ptr_mut(&mut self) -> *mut u8 {
        self.base as *mut u8
    }

    fn ptr(&self) -> *const u8 {
        self.base as *const u8
    }

    pub fn init(&mut self) {
        let ptr = self.ptr_mut();

        unsafe {
            // enable FIFO, write 0b01 to FCR
            ptr.add(2).write_volatile(0b01);

            // LCR word length is 8 bits, 0b11
            const LCR: u8 = 0b11;

            // set LCR
            ptr.add(3).write_volatile(LCR);

            // enable interrupt, write 0b01 to IER
            ptr.add(1).write_volatile(0b01);

            // baud rate. The clock rate is 22.729 MHz or 22_729_000 cy/sec
            // using 2400 baud rate
            // divisor = ceil( (clock_hz) / (baud_rate * 16) )
            const CLOCK_HZ: u32 = 22_729_000;
            const BAUD_RATE: u32 = 2400;
            const DIVISOR: u16 = CLOCK_HZ.div_ceil(BAUD_RATE * 16) as u16;
            const DIVISOR_L: u8 = (DIVISOR >> 8) as u8;
            const DIVISOR_M: u8 = (DIVISOR & 0xff) as u8;

            // enable the DLL and DLM
            ptr.add(3).write_volatile(LCR | 0b1 << 7);

            // put l and h bytes in DLL and DLM respectively
            ptr.add(0).write_volatile(DIVISOR_L);
            ptr.add(1).write_volatile(DIVISOR_M);

            // after we change baud, we never touch the DLL and DLM again
            // so we can set DLAB to 0
            ptr.add(3).write_volatile(LCR);
        }
    }

    pub fn read_raw(&mut self) -> Option<u8> {
        let ptr = self.ptr();

        unsafe {
            // check LSR for data
            if ptr.add(5).read_volatile() & 0b1 == 0 {
                // no data, LSR bit 0 is 0
                None
            } else {
                // read from RBR
                Some(ptr.add(0).read_volatile())
            }
        }
    }

    pub fn read(&mut self) -> Option<char> {
        match self.read_raw() {
            Some(b'\r') => {
                print!("\n");
                return Some('\n');
            }
            Some(b'\x7f') => print!("{} {}", 8 as char, 8 as char),
            Some(byte) => {
                print!("{}", byte as char);
                return Some(byte as char);
            }
            _ => {}
        };

        None
    }

    fn write(&mut self, byte: u8) {
        let ptr = self.ptr_mut();

        unsafe {
            // wait for space in FIFO (THRE=1)
            while ptr.add(5).read_volatile() & (0b1 << 5) == 0 {
                // no space, LSR bit 5 is 0. block until there is space
            }

            // write to THR
            ptr.add(0).write_volatile(byte);
        }
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write(byte);
        }

        Ok(())
    }
}
