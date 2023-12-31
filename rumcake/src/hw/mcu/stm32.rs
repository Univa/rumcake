//! Utilities for interfacing with the hardware, specific to STM32-based MCUs.
//!
//! Note that the contents of this STM32-version of `mcu` module may share some of the same members
//! of other versions of the `mcu` module. This is the case so that parts of `rumcake` can remain
//! hardware-agnostic.

use core::fmt::Debug;
use embassy_stm32::bind_interrupts;
use embassy_stm32::dma::NoDma;
use embassy_stm32::flash::{Bank1Region, Blocking, Flash as HALFlash};
use embassy_stm32::i2c::I2c;
use embassy_stm32::peripherals::{FLASH, PA11, PA12, USB};
use embassy_stm32::time::Hertz;
use embassy_stm32::usb::Driver;
use embedded_hal::blocking::i2c::Write;
use embedded_hal_async::i2c::I2c as AsyncI2c;
use embedded_storage::nor_flash::{ErrorType, NorFlash, ReadNorFlash};
use embedded_storage_async::nor_flash::{
    NorFlash as AsyncNorFlash, ReadNorFlash as AsyncReadNorFlash,
};
use static_cell::StaticCell;

pub use embassy_stm32;

#[cfg(feature = "stm32f072cb")]
pub const SYSCLK: u32 = 48_000_000;

#[cfg(feature = "stm32f303cb")]
pub const SYSCLK: u32 = 72_000_000;

/// A function that allows you to jump to the bootloader, usually for re-flashing the firmware.
pub fn jump_to_bootloader() {
    #[cfg(feature = "stm32f072cb")]
    unsafe {
        cortex_m::asm::bootload(0x1FFFC800 as _)
    };

    #[cfg(feature = "stm32f303cb")]
    unsafe {
        cortex_m::asm::bootload(0x1FFFD800 as _)
    };
}

/// Initialize the MCU's internal clocks.
pub fn initialize_rcc() {
    let mut conf = embassy_stm32::Config::default();
    let mut rcc_conf = embassy_stm32::rcc::Config::default();

    #[cfg(feature = "stm32f072cb")]
    {
        rcc_conf.sys_ck = Some(embassy_stm32::time::Hertz(SYSCLK));
    }

    #[cfg(feature = "stm32f303cb")]
    {
        rcc_conf.sysclk = Some(embassy_stm32::time::Hertz(SYSCLK));
        rcc_conf.hse = Some(embassy_stm32::time::Hertz(8_000_000));
        rcc_conf.pclk1 = Some(embassy_stm32::time::Hertz(24_000_000));
        rcc_conf.pclk2 = Some(embassy_stm32::time::Hertz(24_000_000));
    }

    conf.rcc = rcc_conf;

    embassy_stm32::init(conf);
}

#[macro_export]
macro_rules! input_pin {
    ($p:ident) => {
        unsafe {
            $crate::hw::mcu::embassy_stm32::gpio::Input::new(
                $crate::hw::mcu::embassy_stm32::gpio::Pin::degrade(
                    $crate::hw::mcu::embassy_stm32::peripherals::$p::steal(),
                ),
                $crate::hw::mcu::embassy_stm32::gpio::Pull::Up,
            )
        }
    };
}

#[macro_export]
macro_rules! output_pin {
    ($p:ident) => {
        unsafe {
            $crate::hw::mcu::embassy_stm32::gpio::Output::new(
                $crate::hw::mcu::embassy_stm32::gpio::Pin::degrade(
                    $crate::hw::mcu::embassy_stm32::peripherals::$p::steal(),
                ),
                $crate::hw::mcu::embassy_stm32::gpio::Level::High,
                $crate::hw::mcu::embassy_stm32::gpio::Speed::Low,
            )
        }
    };
}

#[cfg(feature = "usb")]
/// Setup the USB driver. The output of this function usually needs to be passed to another
/// function that sets up the HID readers or writers to be used with a task. For example, you may
/// need to pass this to [`crate::usb::setup_usb_hid_nkro_writer`] to set up a keyboard that
/// communicates with a host device over USB.
pub fn setup_usb_driver<K: crate::usb::USBKeyboard>(
) -> embassy_usb::Builder<'static, Driver<'static, USB>> {
    unsafe {
        #[cfg(feature = "stm32f072cb")]
        bind_interrupts!(
            struct Irqs {
                USB => embassy_stm32::usb::InterruptHandler<embassy_stm32::peripherals::USB>;
            }
        );

        #[cfg(feature = "stm32f303cb")]
        bind_interrupts!(
            struct Irqs {
                USB_LP_CAN_RX0 => embassy_stm32::usb::InterruptHandler<embassy_stm32::peripherals::USB>;
            }
        );

        let mut config = embassy_usb::Config::new(K::USB_VID, K::USB_PID);
        config.manufacturer.replace(K::MANUFACTURER);
        config.product.replace(K::PRODUCT);
        config.serial_number.replace(K::SERIAL_NUMBER);
        config.max_power = 500;

        let usb_driver = Driver::new(USB::steal(), Irqs, PA12::steal(), PA11::steal());

        static DEVICE_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> =
            static_cell::StaticCell::new();
        let device_descriptor = DEVICE_DESCRIPTOR.init([0; 256]);
        static CONFIG_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> =
            static_cell::StaticCell::new();
        let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
        static BOS_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> = static_cell::StaticCell::new();
        let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
        static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        let msos_descriptor = MSOS_DESCRIPTOR.init([0; 256]);
        static CONTROL_BUF: static_cell::StaticCell<[u8; 128]> = static_cell::StaticCell::new();
        let control_buf = CONTROL_BUF.init([0; 128]);

        embassy_usb::Builder::new(
            usb_driver,
            config,
            device_descriptor,
            config_descriptor,
            bos_descriptor,
            msos_descriptor,
            control_buf,
        )
    }
}

/// A wrapper around the [`embassy_stm32::Flash`] struct. This implements
/// [`embedded_storage_async`] traits so that it can work with the [`crate::storage`] system.
pub struct Flash {
    flash: HALFlash<'static, Blocking>,
}

impl ErrorType for Flash {
    type Error = embassy_stm32::flash::Error;
}

impl AsyncReadNorFlash for Flash {
    const READ_SIZE: usize = <HALFlash as ReadNorFlash>::READ_SIZE;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.flash.read(offset, bytes)
    }

    fn capacity(&self) -> usize {
        self.flash.capacity()
    }
}

impl AsyncNorFlash for Flash {
    const WRITE_SIZE: usize = <HALFlash as embedded_storage::nor_flash::NorFlash>::WRITE_SIZE;

    const ERASE_SIZE: usize = <HALFlash as embedded_storage::nor_flash::NorFlash>::ERASE_SIZE;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.flash.erase(from, to)
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.flash.write(offset, bytes)
    }
}

/// Construct an instance of [`Flash`]. This usually needs to be passed to
/// [`crate::storage::Database::setup`], so that your device can use storage features.
pub fn setup_internal_flash() -> Flash {
    Flash {
        flash: unsafe { HALFlash::new_blocking(FLASH::steal()) },
    }
}

#[macro_export]
macro_rules! setup_i2c {
    ($event_interrupt:ident, $error_interrupt:ident, $i2c:ident, $scl:ident, $sda:ident, $rxdma:ident, $txdma:ident) => {
        fn setup_i2c() -> impl $crate::embedded_hal_async::i2c::I2c<Error = impl core::fmt::Debug> {
            unsafe {
                $crate::hw::mcu::embassy_stm32::bind_interrupts! {
                    struct Irqs {
                        $event_interrupt => $crate::hw::mcu::embassy_stm32::i2c::EventInterruptHandler<$crate::hw::mcu::embassy_stm32::peripherals::$i2c>;
                        $error_interrupt => $crate::hw::mcu::embassy_stm32::i2c::ErrorInterruptHandler<$crate::hw::mcu::embassy_stm32::peripherals::$i2c>;
                    }
                };
                let i2c = $crate::hw::mcu::embassy_stm32::peripherals::$i2c::steal();
                let scl = $crate::hw::mcu::embassy_stm32::peripherals::$scl::steal();
                let sda = $crate::hw::mcu::embassy_stm32::peripherals::$sda::steal();
                let rx_dma = $crate::hw::mcu::embassy_stm32::peripherals::$rxdma::steal();
                let tx_dma = $crate::hw::mcu::embassy_stm32::peripherals::$txdma::steal();
                let time = $crate::hw::mcu::embassy_stm32::time::Hertz(100_000);
                $crate::hw::mcu::embassy_stm32::i2c::I2c::new(i2c, scl, sda, Irqs, tx_dma, rx_dma, time, Default::default())
            }
        }
    };
}

#[macro_export]
macro_rules! setup_uart_reader {
    ($interrupt:ident, $uart:ident, $rx:ident, $rxdma:ident) => {
        fn setup_uart_reader() -> impl $crate::embedded_io_async::Read<Error = impl core::fmt::Debug> {
            unsafe {
                $crate::hw::mcu::embassy_stm32::bind_interrupts! {
                    struct Irqs {
                        $interrupt => $crate::hw::mcu::embassy_stm32::usart::InterruptHandler<$crate::hw::mcu::embassy_stm32::peripherals::$uart>;
                    }
                };
                let uart = $crate::hw::mcu::embassy_stm32::peripherals::$uart::steal();
                let rx = $crate::hw::mcu::embassy_stm32::peripherals::$rx::steal();
                let rx_dma = $crate::hw::mcu::embassy_stm32::peripherals::$rxdma::steal();
                $crate::hw::mcu::embassy_stm32::usart::UartRx::new(uart, Irqs, rx, rx_dma, Default::default()).into_ring_buffered(&mut [0; 32]);
            }
        }
    };
}

#[macro_export]
macro_rules! setup_uart_writer {
    ($interrupt:ident, $uart:ident, $tx:ident, $txdma:ident) => {
        fn setup_uart_writer(
        ) -> impl $crate::embedded_io_async::Write<Error = impl core::fmt::Debug> {
            unsafe {
                let uart = $crate::hw::mcu::embassy_stm32::peripherals::$uart::steal();
                let tx = $crate::hw::mcu::embassy_stm32::peripherals::$tx::steal();
                let tx_dma = $crate::hw::mcu::embassy_stm32::peripherals::$txdma::steal();
                $crate::hw::mcu::embassy_stm32::usart::UartTx::new(
                    uart,
                    tx,
                    tx_dma,
                    Default::default(),
                )
            }
        }
    };
}
