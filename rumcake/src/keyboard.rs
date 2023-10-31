//! Basic keyboard traits and tasks.
//!
//! Generally, keyboards will implement [`KeyboardLayout`] and [`KeyboardMatrix`] as needed.
//! Keyboard layouts and matrices are implemented with the help of [TeXitoi's `keyberon` crate](`keyberon`).

use core::convert::Infallible;
use defmt::{debug, info, warn, Debug2Format};
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use heapless::Vec;
use keyberon::debounce::Debouncer;
use keyberon::layout::{CustomEvent, Event, Layers, Layout};
use keyberon::matrix::Matrix;
use usbd_human_interface_device::{
    device::keyboard::NKROBootKeyboardReport, page::Keyboard as KeyboardKeycode,
};

#[macro_export]
macro_rules! remap_matrix {
    ({$([$(#$no1:tt)* $($og_pos:ident $(#$no2:tt)*)* ])*} {$([$($new_pos:ident)*])*}) => {
        macro_rules! remap {
            ($$macro:ident! { $$({$([$($$$new_pos:tt)*])*})* }) => {
                $$macro! {
                    $$({
                        $([
                            $($no1)*
                            $($$$og_pos $($no2)*)*
                        ])*
                    })*
                }
            };
            ($$macro:ident! { $([$($$$new_pos:tt)*])* }) => {
                $$macro! {
                    $([
                        $($no1)*
                        $($$$og_pos $($no2)*)*
                    ])*
                }
            };
        }
    };
}

/// Basic keyboard trait that must be implemented to use rumcake. Defines basic keyboard information.
pub trait Keyboard {
    const MANUFACTURER: &'static str;
    const PRODUCT: &'static str;
    const SERIAL_NUMBER: &'static str = "1";
    const HARDWARE_REVISION: &'static str = "1";
    const FIRMWARE_REVISION: &'static str = "1";
}

/// A trait that must be implemented on a device that communicates with the host device.
pub trait KeyboardLayout {
    const NUM_ENCODERS: u8 = 0; // Only for VIA compatibility, no proper encoder support. This is the default if not set in QMK

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

    /// Create the default keyboard layout.
    ///
    /// It is recommended to use [`build_layout`] to implement this function.
    fn build_layout(
    ) -> &'static Layers<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, { Self::LAYERS }, Keycode>;
}

#[macro_export]
macro_rules! build_layout {
    // Pass the layers to the keyberon macro
    ($layers:literal, $rows:literal, $cols:literal, ($($l:tt)*)) => {
        fn build_layout() -> &'static $crate::keyberon::layout::Layers<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, { Self::LAYERS }, $crate::keyboard::Keycode> {
            static LAYERS: $crate::keyberon::layout::Layers<$cols, $rows, $layers, $crate::keyboard::Keycode> = $crate::keyberon::layout::layout! { $($l)* };
            &LAYERS
        }
    };
    // We count the number of keys in the first row to determine the number of columns
    ($layers:literal, $rows:literal, ({[$($first_row_keys:tt)*] $([$($key:tt)*])*} $($rest:tt)*)) => {
        const LAYOUT_COLS: usize = ${count(first_row_keys)};
        build_layout!($layers, $rows, ${count(first_row_keys)}, ({[$($first_row_keys)*] $([$($key)*])*} $($rest)*));
    };
    // Count the number of "[]" inside the "{}" to determine the number of rows
    ($layers:literal, ({$($rows:tt)*} $($rest:tt)*)) => {
        const LAYOUT_ROWS: usize = ${count(rows)};
        build_layout!($layers, ${count(rows)}, ({$($rows)*} $($rest)*));
    };
    // Count the number of "{}" to determine the number of layers
    ($($layers:tt)*) => {
        const LAYERS: usize = ${count(layers)};
        build_layout!(${count(layers)}, ($($layers)*));
    };
}

pub fn setup_keyboard_layout<K: KeyboardLayout>(
    _k: K,
) -> Layout<{ K::LAYOUT_COLS }, { K::LAYOUT_ROWS }, { K::LAYERS }, Keycode>
where
    [(); K::LAYOUT_COLS]:,
    [(); K::LAYOUT_ROWS]:,
    [(); K::LAYERS]:,
{
    Layout::new(K::build_layout())
}

/// A trait that must be implemented for any device that needs to poll a switch matrix.
pub trait KeyboardMatrix {
    /// Debounce setting.
    const DEBOUNCE_MS: u16 = 5;

    /// Number of matrix columns.
    ///
    /// It is recommended to use [`build_matrix`] to set this constant.
    const MATRIX_COLS: usize;

    /// Number of matrix rows.
    ///
    /// It is recommended to use [`build_matrix`] to set this constant.
    const MATRIX_ROWS: usize;

    /// Create the keyboard matrix by initializing a set of GPIO pins to use for columns and rows.
    ///
    /// It is recommended to use [`build_matrix`] to implement this function.
    fn build_matrix() -> Result<
        Matrix<
            impl InputPin<Error = Infallible>,
            impl OutputPin<Error = Infallible>,
            { Self::MATRIX_COLS },
            { Self::MATRIX_ROWS },
        >,
        Infallible,
    >;

    /// Optional function to remap a matrix position to a position on the keyboard layout defined by [`KeyboardLayout::build_layout`].
    ///
    /// This is useful in split keyboard setups, where all peripherals have a matrix, but only one
    /// of the devices stores the overall keyboard layout.
    fn remap_to_layout(row: u8, col: u8) -> (u8, u8) {
        (row, col)
    }
}

#[macro_export]
macro_rules! build_matrix {
    ({$($r:ident)*} {$($c:ident)*}) => {
        const MATRIX_ROWS: usize = ${count(r)};
        const MATRIX_COLS: usize = ${count(c)};

        fn build_matrix(
        ) -> Result<$crate::keyberon::matrix::Matrix<impl $crate::embedded_hal::digital::v2::InputPin<Error = core::convert::Infallible>, impl $crate::embedded_hal::digital::v2::OutputPin<Error = core::convert::Infallible>, { Self::MATRIX_COLS }, { Self::MATRIX_ROWS }>, core::convert::Infallible> {
            $crate::keyberon::matrix::Matrix::new([
                $(
                    $crate::input_pin!($c),
                )*
            ], [
                $(
                    $crate::output_pin!($r),
                )*
            ])
        }
    }
}

pub fn setup_keyboard_matrix<K: KeyboardMatrix>(
    _k: K,
) -> (
    Matrix<
        impl InputPin<Error = Infallible>,
        impl OutputPin<Error = Infallible>,
        { K::MATRIX_COLS },
        { K::MATRIX_ROWS },
    >,
    Debouncer<[[bool; K::MATRIX_COLS]; K::MATRIX_ROWS]>,
)
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    let matrix = K::build_matrix().unwrap();
    let debouncer = Debouncer::new(
        [[false; K::MATRIX_COLS]; K::MATRIX_ROWS],
        [[false; K::MATRIX_COLS]; K::MATRIX_ROWS],
        K::DEBOUNCE_MS,
    );
    (matrix, debouncer)
}

/// Custom keycodes used to interact with other rumcake features.
///
/// These can be used in your keyboard layout, defined in [`KeyboardLayout::build_layout`]
#[derive(Debug, Clone, Copy)]
pub enum Keycode {
    #[cfg(feature = "underglow")]
    /// Underglow keycode, which can be any variant in [`crate::underglow::animations::UnderglowCommand`]
    Underglow(crate::underglow::animations::UnderglowCommand),
    #[cfg(feature = "backlight")]
    /// Backlight keycode, which can be any variant in [`crate::backlight::animations::BacklightCommand`]
    Backlight(crate::backlight::animations::BacklightCommand),
    #[cfg(feature = "bluetooth")]
    /// Bluetooth keycode, which can be any variant in [`crate::bluetooth::BluetoothCommand`]
    Bluetooth(crate::bluetooth::BluetoothCommand),
}

/// Channel with keyboard events polled from the swtich matrix
///
/// The coordinates received will be remapped according to the implementation of
/// [`KeyboardMatrix::remap_to_layout`].
pub(crate) static POLLED_EVENTS_CHANNEL: Channel<ThreadModeRawMutex, Event, 1> = Channel::new();

#[rumcake_macros::task]
pub async fn matrix_poll<K: KeyboardMatrix>(
    _k: K,
    mut matrix: Matrix<
        impl InputPin<Error = Infallible>,
        impl OutputPin<Error = Infallible>,
        { K::MATRIX_COLS },
        { K::MATRIX_ROWS },
    >,
    mut debouncer: Debouncer<[[bool; K::MATRIX_COLS]; K::MATRIX_ROWS]>,
) where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    loop {
        {
            debug!("[KEYBOARD] Scanning matrix");
            let events = debouncer.events(
                matrix
                    .get_with_delay(|| {
                        embassy_time::block_for(Duration::from_ticks(2));
                    })
                    .unwrap(),
            );
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
pub static MATRIX_EVENTS: PubSubChannel<ThreadModeRawMutex, Event, 4, 4, 1> = PubSubChannel::new();

/// Channel for sending NKRO HID keyboard reports.
///
/// Channel messages should be consumed by the bluetooth task or USB task, so user-level code
/// should **not** attempt to receive messages from the channel, otherwise commands may not be
/// processed appropriately. You should only send to this channel.
pub static KEYBOARD_REPORT_HID_SEND_CHANNEL: Channel<
    ThreadModeRawMutex,
    NKROBootKeyboardReport,
    1,
> = Channel::new();

#[rumcake_macros::task]
pub async fn layout_collect<K: KeyboardLayout>(
    _k: K,
    mut layout: Layout<{ K::LAYOUT_COLS }, { K::LAYOUT_ROWS }, { K::LAYERS }, Keycode>,
) where
    [(); K::LAYOUT_COLS]:,
    [(); K::LAYOUT_ROWS]:,
{
    let mut last_keys = Vec::<KeyboardKeycode, 24>::new();

    loop {
        let keys = {
            let event = POLLED_EVENTS_CHANNEL.receive().await;
            layout.event(event);
            MATRIX_EVENTS.publish_immediate(event); // Just immediately publish since we don't want to hold up any key events to be converted into keycodes.
            let tick = layout.tick();

            debug!("[KEYBOARD] Processing rumcake feature keycodes");

            if let CustomEvent::Press(keycode) = tick {
                match keycode {
                    #[cfg(feature = "underglow")]
                    Keycode::Underglow(command) => {
                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                            .send(*command)
                            .await;
                        #[cfg(feature = "storage")]
                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                            .send(crate::underglow::animations::UnderglowCommand::SaveConfig)
                            .await;
                    }
                    #[cfg(feature = "backlight")]
                    Keycode::Backlight(command) => {
                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(*command)
                            .await;
                        #[cfg(feature = "storage")]
                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(crate::backlight::animations::BacklightCommand::SaveConfig)
                            .await;
                    }
                    #[cfg(feature = "bluetooth")]
                    Keycode::Bluetooth(command) => {
                        crate::bluetooth::BLUETOOTH_COMMAND_CHANNEL
                            .send(*command)
                            .await;
                    }
                    #[allow(unreachable_patterns)]
                    _ => {}
                }
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

            // It's possible for this channel to become filled (e.g. if USB is disabled and there is no Bluetooth connection
            // So, we just try_send instead of using `send`, which waits for capacity. That way, we can still process rumcake keycodes.
            if KEYBOARD_REPORT_HID_SEND_CHANNEL
                .try_send(NKROBootKeyboardReport::new(keys))
                .is_err()
            {
                warn!("[KEYBOARD] Discarding report");
            };
        }

        Timer::after(Duration::from_millis(1)).await;
    }
}
