//! Backlighting features.
//!
//! To use backlighting features, keyboards must implement [`BacklightDevice`]
//! (and optionally [`BacklightMatrixDevice`], if a backlight matrix is desired),
//! along with the trait corresponding to a driver that implements one of
//! [`drivers::SimpleBacklightDriver`], [`drivers::SimpleBacklightMatrixDriver`] or
//! [`drivers::RGBBacklightMatrixDriver`], depending on the desired type of backlighting.

#[cfg(any(
    all(feature = "simple-backlight", feature = "simple-backlight-matrix"),
    all(feature = "simple-backlight", feature = "rgb-backlight-matrix"),
    all(feature = "simple-backlight-matrix", feature = "rgb-backlight-matrix")
))]
compile_error!("Exactly one of `simple-backlight`, `simple-backlight-matrix`, `rgb-backlight-matrix` must be enabled at a time. Please choose the one that you want to use.");

use bitflags::bitflags;
use embassy_futures::select;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker};

use crate::keyboard::MATRIX_EVENTS;
use crate::{LEDEffect, State};

pub mod drivers;

#[cfg(feature = "simple-backlight")]
use drivers::SimpleBacklightDriver as DriverType;
#[cfg(feature = "simple-backlight")]
pub mod simple_animations;
#[cfg(feature = "simple-backlight")]
pub use simple_animations as animations;

#[cfg(feature = "simple-backlight-matrix")]
use drivers::SimpleBacklightMatrixDriver as DriverType;
#[cfg(feature = "simple-backlight-matrix")]
pub mod simple_matrix_animations;
#[cfg(feature = "simple-backlight-matrix")]
pub use simple_matrix_animations as animations;

#[cfg(feature = "rgb-backlight-matrix")]
use drivers::RGBBacklightMatrixDriver as DriverType;
#[cfg(feature = "rgb-backlight-matrix")]
pub mod rgb_matrix_animations;
#[cfg(feature = "rgb-backlight-matrix")]
pub use rgb_matrix_animations as animations;

#[cfg(any(
    feature = "simple-backlight",
    feature = "simple-backlight-matrix",
    feature = "rgb-backlight-matrix"
))]
use self::animations::{
    backlight_effect_items, BacklightAnimator, BacklightCommand, BacklightConfig,
};

/// A trait that keyboards must implement to use backlight features.
pub trait BacklightDevice {
    /// How fast the LEDs refresh to display a new animation frame.
    ///
    /// It is recommended to set this value to a value that your driver can handle,
    /// otherwise your animations will appear to be slowed down.
    ///
    /// **This does not have any effect if the selected animation is static.**
    const FPS: usize = 20;

    #[cfg(any(
        feature = "simple-backlight",
        feature = "simple-backlight-matrix",
        feature = "rgb-backlight-matrix"
    ))]
    backlight_effect_items!();
}

/// Struct that contains information about a lighting matrix of a given size. Includes information
/// about the physical layout of the LEDs, and the flags for each LED.
pub struct BacklightMatrix<const C: usize, const R: usize> {
    /// The **physical** position of each LED on your keyboard.
    ///
    /// A given X or Y coordinate value must fall between 0-255. If any matrix
    /// positions are unused, you can use `None`. It is recommended to use the
    /// [`led_layout`] macro to set this constant.
    pub layout: [[Option<(u8, u8)>; C]; R],

    /// The flags of each LED on your keyboard.
    ///
    /// You can use any combination of [LEDFlags] for each LED. It is recommended
    /// to use the [`led_flags`] macro to set this value.
    pub flags: [[LEDFlags; C]; R],
}

impl<const C: usize, const R: usize> BacklightMatrix<C, R> {
    /// Create a new backlight matrix with the given LED information.
    pub const fn new(layout: [[Option<(u8, u8)>; C]; R], flags: [[LEDFlags; C]; R]) -> Self {
        Self { layout, flags }
    }
}

/// An additional trait that keyboards must implement to use a backlight matrix.
pub trait BacklightMatrixDevice: BacklightDevice {
    /// The number of columns in your lighting matrix
    ///
    /// It is recommended to use the [`setup_backlight_matrix`] macro to set this value.
    const LIGHTING_COLS: usize;

    /// The number of rows in your lighting matrix
    ///
    /// It is recommended to use the [`setup_backlight_matrix`] macro to set this value.
    const LIGHTING_ROWS: usize;

    /// Function to return a reference to the [`BacklightMatrix`], containing information about
    /// physical LED position, and LED flags. It is recommended to use the
    /// [`setup_backlight_matrix`] macro to set this value.
    fn get_backlight_matrix(
    ) -> &'static BacklightMatrix<{ Self::LIGHTING_COLS }, { Self::LIGHTING_ROWS }>;
}

#[doc(hidden)]
pub struct EmptyBacklightMatrix;
impl crate::backlight::BacklightDevice for EmptyBacklightMatrix {}
impl crate::backlight::BacklightMatrixDevice for EmptyBacklightMatrix {
    const LIGHTING_COLS: usize = 0;
    const LIGHTING_ROWS: usize = 0;
    fn get_backlight_matrix(
    ) -> &'static crate::backlight::BacklightMatrix<{ Self::LIGHTING_COLS }, { Self::LIGHTING_ROWS }>
    {
        static EMPTY_BACKLIGHT_MATRIX: crate::backlight::BacklightMatrix<0, 0> =
            crate::backlight::BacklightMatrix::new([], []);
        &EMPTY_BACKLIGHT_MATRIX
    }
}

#[macro_export]
macro_rules! setup_backlight_matrix {
    ($rows:literal, $cols:literal, ({$($led_layout:tt)*} {$($led_flags:tt)*})) => {
        fn get_backlight_matrix(
        ) -> &'static $crate::backlight::BacklightMatrix<{ Self::LIGHTING_COLS }, { Self::LIGHTING_ROWS }>
        {
            static EMPTY_BACKLIGHT_MATRIX: $crate::backlight::BacklightMatrix<$cols, $rows> =
                $crate::backlight::BacklightMatrix::new($crate::led_layout!($($led_layout)*), $crate::led_flags!($($led_flags)*));
            &EMPTY_BACKLIGHT_MATRIX
        }
    };
    // We count the number of keys in the first row to determine the number of columns
    ($rows:literal, ({[$($first_row_leds:tt)*] $([$($led:tt)*])*} $led_flags:tt)) => {
        const LIGHTING_COLS: usize = ${count(first_row_leds)};
        setup_backlight_matrix!($rows, ${count(first_row_leds)}, ({[$($first_row_leds)*] $([$($led)*])*} $led_flags));
    };
    // Count the number of "[]" inside the "{}" to determine the number of rows
    ({$($rows:tt)*} $led_flags:tt) => {
        const LIGHTING_ROWS: usize = ${count(rows)};
        setup_backlight_matrix!(${count(rows)}, ({$($rows)*} $led_flags));
    };
}

#[macro_export]
macro_rules! led_layout {
    ($([$($no1:ident)* $(($x:literal, $y:literal) $($no2:ident)*)* ])*) => {
        [
            $([
                $(${ignore(no1)} None,)*
                $(Some(($x, $y)), $(${ignore(no2)} None,)*)*
            ]),*
        ]
    };
}

#[derive(Debug)]
struct LayoutBounds {
    max: (u8, u8),
    mid: (u8, u8),
    min: (u8, u8),
}

fn get_led_layout_bounds<K: BacklightMatrixDevice + 'static>() -> LayoutBounds
where
    [(); K::LIGHTING_COLS]:,
    [(); K::LIGHTING_ROWS]:,
{
    let mut bounds = LayoutBounds {
        max: (0, 0),
        mid: (0, 0),
        min: (255, 255),
    };

    let mut row = 0;
    while row < K::LIGHTING_ROWS {
        let mut col = 0;
        while col < K::LIGHTING_COLS {
            if let Some((x, y)) = K::get_backlight_matrix().layout[row][col] {
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
        $crate::led_flags!([$($rest)*] -> [$($body)* $crate::backlight::LEDFlags::NONE,])
    };
    ([$($flag:ident)|+ $($rest:tt)*] -> [$($body:tt)*]) => {
        $crate::led_flags!([$($rest)*] -> [$($body)* $($crate::backlight::LEDFlags::$flag)|+,])
    };
    ($([$($flags:tt)*])*) => {
        [
            $(
                $crate::led_flags!([$($flags)*] -> [])
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

#[cfg(any(
    feature = "simple-backlight",
    feature = "simple-backlight-matrix",
    feature = "rgb-backlight-matrix"
))]
/// Channel for sending backlight commands.
///
/// Channel messages should be consumed by the [`backlight_task`], so user-level
/// level code should **not** attempt to receive messages from the channel, otherwise
/// commands may not be processed appropriately. You should only send to this channel.
pub static BACKLIGHT_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, BacklightCommand, 2> =
    Channel::new();

#[cfg(any(
    feature = "simple-backlight",
    feature = "simple-backlight-matrix",
    feature = "rgb-backlight-matrix"
))]
/// State that contains the current configuration for the backlight animator.
pub static BACKLIGHT_CONFIG_STATE: State<BacklightConfig> = State::new(
    BacklightConfig::default(),
    &[
        #[cfg(feature = "storage")]
        &storage::BACKLIGHT_CONFIG_STATE_LISTENER,
    ],
);

#[cfg(feature = "simple-backlight")]
use BacklightDevice as DeviceTrait;
#[cfg(any(feature = "simple-backlight-matrix", feature = "rgb-backlight-matrix"))]
use BacklightMatrixDevice as DeviceTrait;

#[cfg(any(
    feature = "simple-backlight",
    feature = "simple-backlight-matrix",
    feature = "rgb-backlight-matrix"
))]
#[rumcake_macros::task]
pub async fn backlight_task<D: DeviceTrait + 'static>(_k: D, driver: impl DriverType<D> + 'static)
where
    [(); D::LIGHTING_COLS]:,
    [(); D::LIGHTING_ROWS]:,
{
    let mut subscriber = MATRIX_EVENTS.subscriber().unwrap();
    let mut ticker = Ticker::every(Duration::from_millis(1000 / D::FPS as u64));

    // The animator has a local copy of the backlight config state so that it doesn't have to lock the config every frame
    let mut animator = BacklightAnimator::new(BACKLIGHT_CONFIG_STATE.get().await, driver);
    match animator.config.enabled {
        true => animator.turn_on().await,
        false => animator.turn_off().await,
    }
    animator.tick().await; // Force a frame to be rendered in the event that the initial effect is static.

    loop {
        let command = if !(animator.config.enabled && animator.config.effect.is_animated()) {
            // We want to wait for a command if the animator is not rendering any animated effects. This allows the task to sleep when the LEDs are static.
            Some(BACKLIGHT_COMMAND_CHANNEL.receive().await)
        } else {
            #[cfg(not(all(feature = "vial", feature = "rgb-backlight-matrix")))]
            match select::select(ticker.next(), BACKLIGHT_COMMAND_CHANNEL.receive()).await {
                select::Either::First(()) => {
                    while let Some(event) = subscriber.try_next_message_pure() {
                        if animator.config.enabled && animator.config.effect.is_reactive() {
                            animator.register_event(event);
                        }
                    }

                    None
                }
                select::Either::Second(command) => Some(command),
            }

            #[cfg(all(feature = "vial", feature = "rgb-backlight-matrix"))]
            match select::select3(
                ticker.next(),
                BACKLIGHT_COMMAND_CHANNEL.receive(),
                crate::vial::VIAL_DIRECT_SET_CHANNEL.receive(),
            )
            .await
            {
                select::Either3::First(()) => {
                    while let Some(event) = subscriber.try_next_message_pure() {
                        if animator.config.enabled && animator.config.effect.is_reactive() {
                            animator.register_event(event);
                        }
                    }

                    None
                }
                select::Either3::Second(command) => Some(command),
                select::Either3::Third((led, color)) => {
                    let col = led as usize % D::LIGHTING_COLS;
                    let row = led as usize / D::LIGHTING_COLS % D::LIGHTING_ROWS;
                    animator.buf[row][col] = color;
                    continue;
                }
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
}

#[cfg(all(
    feature = "storage",
    any(
        feature = "simple-backlight",
        feature = "simple-backlight-matrix",
        feature = "rgb-backlight-matrix"
    )
))]
pub mod storage {
    use core::any::TypeId;

    use defmt::{info, warn, Debug2Format};
    use embassy_futures::select;
    use embassy_futures::select::Either;
    use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
    use embassy_sync::signal::Signal;
    use embassy_time::Duration;
    use embassy_time::Timer;
    use embedded_storage_async::nor_flash::NorFlash;
    use postcard::experimental::max_size::MaxSize;

    use super::BacklightConfig;
    use super::BACKLIGHT_CONFIG_STATE;

    pub(super) static BACKLIGHT_CONFIG_STATE_LISTENER: Signal<ThreadModeRawMutex, ()> =
        Signal::new();

    static mut BACKLIGHT_STORAGE_STATE: crate::storage::StorageServiceState<
        { core::mem::size_of::<TypeId>() },
        { BacklightConfig::POSTCARD_MAX_SIZE },
    > = crate::storage::StorageServiceState::new();

    pub(super) static BACKLIGHT_SAVE_SIGNAL: Signal<ThreadModeRawMutex, ()> = Signal::new();

    #[rumcake_macros::task]
    pub async fn backlight_storage_task<F: NorFlash>(
        database: &'static crate::storage::Database<'static, F>,
    ) where
        [(); F::ERASE_SIZE]:,
    {
        {
            let mut database = database.lock().await;

            // Check stored backlight config metadata (type id) to see if it has changed
            let metadata: [u8; core::mem::size_of::<TypeId>()] =
                unsafe { core::mem::transmute(TypeId::of::<BacklightConfig>()) };
            let _ = database
                .initialize(
                    unsafe { &mut BACKLIGHT_STORAGE_STATE },
                    crate::storage::StorageKey::BacklightConfig,
                    &metadata,
                )
                .await;

            // Get backlight config from storage
            if let Ok(config) = database
                .read(
                    unsafe { &mut BACKLIGHT_STORAGE_STATE },
                    crate::storage::StorageKey::BacklightConfig,
                )
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
        }

        let save = || async {
            let _ = database
                .lock()
                .await
                .write(
                    unsafe { &mut BACKLIGHT_STORAGE_STATE },
                    crate::storage::StorageKey::BacklightConfig,
                    BACKLIGHT_CONFIG_STATE.get().await,
                )
                .await;
        };

        // Save the backlight config if it hasn't been changed in 5 seconds, or if a save was signalled
        loop {
            match select::select(
                BACKLIGHT_SAVE_SIGNAL.wait(),
                BACKLIGHT_CONFIG_STATE_LISTENER.wait(),
            )
            .await
            {
                Either::First(_) => {
                    save().await;
                }
                Either::Second(_) => {
                    match select::select(
                        select::select(
                            Timer::after(Duration::from_secs(5)),
                            BACKLIGHT_SAVE_SIGNAL.wait(),
                        ),
                        BACKLIGHT_CONFIG_STATE_LISTENER.wait(),
                    )
                    .await
                    {
                        Either::First(_) => {
                            save().await;
                        }
                        Either::Second(_) => {
                            // Re-signal, so that we skip the `wait()` call at the beginning of this loop
                            BACKLIGHT_CONFIG_STATE_LISTENER.signal(());
                        }
                    }
                }
            };
        }
    }
}
