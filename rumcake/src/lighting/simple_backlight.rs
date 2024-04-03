use core::fmt::Debug;
use core::marker::PhantomData;
use core::u8;

use defmt::{error, warn, Debug2Format};
use embassy_sync::channel::Channel;
use keyberon::layout::Event;
use num_derive::FromPrimitive;
use postcard::experimental::max_size::MaxSize;
use rand::rngs::SmallRng;
use rand_core::SeedableRng;
use rumcake_macros::{generate_items_from_enum_variants, Cycle, LEDEffect};
use serde::{Deserialize, Serialize};

use crate::hw::platform::RawMutex;
use crate::lighting::Animator;
use crate::math::{scale, sin};
use crate::{Cycle, LEDEffect, State};

pub trait SimpleBacklightDevice {
    /// How fast the LEDs refresh to display a new animation frame.
    ///
    /// It is recommended to set this value to a value that your driver can handle,
    /// otherwise your animations will appear to be slowed down.
    ///
    /// **This does not have any effect if the selected animation is static.**
    const FPS: usize = 20;

    /// Get a reference to a channel that can receive commands to control the simple backlight
    /// animator from other tasks.
    #[inline(always)]
    fn get_command_channel() -> &'static Channel<RawMutex, SimpleBacklightCommand, 2> {
        static SIMPLE_BACKLIGHT_COMMAND_CHANNEL: Channel<RawMutex, SimpleBacklightCommand, 2> =
            Channel::new();

        &SIMPLE_BACKLIGHT_COMMAND_CHANNEL
    }

    /// Get a reference to a state object that can be used to notify other tasks about changes to
    /// the simple backlight configuration. Note that updating the state object will not control
    /// the output of the simple backlight animator.
    #[inline(always)]
    fn get_state() -> &'static State<'static, SimpleBacklightConfig> {
        static SIMPLE_BACKLIGHT_CONFIG_STATE: State<SimpleBacklightConfig> = State::new(
            SimpleBacklightConfig::default(),
            &[
                #[cfg(feature = "storage")]
                &SIMPLE_BACKLIGHT_CONFIG_STATE_LISTENER,
            ],
        );

        &SIMPLE_BACKLIGHT_CONFIG_STATE
    }

    #[cfg(feature = "storage")]
    #[inline(always)]
    fn get_state_listener() -> &'static embassy_sync::signal::Signal<RawMutex, ()> {
        &SIMPLE_BACKLIGHT_CONFIG_STATE_LISTENER
    }

    #[cfg(feature = "storage")]
    #[inline(always)]
    fn get_save_signal() -> &'static embassy_sync::signal::Signal<RawMutex, ()> {
        &SIMPLE_BACKLIGHT_SAVE_SIGNAL
    }

    #[cfg(feature = "split-central")]
    type CentralDevice: crate::split::central::private::MaybeCentralDevice =
        crate::split::central::private::EmptyCentralDevice;

    simple_backlight_effect_items!();
}

pub(crate) mod private {
    use embassy_sync::channel::Channel;

    use crate::hw::platform::RawMutex;
    use crate::State;

    use super::{SimpleBacklightCommand, SimpleBacklightConfig, SimpleBacklightDevice};

    pub trait MaybeSimpleBacklightDevice {
        #[inline(always)]
        fn get_command_channel() -> Option<&'static Channel<RawMutex, SimpleBacklightCommand, 2>> {
            None
        }

        #[inline(always)]
        fn get_state() -> Option<&'static State<'static, SimpleBacklightConfig>> {
            None
        }
    }

    impl<T: SimpleBacklightDevice> MaybeSimpleBacklightDevice for T {
        #[inline(always)]
        fn get_command_channel() -> Option<&'static Channel<RawMutex, SimpleBacklightCommand, 2>> {
            Some(T::get_command_channel())
        }

        #[inline(always)]
        fn get_state() -> Option<&'static State<'static, SimpleBacklightConfig>> {
            Some(T::get_state())
        }
    }
}

/// A trait that a driver must implement in order to support a simple (no matrix, one color) backlighting scheme.
pub trait SimpleBacklightDriver<K: SimpleBacklightDevice> {
    /// The type of error that the driver will return if [`SimpleBacklightDriver::write`] fails.
    type DriverWriteError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(&mut self, brightness: u8) -> Result<(), Self::DriverWriteError>;

    /// The type of error that the driver will return if [`SimpleBacklightDriver::turn_on`] fails.
    type DriverEnableError: Debug;

    /// Turn the LEDs on using the driver when the animator gets enabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this, and subsequently [`SimpleBacklightDriver::write`]. So, if your driver
    /// doesn't need do anything special to turn the LEDs on, you may simply return `Ok(())`.
    async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError>;

    /// The type of error that the driver will return if [`SimpleBacklightDriver::turn_off`] fails.
    type DriverDisableError: Debug;

    /// Turn the LEDs off using the driver when the animator is disabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this. However, the tick method will not call
    /// [`SimpleBacklightDriver::write`] due to the animator being disabled, so you will need to
    /// turn off the LEDs somehow. For example, you can write a brightness of 0 to all LEDs.
    async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError>;
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, MaxSize)]
pub struct SimpleBacklightConfig {
    pub enabled: bool,
    pub effect: SimpleBacklightEffect,
    pub val: u8,
    pub speed: u8,
}

impl SimpleBacklightConfig {
    pub const fn default() -> Self {
        SimpleBacklightConfig {
            enabled: true,
            effect: SimpleBacklightEffect::Solid,
            val: 255,
            speed: 86,
        }
    }
}

impl Default for SimpleBacklightConfig {
    fn default() -> Self {
        Self::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, MaxSize)]
#[non_exhaustive]
#[repr(u8)]
pub enum SimpleBacklightCommand {
    Toggle = 0,
    TurnOn = 1,
    TurnOff = 2,
    NextEffect = 3,
    PrevEffect = 4,
    SetEffect(SimpleBacklightEffect) = 5,
    SetValue(u8) = 6,
    IncreaseValue(u8) = 7,
    DecreaseValue(u8) = 8,
    SetSpeed(u8) = 9,
    IncreaseSpeed(u8) = 10,
    DecreaseSpeed(u8) = 11,
    #[cfg(feature = "storage")]
    SaveConfig = 12,
    ResetTime = 13, // normally used internally for syncing LEDs for split keyboards
}

#[generate_items_from_enum_variants("const {variant_shouty_snake_case}_ENABLED: bool = true")]
#[derive(
    FromPrimitive,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    LEDEffect,
    Cycle,
    PartialEq,
    Eq,
    MaxSize,
)]
pub enum SimpleBacklightEffect {
    Solid,

    #[animated]
    Breathing,

    #[animated]
    #[reactive]
    Reactive,
}

impl SimpleBacklightEffect {
    pub(crate) fn is_enabled<D: SimpleBacklightDevice>(&self) -> bool {
        match self {
            SimpleBacklightEffect::Solid => D::SOLID_ENABLED,
            SimpleBacklightEffect::Breathing => D::BREATHING_ENABLED,
            SimpleBacklightEffect::Reactive => D::REACTIVE_ENABLED,
        }
    }
}

pub struct SimpleBacklightAnimator<D: SimpleBacklightDevice, R: SimpleBacklightDriver<D>> {
    config: SimpleBacklightConfig,
    buf: u8, // Stores the current brightness/value. Different from `self.config.val`.
    time_of_last_press: u32,
    tick: u32,
    driver: R,
    rng: SmallRng,
    phantom: PhantomData<D>,
}

impl<D: SimpleBacklightDevice, R: SimpleBacklightDriver<D>> SimpleBacklightAnimator<D, R> {
    pub fn new(config: SimpleBacklightConfig, driver: R) -> Self {
        Self {
            config,
            tick: 0,
            driver,
            buf: 0,
            time_of_last_press: 0,
            rng: SmallRng::seed_from_u64(1337),
            phantom: PhantomData,
        }
    }

    pub async fn turn_on(&mut self) {
        if let Err(err) = self.driver.turn_on().await {
            warn!("[SIMPLE_BACKLIGHT] Animations have been enabled, but the backlight LEDs could not be turned on: {}", Debug2Format(&err));
        };
    }

    pub async fn turn_off(&mut self) {
        if let Err(err) = self.driver.turn_off().await {
            warn!("[SIMPLE_BACKLIGHT] Animations have been disabled, but the backlight LEDs could not be turned off: {}", Debug2Format(&err));
        };
    }

    pub fn process_command(&mut self, command: SimpleBacklightCommand) {
        match command {
            SimpleBacklightCommand::Toggle => {
                self.config.enabled = !self.config.enabled;
            }
            SimpleBacklightCommand::TurnOn => {
                self.config.enabled = true;
            }
            SimpleBacklightCommand::TurnOff => {
                self.config.enabled = false;
            }
            SimpleBacklightCommand::NextEffect => {
                while {
                    self.config.effect.increment();
                    self.config.effect.is_enabled::<D>()
                } {}
            }
            SimpleBacklightCommand::PrevEffect => {
                while {
                    self.config.effect.decrement();
                    self.config.effect.is_enabled::<D>()
                } {}
            }
            SimpleBacklightCommand::SetEffect(effect) => {
                self.config.effect = effect;
            }
            SimpleBacklightCommand::SetValue(val) => {
                self.config.val = val;
            }
            SimpleBacklightCommand::IncreaseValue(amount) => {
                self.config.val = self.config.val.saturating_add(amount);
            }
            SimpleBacklightCommand::DecreaseValue(amount) => {
                self.config.val = self.config.val.saturating_sub(amount);
            }
            SimpleBacklightCommand::SetSpeed(speed) => {
                self.config.speed = speed;
            }
            SimpleBacklightCommand::IncreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_add(amount);
            }
            SimpleBacklightCommand::DecreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_sub(amount);
            }
            #[cfg(feature = "storage")]
            SimpleBacklightCommand::SaveConfig => {
                // storage::SIMPLE_BACKLIGHT_SAVE_SIGNAL.signal(());
            }
            SimpleBacklightCommand::ResetTime => {
                self.tick = 0;
            }
        }
    }

    pub fn set_brightness(&mut self, calc: impl Fn(&mut Self, u32) -> u8) {
        let time = (self.tick << 8)
            / (((D::FPS as u32) << 8)
                / (self.config.speed as u32 + 128 + (self.config.speed as u32 >> 1))); // `time` should increment by 255 every second

        self.buf = scale(calc(self, time), self.config.val)
    }

    pub fn register_event(&mut self, event: Event) {
        match event {
            Event::Press(_row, _col) => {
                self.time_of_last_press = (self.tick << 8)
                    / (((D::FPS as u32) << 8)
                        / (self.config.speed as u32 + 128 + (self.config.speed as u32 >> 1)));
            }
            Event::Release(_row, _col) => {} // nothing for now. maybe change some effects to behave depending on the state of a key.
        }
    }

    pub async fn tick(&mut self) {
        if !self.config.enabled {
            return;
        }

        match self.config.effect {
            SimpleBacklightEffect::Solid => {
                if D::SOLID_ENABLED {
                    self.set_brightness(|_animator, _time| u8::MAX)
                }
            }
            SimpleBacklightEffect::Breathing => {
                if D::BREATHING_ENABLED {
                    self.set_brightness(|_animator, time| sin((time >> 2) as u8))
                }
            }
            SimpleBacklightEffect::Reactive => {
                if D::REACTIVE_ENABLED {
                    self.set_brightness(|animator, time| {
                        // LED fades after one second
                        (u8::MAX as u32).saturating_sub(time - animator.time_of_last_press) as u8
                    })
                }
            }
        }

        if let Err(err) = self.driver.write(self.buf).await {
            error!(
                "[BACKLIGHT] Couldn't update backlight: {}",
                Debug2Format(&err)
            );
        };

        self.tick += 1;
    }

    #[cfg(feature = "storage")]
    pub fn create_storage_instance(&self) -> SimpleBacklightStorage<D, R> {
        SimpleBacklightStorage {
            _device_phantom: core::marker::PhantomData,
            _driver_phantom: core::marker::PhantomData,
        }
    }
}

impl<D: SimpleBacklightDevice, R: SimpleBacklightDriver<D>> Animator
    for SimpleBacklightAnimator<D, R>
{
    type CommandType = SimpleBacklightCommand;

    type ConfigType = SimpleBacklightConfig;

    type BufferUpdateArgs = ();

    const FPS: usize = D::FPS;

    async fn initialize(&mut self) {
        self.config = D::get_state().get().await;

        match self.config.enabled {
            true => self.turn_on().await,
            false => self.turn_off().await,
        }
    }

    async fn tick(&mut self) {
        self.tick().await
    }

    fn is_waiting_for_command(&self) -> bool {
        !(self.config.enabled && self.config.effect.is_animated())
    }

    fn process_command(&mut self, command: Self::CommandType) {
        self.process_command(command)
    }

    async fn handle_state_change(&mut self) {
        // Update the config state, after updating the animator's own copy, and check if it was enabled/disabled
        let toggled = D::get_state()
            .update(|config| {
                let toggled = config.enabled != self.config.enabled;
                **config = self.config;
                toggled
            })
            .await;

        if toggled {
            match self.config.enabled {
                true => self.turn_on().await,
                false => self.turn_off().await,
            }
        }

        // Send commands to be consumed by the split peripherals
        #[cfg(feature = "split-central")]
        {
            use crate::split::central::private::MaybeCentralDevice;
            if let Some(channel) = D::CentralDevice::get_message_to_peripheral_channel() {
                channel
                    .send(crate::split::MessageToPeripheral::SimpleBacklight(
                        SimpleBacklightCommand::ResetTime,
                    ))
                    .await;
                channel
                    .send(crate::split::MessageToPeripheral::SimpleBacklight(
                        SimpleBacklightCommand::SetEffect(self.config.effect),
                    ))
                    .await;
                channel
                    .send(crate::split::MessageToPeripheral::SimpleBacklight(
                        SimpleBacklightCommand::SetValue(self.config.val),
                    ))
                    .await;
                channel
                    .send(crate::split::MessageToPeripheral::SimpleBacklight(
                        SimpleBacklightCommand::SetSpeed(self.config.speed),
                    ))
                    .await;
            }
        }
    }

    #[inline(always)]
    fn get_command_channel() -> &'static Channel<RawMutex, Self::CommandType, 2> {
        D::get_command_channel()
    }

    #[inline(always)]
    fn get_state() -> &'static State<'static, Self::ConfigType> {
        D::get_state()
    }
}

#[cfg(feature = "storage")]
pub use storage::*;

#[cfg(feature = "storage")]
mod storage {
    use embassy_sync::signal::Signal;

    use crate::hw::platform::RawMutex;

    use super::{SimpleBacklightAnimator, SimpleBacklightDevice, SimpleBacklightDriver};

    pub(super) static SIMPLE_BACKLIGHT_CONFIG_STATE_LISTENER: Signal<RawMutex, ()> = Signal::new();
    pub(super) static SIMPLE_BACKLIGHT_SAVE_SIGNAL: Signal<RawMutex, ()> = Signal::new();

    pub struct SimpleBacklightStorage<A, D> {
        pub(super) _driver_phantom: core::marker::PhantomData<A>,
        pub(super) _device_phantom: core::marker::PhantomData<D>,
    }

    impl<D: SimpleBacklightDevice, R: SimpleBacklightDriver<D>> crate::lighting::AnimatorStorage
        for SimpleBacklightStorage<D, R>
    {
        type Animator = SimpleBacklightAnimator<D, R>;

        const STORAGE_KEY: crate::storage::StorageKey =
            crate::storage::StorageKey::SimpleBacklightConfig;

        #[inline(always)]
        fn get_state_listener() -> &'static Signal<RawMutex, ()> {
            D::get_state_listener()
        }

        #[inline(always)]
        fn get_save_signal() -> &'static Signal<RawMutex, ()> {
            D::get_save_signal()
        }
    }
}
