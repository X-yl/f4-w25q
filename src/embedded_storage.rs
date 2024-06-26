use core::cmp::min;

use embedded_storage_async::nor_flash::{ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash};
use stm32f4xx_hal::qspi::{QspiError, QspiPins};

use crate::w25q::{SectorAddress, W25Q};

pub struct W25QSequentialStorage<PINS: QspiPins, const CAPACITY: usize> {
    flash: W25Q<PINS>,
}

impl<PINS: QspiPins, const CAPACITY: usize> W25QSequentialStorage<PINS, CAPACITY> {
    pub fn new(flash: W25Q<PINS>) -> Self {
        W25QSequentialStorage { flash }
    }

    pub fn release(self) -> W25Q<PINS> {
        self.flash
    }
}

#[derive(Debug)]
pub struct QspiErrorWrapper { qspi_error: QspiError }

impl NorFlashError for QspiErrorWrapper {
    fn kind(&self) -> NorFlashErrorKind {
        match self.qspi_error {
            QspiError::Busy => NorFlashErrorKind::Other,
            QspiError::Address => NorFlashErrorKind::OutOfBounds,
            QspiError::Unknown => NorFlashErrorKind::Other,
            QspiError::IllegalArgument => NorFlashErrorKind::Other
        }
    }
}

impl<PINS: QspiPins, const CAPACITY: usize> ErrorType for W25QSequentialStorage<PINS, CAPACITY> {
    type Error = QspiErrorWrapper;
}

impl<PINS: QspiPins, const CAPACITY: usize> ReadNorFlash for W25QSequentialStorage<PINS, CAPACITY> {
    const READ_SIZE: usize = 1;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.flash.read((offset as u32).into(), bytes)
            .map_err(|qspi_error| {
                QspiErrorWrapper { qspi_error  }    
            })?;
        Ok(())
    }

    fn capacity(&self) -> usize {
        CAPACITY
    }
}

impl<PINS: QspiPins, const CAPACITY: usize> NorFlash for W25QSequentialStorage<PINS, CAPACITY> {
    const WRITE_SIZE: usize = 1;
    const ERASE_SIZE: usize = 4096;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        if from % 4096 != 0 || to % 4096 != 0 || from > to {
            return Err(QspiErrorWrapper { qspi_error: QspiError::IllegalArgument });
        }
        for i in 0..(from - to / 4096) {
            self.flash.erase_sector(SectorAddress::from_address((from + i * 4096) as u32))
                .map_err(|qspi_error| {
                    QspiErrorWrapper { qspi_error }
            })?.await;
        }
        Ok(())
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.flash.program_page(offset.into(), &bytes[..min(256-(offset % 256) as usize, bytes.len())])
            .map_err(|qspi_error| {
                QspiErrorWrapper { qspi_error }
            })?.await;

        if bytes.len() > 256-(offset % 256) as usize {
            for (i, chunk) in bytes[256-(offset % 256) as usize..].chunks(256).enumerate() {
                self.flash.program_page((((offset & !0xFF) + (i as u32 + 1) * 256) as u32).into(), chunk)
                    .map_err(|qspi_error| {
                        QspiErrorWrapper { qspi_error }
                    })?.await;
            }
        }
        Ok(())
    }
}
