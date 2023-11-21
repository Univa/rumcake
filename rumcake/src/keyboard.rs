//! Basic keyboard traits and tasks.
//!
//! Generally, keyboards will implement [`KeyboardLayout`] and [`KeyboardMatrix`] as needed.
//! Keyboard layouts and matrices are implemented with the help of [TeXitoi's `keyberon` crate](`keyberon`).

use core::convert::Infallible;
use defmt::{debug, info, warn, Debug2Format};
use embassy_sync::mutex::{Mutex, MutexGuard};
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use heapless::Vec;
use keyberon::debounce::Debouncer;
use keyberon::layout::{CustomEvent, Event, Layers, Layout as KeyberonLayout};
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

    /// Get a reference to the keyboard's layout. This should initialize the keyboard layout on
    /// first call.
    ///
    /// It is recommended to use [`build_layout`] to implement this function.
    fn get_layout(
    ) -> &'static Layout<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, { Self::LAYERS }>;

    /// Handle a [`Keycode::Custom`] event. By default this does nothing.
    ///
    /// `press` is set to `true` if the event was a key press. Otherwise, it will be `false`. `id`
    /// corresponds to the `id` used in your keyboard layout.
    fn on_custom_keycode(_id: u8, _press: bool) {}
}

pub struct Layout<const C: usize, const R: usize, const L: usize> {
    original: Layers<C, R, L, Keycode>,
    layout: once_cell::sync::OnceCell<Mutex<ThreadModeRawMutex, KeyberonLayout<C, R, L, Keycode>>>,
}

impl<const C: usize, const R: usize, const L: usize> Layout<C, R, L> {
    pub const fn new(layers: Layers<C, R, L, Keycode>) -> Self {
        Self {
            original: layers,
            layout: once_cell::sync::OnceCell::new(),
        }
    }

    pub async fn lock(&self) -> MutexGuard<ThreadModeRawMutex, KeyberonLayout<C, R, L, Keycode>> {
        self.layout
            .get_or_init(|| Mutex::new(KeyberonLayout::new(self.original)))
            .lock()
            .await
    }

    pub async fn reset(&self) {
        let mut layout = self.lock().await;

        for layer in 0..L {
            for row in 0..R {
                for col in 0..C {
                    layout
                        .change_action(
                            (row as u8, col as u8),
                            layer,
                            self.original[layer][row][col],
                        )
                        .unwrap();
                }
            }
        }
    }
}

#[macro_export]
macro_rules! build_layout {
    // Pass the layers to the keyberon macro
    ($layers:literal, $rows:literal, $cols:literal, ($($l:tt)*)) => {
        // fn get_original_layout() -> $crate::keyberon::layout::Layers<$cols, $rows, $layers, $crate::keyboard::Keycode> {
        //     $crate::keyberon::layout::layout! { $($l)* }
        // }

        fn get_layout(
        ) -> &'static rumcake::keyboard::Layout<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, { Self::LAYERS }> {
            static KEYBOARD_LAYOUT: rumcake::keyboard::Layout<$cols, $rows, $layers> = rumcake::keyboard::Layout::new($crate::keyberon::layout::layout! { $($l)* });
            // const LAYERS: $crate::keyberon::layout::Layers<$cols, $rows, $layers, $crate::keyboard::Keycode> = $crate::keyberon::layout::layout! { $($l)* };
            // static KEYBOARD_LAYOUT: $crate::keyboard::Layout<$cols, $rows, $layers> = $crate::keyboard::Layout::new(LAYERS);
            &KEYBOARD_LAYOUT
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

    /// Optional function to remap a matrix position to a position on the keyboard layout defined
    /// by [`KeyboardLayout::get_layout`].
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
) {
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
/// These can be used in your keyboard layout, defined in [`KeyboardLayout::get_layout`]
#[derive(Debug, Clone, Copy)]
pub enum Keycode {
    /// Custom keycode, which can be used to run custom code. You can use
    /// [`KeyboardLayout::on_custom_keycode`] to handle it.
    Custom(u8),
    #[cfg(feature = "underglow")]
    /// Underglow keycode, which can be any variant in [`crate::underglow::animations::UnderglowCommand`]
    Underglow(crate::underglow::animations::UnderglowCommand),
    #[cfg(any(
        feature = "simple-backlight",
        feature = "simple-backlight-matrix",
        feature = "rgb-backlight-matrix"
    ))]
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
) {
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
pub async fn layout_collect<K: KeyboardLayout + 'static>(_k: K)
where
    [(); K::LAYERS]:,
    [(); K::LAYOUT_COLS]:,
    [(); K::LAYOUT_ROWS]:,
{
    let mut last_keys = Vec::<KeyboardKeycode, 24>::new();
    let layout = K::get_layout();

    loop {
        let keys = {
            let event = POLLED_EVENTS_CHANNEL.receive().await;
            let mut layout = layout.lock().await;
            layout.event(event);
            MATRIX_EVENTS.publish_immediate(event); // Just immediately publish since we don't want to hold up any key events to be converted into keycodes.
            let tick = layout.tick();

            debug!("[KEYBOARD] Processing rumcake feature keycodes");

            match tick {
                CustomEvent::NoEvent => {}
                CustomEvent::Press(keycode) => match keycode {
                    Keycode::Custom(id) => {
                        K::on_custom_keycode(id, true);
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
                    #[cfg(any(
                        feature = "simple-backlight",
                        feature = "simple-backlight-matrix",
                        feature = "rgb-backlight-matrix"
                    ))]
                    Keycode::Backlight(command) => {
                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(command)
                            .await;
                        #[cfg(feature = "storage")]
                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(crate::backlight::animations::BacklightCommand::SaveConfig)
                            .await;
                    }
                    #[cfg(feature = "bluetooth")]
                    Keycode::Bluetooth(command) => {
                        crate::bluetooth::BLUETOOTH_COMMAND_CHANNEL
                            .send(command)
                            .await;
                    }
                },
                CustomEvent::Release(keycode) =>
                {
                    #[allow(irrefutable_let_patterns)]
                    if let Keycode::Custom(id) = keycode {
                        K::on_custom_keycode(id, false);
                    }
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

            // It's possible for this channel to become filled (e.g. if USB is disabled and
            // there is no Bluetooth connection So, we just try_send instead of using `send`,
            // which waits for capacity. That way, we can still process rumcake keycodes.
            if KEYBOARD_REPORT_HID_SEND_CHANNEL
                .try_send(NKROBootKeyboardReport::new(keys))
                .is_err()
            {
                warn!("[KEYBOARD] Discarding report");
            };
        }
    }
}
