use core::convert::Infallible;
use defmt::{debug, info, warn, Debug2Format};
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel, mutex::Mutex};
use embassy_time::{Duration, Timer};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use heapless::Vec;
use keyberon::debounce::Debouncer;
use keyberon::layout::{CustomEvent, Event, Layout};
use keyberon::{layout::Layers, matrix::Matrix};
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

pub trait Keyboard {
    const MANUFACTURER: &'static str;
    const PRODUCT: &'static str;
    const SERIAL_NUMBER: &'static str = "1";
    const HARDWARE_REVISION: &'static str = "1";
    const FIRMWARE_REVISION: &'static str = "1";
}

pub trait KeyboardLayout {
    // Features
    const NUM_ENCODERS: u8 = 0; // Only for VIA compatibility, no proper encoder support. This is the default if not set in QMK

    // Layout settings
    const LAYOUT_COLS: usize;
    const LAYOUT_ROWS: usize;
    const LAYERS: usize;
    fn build_layout(
    ) -> Layers<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, { Self::LAYERS }, Keycode>;
}

#[macro_export]
macro_rules! build_layout {
    // Pass the layers to the keyberon macro
    ($layers:literal, $rows:literal, $cols:literal, ($($l:tt)*)) => {
        fn build_layout() -> $crate::keyberon::layout::Layers<{ Self::LAYOUT_COLS }, { Self::LAYOUT_ROWS }, {${count(l)}}, $crate::keyboard::Keycode> {
            $crate::keyberon::layout::layout! {
                $($l)*
            }
        }
    };
    // We count the number of keys in the first row to determine the number of columns
    ($layers:literal, $rows:literal, ($({[$($first_row_keys:tt)*] $([$($key:tt)*])*})*)) => {
        const LAYOUT_COLS: usize = ${count(first_row_keys)};
        build_layout!($layers, $rows, ${count(first_row_keys)}, ($({[$($first_row_keys)*] $([$($key)*])*})*));
    };
    // Count the number of "[]" inside the "{}" to determine the number of rows
    ($layers:literal, ($({$($rows:tt)*})*)) => {
        const LAYOUT_ROWS: usize = ${count(rows)};
        build_layout!($layers, ${count(rows)}, ($({$($rows)*})*));
    };
    // Count the number of "{}" to determine the number of layers
    ($($layers:tt)*) => {
        const LAYERS: usize = ${count(layers)};
        build_layout!(${count(layers)}, ($($layers)*));
    };
}

#[macro_export]
macro_rules! setup_keyboard_layout {
    ($K:ident) => {{
        static LAYOUT: $crate::static_cell::StaticCell<
            $crate::embassy_sync::mutex::Mutex<
                $crate::embassy_sync::blocking_mutex::raw::ThreadModeRawMutex,
                $crate::keyberon::layout::Layout<
                    { $K::LAYOUT_COLS },
                    { $K::LAYOUT_ROWS },
                    { $K::LAYERS },
                    $crate::keyboard::Keycode,
                >,
            >,
        > = $crate::static_cell::StaticCell::new();
        static LAYERS: $crate::static_cell::StaticCell<
            $crate::keyberon::layout::Layers<
                { $K::LAYOUT_COLS },
                { $K::LAYOUT_ROWS },
                { $K::LAYERS },
                $crate::keyboard::Keycode,
            >,
        > = $crate::static_cell::StaticCell::new();
        let layers = LAYERS.init($K::build_layout());
        let layout = LAYOUT.init($crate::embassy_sync::mutex::Mutex::new(
            $crate::keyberon::layout::Layout::new(layers),
        ));
        layout
    }};
}

pub trait KeyboardMatrix {
    // Debounce settings
    const DEBOUNCE_MS: u16 = 5;

    // Matrix settings
    const MATRIX_COLS: usize;
    const MATRIX_ROWS: usize;
    fn build_matrix() -> Result<
        Matrix<
            impl InputPin<Error = Infallible>,
            impl OutputPin<Error = Infallible>,
            { Self::MATRIX_COLS },
            { Self::MATRIX_ROWS },
        >,
        Infallible,
    >;

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

#[macro_export]
macro_rules! setup_keyboard_matrix {
    ($K:ident) => {{
        let matrix = $K::build_matrix().unwrap();
        static DEBOUNCER: $crate::static_cell::StaticCell<
            $crate::embassy_sync::mutex::Mutex<
                $crate::embassy_sync::blocking_mutex::raw::ThreadModeRawMutex,
                $crate::keyberon::debounce::Debouncer<
                    [[bool; { $K::MATRIX_COLS }]; { $K::MATRIX_ROWS }],
                >,
            >,
        > = $crate::static_cell::StaticCell::new();
        let debouncer = DEBOUNCER.init($crate::embassy_sync::mutex::Mutex::new(
            $crate::keyberon::debounce::Debouncer::new(
                [[false; $K::MATRIX_COLS]; $K::MATRIX_ROWS],
                [[false; $K::MATRIX_COLS]; $K::MATRIX_ROWS],
                $K::DEBOUNCE_MS,
            ),
        ));
        (matrix, debouncer)
    }};
}

// Custom keycodes to be interact with other rumcake features.
#[derive(Debug, Clone, Copy)]
pub enum Keycode {
    #[cfg(feature = "underglow")]
    Underglow(crate::underglow::animations::UnderglowCommand),
    #[cfg(feature = "backlight")]
    Backlight(crate::backlight::animations::BacklightCommand),
    #[cfg(feature = "bluetooth")]
    Bluetooth(crate::nrf_ble::BluetoothCommand),
}

// Channel with keyboard events after polling the matrix
// The coordinates received will be remapped according to the implementation of `remap_to_layout`
pub static POLLED_EVENTS_CHANNEL: Channel<ThreadModeRawMutex, Event, 1> = Channel::new();

#[rumcake_macros::task]
pub async fn matrix_poll<K: KeyboardMatrix>(
    mut matrix: Matrix<
        impl InputPin<Error = Infallible>,
        impl OutputPin<Error = Infallible>,
        { K::MATRIX_COLS },
        { K::MATRIX_ROWS },
    >,
    debouncer: &'static Mutex<
        ThreadModeRawMutex,
        Debouncer<[[bool; K::MATRIX_COLS]; K::MATRIX_ROWS]>,
    >,
) where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    loop {
        {
            debug!("[KEYBOARD] Scanning matrix");
            let mut debouncer = debouncer.lock().await;
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

// A PubSubChannel used to send matrix events to be used by other features (e.g. underglow or backlight reactive effects)
// The coordinates received will be remapped according to the implementation of `remap_to_layout`
pub static MATRIX_EVENTS: PubSubChannel<ThreadModeRawMutex, Event, 4, 4, 1> = PubSubChannel::new();

#[rumcake_macros::task]
pub async fn layout_register<K: KeyboardLayout>(
    layout: &'static Mutex<
        ThreadModeRawMutex,
        Layout<{ K::LAYOUT_COLS }, { K::LAYOUT_ROWS }, { K::LAYERS }, Keycode>,
    >,
) where
    [(); K::LAYOUT_COLS]:,
    [(); K::LAYOUT_ROWS]:,
{
    loop {
        let event = POLLED_EVENTS_CHANNEL.receive().await;
        let mut layout = layout.lock().await;
        layout.event(event);
        MATRIX_EVENTS.publish_immediate(event); // Just immediately publish since we don't want to hold up any key events to be converted into keycodes.
        debug!(
            "[KEYBOARD] Registered key event: {:?}",
            Debug2Format(&event)
        );
    }
}

// Channel with data to send to PC
pub static KEYBOARD_REPORT_HID_SEND_CHANNEL: Channel<
    ThreadModeRawMutex,
    NKROBootKeyboardReport,
    1,
> = Channel::new();

#[rumcake_macros::task]
pub async fn layout_collect<K: KeyboardLayout>(
    layout: &'static Mutex<
        ThreadModeRawMutex,
        Layout<{ K::LAYOUT_COLS }, { K::LAYOUT_ROWS }, { K::LAYERS }, Keycode>,
    >,
) where
    [(); K::LAYOUT_COLS]:,
    [(); K::LAYOUT_ROWS]:,
{
    let mut last_keys = Vec::<KeyboardKeycode, 24>::new();

    loop {
        let keys = {
            let mut layout = layout.lock().await;
            let tick = layout.tick();

            debug!("[KEYBOARD] Processing rumcake feature keycodes");

            if let CustomEvent::Press(keycode) = tick {
                match keycode {
                    #[cfg(feature = "underglow")]
                    Keycode::Underglow(command) => {
                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                            .send(*command)
                            .await;
                        crate::underglow::UNDERGLOW_COMMAND_CHANNEL
                            .send(crate::underglow::animations::UnderglowCommand::SaveConfig)
                            .await;
                    }
                    #[cfg(feature = "backlight")]
                    Keycode::Backlight(command) => {
                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(*command)
                            .await;
                        crate::backlight::BACKLIGHT_COMMAND_CHANNEL
                            .send(crate::backlight::animations::BacklightCommand::SaveConfig)
                            .await;
                    }
                    #[cfg(feature = "bluetooth")]
                    Keycode::Bluetooth(command) => {
                        crate::nrf_ble::BLUETOOTH_COMMAND_CHANNEL
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
