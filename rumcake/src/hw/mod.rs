//! Utilities for interfacing with hardware.

#[cfg(all(not(feature = "stm32"), not(feature = "nrf"), not(feature = "rp")))]
compile_error!("Please enable the appropriate feature flag for the chip you're using.");

#[cfg(any(
    all(feature = "stm32", feature = "nrf"),
    all(feature = "nrf", feature = "rp"),
    all(feature = "rp", feature = "stm32")
))]
compile_error!("Please enable only one chip feature flag.");

#[cfg_attr(feature = "stm32", path = "mcu/stm32.rs")]
#[cfg_attr(feature = "nrf", path = "mcu/nrf.rs")]
#[cfg_attr(feature = "rp", path = "mcu/rp.rs")]
pub mod mcu;

use crate::hw::mcu::jump_to_bootloader;
use crate::State;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr::read_volatile;
use core::ptr::write_volatile;
use embassy_futures::select;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use embedded_hal::digital::v2::OutputPin;

use mcu::RawMutex;

/// State that contains the current battery level. `rumcake` may or may not use this
/// static internally, depending on what MCU is being used. The contents of this state
/// is usually set by a task in the [`mcu`] module. For example, on nRF5x-based MCUs,
/// this is controlled by a task called `adc_task`.
pub static BATTERY_LEVEL_STATE: State<u8> = State::new(
    100,
    &[
        #[cfg(feature = "display")]
        &crate::display::BATTERY_LEVEL_LISTENER,
        #[cfg(feature = "bluetooth")]
        &crate::bluetooth::BATTERY_LEVEL_LISTENER,
    ],
);

/// Possible settings used to determine how the firmware will choose the destination for HID
/// reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Usb,
    Bluetooth,
}

/// State that contains the desired output mode. This configures how the firmware will decide to
/// send HID reports. This doesn't not represent the actual destination of HID reports. Use
/// [`CURRENT_OUTPUT_STATE`] for that.
pub static OUTPUT_MODE_STATE: State<OutputMode> = State::new(
    if cfg!(feature = "bluetooth") {
        OutputMode::Bluetooth
    } else {
        OutputMode::Usb
    },
    &[
        &OUTPUT_MODE_STATE_LISTENER,
        #[cfg(feature = "display")]
        &crate::display::OUTPUT_MODE_STATE_LISTENER,
    ],
);

/// Possible destinations for HID reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HIDOutput {
    Usb,
    Bluetooth,
}

/// State that contains the current destination of HID reports.
pub static CURRENT_OUTPUT_STATE: State<Option<HIDOutput>> = State::new(
    None,
    &[
        #[cfg(feature = "usb")]
        &crate::usb::KB_CURRENT_OUTPUT_STATE_LISTENER,
        #[cfg(feature = "usb")]
        &crate::usb::CONSUMER_CURRENT_OUTPUT_STATE_LISTENER,
        #[cfg(all(feature = "usb", feature = "via"))]
        &crate::usb::VIA_CURRENT_OUTPUT_STATE_LISTENER,
        #[cfg(feature = "bluetooth")]
        &crate::bluetooth::CURRENT_OUTPUT_STATE_LISTENER,
    ],
);

pub(crate) static OUTPUT_MODE_STATE_LISTENER: Signal<RawMutex, ()> = Signal::new();
pub(crate) static USB_RUNNING_STATE_LISTENER: Signal<RawMutex, ()> = Signal::new();
pub(crate) static BLUETOOTH_CONNECTED_STATE_LISTENER: Signal<RawMutex, ()> = Signal::new();

#[rumcake_macros::task]
pub async fn output_switcher() {
    // This task doesn't need to run if only one of USB or Bluetooth is enabled.
    loop {
        let output = match OUTPUT_MODE_STATE.get().await {
            #[cfg(feature = "usb")]
            OutputMode::Usb => {
                if crate::usb::USB_RUNNING_STATE.get().await {
                    Some(HIDOutput::Usb)
                } else {
                    None
                }
            }
            #[cfg(feature = "bluetooth")]
            OutputMode::Bluetooth => {
                if crate::bluetooth::BLUETOOTH_CONNECTED_STATE.get().await {
                    Some(HIDOutput::Bluetooth)
                } else {
                    None
                }
            }
            #[allow(unreachable_patterns)]
            _ => None,
        };

        CURRENT_OUTPUT_STATE.set(output).await;
        defmt::info!("[HW] Output updated: {:?}", defmt::Debug2Format(&output));

        // Wait for a change in state before attempting to update the output again.
        select::select3(
            USB_RUNNING_STATE_LISTENER.wait(),
            BLUETOOTH_CONNECTED_STATE_LISTENER.wait(),
            OUTPUT_MODE_STATE_LISTENER.wait(),
        )
        .await;
    }
}

const BOOTLOADER_MAGIC: u32 = 0xDEADBEEF;

#[link_section = ".uninit.FLAG"]
static mut FLAG: UnsafeCell<MaybeUninit<u32>> = UnsafeCell::new(MaybeUninit::uninit());

pub async unsafe fn check_double_tap_bootloader(timeout: u64) {
    if read_volatile(FLAG.get().cast::<u32>()) == BOOTLOADER_MAGIC {
        write_volatile(FLAG.get().cast(), 0);

        jump_to_bootloader();
    }

    write_volatile(FLAG.get().cast(), BOOTLOADER_MAGIC);

    Timer::after_millis(timeout).await;

    write_volatile(FLAG.get().cast(), 0);
}

extern "C" {
    /// This static value will have an address equal to the `__config_start` address in your
    /// `memory.x` file. You must set this, along with [`__config_end`], if you're using on-chip
    /// flash with the storage task (which is the default). Keep in mind that the start and end
    /// address must be relative to the address of your chip's flash. For example, on STM32F072CBx,
    /// flash memory is located at `0x08000000`, so if you want your config data to start at
    /// `0x08100000`, your start address must be `0x00100000`.
    pub static __config_start: u32;
    /// This static value will have an address equal to the `__config_end` address in your
    /// `memory.x` file. If you want to know what value to set this to in `memory.x`, take
    /// [`__config_start`], and add the size of your config section, in bytes.
    pub static __config_end: u32;
}

pub struct Multiplexer<T, const P: usize> {
    cur_channel: u8,
    pins: [Option<T>; P],
    en: Option<T>,
}

impl<E, T: OutputPin<Error = E>, const P: usize> Multiplexer<T, P> {
    pub fn new(pins: [Option<T>; P], en: Option<T>) -> Self {
        let mut multiplexer = Self {
            pins,
            en,
            cur_channel: 0,
        };
        let _ = multiplexer.select_channel(0);
        multiplexer
    }

    pub fn select_channel(&mut self, mut channel: u8) -> Result<(), E> {
        for i in 0..P {
            if let Some(ref mut pin) = self.pins[i] {
                if channel & 0x01 == 0x01 {
                    pin.set_high()?;
                } else {
                    pin.set_low()?;
                }
            }
            channel >>= 1;
        }
        self.cur_channel = channel;
        Ok(())
    }

    pub fn enable(&mut self) -> Result<(), E> {
        if let Some(ref mut en) = self.en {
            en.set_low()?;
        }
        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), E> {
        if let Some(ref mut en) = self.en {
            en.set_high()?;
        }
        Ok(())
    }
}
