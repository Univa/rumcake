//! Underglow features.
//!
//! To use underglow features, keyboards must implement [`UnderglowDevice`], and the trait
//! corresponding to a driver that implements [`drivers::UnderglowDriver`].

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
use smart_leds::RGB8;

use crate::keyboard::MATRIX_EVENTS;

use self::animations::{
    underglow_effect_items, UnderglowAnimator, UnderglowCommand, UnderglowConfig,
};
use self::drivers::UnderglowDriver;

pub mod animations;
pub mod drivers;

/// A trait that keyboards must implement to use the underglow feature.
pub trait UnderglowDevice {
    /// How fast the LEDs refresh to display a new animation frame.
    ///
    /// It is recommended to set this value to a value that your driver can handle,
    /// otherwise your animations will appear to be slowed down.
    ///
    /// **This does not have any effect if the selected animation is static.**
    const FPS: usize = 30;

    /// The number of LEDs used for underglow.
    ///
    /// This number will be used to determine the size of the frame buffer for underglow
    /// animations.
    const NUM_LEDS: usize;

    // Effect settings
    underglow_effect_items!();
}

/// Channel for sending underglow commands.
///
/// Channel messages should be consumed by the [`underglow_task`], so user-level
/// level code should **not** attempt to receive messages from the channel, otherwise
/// commands may not be processed appropriately. You should only send to this channel.
pub static UNDERGLOW_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, UnderglowCommand, 2> =
    Channel::new();

#[cfg(feature = "via")]
pub static UNDERGLOW_STATE: Signal<ThreadModeRawMutex, UnderglowConfig> = Signal::new();

#[rumcake_macros::task]
pub async fn underglow_task<D: UnderglowDevice>(
    _k: D,
    driver: impl UnderglowDriver<D, Color = RGB8>,
) where
    [(); D::NUM_LEDS]:,
{
    // TODO: Get the default from EEPROM if possible
    let mut animator = UnderglowAnimator::new(Default::default(), driver);

    let mut subscriber = MATRIX_EVENTS.subscriber().unwrap();

    let mut ticker = Ticker::every(Duration::from_millis(1000 / D::FPS as u64));

    animator.tick().await; // Force a frame to be rendered in the event that the initial effect is static.

    loop {
        if !animator.is_animated() {
            // We want to wait for a command if the animator is not rendering any animated effects. This allows the task to sleep when the LEDs are static.
            let command = UNDERGLOW_COMMAND_CHANNEL.receive().await;
            animator.process_command(command).await;
        }

        while let Ok(command) = UNDERGLOW_COMMAND_CHANNEL.try_receive() {
            animator.process_command(command).await;
        }

        if let Some(event) = subscriber.try_next_message_pure() {
            animator.register_event(event);
        }

        animator.tick().await;

        ticker.next().await;
    }
}
