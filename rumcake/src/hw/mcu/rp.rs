//! Utilities for interfacing with the hardware, specific to RP-based MCUs.
//!
//! Note that the contents of this RP-version of `mcu` module may share some of the same members
//! of other versions of the `mcu` module. This is the case so that parts of `rumcake` can remain
//! hardware-agnostic.

use defmt::assert;
use embassy_rp::bind_interrupts;
use embassy_rp::config::Config;
use embassy_rp::flash::Async;
use embassy_rp::flash::Flash as HALFlash;
use embassy_rp::peripherals::{FLASH, USB};
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

pub use rumcake_macros::{
    input_pin, output_pin, setup_buffered_uart, setup_dma_channel, setup_i2c,
};

pub use embassy_rp;

pub const SYSCLK: u32 = 125_000_000;

pub type RawMutex = ThreadModeRawMutex;

/// A function that allows you to jump to the bootloader, usually for re-flashing the firmware.
pub fn jump_to_bootloader() {
    reset_to_usb_boot(0, 0);
}

/// Initialize the MCU's internal clocks.
pub fn initialize_rcc() {
    let conf = Config::default();
    embassy_rp::init(conf);
    assert!(
        SYSCLK == embassy_rp::clocks::clk_sys_freq(),
        "SYSCLK is not correct."
    );
}

#[cfg(feature = "usb")]
/// Setup the USB driver. The output of this function usually needs to be passed to another
/// function that sets up the HID readers or writers to be used with a task. For example, you may
/// need to pass this to [`crate::usb::setup_usb_hid_nkro_writer`] to set up a keyboard that
/// communicates with a host device over USB.
pub fn setup_usb_driver<K: crate::usb::USBKeyboard>(
) -> embassy_usb::Builder<'static, Driver<'static, USB>> {
    unsafe {
        #[cfg(feature = "rp2040")]
        bind_interrupts!(
            struct Irqs {
                USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
            }
        );

        let mut config = embassy_usb::Config::new(K::USB_VID, K::USB_PID);
        config.manufacturer.replace(K::MANUFACTURER);
        config.product.replace(K::PRODUCT);
        config.serial_number.replace(K::SERIAL_NUMBER);
        config.max_power = 500;

        let usb_driver = Driver::new(USB::steal(), Irqs);

        static DEVICE_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> =
            static_cell::StaticCell::new();
        let device_descriptor = DEVICE_DESCRIPTOR.init([0; 256]);
        static CONFIG_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> =
            static_cell::StaticCell::new();
        let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
        static BOS_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> = static_cell::StaticCell::new();
        let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
        static MSOS_DESCRIPTOR: static_cell::StaticCell<[u8; 256]> = static_cell::StaticCell::new();
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

pub type Flash<'a, const FLASH_SIZE: usize> = HALFlash<'a, FLASH, Async, FLASH_SIZE>;

/// Construct an instance of [`Flash`]. This usually needs to be passed to
/// [`crate::storage::Database::setup`], so that your device can use storage features.
pub fn setup_internal_flash<'a, const FLASH_SIZE: usize>(
    channel: impl crate::hw::mcu::embassy_rp::Peripheral<P = impl crate::hw::mcu::embassy_rp::dma::Channel>
        + 'a,
) -> Flash<'a, FLASH_SIZE> {
    unsafe { Flash::new(FLASH::steal(), channel) }
}
