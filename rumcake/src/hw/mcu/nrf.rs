use embassy_nrf::bind_interrupts;
use embassy_nrf::interrupt::{InterruptExt, Priority};
use embassy_nrf::nvmc::Nvmc;
use embassy_nrf::peripherals::SAADC;
use embassy_nrf::saadc::{ChannelConfig, Input, Saadc, VddhDiv5Input};
use embassy_nrf::usb::Driver;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_storage::nor_flash::{ErrorType, NorFlash, ReadNorFlash};
use embedded_storage_async::nor_flash::{
    NorFlash as AsyncNorFlash, ReadNorFlash as AsyncReadNorFlash,
};
use static_cell::StaticCell;

use crate::hw::BATTERY_LEVEL_STATE;

#[cfg(feature = "nrf-ble")]
pub use nrf_softdevice;

#[cfg(feature = "nrf52840")]
pub const SYSCLK: u32 = 48_000_000;

pub fn jump_to_bootloader() {
    // TODO
}

pub fn initialize_rcc() {
    let mut conf = embassy_nrf::config::Config::default();
    conf.time_interrupt_priority = Priority::P2;
    embassy_nrf::init(conf);
}

#[macro_export]
macro_rules! input_pin {
    ($p:ident) => {
        unsafe {
            $crate::embassy_nrf::gpio::Input::new(
                $crate::embassy_nrf::gpio::Pin::degrade(
                    $crate::embassy_nrf::peripherals::$p::steal(),
                ),
                $crate::embassy_nrf::gpio::Pull::Up,
            )
        }
    };
}

#[macro_export]
macro_rules! output_pin {
    ($p:ident) => {
        unsafe {
            $crate::embassy_nrf::gpio::Output::new(
                $crate::embassy_nrf::gpio::Pin::degrade(
                    $crate::embassy_nrf::peripherals::$p::steal(),
                ),
                $crate::embassy_nrf::gpio::Level::High,
                $crate::embassy_nrf::gpio::OutputDrive::Standard,
            )
        }
    };
}

#[cfg(feature = "nrf-ble")]
static VBUS_DETECT: once_cell::sync::OnceCell<embassy_nrf::usb::vbus_detect::SoftwareVbusDetect> =
    once_cell::sync::OnceCell::new();

#[cfg(feature = "usb")]
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
        static CONTROL_BUF: StaticCell<[u8; 128]> = StaticCell::new();
        let control_buf = CONTROL_BUF.init([0; 128]);

        embassy_usb::Builder::new(
            usb_driver,
            config,
            device_descriptor,
            config_descriptor,
            bos_descriptor,
            control_buf,
        )
    }
}

pub struct Flash {
    flash: Nvmc<'static>,
}

impl ErrorType for Flash {
    type Error = embassy_nrf::nvmc::Error;
}

impl AsyncReadNorFlash for Flash {
    const READ_SIZE: usize = <Nvmc as ReadNorFlash>::READ_SIZE;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.flash.read(offset, bytes)
    }

    fn capacity(&self) -> usize {
        self.flash.capacity()
    }
}

impl AsyncNorFlash for Flash {
    const WRITE_SIZE: usize = <Nvmc as embedded_storage::nor_flash::NorFlash>::WRITE_SIZE;

    const ERASE_SIZE: usize = <Nvmc as embedded_storage::nor_flash::NorFlash>::ERASE_SIZE;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.flash.erase(from, to)
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.flash.write(offset, bytes)
    }
}

pub fn setup_internal_flash() -> Flash {
    Flash {
        flash: unsafe { Nvmc::new(embassy_nrf::peripherals::NVMC::steal()) },
    }
}

pub fn setup_internal_softdevice_flash(sd: &nrf_softdevice::Softdevice) -> nrf_softdevice::Flash {
    nrf_softdevice::Flash::take(sd)
}

#[rumcake_macros::task]
pub async fn adc_task() {
    let mut adc = unsafe {
        bind_interrupts! {
            struct Irqs {
                SAADC => embassy_nrf::saadc::InterruptHandler;
            }
        }
        embassy_nrf::interrupt::SAADC.set_priority(embassy_nrf::interrupt::Priority::P2);
        let vddh = VddhDiv5Input;
        let channel = ChannelConfig::single_ended(vddh.degrade_saadc());
        Saadc::new(SAADC::steal(), Irqs, Default::default(), [channel])
    };

    adc.calibrate().await;

    loop {
        let mut buf: [i16; 1] = [0; 1];
        adc.sample(&mut buf).await;

        let sample = &buf[0];
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
}

#[macro_export]
macro_rules! setup_i2c_inner {
    ($interrupt:ident, $i2c:ident, $sda:ident, $scl:ident) => {
        {
            use $crate::embassy_nrf::interrupt::InterruptExt;
            unsafe {
                $crate::embassy_nrf::bind_interrupts! {
                    struct Irqs {
                        $interrupt => $crate::embassy_nrf::twim::InterruptHandler<$crate::embassy_nrf::peripherals::$i2c>;
                    }
                };
                $crate::embassy_nrf::interrupt::$interrupt.set_priority($crate::embassy_nrf::interrupt::Priority::P2);
                let i2c = $crate::embassy_nrf::peripherals::$i2c::steal();
                let sda = $crate::embassy_nrf::peripherals::$sda::steal();
                let scl = $crate::embassy_nrf::peripherals::$scl::steal();
                $crate::embassy_nrf::twim::Twim::new(i2c, Irqs, sda, scl, Default::default())
            }
        }
    };
}

#[macro_export]
macro_rules! setup_i2c {
    ($interrupt:ident, $i2c:ident, $sda:ident, $scl:ident) => {
        fn setup_i2c() -> impl $crate::embedded_hal_async::i2c::I2c<Error = impl core::fmt::Debug> {
            $crate::setup_i2c_inner!($interrupt, $i2c, $sda, $scl)
        }
    };
}

#[macro_export]
macro_rules! setup_i2c_blocking {
    ($interrupt:ident, $i2c:ident, $sda:ident, $scl:ident) => {
        fn setup_i2c(
        ) -> impl $crate::embedded_hal::blocking::i2c::Write<Error = impl core::fmt::Debug> {
            $crate::setup_i2c_inner!($interrupt, $i2c, $sda, $scl)
        }
    };
}

#[cfg(feature = "nrf-ble")]
pub static BLUETOOTH_ADVERTISING_MUTEX: Mutex<ThreadModeRawMutex, ()> = Mutex::new(());

#[cfg(feature = "nrf-ble")]
pub trait BluetoothDevice {
    const BLUETOOTH_ADDRESS: [u8; 6];
}

#[cfg(feature = "nrf-ble")]
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

    let vbus_detect = VBUS_DETECT.get().unwrap();

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
