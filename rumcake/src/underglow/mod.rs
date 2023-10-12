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

pub trait UnderglowDevice {
    const FPS: usize = 30;
    const NUM_LEDS: usize;

    // Effect settings
    underglow_effect_items!();
}

// Channel for sending and receiving underglow commands.
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
