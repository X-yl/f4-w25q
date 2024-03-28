#![no_std]

pub mod w25q;
#[cfg(feature = "littlefs2")]
pub mod fs;
#[cfg(feature = "embedded-storage")]
pub mod embedded_storage;