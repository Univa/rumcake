//! Mouse/pointer traits and tasks

use defmt::warn;
use embassy_time::{Duration, Ticker};
use num::Saturating;
use usbd_human_interface_device::device::mouse::WheelMouseReport;

use crate::hw::{HIDDevice, CURRENT_OUTPUT_STATE};

use self::mouse::{MouseButtonFlags, MouseEvent};

pub trait PointingDevice {}

pub trait PointingDriver {
    /// Get events from a pointer device. The implementor is free to wait for an event for an
    /// indefinite amount of time, so that the task can sleep.
    async fn tick(&mut self) -> impl Iterator<Item = MouseEvent>;
}

#[rumcake_macros::task]
pub async fn poll_pointing_device<K: PointingDevice + HIDDevice>(
    _k: K,
    mut driver: impl PointingDriver,
) {
    let mut ticker = Ticker::every(Duration::from_millis(1));
    let mouse_report_channel = K::get_mouse_report_send_channel();
    let mut buttons = MouseButtonFlags::empty();

    loop {
        let events = driver.tick().await;
        let mut x = 0;
        let mut y = 0;
        let mut vertical_wheel = 0;
        let mut horizontal_wheel = 0;

        for e in events {
            match e {
                MouseEvent::Press(bits) => {
                    buttons |= bits;
                }
                MouseEvent::Release(bits) => {
                    buttons &= bits.complement();
                }
                MouseEvent::Movement(new_x, new_y) => {
                    x = x.saturating_add(new_x);
                    y = y.saturating_add(new_y);
                }
                MouseEvent::Scroll(x_amount, y_amount) => {
                    horizontal_wheel = horizontal_wheel.saturating_add(x_amount);
                    vertical_wheel = vertical_wheel.saturating_add(y_amount);
                }
            }
        }

        // Use send instead of try_send to avoid dropped inputs. If USB and Bluetooth are both not
        // connected, this channel can become filled, so we discard the report in that case.
        if CURRENT_OUTPUT_STATE.get().await.is_some() {
            mouse_report_channel
                .send(WheelMouseReport {
                    buttons: buttons.bits(),
                    x,
                    y,
                    vertical_wheel,
                    horizontal_wheel,
                })
                .await;
        } else {
            warn!("[POINTER] Discarding report");
        }

        ticker.next().await;
    }
}

pub mod mouse {
    // TODO: move this logic into its own crate?
    use bitflags::bitflags;

    bitflags! {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct MouseButtonFlags: u8 {
            const LEFT = 0b00000001;
            const RIGHT = 0b00000010;
            const MIDDLE = 0b00000100;
            const BACK = 0b00001000;
            const FORWARD = 0b00010000;
            const BUTTON6 = 0b00100000;
            const BUTTON7 = 0b01000000;
            const BUTTON8 = 0b10000000;
        }
    }

    #[derive(Clone, Copy)]
    pub enum MouseEvent {
        Press(MouseButtonFlags),
        Release(MouseButtonFlags),
        Movement(i8, i8),
        Scroll(i8, i8),
    }
}

pub mod touchpad {
    // TODO: move this logic into its own crate?

    use heapless::Vec;

    use super::mouse::{MouseButtonFlags, MouseEvent};

    pub enum TouchpadEvent {
        /// Tap of a button on a touchpad.
        Tap(MouseButtonFlags),

        /// Holding a button. This should be registered continuously, for as long as the button is
        /// held.
        Hold(MouseButtonFlags),

        /// Touchpad movement.
        Movement(i8, i8),

        /// Scrolling movement on a touchpad, which supports both vertical and horizontal
        /// scrolling.
        Scroll(i8, i8),
    }

    pub struct Touchpad {
        events: Vec<MouseEvent, 4>,
        release_on_next_tick: MouseButtonFlags,
        holding: MouseButtonFlags,
        hold_registered: bool,
    }

    impl Default for Touchpad {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Touchpad {
        pub fn new() -> Self {
            Self {
                events: Vec::new(),
                release_on_next_tick: MouseButtonFlags::empty(),
                holding: MouseButtonFlags::empty(),
                hold_registered: false,
            }
        }

        /// Call this at a regular interval
        pub fn tick(&mut self) {
            // Release held buttons if a hold wasn't registered last tick.
            if !self.hold_registered {
                let _ = self.events.push(MouseEvent::Release(self.holding)).is_ok();
            }

            // Clear existing events
            self.events.clear();
            self.hold_registered = false;

            if !self.release_on_next_tick.is_empty()
                && self
                    .events
                    .push(MouseEvent::Release(self.release_on_next_tick))
                    .is_ok()
            {
                self.release_on_next_tick = MouseButtonFlags::empty();
            }
        }

        pub fn register(&mut self, event: TouchpadEvent) {
            match event {
                TouchpadEvent::Tap(buttons) => {
                    self.release_on_next_tick = buttons;
                    let _ = self.events.push(MouseEvent::Press(buttons));
                }
                TouchpadEvent::Hold(buttons) => {
                    self.hold_registered = true;

                    if buttons != self.holding
                        && self.events.push(MouseEvent::Release(self.holding)).is_ok()
                        && self.events.push(MouseEvent::Press(buttons)).is_ok()
                    {
                        self.holding = buttons;
                    }
                }
                TouchpadEvent::Movement(new_x, new_y) => {
                    let _ = self.events.push(MouseEvent::Movement(new_x, new_y));
                }
                TouchpadEvent::Scroll(new_x, new_y) => {
                    let _ = self.events.push(MouseEvent::Scroll(new_x, new_y));
                }
            }
        }

        pub fn events(&self) -> impl Iterator<Item = MouseEvent> + '_ {
            self.events.iter().copied()
        }
    }
}
