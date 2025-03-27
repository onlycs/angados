#[macro_export]
macro_rules! print {
    ($($args:tt)+) => {{
        use core::fmt::Write;
        let _ = write!(crate::uart::Uart::new(0x1000_0000), $($args)+);
    }};
}

#[macro_export]
macro_rules! println {
	($($args:tt)+) => {{
	   use core::fmt::Write;
	   let _ = writeln!(crate::uart::Uart::new(0x1000_0000), $($args)+);
	}};
    () => {{
        use core::fmt::Write;
        let _ = writeln!(crate::uart::Uart::new(0x1000_0000));
    }};
}

#[macro_export]
macro_rules! read {
    () => {{ crate::uart::Uart::new(0x1000_0000).read() }};
}
