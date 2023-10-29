//! Backlighting features.
//!
//! To use backlighting features, keyboards must implement [`BacklightDevice`]
//! (and optionally [`BacklightMatrixDevice`], if a backlight matrix is desired),
//! along with the trait corresponding to a driver that implements one of
//! [`drivers::SimpleBacklightDriver`], [`drivers::SimpleBacklightMatrixDriver`] or
//! [`drivers::RGBBacklightMatrixDriver`], depending on the desired type of backlighting.

use bitflags::bitflags;
use defmt::{info, warn, Debug2Format};
use embassy_futures::join;
use embassy_futures::select::{self, select, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker, Timer};

use crate::keyboard::{KeyboardMatrix, MATRIX_EVENTS};
use crate::{LEDEffect, State};

pub mod drivers;
pub mod simple_animations;
pub mod simple_matrix_animations;

// use drivers::SimpleBacklightDriver as DriverType;
// pub use simple_animations as animations;
use drivers::SimpleBacklightMatrixDriver as DriverType;
pub use simple_matrix_animations as animations;

use self::animations::{
    backlight_effect_items, BacklightAnimator, BacklightCommand, BacklightConfig,
};

/// A trait that keyboards must implement to use backlight features.
pub trait BacklightDevice: KeyboardMatrix {
    /// How fast the LEDs refresh to display a new animation frame.
    ///
    /// It is recommended to set this value to a value that your driver can handle,
    /// otherwise your animations will appear to be slowed down.
    ///
    /// **This does not have any effect if the selected animation is static.**
    const FPS: usize = 20;

    // Effect settings
    backlight_effect_items!();
}

/// An additional trait that keyboards must implement to use a backlight matrix.
pub trait BacklightMatrixDevice: BacklightDevice
where
    [(); Self::MATRIX_COLS]:,
    [(); Self::MATRIX_ROWS]:,
{
    /// The **physical** position of each LED on your keyboard.
    ///
    /// It is assumed that the LED matrix is the same size as the switch matrix, so
    /// [`KeyboardMatrix::MATRIX_COLS`], and [`KeyboardMatrix::MATRIX_ROWS`], will determine the
    /// size of the frame buffer used for LED matrix animations.
    ///
    /// A given X or Y coordinate value must fall between 0-255. If any matrix
    /// positions are unused, you can use `None`. It is recommended to use the
    /// [led_layout] macro to set this constant.
    const LED_LAYOUT: [[Option<(u8, u8)>; Self::MATRIX_COLS]; Self::MATRIX_ROWS];

    /// The flags of each LED on your keyboard.
    ///
    /// You can use any combination of [LEDFlags] for each LED. It is recommended
    /// to use the [led_flags] macro to set this constant.
    const LED_FLAGS: [[LEDFlags; Self::MATRIX_COLS]; Self::MATRIX_ROWS];
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

bitflags! {
    /// Flags used to mark the purpose of an LED in a backlight matrix.
    ///
    /// Bits used for the flags correspond to QMK's implementation.
    pub struct LEDFlags: u8 {
        const NONE = 0b00000000;
        const ALPHA = 0b00000001;
        const KEYLIGHT = 0b00000100;
        const INDICATOR = 0b00001000;
    }
}

/// Channel for sending backlight commands.
///
/// Channel messages should be consumed by the [`backlight_task`], so user-level
/// level code should **not** attempt to receive messages from the channel, otherwise
/// commands may not be processed appropriately. You should only send to this channel.
pub static BACKLIGHT_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, BacklightCommand, 2> =
    Channel::new();

/// State that contains the current configuration for the backlight animator.
pub static BACKLIGHT_CONFIG_STATE: State<BacklightConfig> = State::new(
    BacklightConfig::default(),
    &[&BACKLIGHT_CONFIG_STATE_LISTENER],
);

static BACKLIGHT_CONFIG_STATE_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();

#[cfg(feature = "storage")]
/// Service that receives requests to read, write or delete [`BacklightConfig`] data from a storage
/// peripheral.
pub static BACKLIGHT_CONFIG_STORAGE_SERVICE: crate::storage::StorageService<
    BacklightConfig,
    { crate::storage::StorageKey::BacklightConfig as u8 },
    4,
> = crate::storage::StorageService::new();

#[cfg(feature = "storage")]
static BACKLIGHT_CONFIG_STORAGE_CLIENT: crate::storage::StorageClient<
    BacklightConfig,
    { crate::storage::StorageKey::BacklightConfig as u8 },
    4,
> = BACKLIGHT_CONFIG_STORAGE_SERVICE.client();

// use BacklightDevice as DeviceTrait;
use BacklightMatrixDevice as DeviceTrait;

#[rumcake_macros::task]
pub async fn backlight_task<D: DeviceTrait>(_k: D, driver: impl DriverType<D> + 'static)
where
    [(); D::MATRIX_COLS]:,
    [(); D::MATRIX_ROWS]:,
{
    let mut subscriber = MATRIX_EVENTS.subscriber().unwrap();
    let mut ticker = Ticker::every(Duration::from_millis(1000 / D::FPS as u64));

    // Get backlight config from storage
    #[cfg(feature = "storage")]
    if let crate::storage::StorageResponse::Read(Ok(config)) = BACKLIGHT_CONFIG_STORAGE_CLIENT
        .request(crate::storage::StorageRequest::Read)
        .await
    {
        info!(
            "[BACKLIGHT] Obtained backlight config from storage: {}",
            Debug2Format(&config)
        );
        // Quietly update the config state so that we don't save the config to storage again
        BACKLIGHT_CONFIG_STATE.quiet_set(config).await;
    } else {
        warn!("[BACKLIGHT] Could not get backlight config from storage, using default config.",);
    };

    // The animator has a local copy of the backlight config state so that it doesn't have to lock the config every frame
    let mut animator = BacklightAnimator::new(BACKLIGHT_CONFIG_STATE.get().await, driver);
    match animator.config.enabled {
        true => animator.turn_on().await,
        false => animator.turn_off().await,
    }
    animator.tick().await; // Force a frame to be rendered in the event that the initial effect is static.

    let render_fut = async {
        loop {
            let command = if !(animator.config.enabled && animator.config.effect.is_animated()) {
                // We want to wait for a command if the animator is not rendering any animated effects. This allows the task to sleep when the LEDs are static.
                Some(BACKLIGHT_COMMAND_CHANNEL.receive().await)
            } else {
                match select(ticker.next(), BACKLIGHT_COMMAND_CHANNEL.receive()).await {
                    Either::First(()) => {
                        while let Some(event) = subscriber.try_next_message_pure() {
                            if animator.config.enabled && animator.config.effect.is_reactive() {
                                animator.register_event(event);
                            }
                        }

                        None
                    }
                    Either::Second(command) => Some(command),
                }
            };

            // Process the command if one was received, otherwise continue to render
            if let Some(command) = command {
                animator.process_command(command).await;

                // Process commands until there are no more to process
                while let Ok(command) = BACKLIGHT_COMMAND_CHANNEL.try_receive() {
                    animator.process_command(command).await;
                }

                // Update the config state, after updating the animator's own copy, and check if it was enabled/disabled
                let toggled = BACKLIGHT_CONFIG_STATE
                    .update(|config| {
                        let toggled = config.enabled != animator.config.enabled;
                        **config = animator.config;
                        toggled
                    })
                    .await;

                if toggled {
                    match animator.config.enabled {
                        true => animator.turn_on().await,
                        false => animator.turn_off().await,
                    }
                }

                // Send commands to be consumed by the split peripherals
                #[cfg(feature = "split-central")]
                {
                    crate::split::central::MESSAGE_TO_PERIPHERALS
                        .send(crate::split::MessageToPeripheral::Backlight(
                            BacklightCommand::SetTime(animator.tick),
                        ))
                        .await;
                    crate::split::central::MESSAGE_TO_PERIPHERALS
                        .send(crate::split::MessageToPeripheral::Backlight(
                            BacklightCommand::SetConfig(animator.config),
                        ))
                        .await;
                }

                // Ignore any unprocessed matrix events
                while subscriber.try_next_message_pure().is_some() {}

                // Reset the ticker so that it doesn't try to catch up on "missed" ticks.
                ticker.reset();
            }

            animator.tick().await;
        }
    };

    let storage_fut = async {
        // Save the backlight config if it hasn't been changed in 5 seconds.
        loop {
            BACKLIGHT_CONFIG_STATE_LISTENER.wait().await;

            match select::select(
                Timer::after(Duration::from_secs(5)),
                BACKLIGHT_CONFIG_STATE_LISTENER.wait(),
            )
            .await
            {
                Either::First(_) => {
                    #[cfg(feature = "storage")]
                    BACKLIGHT_CONFIG_STORAGE_CLIENT
                        .request(crate::storage::StorageRequest::Write(
                            BACKLIGHT_CONFIG_STATE.get().await,
                        ))
                        .await;
                }
                Either::Second(_) => {
                    // Re-signal, so that we skip the `wait()` call at the beginning of this loop
                    BACKLIGHT_CONFIG_STATE_LISTENER.signal(());
                }
            }
        }
    };

    join::join(render_fut, storage_fut).await;
}
