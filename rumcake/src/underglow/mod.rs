//! Underglow features.
//!
//! To use underglow features, keyboards must implement [`UnderglowDevice`], and the trait
//! corresponding to a driver that implements [`drivers::UnderglowDriver`].

use defmt::{info, warn, Debug2Format};
use embassy_futures::join;
use embassy_futures::select::{self, select, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker, Timer};
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
pub static UNDERGLOW_CONFIG_STATE: State<UnderglowConfig> = State::new(
    UnderglowConfig::default(),
    &[&UNDERGLOW_CONFIG_STATE_LISTENER],
);

static UNDERGLOW_CONFIG_STATE_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();

#[cfg(feature = "storage")]
/// Channel for sending requests to save/read underglow configuration to a storage peripheral
pub static UNDERGLOW_CONFIG_STORAGE_SERVICE: crate::storage::StorageService<
    UnderglowConfig,
    { crate::storage::StorageKey::UnderglowConfig as u8 },
    4,
> = crate::storage::StorageService::new();

#[cfg(feature = "storage")]
static UNDERGLOW_CONFIG_STORAGE_CLIENT: crate::storage::StorageClient<
    UnderglowConfig,
    { crate::storage::StorageKey::UnderglowConfig as u8 },
    4,
> = UNDERGLOW_CONFIG_STORAGE_SERVICE.client();

#[rumcake_macros::task]
pub async fn underglow_task<D: UnderglowDevice>(
    _k: D,
    driver: impl UnderglowDriver<D, Color = RGB8>,
) where
    [(); D::NUM_LEDS]:,
{
    let mut subscriber = MATRIX_EVENTS.subscriber().unwrap();
    let mut ticker = Ticker::every(Duration::from_millis(1000 / D::FPS as u64));

    // Get underglow config from storage
    #[cfg(feature = "storage")]
    if let crate::storage::StorageResponse::Read(Ok(config)) = UNDERGLOW_CONFIG_STORAGE_CLIENT
        .request(crate::storage::StorageRequest::Read)
        .await
    {
        info!(
            "[UNDERGLOW] Obtained underglow config from storage: {}",
            Debug2Format(&config)
        );
        // Quietly update the config state so that we don't save the config to storage again
        UNDERGLOW_CONFIG_STATE.quiet_set(config).await;
    } else {
        warn!("[UNDERGLOW] Could not get underglow config from storage, using default config.",);
    }

    // This animator has a local copy of the underglow config state so that it doesn't have to lock the config every frame
    let mut animator = UnderglowAnimator::new(UNDERGLOW_CONFIG_STATE.get().await, driver);
    match animator.config.enabled {
        true => animator.turn_on().await,
        false => animator.turn_off().await,
    }
    animator.tick().await; // Force a frame to be rendered in the event that the initial effect is static.

    let render_fut = async {
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
                animator.process_command(command).await;

                // Process commands until there are no more to process
                while let Ok(command) = UNDERGLOW_COMMAND_CHANNEL.try_receive() {
                    animator.process_command(command).await;
                }

                // Update the config state, after updating the animator's own copy, and check if it was enabled/disabled
                let toggled = UNDERGLOW_CONFIG_STATE
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
    };

    let storage_fut = async {
        // Save the underglow config if it hasn't been changed in 5 seconds.
        loop {
            UNDERGLOW_CONFIG_STATE_LISTENER.wait().await;

            match select::select(
                Timer::after(Duration::from_secs(5)),
                UNDERGLOW_CONFIG_STATE_LISTENER.wait(),
            )
            .await
            {
                Either::First(_) => {
                    #[cfg(feature = "storage")]
                    UNDERGLOW_CONFIG_STORAGE_CLIENT
                        .request(crate::storage::StorageRequest::Write(
                            UNDERGLOW_CONFIG_STATE.get().await,
                        ))
                        .await;
                }
                Either::Second(_) => {
                    // Re-signal, so that we skip the `wait()` call at the beginning of this loop
                    UNDERGLOW_CONFIG_STATE_LISTENER.signal(());
                }
            }
        }
    };

    join::join(render_fut, storage_fut).await;
}
