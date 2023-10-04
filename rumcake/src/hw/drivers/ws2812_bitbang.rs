// TODO: move into its own repo?
// Driver for RGB LEDs that use the WS2812's protocol. Driver implementation uses embassy's timer driver, and applies gamma correction.

#[cfg(any(feature = "stm32", feature = "nrf"))]
use core::arch::arm::__nop as nop;

use crate::hw::mcu::SYSCLK;
use embassy_time::Duration;
use embedded_hal::digital::v2::OutputPin;
use smart_leds::RGB8;

#[macro_export]
macro_rules! ws2812_pin {
    ($p:ident) => {
        fn ws2812_pin() -> impl $crate::embedded_hal::digital::v2::OutputPin {
            $crate::output_pin!($p)
        }
    };
}

// in nanoseconds, taken from WS2812 datasheet
const T0H: u64 = 350;
const T0L: u64 = 900;

const T1H: u64 = 900;
const T1L: u64 = 350;

// in microseconds
const RES: u64 = 280;

const fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

const NOP_FUDGE: f64 = 0.6;

const TICK_CONV_FACTOR: f64 = (SYSCLK as u64 / gcd(SYSCLK as u64, 1_000_000_000)) as f64
    / (1_000_000_000 / gcd(SYSCLK as u64, 1_000_000_000)) as f64;

pub struct Ws2812<P: OutputPin> {
    pin: P,
}

impl<P: OutputPin> Ws2812<P> {
    pub fn new(mut pin: P) -> Ws2812<P> {
        pin.set_low().ok();
        Self { pin }
    }

    #[inline(always)]
    pub fn write_byte(&mut self, mut data: u8) {
        for _ in 0..8 {
            if data & 0x80 == 0x80 {
                self.pin.set_high().ok();
                for _ in 0..(T1H as f64 * TICK_CONV_FACTOR * NOP_FUDGE) as i32 {
                    unsafe {
                        nop();
                    }
                }
                self.pin.set_low().ok();
                for _ in 0..(T1L as f64 * TICK_CONV_FACTOR * NOP_FUDGE) as i32 {
                    unsafe {
                        nop();
                    }
                }
            } else {
                self.pin.set_high().ok();
                for _ in 0..(T0H as f64 * TICK_CONV_FACTOR * NOP_FUDGE) as i32 {
                    unsafe {
                        nop();
                    }
                }
                self.pin.set_low().ok();
                for _ in 0..(T0L as f64 * TICK_CONV_FACTOR * NOP_FUDGE) as i32 {
                    unsafe {
                        nop();
                    }
                }
            };
            data <<= 1;
        }
    }

    pub fn write_colors(&mut self, colors: impl Iterator<Item = RGB8>) {
        for color in colors {
            self.write_byte(color.g);
            self.write_byte(color.r);
            self.write_byte(color.b);
        }

        // Reset time
        // Technically this isn't needed as long as the user sets a reasonable FPS value, but we'll keep it anyways.
        embassy_time::block_for(Duration::from_micros(RES));
    }
}
