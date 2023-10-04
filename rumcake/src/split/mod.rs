use keyberon::layout::Event;
use serde::{Deserialize, Serialize};

pub mod drivers;

#[cfg(feature = "split-central")]
pub mod central;

#[cfg(feature = "split-peripheral")]
pub mod peripheral;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum MessageToCentral {
    KeyPress(u8, u8),
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
pub enum MessageToPeripheral {
    #[cfg(feature = "backlight")]
    Backlight(crate::backlight::animations::BacklightCommand),
    #[cfg(feature = "underglow")]
    Underglow(crate::underglow::animations::UnderglowCommand),
}
