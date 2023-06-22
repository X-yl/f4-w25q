use cortex_m_semihosting::hprintln;
use littlefs2::driver::Storage;
use stm32f4xx_hal::qspi::QspiPins;

use crate::w25q::{SectorAddress, W25Q};

impl<PINS: QspiPins> Storage for W25Q<PINS> {
    const READ_SIZE: usize = 64;
    const WRITE_SIZE: usize = 256;
    const BLOCK_SIZE: usize = 4096;
    const BLOCK_COUNT: usize = 4096;

    type CACHE_SIZE = typenum::U512;
    type LOOKAHEAD_SIZE = typenum::U256;

    fn read(&mut self, off: usize, buf: &mut [u8]) -> littlefs2::io::Result<usize> {
        self.read((off as u32).into(), buf)
            .map_err(|_| littlefs2::io::Error::Io)
            .map(|_| buf.len())
    }

    fn write(&mut self, off: usize, data: &[u8]) -> littlefs2::io::Result<usize> {
        if off % 256 != 0 {
            hprintln!("write: offset not page aligned");
        }
        let res = self
            .program_page((off as u32).into(), data)
            .map_err(|_| littlefs2::io::Error::Io)
            .map(|_| data.len());
        self.wait_on_busy().map_err(|_| littlefs2::io::Error::Io)?;
        res
    }

    fn erase(&mut self, off: usize, len: usize) -> littlefs2::io::Result<usize> {
        self.erase_sector(SectorAddress::from_address(off as u32))
            .map_err(|_| littlefs2::io::Error::Io)
            .map(|_| len)
    }
}
