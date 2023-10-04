use bitflags::bitflags;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};

use self::drivers::SimpleBacklightMatrixDriver;
use self::simple_matrix_animations::{
    backlight_effect_items, BacklightAnimator, BacklightCommand, BacklightConfig,
};
use crate::keyboard::{KeyboardMatrix, MATRIX_EVENTS};

pub mod drivers;
// pub mod simple_animations;
pub mod simple_matrix_animations;
pub use simple_matrix_animations as animations;

pub trait BacklightDevice: KeyboardMatrix {
    const FPS: usize = 20;
}

pub trait BacklightMatrixDevice: BacklightDevice
where
    [(); Self::MATRIX_COLS]:,
    [(); Self::MATRIX_ROWS]:,
{
    const LED_LAYOUT: [[Option<(u8, u8)>; Self::MATRIX_COLS]; Self::MATRIX_ROWS];
    const LED_FLAGS: [[LEDFlags; Self::MATRIX_COLS]; Self::MATRIX_ROWS];

    // Effect settings
    backlight_effect_items!();
}

#[macro_export]
macro_rules! led_layout {
    ($([$($no1:ident)* $(($x:literal, $y:literal) $($no2:ident)*)* ])*) => {
        const LED_LAYOUT: [[Option<(u8, u8)>; Self::MATRIX_COLS]; Self::MATRIX_ROWS] = [
            $([
                $(${ignore(no1)} None,)*
                $(Some(($x, $y)), $(${ignore(no2)} None,)*)*
            ]),*
        ];
    };
}

#[derive(Debug)]
struct LayoutBounds {
    max: (u8, u8),
    mid: (u8, u8),
    min: (u8, u8),
}

const fn get_led_layout_bounds<K: BacklightMatrixDevice>() -> LayoutBounds
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    let mut bounds = LayoutBounds {
        max: (0, 0),
        mid: (0, 0),
        min: (255, 255),
    };

    let mut row = 0;
    while row < K::MATRIX_ROWS {
        let mut col = 0;
        while col < K::MATRIX_COLS {
            if let Some((x, y)) = K::LED_LAYOUT[row][col] {
                bounds.min = (
                    if x <= bounds.min.0 { x } else { bounds.min.0 },
                    if y <= bounds.min.1 { y } else { bounds.min.1 },
                );
                bounds.max = (
                    if x >= bounds.max.0 { x } else { bounds.max.0 },
                    if y >= bounds.max.1 { y } else { bounds.max.1 },
                );
            }
            col += 1;
        }
        row += 1;
    }

    bounds.mid.0 = (bounds.max.0 - bounds.min.0) / 2 + bounds.min.0;
    bounds.mid.1 = (bounds.max.1 - bounds.min.1) / 2 + bounds.min.1;

    bounds
}

#[macro_export]
macro_rules! led_flags {
    ([] -> [$($body:tt)*]) => {
        [$($body)*]
    };
    ([No $($rest:tt)*] -> [$($body:tt)*]) => {
        led_flags!([$($rest)*] -> [$($body)* $crate::backlight::LEDFlags::NONE,])
    };
    ([$($flag:ident)|+ $($rest:tt)*] -> [$($body:tt)*]) => {
        led_flags!([$($rest)*] -> [$($body)* $($crate::backlight::LEDFlags::$flag)|+,])
    };
    ($([$($flags:tt)*])*) => {
        const LED_FLAGS: [[$crate::backlight::LEDFlags; Self::MATRIX_COLS]; Self::MATRIX_ROWS] = [
            $(
                led_flags!([$($flags)*] -> [])
            ),*
        ];
    };
}

// Bits used for the flags correspond to QMK's implementation.
bitflags! {
    pub struct LEDFlags: u8 {
        const NONE = 0b00000000;
        const ALPHA = 0b00000001;
        const KEYLIGHT = 0b00000100;
        const INDICATOR = 0b00001000;
    }
}

// Channel for sending and receiving underglow commands.
pub static BACKLIGHT_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, BacklightCommand, 2> =
    Channel::new();

#[cfg(feature = "via")]
pub static BACKLIGHT_STATE: Signal<ThreadModeRawMutex, BacklightConfig> = Signal::new();

#[rumcake_macros::task]
pub async fn backlight_task<D: BacklightMatrixDevice>(
    driver: impl SimpleBacklightMatrixDriver<D> + 'static,
) where
    [(); D::MATRIX_COLS]:,
    [(); D::MATRIX_ROWS]:,
{
    // TODO: Get the default from EEPROM if possible
    let mut animator = BacklightAnimator::new(Default::default(), driver);
    animator.tick().await; // Force a frame to be rendered in the event that the initial effect is static.

    let mut subscriber = MATRIX_EVENTS.subscriber().unwrap();

    let mut ticker = Ticker::every(Duration::from_millis(1000 / D::FPS as u64));

    loop {
        if !animator.is_animated() {
            // We want to wait for a command if the animator is not rendering any animated effects. This allows the task to sleep when the LEDs are static.
            let command = BACKLIGHT_COMMAND_CHANNEL.receive().await;
            animator.process_command(command).await;
        }

        while let Ok(command) = BACKLIGHT_COMMAND_CHANNEL.try_receive() {
            animator.process_command(command).await;
        }

        if let Some(event) = subscriber.try_next_message_pure() {
            animator.register_event(event);
        }

        animator.tick().await;

        ticker.next().await;
    }
}
