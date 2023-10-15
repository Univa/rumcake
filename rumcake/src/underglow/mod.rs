//! Underglow features.
//!
//! To use underglow features, keyboards must implement [`UnderglowDevice`], and the trait
//! corresponding to a driver that implements [`drivers::UnderglowDriver`].

use embassy_futures::select::{self, select, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
use smart_leds::RGB8;

use crate::keyboard::MATRIX_EVENTS;
use crate::{LEDEffect, State};

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

/// State that contains the current configuration for the underglow animator.
pub static UNDERGLOW_CONFIG_STATE: State<UnderglowConfig> =
    State::new(UnderglowConfig::default(), &[]);

#[rumcake_macros::task]
pub async fn underglow_task<D: UnderglowDevice>(
    _k: D,
    driver: impl UnderglowDriver<D, Color = RGB8>,
) where
    [(); D::NUM_LEDS]:,
{
    let mut subscriber = MATRIX_EVENTS.subscriber().unwrap();
    let mut ticker = Ticker::every(Duration::from_millis(1000 / D::FPS as u64));

    // TODO: Get the default from EEPROM if possible
    // This animator has a local copy of the underglow config state so that it doesn't have to lock the config every frame
    let mut animator = UnderglowAnimator::new(UNDERGLOW_CONFIG_STATE.get().await, driver);
    match animator.config.enabled {
        true => animator.turn_on().await,
        false => animator.turn_off().await,
    }
    animator.tick().await; // Force a frame to be rendered in the event that the initial effect is static.

    loop {
        let command = if !(animator.config.enabled && animator.config.effect.is_animated()) {
            // We want to wait for a command if the animator is not rendering any animated effects. This allows the task to sleep when the LEDs are static.
            Some(UNDERGLOW_COMMAND_CHANNEL.receive().await)
        } else {
            match select(ticker.next(), UNDERGLOW_COMMAND_CHANNEL.receive()).await {
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
            // Update the config state, including the animator's own copy, check if it was enabled/disabled
            let toggled = UNDERGLOW_CONFIG_STATE
                .update(|config| {
                    animator.process_command(command);

                    // Process commands until there are no more to process
                    while let Ok(command) = UNDERGLOW_COMMAND_CHANNEL.try_receive() {
                        animator.process_command(command);
                    }

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
                    .send(crate::split::MessageToPeripheral::Underglow(
                        UnderglowCommand::SetTime(animator.tick),
                    ))
                    .await;
                crate::split::central::MESSAGE_TO_PERIPHERALS
                    .send(crate::split::MessageToPeripheral::Underglow(
                        UnderglowCommand::SetConfig(animator.config),
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
