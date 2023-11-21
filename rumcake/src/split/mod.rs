//! Split keyboard features.

use keyberon::layout::Event;
use serde::{Deserialize, Serialize};

pub mod drivers;

#[cfg(feature = "split-central")]
pub mod central;

#[cfg(feature = "split-peripheral")]
pub mod peripheral;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Possible messages that can be sent to a central device.
pub enum MessageToCentral {
    /// Key press in the form of (row, col).
    KeyPress(u8, u8),
    /// Key release in the form of (row, col).
    KeyRelease(u8, u8),
}

impl From<Event> for MessageToCentral {
    fn from(event: Event) -> Self {
        match event {
            Event::Press(row, col) => MessageToCentral::KeyPress(row, col),
            Event::Release(row, col) => MessageToCentral::KeyRelease(row, col),
        }
    }
}

impl TryFrom<MessageToCentral> for Event {
    type Error = ();

    fn try_from(message: MessageToCentral) -> Result<Self, Self::Error> {
        match message {
            MessageToCentral::KeyPress(row, col) => Ok(Event::Press(row, col)),
            MessageToCentral::KeyRelease(row, col) => Ok(Event::Release(row, col)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
/// Possible messages that can be sent to a peripheral device.
pub enum MessageToPeripheral {
    #[cfg(any(
        feature = "simple-backlight",
        feature = "simple-backlight-matrix",
        feature = "rgb-backlight-matrix"
    ))]
    /// A [`BacklightCommand`](crate::backlight::animations::BacklightCommand) to be processed by the peripheral's backlight animator.
    Backlight(crate::backlight::animations::BacklightCommand),
    #[cfg(feature = "underglow")]
    /// An [`UnderglowCommand`](crate::underglow::animations::UnderglowCommand) to be processed by the peripheral's backlight animator.
    Underglow(crate::underglow::animations::UnderglowCommand),
}
