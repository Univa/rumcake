//! Basic keyboard traits and tasks.
//!
//! Generally, keyboards will implement [`KeyboardLayout`] and [`KeyboardMatrix`] as needed.
//! Keyboard layouts and matrices are implemented with the help of [TeXitoi's `keyberon` crate](`keyberon`).

use core::convert::Infallible;
use defmt::{debug, info, warn, Debug2Format};
use embassy_sync::channel::Channel;
use embassy_sync::mutex::{Mutex, MutexGuard};
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel};
use embassy_time::{Duration, Ticker, Timer};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use heapless::Vec;
use keyberon::debounce::Debouncer;
use keyberon::layout::{CustomEvent, Event, Layers, Layout as KeyberonLayout};
use keyberon::matrix::{DirectPinMatrix, Matrix};
use usbd_human_interface_device::device::consumer::MultipleConsumerReport;
use usbd_human_interface_device::{
    device::keyboard::NKROBootKeyboardReport, page::Keyboard as KeyboardKeycode,
};

#[cfg(feature = "media-keycodes")]
pub use usbd_human_interface_device::page::Consumer;

use crate::hw::mcu::RawMutex;
use crate::hw::CURRENT_OUTPUT_STATE;

pub use rumcake_macros::{
    build_direct_pin_matrix, build_layout, build_standard_matrix, remap_matrix,
};

/// Basic keyboard trait that must be implemented to use rumcake. Defines basic keyboard information.
pub trait Keyboard {
    /// Manufacturer of your keyboard.
    const MANUFACTURER: &'static str;

    /// Name of your keyboard.
    const PRODUCT: &'static str;

    /// Serial number of your keyboard.
    const SERIAL_NUMBER: &'static str = "1";

    /// Hardware version number for your keyboard.
    const HARDWARE_REVISION: &'static str = "1";

    /// Firmware version number for your keyboard.
    const FIRMWARE_REVISION: &'static str = "1";
}

/// A trait that must be implemented on a device that communicates with the host device.
pub trait KeyboardLayout {
    const NUM_ENCODERS: usize = 0; // Only for VIA compatibility, no proper encoder support. This is the default if not set in QMK

    /// Number of columns in the layout.
    ///
    /// It is recommended to use [`build_layout`] to set this constant.
    const LAYOUT_COLS: usize;

    /// Number of rows in the layout.
    ///
    /// It is recommended to use [`build_layout`] to set this constant.
    const LAYOUT_ROWS: usize;

    /// Number of layers in the layout.
    ///
    /// It is recommended to use [`build_layout`] to set this constant.
    const LAYERS: usize;

    /// Get a reference to the mutex-guarded keyboard layout, which can then be locked to be used
    /// like a normal [`keyberon::layout::Layout`].
    ///
    /// It is recommended to use [`build_layout`] to implement this function.
    fn get_layout(
    ) -> &'static Layout<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, { Self::LAYERS }>;

    /// A function that returns the original layout. This can be used to reset the layout in case
    /// any changes are made to it.
    ///
    /// It is recommended to use [`build_layout`] to implement this function.
    fn get_original_layout(
    ) -> Layers<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, { Self::LAYERS }, Keycode>;

    /// Handle a [`Keycode::Custom`] event. By default this does nothing.
    ///
    /// `press` is set to `true` if the event was a key press. Otherwise, it will be `false`. `id`
    /// corresponds to the `id` used in your keyboard layout.
    fn on_custom_keycode(_id: u8, _press: bool) {}
}

/// A mutex-guaraded [`keyberon::layout::Layout`]. This also stores the original layout, so that it
/// can be reset to it's initial state if modifications are made to it.
pub struct Layout<const C: usize, const R: usize, const L: usize> {
    layout: once_cell::sync::OnceCell<Mutex<RawMutex, KeyberonLayout<C, R, L, Keycode>>>,
}

impl<const C: usize, const R: usize, const L: usize> Layout<C, R, L> {
    pub const fn new() -> Self {
        Self {
            layout: once_cell::sync::OnceCell::new(),
        }
    }

    pub fn init(&self, layers: &'static mut Layers<C, R, L, Keycode>) {
        self.layout
            .get_or_init(|| Mutex::new(KeyberonLayout::new(layers)));
    }

    pub async fn lock(&self) -> MutexGuard<RawMutex, KeyberonLayout<C, R, L, Keycode>> {
        self.layout.get().unwrap().lock().await
    }
}

/// A trait that must be implemented for any device that needs to poll a switch matrix.
pub trait KeyboardMatrix {
    /// Debounce setting.
    const DEBOUNCE_MS: u16 = 5;

    /// Number of matrix columns.
    ///
    /// It is recommended to use one of the `build_*_matrix` macros to set this constant.
    const MATRIX_COLS: usize;

    /// Number of matrix rows.
    ///
    /// It is recommended to use one of the `build_*_matrix` macros to set this constant.
    const MATRIX_ROWS: usize;

    /// Create the keyboard matrix by initializing a set of GPIO pins to use for columns and rows.
    ///
    /// It is recommended to use one of the `build_*_matrix` macros to set this constant.
    fn get_matrix() -> &'static PollableMatrix<impl Pollable>;

    /// Optional function to remap a matrix position to a position on the keyboard layout defined
    /// by [`KeyboardLayout::get_layout`].
    ///
    /// This is useful in split keyboard setups, where all peripherals have a matrix, but only one
    /// of the devices stores the overall keyboard layout.
    fn remap_to_layout(row: u8, col: u8) -> (u8, u8) {
        (row, col)
    }
}

/// Setup a traditional keyboard matrix with diodes, with a debouncer. The output of this function
/// can be passed to the matrix polling task directly.
pub fn setup_standard_keyboard_matrix<
    E,
    I: InputPin<Error = E>,
    O: OutputPin<Error = E>,
    const CS: usize,
    const RS: usize,
>(
    cols: [I; CS],
    rows: [O; RS],
    debounce_ms: u16,
) -> Result<PollableStandardMatrix<I, O, CS, RS>, E> {
    let matrix = Matrix::new(cols, rows)?;
    let debouncer = Debouncer::new([[false; CS]; RS], [[false; CS]; RS], debounce_ms);
    Ok((matrix, debouncer))
}

/// Setup a diodeless keyboard matrix, with a debouncer. The output of this function can be passed
/// to the matrix polling task directly.
pub fn setup_direct_pin_keyboard_matrix<
    E,
    I: InputPin<Error = E>,
    const CS: usize,
    const RS: usize,
>(
    pins: [[Option<I>; CS]; RS],
    debounce_ms: u16,
) -> Result<PollableDirectPinMatrix<I, CS, RS>, E> {
    let matrix = DirectPinMatrix::new(pins)?;
    let debouncer = Debouncer::new([[false; CS]; RS], [[false; CS]; RS], debounce_ms);
    Ok((matrix, debouncer))
}

/// Custom keycodes used to interact with other rumcake features.
///
/// These can be used in your keyboard layout, defined in [`KeyboardLayout::get_layout`]
#[derive(Debug, Clone, Copy)]
pub enum Keycode {
    /// Custom keycode, which can be used to run custom code. You can use
    /// [`KeyboardLayout::on_custom_keycode`] to handle it.
    Custom(u8),

    #[cfg(feature = "media-keycodes")]
    /// Media keycode, which can be any variant in [`usbd_human_interface_device::page::Consumer`]
    Media(usbd_human_interface_device::page::Consumer),

    #[cfg(feature = "underglow")]
    /// Underglow keycode, which can be any variant in [`crate::underglow::animations::UnderglowCommand`]
    Underglow(crate::underglow::animations::UnderglowCommand),

    #[cfg(feature = "simple-backlight")]
    /// Keycode used to control a simple backlight system, which can be any variant in
    /// [`crate::backlight::simple_backlight::animations::BacklightCommand`]
    SimpleBacklight(crate::backlight::simple_backlight::animations::BacklightCommand),

    #[cfg(feature = "simple-backlight-matrix")]
    /// Keycode used to control a simple backlight matrix system, which can be any variant in
    /// [`crate::backlight::simple_backlight_matrix::animations::BacklightCommand`]
    SimpleBacklightMatrix(crate::backlight::simple_backlight_matrix::animations::BacklightCommand),

    #[cfg(feature = "rgb-backlight-matrix")]
    /// Keycode used to control an RGB backlight matrix system, which can be any variant in
    /// [`crate::backlight::rgb_backlight_matrix::animations::BacklightCommand`]
    RGBBacklightMatrix(crate::backlight::rgb_backlight_matrix::animations::BacklightCommand),

    #[cfg(feature = "bluetooth")]
    /// Bluetooth keycode, which can be any variant in [`crate::bluetooth::BluetoothCommand`]
    Bluetooth(crate::bluetooth::BluetoothCommand),
}

pub struct PollableMatrix<T> {
    matrix: Mutex<RawMutex, T>,
}

impl<T: Pollable> PollableMatrix<T> {
    pub const fn new(m: T) -> Self {
        Self {
            matrix: Mutex::new(m),
        }
    }
}

/// Trait that allows you to implement matrix polling functionality. This trait is already
/// implemented for all of the existing [`keyberon::matrix`] structs. You can also implement this
/// trait for your own types to write custom matrix polling logic that can be used with the matrix
/// polling task.
pub trait Pollable {
    /// Poll the matrix for events
    fn events(&mut self) -> impl Iterator<Item = Event>;
}

pub type PollableStandardMatrix<
    I: InputPin<Error = Infallible>,
    O: OutputPin<Error = Infallible>,
    const CS: usize,
    const RS: usize,
> = (Matrix<I, O, CS, RS>, Debouncer<[[bool; CS]; RS]>);

impl<
        I: InputPin<Error = Infallible>,
        O: OutputPin<Error = Infallible>,
        const CS: usize,
        const RS: usize,
    > Pollable for PollableStandardMatrix<I, O, CS, RS>
{
    fn events(&mut self) -> impl Iterator<Item = Event> {
        self.1.events(
            self.0
                .get_with_delay(|| {
                    embassy_time::block_for(Duration::from_ticks(2));
                })
                .unwrap(),
        )
    }
}

pub type PollableDirectPinMatrix<
    I: InputPin<Error = Infallible>,
    const CS: usize,
    const RS: usize,
> = (DirectPinMatrix<I, CS, RS>, Debouncer<[[bool; CS]; RS]>);

impl<I: InputPin<Error = Infallible>, const CS: usize, const RS: usize> Pollable
    for PollableDirectPinMatrix<I, CS, RS>
{
    fn events(&mut self) -> impl Iterator<Item = Event> {
        self.1.events(self.0.get().unwrap())
    }
}

/// Channel with keyboard events polled from the swtich matrix
///
/// The coordinates received will be remapped according to the implementation of
/// [`KeyboardMatrix::remap_to_layout`].
pub(crate) static POLLED_EVENTS_CHANNEL: Channel<RawMutex, Event, 1> = Channel::new();

#[rumcake_macros::task]
pub async fn matrix_poll<K: KeyboardMatrix + 'static>(_k: K) {
    let matrix = K::get_matrix();

    loop {
        {
            debug!("[KEYBOARD] Scanning matrix");
            let mut matrix = matrix.matrix.lock().await;
            let events = matrix.events();
            for e in events {
                let (row, col) = e.coord();
                let (new_row, new_col) = K::remap_to_layout(row, col);

                let remapped_event = match e {
                    Event::Press(_, _) => Event::Press(new_row, new_col),
                    Event::Release(_, _) => Event::Release(new_row, new_col),
                };

                info!(
                    "[KEYBOARD] Key event: {:?}, Remapped: {:?}",
                    Debug2Format(&e),
                    Debug2Format(&remapped_event)
                );

                POLLED_EVENTS_CHANNEL.send(remapped_event).await;
            }
        }
        Timer::after(Duration::from_micros(500)).await;
    }
}

/// A [`PubSubChannel`] used to send matrix events to be consumed by other tasks (e.g. underglow or
/// backlight reactive effects) The coordinates received will be remapped according to the
/// implementation of [`KeyboardMatrix::remap_to_layout`].
///
/// There can be a maximum of 4 subscribers, and the number of subscribers actually used
/// depend on what features you have enabled. With underglow and backlight enabled, 2 subscriber
/// slots will be used.
pub static MATRIX_EVENTS: PubSubChannel<RawMutex, Event, 4, 4, 1> = PubSubChannel::new();

/// Channel for sending NKRO HID keyboard reports.
///
/// Channel messages should be consumed by the bluetooth task or USB task, so user-level code
/// should **not** attempt to receive messages from the channel, otherwise commands may not be
/// processed appropriately. You should only send to this channel.
pub static KEYBOARD_REPORT_HID_SEND_CHANNEL: Channel<RawMutex, NKROBootKeyboardReport, 1> =
    Channel::new();

/// Channel for sending consumer HID reports.
///
/// Channel messages should be consumed by the bluetooth task or USB task, so user-level code
/// should **not** attempt to receive messages from the channel, otherwise commands may not be
/// processed appropriately. You should only send to this channel.
pub static CONSUMER_REPORT_HID_SEND_CHANNEL: Channel<RawMutex, MultipleConsumerReport, 1> =
    Channel::new();

#[rumcake_macros::task]
pub async fn layout_collect<K: KeyboardLayout + 'static>(_k: K)
where
    [(); K::LAYERS]:,
    [(); K::LAYOUT_COLS]:,
    [(); K::LAYOUT_ROWS]:,
{
    let mut last_keys = Vec::<KeyboardKeycode, 24>::new();
    let layout = K::get_layout();

    #[cfg(feature = "media-keycodes")]
    let mut codes = [Consumer::Unassigned; 4];

    let mut ticker = Ticker::every(Duration::from_millis(1));

    loop {
        let keys = {
            let mut layout = layout.lock().await;

            if let Ok(event) = POLLED_EVENTS_CHANNEL.try_receive() {
                layout.event(event);
                MATRIX_EVENTS.publish_immediate(event); // Just immediately publish since we don't want to hold up any key events to be converted into keycodes.
            };

            let tick = layout.tick();

            debug!("[KEYBOARD] Processing rumcake feature keycodes");

            match tick {
                CustomEvent::NoEvent => {}
                CustomEvent::Press(keycode) => match keycode {
                    Keycode::Custom(id) => {
                        K::on_custom_keycode(id, true);
                    }
                    #[cfg(feature = "media-keycodes")]
                    Keycode::Media(keycode) => {
                        if let Some(c) =
                            codes.iter_mut().find(|c| matches!(c, Consumer::Unassigned))
                        {
                            *c = keycode;
                        }
                        CONSUMER_REPORT_HID_SEND_CHANNEL
                            .send(MultipleConsumerReport { codes })
                            .await;
                    }
                    #[cfg(feature = "underglow")]
                    Keycode::Underglow(command) => {
                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                            .send(command)
                            .await;
                        #[cfg(feature = "storage")]
                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                            .send(crate::underglow::animations::UnderglowCommand::SaveConfig)
                            .await;
                    }
                    #[cfg(feature = "simple-backlight")]
                    Keycode::SimpleBacklight(command) => {
                        crate::backlight::simple_backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(command)
                            .await;
                        #[cfg(feature = "storage")]
                        crate::backlight::simple_backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(crate::backlight::simple_backlight::animations::BacklightCommand::SaveConfig)
                            .await;
                    }
                    #[cfg(feature = "simple-backlight-matrix")]
                    Keycode::SimpleBacklightMatrix(command) => {
                        crate::backlight::simple_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
                            .send(command)
                            .await;
                        #[cfg(feature = "storage")]
                        crate::backlight::simple_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
                            .send(crate::backlight::simple_backlight_matrix::animations::BacklightCommand::SaveConfig)
                            .await;
                    }
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Keycode::RGBBacklightMatrix(command) => {
                        crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
                            .send(command)
                            .await;
                        #[cfg(feature = "storage")]
                        crate::backlight::rgb_backlight_matrix::BACKLIGHT_COMMAND_CHANNEL
                            .send(crate::backlight::rgb_backlight_matrix::animations::BacklightCommand::SaveConfig)
                            .await;
                    }
                    #[cfg(feature = "bluetooth")]
                    Keycode::Bluetooth(command) => {
                        crate::bluetooth::BLUETOOTH_COMMAND_CHANNEL
                            .send(command)
                            .await;
                    }
                },
                CustomEvent::Release(keycode) => match keycode {
                    Keycode::Custom(id) => {
                        K::on_custom_keycode(id, false);
                    }
                    #[cfg(feature = "media-keycodes")]
                    Keycode::Media(keycode) => {
                        if let Some(c) = codes.iter_mut().find(|c| **c == keycode) {
                            *c = Consumer::Unassigned;
                        }
                        CONSUMER_REPORT_HID_SEND_CHANNEL
                            .send(MultipleConsumerReport { codes })
                            .await;
                    }
                    #[allow(unreachable_patterns)]
                    _ => {}
                },
            }

            debug!("[KEYBOARD] Collecting keyboard keycodes");

            let keys = layout
                .keycodes()
                .filter_map(|k| KeyboardKeycode::try_from(k as u8).ok())
                .collect::<Vec<KeyboardKeycode, 24>>();

            debug!("[KEYBOARD] Collected {:?}", Debug2Format(&keys));

            keys
        }; // unlock the layout, so that another task can register new layout events

        if last_keys != keys {
            last_keys.clone_from(&keys);

            debug!("[KEYBOARD] Preparing new report");

            // Use send instead of try_send to avoid dropped inputs, which can happen if keycodes
            // update really quickly (this is usually the case for macros). If USB and Bluetooth
            // are both not connected, this channel can become filled, so we discard the report in
            // that case.
            if CURRENT_OUTPUT_STATE.get().await.is_some() {
                KEYBOARD_REPORT_HID_SEND_CHANNEL
                    .send(NKROBootKeyboardReport::new(keys))
                    .await;
            } else {
                warn!("[KEYBOARD] Discarding report");
            }
        }

        ticker.next().await;
    }
}
