//! Utilities for interfacing with the hardware, specific to RP-based MCUs.
//!
//! Note that the contents of this RP-version of `platform` module may share some of the same
//! members of other versions of the `platform` module. This is the case so that parts of `rumcake`
//! can remain hardware-agnostic.

use core::cell::RefCell;
use core::ops::DerefMut;

use defmt::assert;
use embassy_rp::adc::{Adc, Async as AdcAsync, Channel};
use embassy_rp::bind_interrupts;
use embassy_rp::config::Config;
use embassy_rp::flash::Async;
use embassy_rp::flash::Flash as HALFlash;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::{ADC, FLASH, USB};
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::blocking_mutex::ThreadModeMutex;

pub use rumcake_macros::{
    rp_input_pin as input_pin, rp_output_pin as output_pin,
    rp_setup_adc_sampler as setup_adc_sampler, rp_setup_buffered_uart as setup_buffered_uart,
    rp_setup_i2c as setup_i2c,
};

pub use embassy_rp;

use crate::keyboard::MatrixSampler;

use super::Multiplexer;

pub const SYSCLK: u32 = 125_000_000;

pub type RawMutex = ThreadModeRawMutex;
pub type BlockingMutex<T> = ThreadModeMutex<T>;

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
            config_descriptor,
            bos_descriptor,
            msos_descriptor,
            control_buf,
        )
    }
}

/// Different types of analog pins.
pub enum AnalogPinType<'a, const MP: usize> {
    /// A pin that is connected to an analog multiplexer. Must contain a [`Multiplexer`]
    /// definition.
    Multiplexed(Multiplexer<Output<'a>, MP>),
    /// A pin that is directly connected to the analog source.
    Direct,
}

pub type AdcSampleType = u16;

// TODO: use a different mutex if using multiple cores on the MCU, thread mode mutex is not safe for multicore.
/// A sampler for the analog pins on an RP MCU. This sampler can handle analog pins that may be
/// multiplexed, or directly wired to the analog source. This can also be used to power an analog
/// keyboard matrix.
pub struct AdcSampler<'a, const MP: usize, const C: usize> {
    adc_sampler: BlockingMutex<RefCell<RawAdcSampler<'a, MP, C>>>,
}

struct RawAdcSampler<'a, const MP: usize, const C: usize> {
    idx_to_pin_type: [AnalogPinType<'a, MP>; C],
    channels: [Channel<'a>; C],
    adc: Adc<'a, AdcAsync>,
}

impl<'a, const MP: usize, const C: usize> AdcSampler<'a, MP, C> {
    /// Create a new instance of the ADC sampler.
    pub fn new(idx_to_pin_type: [AnalogPinType<'a, MP>; C], analog_pins: [Channel<'a>; C]) -> Self {
        let adc = unsafe {
            bind_interrupts! {
                struct Irqs {
                    ADC_IRQ_FIFO => embassy_rp::adc::InterruptHandler;
                }
            }

            Adc::new(ADC::steal(), Irqs, Default::default())
        };

        Self {
            adc_sampler: BlockingMutex::new(RefCell::new(RawAdcSampler {
                idx_to_pin_type,
                channels: analog_pins,
                adc,
            })),
        }
    }

    /// Obtain a sample from the ADC. The `ch` argument corresponds to the index of the analog pin
    /// you want to sample (which you provided in the [`Self::new()`] method). If the pin is
    /// multiplexed, the `sub_ch` argument is used to determine which multiplexer channel to sample
    /// from. Otherwise, the `sub_ch` argument is ignored.
    pub fn get_sample(&self, ch: usize, sub_ch: usize) -> Option<AdcSampleType> {
        self.adc_sampler.lock(|adc_sampler| {
            let mut adc_sampler = adc_sampler.borrow_mut();
            let RawAdcSampler {
                idx_to_pin_type,
                channels,
                adc,
            } = adc_sampler.deref_mut();

            idx_to_pin_type.get_mut(ch).map(|channel| match channel {
                AnalogPinType::Multiplexed(ref mut multiplexer) => {
                    multiplexer.select_channel(sub_ch as u8).unwrap();
                    adc.blocking_read(&mut channels[ch]).unwrap()
                }
                AnalogPinType::Direct => adc.blocking_read(&mut channels[ch]).unwrap(),
            })
        })
    }
}

impl<'a, const MP: usize, const C: usize> MatrixSampler for AdcSampler<'a, MP, C> {
    type SampleType = AdcSampleType;

    fn get_sample(&self, ch: usize, sub_ch: usize) -> Option<Self::SampleType> {
        self.get_sample(ch, sub_ch)
    }
}

pub type Flash<'a, const FLASH_SIZE: usize> = HALFlash<'a, FLASH, Async, FLASH_SIZE>;

/// Construct an instance of [`Flash`]. This usually needs to be passed to
/// [`crate::storage::Database::setup`], so that your device can use storage features.
pub fn setup_internal_flash<'a, const FLASH_SIZE: usize>(
    channel: impl crate::hw::platform::embassy_rp::Peripheral<
            P = impl crate::hw::platform::embassy_rp::dma::Channel,
        > + 'a,
) -> Flash<'a, FLASH_SIZE> {
    unsafe { Flash::new(FLASH::steal(), channel) }
}
