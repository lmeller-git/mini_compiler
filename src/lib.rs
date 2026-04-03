use std::sync::atomic::AtomicU8;

pub mod backend;
pub mod frontend;
pub static VERBOSITY: AtomicU8 = AtomicU8::new(0);

#[macro_export]
macro_rules! print_if {
    ($min_verbosity:expr) => {
        if $crate::VERBOSITY.load(::core::sync::atomic::Ordering::Relaxed) > $min_verbosity {
            println!();
        }
    };
    ($min_verbosity:expr, $($arg:tt)*) => {
        if $crate::VERBOSITY.load(::core::sync::atomic::Ordering::Relaxed) > $min_verbosity {
            println!("{}", format_args!($($arg)*))
        }
    };
}
