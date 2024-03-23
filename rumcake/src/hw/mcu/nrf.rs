//! Utilities for interfacing with the hardware, specific to nRF5x-based MCUs.
//!
//! Note that the contents of this nRF5x-version of `mcu` module may share some of the same members
//! of other versions of the `mcu` module. This is the case so that parts of `rumcake` can remain
//! hardware-agnostic.

use core::cell::RefCell;
use core::mem::MaybeUninit;
use core::ops::DerefMut;

use defmt::error;
use embassy_futures::select::select;
use embassy_nrf::bind_interrupts;
use embassy_nrf::gpio::Output;
use embassy_nrf::interrupt::{InterruptExt, Priority};
use embassy_nrf::nvmc::Nvmc;
use embassy_nrf::peripherals::SAADC;
use embassy_nrf::ppi::ConfigurableChannel;
use embassy_nrf::saadc::{ChannelConfig, Input, Saadc, VddhDiv5Input};
use embassy_nrf::timer::Instance;
use embassy_nrf::usb::Driver;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::blocking_mutex::ThreadModeMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use crate::hw::BATTERY_LEVEL_STATE;
use crate::keyboard::MatrixSampler;

pub use rumcake_macros::{
    input_pin, output_pin, setup_adc_sampler, setup_buffered_uarte, setup_i2c, setup_i2c_blocking,
};

pub use embassy_nrf;

#[cfg(feature = "nrf-ble")]
pub use nrf_softdevice;

use super::Multiplexer;

#[cfg(feature = "nrf52840")]
pub const SYSCLK: u32 = 48_000_000;

pub type RawMutex = ThreadModeRawMutex;
pub type BlockingMutex<T> = ThreadModeMutex<T>;

pub fn jump_to_bootloader() {
    // TODO
}

pub fn initialize_rcc() {
    let mut conf = embassy_nrf::config::Config::default();
    conf.time_interrupt_priority = Priority::P2;
    embassy_nrf::init(conf);
}

#[cfg(feature = "nrf-ble")]
static VBUS_DETECT: once_cell::sync::OnceCell<embassy_nrf::usb::vbus_detect::SoftwareVbusDetect> =
    once_cell::sync::OnceCell::new();

#[cfg(feature = "usb")]
/// Setup the USB driver. The output of this function usually needs to be passed to another
/// function that sets up the HID readers or writers to be used with a task. For example, you may
/// need to pass this to [`crate::usb::setup_usb_hid_nkro_writer`] to set up a keyboard that
/// communicates with a host device over USB.
pub fn setup_usb_driver<K: crate::usb::USBKeyboard + 'static>() -> embassy_usb::Builder<
    'static,
    Driver<'static, embassy_nrf::peripherals::USBD, impl embassy_nrf::usb::vbus_detect::VbusDetect>,
> {
    unsafe {
        #[cfg(feature = "nrf52840")]
        bind_interrupts!(
            struct Irqs {
                USBD => embassy_nrf::usb::InterruptHandler<embassy_nrf::peripherals::USBD>;
                POWER_CLOCK => embassy_nrf::usb::vbus_detect::InterruptHandler;
            }
        );

        embassy_nrf::interrupt::USBD.set_priority(embassy_nrf::interrupt::Priority::P2);
        embassy_nrf::interrupt::POWER_CLOCK.set_priority(embassy_nrf::interrupt::Priority::P2);

        let mut config = embassy_usb::Config::new(K::USB_VID, K::USB_PID);
        config.manufacturer.replace(K::MANUFACTURER);
        config.product.replace(K::PRODUCT);
        config.serial_number.replace(K::SERIAL_NUMBER);
        config.max_power = 100;

        #[cfg(feature = "nrf-ble")]
        let vbus_detect = VBUS_DETECT
            .get_or_init(|| embassy_nrf::usb::vbus_detect::SoftwareVbusDetect::new(true, true));

        let usb_driver = Driver::new(
            embassy_nrf::peripherals::USBD::steal(),
            Irqs,
            #[cfg(feature = "nrf-ble")]
            vbus_detect,
            #[cfg(not(feature = "nrf-ble"))]
            embassy_nrf::usb::vbus_detect::HardwareVbusDetect::new(Irqs),
        );

        static DEVICE_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        let device_descriptor = DEVICE_DESCRIPTOR.init([0; 256]);
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
        static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        let msos_descriptor = MSOS_DESCRIPTOR.init([0; 256]);
        static CONTROL_BUF: StaticCell<[u8; 128]> = StaticCell::new();
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

/// A wrapper around the [`embassy_nrf::nvmc::Nvmc`] struct. This implements
/// [`embedded_storage_async`] traits so that it can work with the [`crate::storage`] system.
pub struct Flash {
    flash: Nvmc<'static>,
}

#[cfg(feature = "storage")]
impl crate::storage::FlashStorage for Flash {
    type Error = embassy_nrf::nvmc::Error;

    const ERASE_SIZE: usize = <Nvmc as embedded_storage::nor_flash::NorFlash>::ERASE_SIZE;

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
        flash: unsafe { Nvmc::new(embassy_nrf::peripherals::NVMC::steal()) },
    }
}

#[cfg(feature = "nrf-ble")]
/// Takes an instance of [`nrf_softdevice::Flash`]. This usually needs to be passed to
/// [`crate::storage::Database::setup`], so that your device can use storage features. If you are
/// using bluetooth features, you should use this instead of [`setup_internal_softdevice_flash`].
pub fn setup_internal_softdevice_flash(sd: &nrf_softdevice::Softdevice) -> nrf_softdevice::Flash {
    nrf_softdevice::Flash::take(sd)
}

pub type AdcSampleType = i16;

/// Different types of analog pins.
pub enum AnalogPinType<'a, const MP: usize>
where
    [(); 2_usize.pow(MP as u32)]:,
{
    /// A pin that is connected to an analog multiplexer. Must contain a buffer to store the
    /// samples obtained by the ADC, and a [`Multiplexer`] definition.
    Multiplexed(
        [AdcSampleType; 2_usize.pow(MP as u32)],
        Multiplexer<Output<'a>, MP>,
    ),
    /// A pin that is directly connected to the analog source. Must contain a buffer to store the
    /// sample obtained by the ADC.
    Direct([AdcSampleType; 1]),
}

/// A sampler for the analog pins on an nRF MCU. This sampler can handle analog pins that may be
/// multiplexed, or directly wired to the analog source. This can also be used to power an analog
/// keyboard matrix.
pub struct AdcSampler<'a, TIM, PPI0, PPI1, const MP: usize, const C: usize>
where
    [(); C + 1]:,
    [(); 2_usize.pow(MP as u32)]:,
{
    idx_to_pin_type: BlockingMutex<RefCell<[AnalogPinType<'a, MP>; C]>>,
    adc_sampler: Mutex<RawMutex, RawAdcSampler<'a, TIM, PPI0, PPI1, C>>,
}

struct RawAdcSampler<'a, TIM, PPI0, PPI1, const C: usize>
where
    [(); C + 1]:,
{
    adc: Saadc<'a, { C + 1 }>,
    timer: TIM,
    ppi_ch0: PPI0,
    ppi_ch1: PPI1,
}

impl<
        'a,
        TIM: Instance,
        PPI0: ConfigurableChannel,
        PPI1: ConfigurableChannel,
        const MP: usize,
        const C: usize,
    > AdcSampler<'a, TIM, PPI0, PPI1, MP, C>
where
    [(); C + 1]:,
    [(); 2_usize.pow(MP as u32)]:,
{
    /// Create a new instance of the ADC sampler.
    pub fn new(
        idx_to_pin_type: [AnalogPinType<'a, MP>; C],
        configs: [ChannelConfig<'_>; C],
        timer: TIM,
        ppi_ch0: PPI0,
        ppi_ch1: PPI1,
    ) -> Self {
        Self {
            idx_to_pin_type: BlockingMutex::new(RefCell::new(idx_to_pin_type)),
            adc_sampler: unsafe {
                bind_interrupts! {
                    struct Irqs {
                        SAADC => embassy_nrf::saadc::InterruptHandler;
                    }
                }
                embassy_nrf::interrupt::SAADC.set_priority(embassy_nrf::interrupt::Priority::P2);
                let channels = {
                    let mut uninit_arr: [MaybeUninit<ChannelConfig<'_>>; C + 1] =
                        MaybeUninit::uninit().assume_init();

                    let mut bat_ch_config =
                        ChannelConfig::single_ended(VddhDiv5Input.degrade_saadc());
                    bat_ch_config.time = embassy_nrf::saadc::Time::_3US;
                    uninit_arr[0] = MaybeUninit::new(bat_ch_config);

                    for (i, mut config) in configs.into_iter().enumerate() {
                        config.time = embassy_nrf::saadc::Time::_3US;
                        uninit_arr[i + 1] = MaybeUninit::new(config)
                    }

                    uninit_arr.map(|config| config.assume_init())
                };

                Mutex::new(RawAdcSampler {
                    adc: Saadc::new(SAADC::steal(), Irqs, Default::default(), channels),
                    timer,
                    ppi_ch0,
                    ppi_ch1,
                })
            },
        }
    }

    /// Run the sampler. This can only be used by running `adc_task`.
    #[allow(clippy::await_holding_refcell_ref)]
    async fn run_sampler(&self) {
        let mut adc_sampler = self.adc_sampler.lock().await;
        let RawAdcSampler {
            adc,
            timer,
            ppi_ch0,
            ppi_ch1,
            ..
        } = adc_sampler.deref_mut();

        adc.calibrate().await;

        let mut bufs = [[[0; C + 1]; 1]; 2];

        // sample acquisition time: 3 microseconds (based on default saadc::Config)
        // sample conversion time: 2 microseconds (worst case, based on datasheet)
        // 1/(tacq + tconv): 200kHz
        // fsample = 33.333kHz (with sample threshold = 30 and 1MHz timer, buffer starts to get filled every 30 microseconds)
        // NOTE: depending on the number of keys and multiplexers, we need to go through multiple cycles of sampling to obtain all the key samples
        // example: for 80 keys evenly divided among 5 multiplexers (6 channels total when you include VddhDiv5Input), you need to sample 16 times.
        // sampling 6 channels 1 time will take ((3 + 2) microseconds * 6) = 30us.
        // sampling 6 channels 16 times will take 480us, enough for the default matrix polling rate of 500us.
        adc.run_task_sampler(
            timer,
            ppi_ch0,
            ppi_ch1,
            embassy_nrf::timer::Frequency::F1MHz,
            30,
            &mut bufs,
            move |buf| {
                let buf = buf[0];
                self.idx_to_pin_type.lock(|pin_types| {
                    let mut pin_types = pin_types.borrow_mut();
                    BAT_SAMPLE_CHANNEL.signal(buf[0]);
                    for (i, value) in buf.iter().skip(1).enumerate() {
                        match &mut pin_types[i] {
                            AnalogPinType::Multiplexed(values, multiplexer) => {
                                values[multiplexer.cur_channel as usize] = *value;
                                multiplexer
                                    .select_channel(
                                        ((multiplexer.cur_channel as usize + 1)
                                            % 2_usize.pow(MP as u32))
                                            as u8,
                                    )
                                    .unwrap();
                            }
                            AnalogPinType::Direct(values) => {
                                values[0] = *value;
                            }
                        };
                    }
                });

                embassy_nrf::saadc::CallbackResult::Continue
            },
        )
        .await;
    }

    /// Obtain a sample from the ADC. The `ch` argument corresponds to the index of the analog pin
    /// you want to sample (which you provided in the [`Self::new()`] method). If the pin is
    /// multiplexed, the `sub_ch` argument is used to determine which multiplexer channel to sample
    /// from. Otherwise, the `sub_ch` argument is ignored.
    pub fn get_sample(&self, channel: usize, sub_channel: usize) -> Option<u16> {
        self.idx_to_pin_type.lock(|pin_types| {
            pin_types
                .borrow()
                .get(channel)
                .and_then(|ch| match ch {
                    AnalogPinType::Multiplexed(values, _) => values.get(sub_channel).copied(),
                    AnalogPinType::Direct([result]) => Some(*result),
                })
                .map(|value| (value - i16::MIN) as u16)
        })
    }
}

impl<
        'a,
        TIM: Instance,
        PPI0: ConfigurableChannel,
        PPI1: ConfigurableChannel,
        const MP: usize,
        const C: usize,
    > MatrixSampler for AdcSampler<'a, TIM, PPI0, PPI1, MP, C>
where
    [(); C + 1]:,
    [(); 2_usize.pow(MP as u32)]:,
{
    type SampleType = u16;

    fn get_sample(&self, ch: usize, sub_ch: usize) -> Option<Self::SampleType> {
        self.get_sample(ch, sub_ch)
    }
}

static BAT_SAMPLE_CHANNEL: Signal<RawMutex, AdcSampleType> = Signal::new();

#[rumcake_macros::task]
pub async fn adc_task<'a, const MP: usize, const N: usize>(
    sampler: &AdcSampler<
        'a,
        impl Instance,
        impl ConfigurableChannel,
        impl ConfigurableChannel,
        MP,
        N,
    >,
) where
    [(); N + 1]:,
    [(); 2_usize.pow(MP as u32)]:,
{
    let adc_fut = sampler.run_sampler();

    let bat_fut = async {
        loop {
            let sample = BAT_SAMPLE_CHANNEL.wait().await;
            let mv = sample * 5;

            let pct = if mv >= 4200 {
                100
            } else if mv <= 3450 {
                0
            } else {
                (mv * 2 / 15 - 459) as u8
            };

            BATTERY_LEVEL_STATE.set(pct).await;

            Timer::after(Duration::from_secs(10)).await;
        }
    };

    select(adc_fut, bat_fut).await;

    error!("[NRF_ADC] ADC sampler has stopped. This should not happen.");
}

#[cfg(feature = "nrf-ble")]
/// A mutex that is locked when the softdevice is advertising. This is mainly to prevent
/// [`nrf_softdevice::ble::peripheral::ADV_PORTAL`] from being opened by more than one task at the
/// same time.
pub static BLUETOOTH_ADVERTISING_MUTEX: Mutex<RawMutex, ()> = Mutex::new(());

#[cfg(feature = "nrf-ble")]
/// A basic trait that all nRF5x-based devices that use bluetooth features must implement.
pub trait BluetoothDevice {
    const BLUETOOTH_ADDRESS: [u8; 6];
}

#[cfg(feature = "nrf-ble")]
/// Initialize the softdevice. This sets the bluetooth address to the one defined in
/// [`BluetoothDevice::BLUETOOTH_ADDRESS`], and configures the softdevice with some defaults for
/// [`nrf_softdevice::Config`].
pub fn setup_softdevice<K: BluetoothDevice + crate::keyboard::Keyboard>(
) -> &'static mut nrf_softdevice::Softdevice {
    use nrf_softdevice::ble::{set_address, Address, AddressType};

    let config = nrf_softdevice::Config {
        clock: Some(nrf_softdevice::raw::nrf_clock_lf_cfg_t {
            source: nrf_softdevice::raw::NRF_CLOCK_LF_SRC_XTAL as u8,
            rc_ctiv: 0,
            rc_temp_ctiv: 0,
            accuracy: nrf_softdevice::raw::NRF_CLOCK_LF_ACCURACY_20_PPM as u8,
        }),
        gatts_attr_tab_size: Some(nrf_softdevice::raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: 32768,
        }),
        conn_gap: Some(nrf_softdevice::raw::ble_gap_conn_cfg_t {
            conn_count: 6,
            event_length: 24,
        }),
        conn_gatt: Some(nrf_softdevice::raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gap_role_count: Some(nrf_softdevice::raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 4,
            central_role_count: 4,
            central_sec_count: 0,
            _bitfield_1: nrf_softdevice::raw::ble_gap_cfg_role_count_t::new_bitfield_1(0),
        }),
        gap_device_name: Some(nrf_softdevice::raw::ble_gap_cfg_device_name_t {
            p_value: K::PRODUCT.as_ptr() as _,
            current_len: K::PRODUCT.len() as u16,
            max_len: K::PRODUCT.len() as u16,
            write_perm: unsafe { core::mem::zeroed() },
            _bitfield_1: nrf_softdevice::raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                nrf_softdevice::raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };

    let sd = nrf_softdevice::Softdevice::enable(&config);

    set_address(
        sd,
        &Address::new(AddressType::RandomStatic, K::BLUETOOTH_ADDRESS),
    );

    sd
}

#[cfg(feature = "nrf-ble")]
#[rumcake_macros::task]
pub async fn softdevice_task(sd: &'static nrf_softdevice::Softdevice) {
    unsafe {
        nrf_softdevice::raw::sd_power_usbpwrrdy_enable(true as u8);
        nrf_softdevice::raw::sd_power_usbdetected_enable(true as u8);
        nrf_softdevice::raw::sd_power_usbremoved_enable(true as u8);
    }

    let vbus_detect = VBUS_DETECT
        .get_or_init(|| embassy_nrf::usb::vbus_detect::SoftwareVbusDetect::new(true, true));

    sd.run_with_callback(|e| match e {
        nrf_softdevice::SocEvent::PowerUsbPowerReady => {
            vbus_detect.ready();
        }
        nrf_softdevice::SocEvent::PowerUsbDetected => {
            vbus_detect.detected(true);
        }
        nrf_softdevice::SocEvent::PowerUsbRemoved => {
            vbus_detect.detected(false);
        }
        _ => {}
    })
    .await;
}
