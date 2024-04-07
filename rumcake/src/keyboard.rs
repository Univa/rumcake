//! Basic keyboard traits and tasks.
//!
//! Generally, keyboards will implement [`KeyboardLayout`] and [`KeyboardMatrix`] as needed.
//! Keyboard layouts and matrices are implemented with the help of [TeXitoi's `keyberon` crate](`keyberon`).

use core::convert::Infallible;
use core::fmt::Debug;
use core::ops::Range;

use defmt::{debug, info, warn, Debug2Format};
use embassy_futures::select::{select, select_slice, Either};
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::pubsub::{PubSubBehavior, PubSubChannel};
use embassy_time::{Duration, Ticker, Timer};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_hal_async::digital::Wait;
use heapless::Vec;
use keyberon::analog::{AnalogActuator, AnalogAcutationMode};
use keyberon::debounce::Debouncer;
use keyberon::layout::{CustomEvent, Event, Layers, Layout as KeyberonLayout};
use keyberon::matrix::{AnalogMatrix, DirectPinMatrix, Matrix};
use num_traits::SaturatingSub;
use usbd_human_interface_device::device::consumer::MultipleConsumerReport;
use usbd_human_interface_device::{
    device::keyboard::NKROBootKeyboardReport, page::Keyboard as KeyboardKeycode,
};

#[cfg(feature = "media-keycodes")]
pub use usbd_human_interface_device::page::Consumer;

use crate::hw::platform::RawMutex;
use crate::hw::{HIDDevice, CURRENT_OUTPUT_STATE};

pub use rumcake_macros::{
    build_analog_matrix, build_direct_pin_matrix, build_layout, build_standard_matrix,
    remap_matrix, setup_encoders,
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
    /// Get a reference to a channel that can receive matrix events from other tasks to be
    /// processed into keycodes.
    fn get_matrix_events_channel() -> &'static Channel<RawMutex, Event, 1> {
        static POLLED_EVENTS_CHANNEL: Channel<RawMutex, Event, 1> = Channel::new();

        &POLLED_EVENTS_CHANNEL
    }

    const NUM_ENCODERS: usize = 0; // No encoder via support yet. This is the default if not set in QMK

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

    #[cfg(feature = "simple-backlight")]
    type SimpleBacklightDeviceType: crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice =
        crate::lighting::private::EmptyLightingDevice;

    #[cfg(feature = "simple-backlight-matrix")]
    type SimpleBacklightMatrixDeviceType: crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice = crate::lighting::private::EmptyLightingDevice;

    #[cfg(feature = "rgb-backlight-matrix")]
    type RGBBacklightMatrixDeviceType: crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice = crate::lighting::private::EmptyLightingDevice;

    #[cfg(feature = "underglow")]
    type UnderglowDeviceType: crate::lighting::underglow::private::MaybeUnderglowDevice =
        crate::lighting::private::EmptyLightingDevice;
}

/// A mutex-guaraded [`keyberon::layout::Layout`]. This also stores the original layout, so that it
/// can be reset to it's initial state if modifications are made to it.
pub struct Layout<const C: usize, const R: usize, const L: usize> {
    pub(crate) layout: Mutex<RawMutex, KeyberonLayout<C, R, L, Keycode>>,
}

impl<const C: usize, const R: usize, const L: usize> Layout<C, R, L> {
    pub const fn new(layout: KeyberonLayout<C, R, L, Keycode>) -> Self {
        Self {
            layout: Mutex::new(layout),
        }
    }
}

pub trait DeviceWithEncoders {
    type Layout: private::MaybeKeyboardLayout = private::EmptyKeyboardLayout;

    const ENCODER_COUNT: usize;

    fn get_encoders() -> [impl Encoder; Self::ENCODER_COUNT];

    fn get_layout_mappings() -> [[(u8, u8); 3]; Self::ENCODER_COUNT];
}

pub trait Encoder {
    async fn wait_for_event(&mut self) -> EncoderEvent;
}

pub enum EncoderEvent {
    ClockwiseRotation,
    CounterClockwiseRotation,
    Press,
    Release,
}

pub struct EC11Encoder<SW, A, B> {
    sw: SW,
    a: A,
    b: B,
}

impl<SW: Wait + InputPin, A: Wait, B: InputPin> EC11Encoder<SW, A, B> {
    pub fn new(sw: SW, a: A, b: B) -> Self {
        Self { sw, a, b }
    }

    pub async fn wait_for_event(&mut self) -> EncoderEvent {
        let Self { sw, a, b } = self;

        match select(a.wait_for_falling_edge(), sw.wait_for_any_edge()).await {
            Either::First(_) => {
                if b.is_high().unwrap_or_default() {
                    EncoderEvent::CounterClockwiseRotation
                } else {
                    EncoderEvent::ClockwiseRotation
                }
            }
            Either::Second(_) => {
                if sw.is_low().unwrap_or_default() {
                    EncoderEvent::Press
                } else {
                    EncoderEvent::Release
                }
            }
        }
    }
}

impl<SW: Wait + InputPin, A: Wait, B: InputPin> Encoder for EC11Encoder<SW, A, B> {
    async fn wait_for_event(&mut self) -> EncoderEvent {
        self.wait_for_event().await
    }
}

/// A trait that must be implemented for any device that needs to poll a switch matrix.
pub trait KeyboardMatrix {
    /// The layout to send matrix events to.
    type Layout: private::MaybeKeyboardLayout = private::EmptyKeyboardLayout;

    #[cfg(feature = "split-peripheral")]
    /// The peripheral device in a split keyboard setup to send matrix events to.
    type PeripheralDeviceType: crate::split::peripheral::private::MaybePeripheralDevice =
        crate::split::peripheral::private::EmptyPeripheralDevice;

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

/// Setup an analog keyboard matrix. The output of this function can be passed to the matrix
/// polling task directly.
pub fn setup_analog_keyboard_matrix<S: MatrixSampler, const CS: usize, const RS: usize>(
    sampler: &S,
    pos_to_ch: [[(u8, u8); CS]; RS],
    ranges: [[Range<S::SampleType>; CS]; RS],
) -> PollableAnalogMatrix<S, CS, RS> {
    let sampler = AnalogMatrixSampler { pos_to_ch, sampler };
    let matrix = AnalogMatrix::new(ranges);
    let actuator = AnalogActuator::new([[AnalogAcutationMode::default(); CS]; RS], [[127; CS]; RS]);
    (sampler, matrix, actuator)
}

/// Custom keycodes used to interact with other rumcake features.
///
/// These can be used in your keyboard layout, defined in [`KeyboardLayout::get_layout`]
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
#[repr(u8)]
pub enum Keycode {
    /// Custom keycode, which can be used to run custom code. You can use
    /// [`KeyboardLayout::on_custom_keycode`] to handle it.
    Custom(u8) = 0,

    /// Hardware keycode, which can be any variant in [`crate::hw::HardwareCommand`]
    Hardware(crate::hw::HardwareCommand) = 1,

    #[cfg(feature = "media-keycodes")]
    /// Media keycode, which can be any variant in [`usbd_human_interface_device::page::Consumer`]
    Media(usbd_human_interface_device::page::Consumer) = 2,

    #[cfg(feature = "simple-backlight")]
    /// Keycode used to control a simple backlight system, which can be any variant in
    /// [`crate::lighting::simple_backlight::SimpleBacklightCommand`]
    SimpleBacklight(crate::lighting::simple_backlight::SimpleBacklightCommand) = 3,

    #[cfg(feature = "simple-backlight-matrix")]
    /// Keycode used to control a simple backlight matrix system, which can be any variant in
    /// [`crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand`]
    SimpleBacklightMatrix(crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand) =
        4,

    #[cfg(feature = "rgb-backlight-matrix")]
    /// Keycode used to control an RGB backlight matrix system, which can be any variant in
    /// [`crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand`]
    RGBBacklightMatrix(crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand) = 5,

    #[cfg(feature = "underglow")]
    /// Underglow keycode, which can be any variant in
    /// [`crate::lighting::underglow::UnderglowCommand`]
    Underglow(crate::lighting::underglow::UnderglowCommand) = 6,
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

pub type PollableStandardMatrix<I, O, const CS: usize, const RS: usize> =
    (Matrix<I, O, CS, RS>, Debouncer<[[bool; CS]; RS]>);

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

pub type PollableDirectPinMatrix<I, const CS: usize, const RS: usize> =
    (DirectPinMatrix<I, CS, RS>, Debouncer<[[bool; CS]; RS]>);

impl<I: InputPin<Error = Infallible>, const CS: usize, const RS: usize> Pollable
    for PollableDirectPinMatrix<I, CS, RS>
{
    fn events(&mut self) -> impl Iterator<Item = Event> {
        self.1.events(self.0.get().unwrap())
    }
}

/// Trait that allows you to use ADC hardware to pull samples for an analog matrix.
pub trait MatrixSampler {
    /// Type of samples generated by the ADC.
    type SampleType: SaturatingSub + PartialOrd;

    /// Get the sample for the given analog pin. `sub_ch` is used only if the pin is multiplexed.
    fn get_sample(&self, ch: usize, sub_ch: usize) -> Option<Self::SampleType>;
}

pub struct AnalogMatrixSampler<'a, S, const CS: usize, const RS: usize> {
    pos_to_ch: [[(u8, u8); CS]; RS],
    sampler: &'a S,
}

impl<'a, S: MatrixSampler, const CS: usize, const RS: usize> AnalogMatrixSampler<'a, S, CS, RS> {
    fn get_key_state(&self, row: usize, col: usize) -> Option<S::SampleType> {
        self.pos_to_ch
            .get(row)
            .and_then(|row| row.get(col))
            .and_then(|(ch, sub_ch)| {
                MatrixSampler::get_sample(self.sampler, *ch as usize, *sub_ch as usize)
            })
    }
}

#[derive(Debug)]
enum SampleError {
    NoSampleForKeyPosition(usize, usize),
}

pub type PollableAnalogMatrix<'a, S, const CS: usize, const RS: usize> = (
    AnalogMatrixSampler<'a, S, CS, RS>,
    AnalogMatrix<<S as MatrixSampler>::SampleType, CS, RS>,
    AnalogActuator<CS, RS>,
);

impl<S: MatrixSampler, const CS: usize, const RS: usize> Pollable
    for PollableAnalogMatrix<'_, S, CS, RS>
where
    u32: From<S::SampleType>,
{
    fn events(&mut self) -> impl Iterator<Item = Event> {
        let matrix_state = self
            .1
            .get(|row, col| {
                self.0
                    .get_key_state(row, col)
                    .ok_or(SampleError::NoSampleForKeyPosition(row, col))
            })
            .unwrap();

        self.2.events(matrix_state)
    }
}

#[rumcake_macros::task]
pub async fn ec11_encoders_poll<K: DeviceWithEncoders>(_k: K)
where
    [(); K::ENCODER_COUNT]:,
{
    let mappings = K::get_layout_mappings();
    let mut encoders = K::get_encoders();

    let layout_channel = <K::Layout as private::MaybeKeyboardLayout>::get_matrix_events_channel();
    let mut events: Vec<Event, 2> = Vec::new();

    loop {
        events.clear();
        let (event, idx) = select_slice(
            &mut encoders
                .iter_mut()
                .map(|e| e.wait_for_event())
                .collect::<Vec<_, { K::ENCODER_COUNT }>>(),
        )
        .await;

        let [sw_pos, cw_pos, ccw_pos] = mappings[idx];

        match event {
            EncoderEvent::ClockwiseRotation => {
                events.push(Event::Press(cw_pos.0, cw_pos.1));
                events.push(Event::Release(cw_pos.0, cw_pos.1));
            }
            EncoderEvent::CounterClockwiseRotation => {
                events.push(Event::Press(ccw_pos.0, ccw_pos.1));
                events.push(Event::Release(ccw_pos.0, ccw_pos.1));
            }
            EncoderEvent::Press => {
                events.push(Event::Press(sw_pos.0, sw_pos.1));
            }
            EncoderEvent::Release => {
                events.push(Event::Release(sw_pos.0, sw_pos.1));
            }
        };

        for e in &events {
            if let Some(layout_channel) = layout_channel {
                layout_channel.send(*e).await;
            }
        }

        Timer::after(Duration::from_millis(1)).await;
    }
}

#[rumcake_macros::task]
pub async fn matrix_poll<K: KeyboardMatrix + 'static>(_k: K) {
    let matrix = K::get_matrix();
    let layout_channel = <K::Layout as private::MaybeKeyboardLayout>::get_matrix_events_channel();

    #[cfg(feature = "split-peripheral")]
    let peripheral_channel = <K::PeripheralDeviceType as crate::split::peripheral::private::MaybePeripheralDevice>::get_matrix_events_channel();

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

                if let Some(layout_channel) = layout_channel {
                    layout_channel.send(remapped_event).await
                };

                #[cfg(feature = "split-peripheral")]
                if let Some(peripheral_channel) = peripheral_channel {
                    peripheral_channel.send(remapped_event).await
                };
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

#[rumcake_macros::task]
pub async fn layout_collect<K: KeyboardLayout + HIDDevice + 'static>(_k: K)
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
    let matrix_channel = K::get_matrix_events_channel();

    #[cfg(feature = "media-keycodes")]
    let consumer_report_channel = K::get_consumer_report_send_channel();

    let keyboard_report = K::get_keyboard_report_send_channel();

    let mut should_tick_repeatedly = false;

    loop {
        let keys = {
            let event = if should_tick_repeatedly {
                matrix_channel.try_receive().ok()
            } else {
                Some(matrix_channel.receive().await)
            };

            let mut layout = layout.layout.lock().await;

            if let Some(event) = event {
                layout.event(event);
                MATRIX_EVENTS.publish_immediate(event); // Just immediately publish since we don't want to hold up any key events to be converted into keycodes.
            };

            let tick = layout.tick();

            let new_layout_state = layout.is_active();
            if !should_tick_repeatedly && new_layout_state {
                ticker.reset()
            }
            should_tick_repeatedly = new_layout_state;

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
                        consumer_report_channel
                            .send(MultipleConsumerReport { codes })
                            .await;
                    }
                    #[cfg(feature = "underglow")]
                    Keycode::Underglow(command) => {
                        if let Some(channel) = <K::UnderglowDeviceType as crate::lighting::underglow::private::MaybeUnderglowDevice>::get_command_channel() {
                            channel.send(command).await;
                            #[cfg(feature = "storage")]
                            channel
                                .send(crate::lighting::underglow::UnderglowCommand::SaveConfig)
                                .await;
                        }
                    }
                    #[cfg(feature = "simple-backlight")]
                    Keycode::SimpleBacklight(command) => {
                        if let Some(channel) = <K::SimpleBacklightDeviceType as crate::lighting::simple_backlight::private::MaybeSimpleBacklightDevice>::get_command_channel() {
                            channel.send(command).await;
                            #[cfg(feature = "storage")]
                            channel.send(crate::lighting::simple_backlight::SimpleBacklightCommand::SaveConfig)
                            .await;
                        }
                    }
                    #[cfg(feature = "simple-backlight-matrix")]
                    Keycode::SimpleBacklightMatrix(command) => {
                        if let Some(channel) = <K::SimpleBacklightMatrixDeviceType as crate::lighting::simple_backlight_matrix::private::MaybeSimpleBacklightMatrixDevice>::get_command_channel() {
                            channel.send(command).await;
                            #[cfg(feature = "storage")]
                            channel.send(crate::lighting::simple_backlight_matrix::SimpleBacklightMatrixCommand::SaveConfig)
                            .await;
                        }
                    }
                    #[cfg(feature = "rgb-backlight-matrix")]
                    Keycode::RGBBacklightMatrix(command) => {
                        if let Some(channel) = <K::RGBBacklightMatrixDeviceType as crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice>::get_command_channel() {
                            channel.send(command).await;
                            #[cfg(feature = "storage")]
                            channel.send(crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixCommand::SaveConfig)
                            .await;
                        }
                    }
                    Keycode::Hardware(command) => {
                        crate::hw::HARDWARE_COMMAND_CHANNEL
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
                        consumer_report_channel
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
                keyboard_report
                    .send(NKROBootKeyboardReport::new(keys))
                    .await;
            } else {
                warn!("[KEYBOARD] Discarding report");
            }
        }

        ticker.next().await;
    }
}

pub(crate) mod private {
    use embassy_sync::channel::Channel;
    use keyberon::layout::Event;

    use crate::hw::platform::RawMutex;

    use super::KeyboardLayout;

    pub struct EmptyKeyboardLayout;
    impl MaybeKeyboardLayout for EmptyKeyboardLayout {}

    pub trait MaybeKeyboardLayout {
        fn get_matrix_events_channel() -> Option<&'static Channel<RawMutex, Event, 1>> {
            None
        }
    }

    impl<T: KeyboardLayout> MaybeKeyboardLayout for T {
        fn get_matrix_events_channel() -> Option<&'static Channel<RawMutex, Event, 1>> {
            Some(T::get_matrix_events_channel())
        }
    }
}
