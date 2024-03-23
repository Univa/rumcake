//! Utilities for interfacing with the hardware, specific to STM32-based MCUs.
//!
//! Note that the contents of this STM32-version of `mcu` module may share some of the same members
//! of other versions of the `mcu` module. This is the case so that parts of `rumcake` can remain
//! hardware-agnostic.

use core::cell::RefCell;
use core::future::poll_fn;
use core::marker::PhantomData;
use core::ops::DerefMut;
use core::task::Poll;

use embassy_futures::block_on;
use embassy_stm32::adc::{Adc, AdcPin, Instance, InterruptHandler, SampleTime};
use embassy_stm32::flash::{Blocking, Flash as HALFlash};
use embassy_stm32::gpio::Output;
use embassy_stm32::interrupt::typelevel::Binding;
use embassy_stm32::peripherals::{FLASH, PA11, PA12, USB};
use embassy_stm32::rcc::{Pll, PllMul, PllPreDiv, PllSource, Sysclk};
use embassy_stm32::usb::Driver;
use embassy_stm32::{bind_interrupts, Peripheral};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::blocking_mutex::ThreadModeMutex;
use static_cell::StaticCell;

pub use rumcake_macros::{
    input_pin, output_pin, setup_adc_sampler, setup_buffered_uart, setup_i2c,
};

pub use embassy_stm32;

use crate::keyboard::MatrixSampler;

use super::Multiplexer;

#[cfg(feature = "stm32f072cb")]
pub const SYSCLK: u32 = 48_000_000;

#[cfg(feature = "stm32f303cb")]
pub const SYSCLK: u32 = 72_000_000;

pub type RawMutex = ThreadModeRawMutex;
pub type BlockingMutex<T> = ThreadModeMutex<T>;

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

const fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Initialize the MCU's internal clocks.
pub fn initialize_rcc() {
    let mut conf = embassy_stm32::Config::default();
    let mut rcc_conf = embassy_stm32::rcc::Config::default();

    #[cfg(feature = "stm32f072cb")]
    {
        use embassy_stm32::rcc::HSI_FREQ;

        rcc_conf.pll = Some(Pll {
            src: PllSource::HSI,
            prediv: PllPreDiv::DIV2,
            mul: PllMul::from(((2 * 2 * SYSCLK + HSI_FREQ.0) / HSI_FREQ.0 / 2) as u8 - 2),
        });
        rcc_conf.sys = Sysclk::PLL1_P;
    }

    #[cfg(feature = "stm32f303cb")]
    {
        use embassy_stm32::rcc::{APBPrescaler, AdcClockSource, AdcPllPrescaler, Hse};

        let hse = embassy_stm32::time::mhz(8);
        let div = gcd(SYSCLK, hse.0);

        rcc_conf.hse = Some(Hse {
            freq: hse,
            mode: embassy_stm32::rcc::HseMode::Oscillator,
        });
        rcc_conf.pll = Some(Pll {
            src: PllSource::HSE,
            prediv: PllPreDiv::from((hse.0 / div) as u8 - 1),
            mul: PllMul::from((SYSCLK / div) as u8 - 2),
        });
        rcc_conf.apb1_pre = APBPrescaler::DIV2;
        rcc_conf.apb2_pre = APBPrescaler::DIV2;
        rcc_conf.adc = AdcClockSource::Pll(AdcPllPrescaler::DIV1);
        rcc_conf.sys = Sysclk::PLL1_P;
    }

    conf.rcc = rcc_conf;

    embassy_stm32::init(conf);
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

pub struct Channel<A: Instance> {
    channel: u8,
    _phantom: PhantomData<A>,
}

impl<A: Instance> Channel<A> {
    pub fn new(mut pin: impl AdcPin<A>) -> Self {
        #[cfg(feature = "stm32f072cb")]
        pin.set_as_analog();

        Self {
            channel: pin.channel(),
            _phantom: PhantomData,
        }
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

/// A sampler for the analog pins on an STM32 MCU. This sampler can handle analog pins that may be
/// multiplexed, or directly wired to the analog source. This can also be used to power an analog
/// keyboard matrix.
pub struct AdcSampler<'a, ADC: Instance, const MP: usize, const C: usize> {
    adc_sampler: BlockingMutex<RefCell<RawAdcSampler<'a, ADC, MP, C>>>,
}

struct RawAdcSampler<'a, ADC: Instance, const MP: usize, const C: usize> {
    _adc: Adc<'a, ADC>,
    idx_to_pin_type: [AnalogPinType<'a, MP>; C],
    analog_pins: [Channel<ADC>; C],
}

impl<'a, ADC: Instance, const MP: usize, const C: usize> AdcSampler<'a, ADC, MP, C> {
    /// Create a new instance of the ADC sampler.
    pub fn new(
        adc: impl Peripheral<P = ADC> + 'a,
        irq: impl Binding<ADC::Interrupt, InterruptHandler<ADC>> + 'a,
        idx_to_pin_type: [AnalogPinType<'a, MP>; C],
        analog_pins: [Channel<ADC>; C],
    ) -> Self {
        let _adc = Adc::new(adc, irq, &mut embassy_time::Delay);

        #[cfg(feature = "stm32f303cb")]
        for Channel { channel: ch, .. } in analog_pins.iter() {
            let sample_time = SampleTime::CYCLES1_5;
            if *ch <= 9 {
                ADC::regs()
                    .smpr1()
                    .modify(|reg| reg.set_smp(*ch as _, sample_time));
            } else {
                ADC::regs()
                    .smpr2()
                    .modify(|reg| reg.set_smp((ch - 10) as _, sample_time));
            }
        }

        #[cfg(feature = "stm32f072cb")]
        {
            let sample_time = SampleTime::CYCLES1_5;
            ADC::regs().smpr().modify(|reg| reg.set_smp(sample_time))
        }

        Self {
            adc_sampler: BlockingMutex::new(RefCell::new(RawAdcSampler {
                _adc,
                idx_to_pin_type,
                analog_pins,
            })),
        }
    }

    async fn read(&self, ch: &mut Channel<ADC>) -> AdcSampleType {
        // This is just Adc::read and Adc::convert inlined.
        // Sample time setup is done in `Self::new()`.

        #[cfg(feature = "stm32f303cb")]
        {
            ADC::regs().sqr1().write(|w| w.set_sq(0, ch.channel));

            ADC::regs().isr().write(|_| {});

            ADC::regs().ier().modify(|w| w.set_eocie(true));
            ADC::regs().cr().modify(|w| w.set_adstart(true));

            poll_fn(|cx| {
                ADC::state().waker.register(cx.waker());

                if ADC::regs().isr().read().eoc() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;

            ADC::regs().isr().write(|_| {});

            ADC::regs().dr().read().rdata()
        }

        #[cfg(feature = "stm32f072cb")]
        {
            ADC::regs()
                .chselr()
                .write(|reg| reg.set_chselx(ch.channel as usize, true));

            ADC::regs().isr().modify(|reg| {
                reg.set_eoc(true);
                reg.set_eosmp(true);
            });

            ADC::regs().ier().modify(|w| w.set_eocie(true));
            ADC::regs().cr().modify(|reg| reg.set_adstart(true));

            poll_fn(|cx| {
                ADC::state().waker.register(cx.waker());

                if ADC::regs().isr().read().eoc() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;

            ADC::regs().dr().read().data()
        }
    }

    /// Obtain a sample from the ADC. The `ch` argument corresponds to the index of the analog pin
    /// you want to sample (which you provided in the [`Self::new`] method). If the pin is
    /// multiplexed, the `sub_ch` argument is used to determine which multiplexer channel to sample
    /// from. Otherwise, the `sub_ch` argument is ignored.
    pub fn get_sample(&self, ch: usize, sub_ch: usize) -> Option<AdcSampleType> {
        self.adc_sampler.lock(|adc_sampler| {
            let mut adc_sampler = adc_sampler.borrow_mut();
            let RawAdcSampler {
                idx_to_pin_type,
                analog_pins,
                ..
            } = adc_sampler.deref_mut();

            idx_to_pin_type.get_mut(ch).map(|channel| match channel {
                AnalogPinType::Multiplexed(ref mut multiplexer) => {
                    multiplexer.select_channel(sub_ch as u8).unwrap();
                    block_on(self.read(&mut analog_pins[ch]))
                }
                AnalogPinType::Direct => block_on(self.read(&mut analog_pins[ch])),
            })
        })
    }
}

// ok now this is epic

impl<'a, ADC: Instance, const MP: usize, const C: usize> MatrixSampler
    for AdcSampler<'a, ADC, MP, C>
{
    type SampleType = AdcSampleType;

    fn get_sample(&self, ch: usize, sub_ch: usize) -> Option<Self::SampleType> {
        self.get_sample(ch, sub_ch)
    }
}

impl<
        'a,
        ADC: Instance,
        const MP: usize,
        const C: usize,
        ADC2: Instance,
        const MP2: usize,
        const C2: usize,
    > MatrixSampler for (AdcSampler<'a, ADC, MP, C>, AdcSampler<'a, ADC2, MP2, C2>)
{
    type SampleType = AdcSampleType;

    fn get_sample(&self, ch: usize, sub_ch: usize) -> Option<Self::SampleType> {
        if ch < C {
            return self.0.get_sample(ch, sub_ch);
        }

        self.1.get_sample(ch, sub_ch)
    }
}

impl<
        'a,
        ADC: Instance,
        const MP: usize,
        const C: usize,
        ADC2: Instance,
        const MP2: usize,
        const C2: usize,
        ADC3: Instance,
        const MP3: usize,
        const C3: usize,
    > MatrixSampler
    for (
        AdcSampler<'a, ADC, MP, C>,
        AdcSampler<'a, ADC2, MP2, C2>,
        AdcSampler<'a, ADC3, MP3, C3>,
    )
{
    type SampleType = AdcSampleType;

    fn get_sample(&self, ch: usize, sub_ch: usize) -> Option<Self::SampleType> {
        if ch < C {
            return self.0.get_sample(ch, sub_ch);
        }

        if ch < C2 {
            return self.1.get_sample(ch, sub_ch);
        }

        self.2.get_sample(ch, sub_ch)
    }
}

impl<
        'a,
        ADC: Instance,
        const MP: usize,
        const C: usize,
        ADC2: Instance,
        const MP2: usize,
        const C2: usize,
        ADC3: Instance,
        const MP3: usize,
        const C3: usize,
        ADC4: Instance,
        const MP4: usize,
        const C4: usize,
    > MatrixSampler
    for (
        AdcSampler<'a, ADC, MP, C>,
        AdcSampler<'a, ADC2, MP2, C2>,
        AdcSampler<'a, ADC3, MP3, C3>,
        AdcSampler<'a, ADC4, MP4, C4>,
    )
{
    type SampleType = AdcSampleType;

    fn get_sample(&self, ch: usize, sub_ch: usize) -> Option<Self::SampleType> {
        if ch < C {
            return self.0.get_sample(ch, sub_ch);
        }

        if ch < C2 {
            return self.1.get_sample(ch, sub_ch);
        }

        if ch < C3 {
            return self.2.get_sample(ch, sub_ch);
        }

        self.3.get_sample(ch, sub_ch)
    }
}

/// A wrapper around the [`embassy_stm32::Flash`] struct. This implements
/// [`embedded_storage_async`] traits so that it can work with the [`crate::storage`] system.
pub struct Flash {
    flash: HALFlash<'static, Blocking>,
}

#[cfg(feature = "storage")]
impl crate::storage::FlashStorage for Flash {
    type Error = embassy_stm32::flash::Error;

    const ERASE_SIZE: usize = <HALFlash as embedded_storage::nor_flash::NorFlash>::ERASE_SIZE;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        use embedded_storage::nor_flash::NorFlash;
        self.flash.erase(from, to)
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        use embedded_storage::nor_flash::NorFlash;
        self.flash.write(offset, bytes)
    }

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        use embedded_storage::nor_flash::ReadNorFlash;
        self.flash.read(offset, bytes)
    }

    fn blocking_read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        use embedded_storage::nor_flash::ReadNorFlash;
        self.flash.read(offset, bytes)
    }
}

/// Construct an instance of [`Flash`]. This usually needs to be passed to
/// [`crate::storage::Database::setup`], so that your device can use storage features.
pub fn setup_internal_flash() -> Flash {
    Flash {
        flash: unsafe { HALFlash::new_blocking(FLASH::steal()) },
    }
}
