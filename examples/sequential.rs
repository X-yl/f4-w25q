#![no_std]
#![no_main]

use core::panic::PanicInfo;
use cortex_m::Peripherals;
use cortex_m_rt::entry;
use f4_w25q::w25q::W25Q;
use stm32f4xx_hal::gpio::GpioExt;
use stm32f4xx_hal::pac;
use stm32f4xx_hal::qspi::{AddressSize, Bank1, FlashSize, Qspi, QspiConfig, SampleShift};
use sequential_storage::cache::NoCache;
use sequential_storage::queue;
use f4_w25q::embedded_storage::W25QSequentialStorage;

#[entry]
fn main() -> ! {
    if let (Some(mut dp), Some(mut cp)) = (pac::Peripherals::take(), Peripherals::take()) {
        let gpiob = dp.GPIOB.split();
        let gpioc = dp.GPIOC.split();
        let gpioa = dp.GPIOA.split();

        let qspi = Qspi::<Bank1>::new(
            dp.QUADSPI,
            (
                gpiob.pb6, gpioc.pc9, gpioc.pc10, gpioc.pc8, gpioa.pa1, gpiob.pb1,
            ),
            QspiConfig::default()
                .address_size(AddressSize::Addr24Bit)
                .flash_size(FlashSize::from_megabytes(16))
                .clock_prescaler(0)
                .sample_shift(SampleShift::HalfACycle),
        );

        let mut flash = W25QSequentialStorage::<Bank1, 16384>::new(W25Q::new(qspi).unwrap());
        let _ = queue::push(&mut flash, 0..8192, &mut NoCache::new(), b"hello, world", false);
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
