//! Split keyboard features.

use keyberon::layout::Event;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::pointer::mouse::{MouseButtonFlags, MouseEvent};

#[cfg(feature = "split-central")]
pub mod central;

#[cfg(feature = "split-peripheral")]
pub mod peripheral;

/// Possible messages that can be sent to a central device.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, MaxSize)]
#[repr(u8)]
pub enum MessageToCentral {
    /// Key press in the form of (row, col).
    KeyPress(u8, u8),
    /// Key release in the form of (row, col).
    KeyRelease(u8, u8),
    /// Mouse movement in the direction of (x, y).
    MouseMovement(i8, i8),
    /// Mouse buttons that have been pressed in the current tick.
    MousePress(MouseButtonFlags),
    /// Mouse buttons that have been released in the current tick.
    MouseRelease(MouseButtonFlags),
    /// Scrolling in the direction of (x, y).
    MouseScroll(i8, i8),
}

/// Size of buffer used when sending messages to a central device
pub const MESSAGE_TO_CENTRAL_BUFFER_SIZE: usize = MessageToCentral::POSTCARD_MAX_SIZE + 3;

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
            _ => Err(()),
        }
    }
}

impl From<MouseEvent> for MessageToCentral {
    fn from(value: MouseEvent) -> Self {
        match value {
            MouseEvent::Press(buttons) => MessageToCentral::MousePress(buttons),
            MouseEvent::Release(buttons) => MessageToCentral::MouseRelease(buttons),
            MouseEvent::Movement(x, y) => MessageToCentral::MouseMovement(x, y),
            MouseEvent::Scroll(x, y) => MessageToCentral::MouseScroll(x, y),
        }
    }
}

impl TryFrom<MessageToCentral> for MouseEvent {
    type Error = ();

    fn try_from(value: MessageToCentral) -> Result<Self, Self::Error> {
        match value {
            MessageToCentral::MouseMovement(x, y) => Ok(MouseEvent::Movement(x, y)),
            MessageToCentral::MousePress(buttons) => Ok(MouseEvent::Press(buttons)),
            MessageToCentral::MouseRelease(buttons) => Ok(MouseEvent::Release(buttons)),
            MessageToCentral::MouseScroll(x, y) => Ok(MouseEvent::Scroll(x, y)),
            _ => Err(()),
        }
    }
}

/// Possible messages that can be sent to a peripheral device.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, MaxSize)]
#[non_exhaustive]
#[cfg_attr(
    any(
        feature = "simple-backlight",
        feature = "simple-backlight-matrix",
        feature = "rgb-backlight-matrix",
        feature = "underglow",
    ),
    repr(u8)
)]
pub enum MessageToPeripheral {
    #[cfg(feature = "simple-backlight")]
    /// A [`SimpleBacklightCommand`](crate::lighting::simple_backlight::SimpleBacklightCommand) to
    /// be processed by the peripheral's simple backlight animator.
    SimpleBacklight(crate::lighting::simple_backlight::SimpleBacklightCommand) = 3,

    #[cfg(feature = "simple-backlight-matrix")]
    /// A
    /// [`SimpleBacklightMatrixCommand`](crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand)
    /// to be processed by the peripheral's simple backlight matrix animator.
    SimpleBacklightMatrix(crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand) =
        4,

    #[cfg(feature = "rgb-backlight-matrix")]
    /// A
    /// [`RGBBacklightMatrixCommand`](crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand)
    /// to be processed by the peripheral's RGB backlight matrix animator.
    RGBBacklightMatrix(crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand) = 5,

    #[cfg(feature = "underglow")]
    /// An [`UnderglowCommand`](crate::lighting::underglow::UnderglowCommand) to be processed by the peripheral's backlight animator.
    Underglow(crate::lighting::underglow::UnderglowCommand) = 6,
}

/// Size of buffer used when sending messages to a peripheral device
pub const MESSAGE_TO_PERIPHERAL_BUFFER_SIZE: usize = MessageToPeripheral::POSTCARD_MAX_SIZE + 3;
