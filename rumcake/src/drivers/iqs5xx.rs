//! Rumcaker driver implementations for [rwalkr's IQS5xx driver](`iqs5xx`)
//!
//! This provides implementations for [`PointingDriver`](`crate::pointer::PointingDriver`).
//!
//! To use this driver as a pointing device, you must implement [`IQS5xxPointerDriver`], and pass
//! it to [`setup_driver`]. Then the result of this can be passed to the [`poll_pointing_device`]
//! task.

use defmt::Debug2Format;
use embassy_time::Delay;
use embedded_hal::blocking::i2c::{Write, WriteRead};
use embedded_hal::digital::v2::{InputPin, OutputPin};
pub use iqs5xx;
use iqs5xx::{Event, IQS5xx as IQS5xxDriver, Report};

use crate::pointer::mouse::{MouseButtonFlags, MouseEvent};
use crate::pointer::touchpad::{Touchpad, TouchpadEvent};
use crate::pointer::PointingDriver;

struct IQS5xx<E, I2C, RDY, RST> {
    driver: IQS5xxDriver<I2C, RDY, RST>,
    touchpad_state: Touchpad,
    event_handler: E,
}

pub fn setup_driver<E, I2C: Write + WriteRead, RDY: InputPin, RST: OutputPin>(
    i2c: I2C,
    rdy: RDY,
    rst: RST,
    event_handler: E,
) -> IQS5xx<E, I2C, RDY, RST> {
    let mut iqs = IQS5xx {
        driver: IQS5xxDriver::new(i2c, 0, rdy, rst),
        event_handler,
        touchpad_state: Touchpad::new(),
    };
    iqs.driver.reset(&mut Delay).unwrap();
    iqs.driver.poll_ready(&mut Delay).unwrap();
    iqs.driver.init().unwrap();
    iqs
}

pub trait IQS5xxPointerDriver {
    /// This function gets called at a regular interval (usually every millisecond). You can
    /// re-implement this if you the type you're implementing this trait on needs to update its
    /// state over time. This can be useful if you want to implement more complicated touchpad
    /// functionality which isn't already supported by the [`Touchpad`] struct.
    fn tick(&mut self) {}

    fn handle_event(&mut self, state: &mut Touchpad, _report: Report, event: Event) {
        match event {
            iqs5xx::Event::Move { x, y } => {
                state.register(TouchpadEvent::Movement(x as i8, y as i8));
            }
            iqs5xx::Event::SingleTap { x, y } => {
                state.register(TouchpadEvent::Tap(MouseButtonFlags::LEFT));
            }
            iqs5xx::Event::PressHold { x, y } => {
                state.register(TouchpadEvent::Hold(MouseButtonFlags::LEFT));
                state.register(TouchpadEvent::Movement(x as i8, y as i8));
            }
            iqs5xx::Event::TwoFingerTap => {
                state.register(TouchpadEvent::Tap(MouseButtonFlags::RIGHT));
            }
            iqs5xx::Event::Scroll { x, y: _ } if x != 0 => {
                state.register(TouchpadEvent::Scroll(x as i8, 0));
            }
            iqs5xx::Event::Scroll { x: _, y } if y != 0 => {
                state.register(TouchpadEvent::Scroll(0, y as i8));
            }
            _ => {}
        };
    }
}

impl<E: IQS5xxPointerDriver, I2C: Write + WriteRead, RDY, RST> IQS5xx<E, I2C, RDY, RST>
where
    RDY: InputPin,
    RST: OutputPin,
{
    async fn tick(&mut self) -> impl Iterator<Item = MouseEvent> + '_ {
        self.touchpad_state.tick();
        self.event_handler.tick();

        let report = self.driver.try_transact(|driver| driver.get_report());

        match report {
            Ok(Some(report)) => {
                let event = iqs5xx::Event::from(&report);
                self.event_handler
                    .handle_event(&mut self.touchpad_state, report, event);
            }
            Err(error) => {
                defmt::warn!(
                    "[IQS5XX_DRIVER] Could not get report: {}",
                    Debug2Format(&error)
                );
            }
            _ => {}
        }

        self.touchpad_state.events()
    }
}

impl<E: IQS5xxPointerDriver, I2C: Write + WriteRead, RDY, RST> PointingDriver
    for IQS5xx<E, I2C, RDY, RST>
where
    RDY: InputPin,
    RST: OutputPin,
{
    async fn tick(&mut self) -> impl Iterator<Item = MouseEvent> {
        self.tick().await
    }
}
