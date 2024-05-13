## f4-w25q

Support for the W25Q family of flash chips for the STM32F4 family of microcontrollers using stm32f4xx-hal.

## Features

- Expose the flash chip as a [littlefs](https://docs.rs/littlefs2/latest/littlefs2/) file system
- Expose the flash chip as an [embedded_storage_async](https://github.com/rust-embedded-community/embedded-storage) device

## Examples

See examples/ directory

### Note

A default target of stm32f412 is selected so the crate is able to be published. Disable this to select a different variant.

