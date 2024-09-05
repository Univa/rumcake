use bitflags::bitflags;
use embassy_futures::select::{select, select3, Either, Either3};
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker};
use keyberon::layout::Event;

use crate::hw::platform::RawMutex;
use crate::keyboard::MATRIX_EVENTS;
use crate::State;

pub use rumcake_macros::{led_flags, led_layout, setup_backlight_matrix};

#[cfg(feature = "rgb-backlight-matrix")]
pub mod rgb_backlight_matrix;
#[cfg(feature = "simple-backlight")]
pub mod simple_backlight;
#[cfg(feature = "simple-backlight-matrix")]
pub mod simple_backlight_matrix;
#[cfg(feature = "underglow")]
pub mod underglow;

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
pub trait BacklightMatrixDevice {
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
    fn get_backlight_matrix() -> BacklightMatrix<{ Self::LIGHTING_COLS }, { Self::LIGHTING_ROWS }>;
}

pub(crate) mod private {
    use super::{BacklightMatrix, BacklightMatrixDevice};

    pub struct EmptyLightingDevice;
    impl BacklightMatrixDevice for EmptyLightingDevice {
        const LIGHTING_COLS: usize = 0;

        const LIGHTING_ROWS: usize = 0;

        fn get_backlight_matrix(
        ) -> BacklightMatrix<{ Self::LIGHTING_COLS }, { Self::LIGHTING_ROWS }> {
            unreachable!("EmptyLightingDevice should not be used with an animator.")
        }
    }

    #[cfg(feature = "simple-backlight")]
    impl super::simple_backlight::private::MaybeSimpleBacklightDevice for EmptyLightingDevice {}
    #[cfg(feature = "simple-backlight-matrix")]
    impl super::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice
        for EmptyLightingDevice
    {
    }
    #[cfg(feature = "rgb-backlight-matrix")]
    impl super::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice for EmptyLightingDevice {}
    #[cfg(feature = "underglow")]
    impl super::underglow::private::MaybeUnderglowDevice for EmptyLightingDevice {}
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

/// Trait which can be used to implement an animator that can be used with the lighting task.
pub trait Animator {
    /// Type used to control the animator.
    type CommandType;

    /// Type used to describe the current state of the animator's configuration
    type ConfigType: Clone + PartialEq;

    /// Type used in the animator's buffer
    type BufferUpdateArgs;

    /// Controls the frame rate of animated effects.
    const FPS: usize;

    /// Initialize the animator if it needs to be. By default, this does not do anything.
    async fn initialize(&mut self) {}

    /// Render a frame using the animator. This method isn't necessarily always called repeatedly.
    /// See [`Animator::is_waiting_for_command`].
    async fn tick(&mut self);

    /// If the animator is not rendering any animated effects, then we can optimize for power usage
    /// by not repeatedly calling the tick method. This should return `true` if the animator can
    /// wait for a command before rendering the next frame. The default implementation always
    /// returns false, causing the lighting task to always repeatedly call the tick method.
    fn is_waiting_for_command(&self) -> bool {
        false
    }

    /// Register matrix events if the animator can generate animations that react to key events.
    fn register_matrix_event(&mut self, event: Event) {}

    /// Process a command to control the animator. The commands that are passed to this method are
    /// usually sourced from another task via the channel provided to the lighting task. For
    /// example, commands can come from Via, or your keyboard layout.
    fn process_command(&mut self, command: Self::CommandType);

    /// Perform some tasks after processing a batch of commands. This is can be used to notify
    /// other tasks about changes to the animator's state. By default this does nothing.
    async fn handle_state_change(&mut self) {}

    /// Update the buffer with the provided arguments. This is used when an additional channel is
    /// provided to the lighting task, which allows other tasks to modify the animator's buffer
    /// directly.
    fn update_buffer(&mut self, args: Self::BufferUpdateArgs) {}

    /// Get a reference to a channel that can receive commands from other tasks to control the
    /// animator.
    fn get_command_channel() -> &'static Channel<RawMutex, Self::CommandType, 2>;

    /// Get a reference to a state object that can be used to notify other tasks about changes to
    /// the animator's configuration.
    fn get_state() -> &'static State<'static, Self::ConfigType>;
}

#[rumcake_macros::task]
pub async fn lighting_task<A: Animator + 'static>(
    mut animator: A,
    buf_channel: Option<&Channel<RawMutex, A::BufferUpdateArgs, 4>>,
) {
    let mut subscriber = MATRIX_EVENTS.subscriber().unwrap();
    let channel = A::get_command_channel();
    let mut ticker = Ticker::every(Duration::from_millis(1000 / A::FPS as u64));

    // Initialize the driver if it needs to be
    animator.initialize().await;

    // Render the first frame. This is usually needed if the animator starts on a static effect
    animator.tick().await;

    loop {
        let command = if animator.is_waiting_for_command() {
            // We want to wait for a command if the animator is not rendering any animated effects. This allows the task to sleep when the LEDs are static.
            Some(channel.receive().await)
        } else if let Some(buf_channel) = buf_channel {
            match select3(ticker.next(), channel.receive(), buf_channel.receive()).await {
                Either3::First(()) => {
                    while let Some(event) = subscriber.try_next_message_pure() {
                        animator.register_matrix_event(event);
                    }

                    None
                }
                Either3::Second(command) => Some(command),
                Either3::Third(args) => {
                    animator.update_buffer(args);
                    continue;
                }
            }
        } else {
            match select(ticker.next(), channel.receive()).await {
                Either::First(()) => {
                    while let Some(event) = subscriber.try_next_message_pure() {
                        animator.register_matrix_event(event);
                    }

                    None
                }
                Either::Second(command) => Some(command),
            }
        };

        if let Some(command) = command {
            animator.process_command(command);

            // Process commands until there are no more to process
            while let Ok(command) = channel.try_receive() {
                animator.process_command(command);
            }

            animator.handle_state_change().await;

            // Ignore any unprocessed matrix events
            while subscriber.try_next_message_pure().is_some() {}

            // Reset the ticker so that it doesn't try to catch up on "missed" ticks.
            ticker.reset();
        }

        animator.tick().await;
    }
}

#[cfg(feature = "storage")]
pub use storage::*;

#[cfg(feature = "storage")]
mod storage {
    use core::any::TypeId;
    use core::fmt::Debug;

    use defmt::{info, warn, Debug2Format};
    use embassy_futures::select;
    use embassy_futures::select::Either;
    use embassy_sync::signal::Signal;
    use embassy_time::Duration;
    use embassy_time::Timer;
    use serde::de::DeserializeOwned;
    use serde::Serialize;

    use crate::hw::platform::RawMutex;
    use crate::storage::StorageKey;
    use crate::storage::{FlashStorage, StorageDevice};

    use super::Animator;

    pub trait AnimatorStorage {
        type Animator: Animator;
        const STORAGE_KEY: StorageKey;
        fn get_state_listener() -> &'static Signal<RawMutex, ()>;
        fn get_save_signal() -> &'static Signal<RawMutex, ()>;
    }

    /// Obtains the lighting config from storage. If it fails to get data, it will return a default
    /// config.
    pub async fn initialize_lighting_data<
        K: StorageDevice + 'static,
        A: AnimatorStorage + 'static,
        F: FlashStorage,
    >(
        _animator_storage: &A,
        database: &crate::storage::StorageService<'_, F, K>,
    ) -> <A::Animator as Animator>::ConfigType
    where
        [(); F::ERASE_SIZE]:,
        <A::Animator as Animator>::ConfigType: core::default::Default + DeserializeOwned + Debug,
    {
        // Check stored animator config metadata (type id) to see if it has changed
        let metadata: [u8; core::mem::size_of::<TypeId>()] =
            unsafe { core::mem::transmute(TypeId::of::<<A::Animator as Animator>::ConfigType>()) };
        let _ = database.check_metadata(A::STORAGE_KEY, &metadata).await;

        // Get animator config from storage
        if let Ok(config) = database.read(A::STORAGE_KEY).await {
            info!(
                "[LIGHTING] Obtained {} from storage: {}",
                Debug2Format(&A::STORAGE_KEY),
                Debug2Format(&config)
            );
            config
        } else {
            warn!(
                "[LIGHTING] Could not get {} from storage, using default config.",
                Debug2Format(&A::STORAGE_KEY),
            );
            Default::default()
        }
    }

    #[rumcake_macros::task]
    pub async fn lighting_storage_task<
        K: StorageDevice,
        F: FlashStorage,
        A: AnimatorStorage + 'static,
    >(
        _animator_storage: A,
        database: &crate::storage::StorageService<'_, F, K>,
    ) where
        <A::Animator as Animator>::ConfigType: Debug + DeserializeOwned + Serialize,
        [(); F::ERASE_SIZE]:,
    {
        let save_signal = A::get_save_signal();
        let config_state = A::Animator::get_state();
        let config_state_listener = A::get_state_listener();

        let save = || async {
            let _ = database
                .write(A::STORAGE_KEY, config_state.get().await)
                .await;
        };

        // Save the animator config if it hasn't been changed in 5 seconds, or if a save was signalled
        loop {
            match select::select(save_signal.wait(), config_state_listener.wait()).await {
                Either::First(_) => {
                    save().await;
                }
                Either::Second(_) => {
                    match select::select(
                        select::select(Timer::after(Duration::from_secs(5)), save_signal.wait()),
                        config_state_listener.wait(),
                    )
                    .await
                    {
                        Either::First(_) => {
                            save().await;
                        }
                        Either::Second(_) => {
                            // Re-signal, so that we skip the `wait()` call at the beginning of this loop
                            config_state_listener.signal(());
                        }
                    }
                }
            };
        }
    }
}
