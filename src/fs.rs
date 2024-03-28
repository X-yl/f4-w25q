use littlefs2::driver::Storage;
use stm32f4xx_hal::qspi::QspiPins;

use crate::w25q::{SectorAddress, W25Q};


pub struct W25QWrapper<const BLOCK_COUNT: usize, PINS: QspiPins> {
    flash: W25Q<PINS>,
}

impl<const BLOCK_COUNT: usize, PINS: QspiPins> W25QWrapper<BLOCK_COUNT, PINS> {
    pub fn new(flash: W25Q<PINS>) -> Self {
        Self {
            flash,
        }
    }
}

impl<const BLOCK_COUNT: usize, PINS: QspiPins> Storage for W25QWrapper<BLOCK_COUNT, PINS> {
    const BLOCK_COUNT: usize = BLOCK_COUNT;
    const READ_SIZE: usize = 256;
    const WRITE_SIZE: usize = 256;
    const BLOCK_SIZE: usize = 4096;

    type CACHE_SIZE = typenum::U512;
    type LOOKAHEAD_SIZE = typenum::U256;

    fn read(&mut self, off: usize, buf: &mut [u8]) -> littlefs2::io::Result<usize> {
        self.flash.read((off as u32).into(), buf)
            .map_err(|_| {
                littlefs2::io::Error::Io
            })
            .map(|_| buf.len())
    }

    fn write(&mut self, off: usize, data: &[u8]) -> littlefs2::io::Result<usize> {
        for (i, page) in data.chunks_exact(256).enumerate() {
            self.flash.program_page(((off + i * 256) as u32).into(), page)
                .map_err(|_| {
                    littlefs2::io::Error::Io
                })?;
            self.flash.wait_on_busy().unwrap();
        }

        Ok(data.len())
    }

    fn erase(&mut self, off: usize, len: usize) -> littlefs2::io::Result<usize> {
        for i in 0..(len / 4096) {
            self.flash.erase_sector(SectorAddress::from_address((off + i * 4096) as u32))
                .map_err(|_| {
                    littlefs2::io::Error::Io
                })?;
        }
        Ok(len)
    }
}
