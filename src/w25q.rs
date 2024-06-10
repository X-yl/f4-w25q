use core::{fmt::Debug, future::Future, pin::Pin, task::{Context, Poll}};
use stm32f4xx_hal::qspi::{
    MemoryMapped, Qspi, QspiError, QspiMemoryMappedConfig, QspiMode, QspiPins, QspiReadCommand,
    QspiWriteCommand,
};

pub struct W25Q<PINS: QspiPins> {
    qspi: Qspi<PINS>,
}

#[derive(Debug, Copy, Clone)]
pub struct DeviceId(pub u8);

const SECTOR_SIZE: u32 = 4096;

#[derive(Debug, Copy, Clone)]
pub struct SectorAddress(u32);

impl SectorAddress {
    pub fn from_address(address: u32) -> Self {
        SectorAddress(address / SECTOR_SIZE)
    }

    pub fn to_address(&self) -> u32 {
        self.0 * SECTOR_SIZE
    }
}

// This struct allows functions to return prior to the chip signaling that it is done.
// It impls Future so can be awaited in async code. Otherwise, it can be dropped at which point,
// it will wait for the operation to complete.
// When this struct is dropped, it will wait for the operation to complete.
// This is useful for operations that take a long time, such as erasing a sector.
// The operation can be started and the struct can be retained until the chip is needed again.
// Once the struct is dropped, the borrow is released and the next operation can be started.
// ```
// let mut w25q = W25Q::new(qspi).unwrap();
// let pending = w25q.erase_sector(SectorAddress::from_address(0x0)).unwrap();
// // Do other stuff
// drop(pending);
// w25q.read(0x0, &mut buf).unwrap();
// ```
pub struct PendingOperation<'a, PINS: QspiPins> { 
    w25q: &'a mut W25Q<PINS>, 
}

impl<'a, PINS: QspiPins> Future for PendingOperation<'a, PINS> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.get_mut().w25q.is_busy() {
            // FIXME: use waker properly (requires status polling mode to be implemented)
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        } 
    }
}

impl<'a, PINS: QspiPins> Drop for PendingOperation<'a, PINS> {
    fn drop(&mut self) {
        self.w25q.wait_on_busy().unwrap();
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Address(u32);

impl Address {
    pub fn from_page_and_offset(page: u32, offset: u32) -> Self {
        Address(page * 256 + offset)
    }
}

impl Into<u32> for Address {
    fn into(self) -> u32 {
        self.0
    }
}

impl From<u32> for Address {
    fn from(address: u32) -> Self {
        Address(address)
    }
}

impl<PINS> W25Q<PINS>
where
    PINS: QspiPins,
{
    pub fn new(qspi: Qspi<PINS>) -> Result<Self, QspiError> {
        let mut chip = Self { qspi };
        chip.release_from_power_down()?;
        chip.quad_enable()?;
        Ok(chip)
    }

    pub fn release_from_power_down(&mut self) -> Result<DeviceId, QspiError> {
        let mut buf = [0u8; 1];

        self.qspi.indirect_read(
            QspiReadCommand::new(&mut buf, QspiMode::SingleChannel)
                .instruction(0xAB, QspiMode::SingleChannel)
                .address(0x0, QspiMode::SingleChannel),
        )?;

        Ok(DeviceId(buf[0]))
    }

    pub fn is_busy(&mut self) -> bool {
        let mut buf = [0u8; 1];

        self.qspi.indirect_read(
            QspiReadCommand::new(&mut buf, QspiMode::SingleChannel)
                .instruction(0x05, QspiMode::SingleChannel),
        ).unwrap();

        buf[0] & 0x01 == 0x01
    }

    pub fn wait_on_busy(&mut self) -> Result<(), QspiError> {
        while self.is_busy() {}
        Ok(())
    }

    pub fn chip_erase(&mut self) -> Result<PendingOperation<'_, PINS>, QspiError> {
        self.write_enable()?;
        self.wait_on_busy()?;
        self.qspi.indirect_write(
            QspiWriteCommand::default().instruction(0x60, QspiMode::SingleChannel),
        )?;

        Ok(PendingOperation { w25q: self })
    }

    pub fn erase_sector(&mut self, address: SectorAddress) -> Result<PendingOperation<'_, PINS>, QspiError> {
        self.write_enable()?;
        self.wait_on_busy()?;
        self.qspi.indirect_write(
            QspiWriteCommand::default()
                .instruction(0x20, QspiMode::SingleChannel)
                .address(address.to_address(), QspiMode::SingleChannel),
        )?;

        Ok(PendingOperation { w25q: self })
    }

    pub fn write_enable(&mut self) -> Result<(), QspiError> {
        self.qspi.indirect_write(
            QspiWriteCommand::default().instruction(0x06, QspiMode::SingleChannel),
        )?;
        self.wait_on_busy()?;

        Ok(())
    }

    pub fn quad_enable(&mut self) -> Result<(), QspiError> {
        // First check if quad is already enabled
        let mut buf = [0u8; 1];
        self.qspi.indirect_read(
            QspiReadCommand::new(&mut buf, QspiMode::SingleChannel)
                .instruction(0x35, QspiMode::SingleChannel),
        )?;

        if buf[0] & 0x02 == 0x02 {
            return Ok(());
        }

        // If not, first we need to make the register writable
        self.write_enable()?;

        // Then we can set the quad enable bit
        self.qspi.indirect_write(
            QspiWriteCommand::default()
                .instruction(0x31, QspiMode::SingleChannel)
                .address(0x0, QspiMode::SingleChannel)
                .data(&[buf[0] | 0x2], QspiMode::SingleChannel),
        )?;
        Ok(())
    }

    // WARNING: This function does not check if the page is already erased.
    // You must call erase_sector() before writing to a sector!!
    pub fn program_page(&mut self, address: Address, data: &[u8]) -> Result<PendingOperation<'_, PINS>, QspiError> {
        self.write_enable()?;

        self.qspi.indirect_write(
            QspiWriteCommand::default()
                .instruction(0x32, QspiMode::SingleChannel)
                .address(address.into(), QspiMode::SingleChannel)
                .data(data, QspiMode::QuadChannel),
        )?;

        Ok(PendingOperation { w25q: self })
    }

    pub fn read(&mut self, address: Address, data: &mut [u8]) -> Result<(), QspiError> {
        self.qspi.indirect_read(
            QspiReadCommand::new(data, QspiMode::QuadChannel)
                .instruction(0xEB, QspiMode::SingleChannel)
                .address(address.into(), QspiMode::QuadChannel)
                .dummy_cycles(6),
        )?;

        Ok(())
    }

    pub fn memory_mapped<'a>(&'a mut self) -> Result<MemoryMapped<'a, PINS>, QspiError> {
        self.qspi.memory_mapped(
            QspiMemoryMappedConfig::default()
                .instruction(0xEB, QspiMode::SingleChannel)
                .address_mode(QspiMode::QuadChannel)
                .data_mode(QspiMode::QuadChannel)
                .dummy_cycles(6),
        )
    }
}
