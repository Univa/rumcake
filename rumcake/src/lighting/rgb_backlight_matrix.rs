use core::fmt::Debug;

use defmt::{error, warn, Debug2Format};
use embassy_sync::channel::Channel;
use keyberon::layout::Event;
use num_derive::FromPrimitive;
use postcard::experimental::max_size::MaxSize;
use rand::rngs::SmallRng;
use rand_core::SeedableRng;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rumcake_macros::{generate_items_from_enum_variants, Cycle, LEDEffect};
use serde::{Deserialize, Serialize};
use smart_leds::hsv::Hsv;
use smart_leds::RGB8;

use crate::hw::platform::RawMutex;
use crate::lighting::{get_led_layout_bounds, Animator, BacklightMatrixDevice, LayoutBounds};
use crate::{Cycle, LEDEffect, State};

/// A trait that keyboards must implement to use backlight features.
pub trait RGBBacklightMatrixDevice: BacklightMatrixDevice {
    /// How fast the LEDs refresh to display a new animation frame.
    ///
    /// It is recommended to set this value to a value that your driver can handle,
    /// otherwise your animations will appear to be slowed down.
    ///
    /// **This does not have any effect if the selected animation is static.**
    const FPS: usize = 20;

    /// Get a reference to a channel that can receive commands to control the underglow animator
    /// from other tasks.
    #[inline(always)]
    fn get_command_channel() -> &'static Channel<RawMutex, RGBBacklightMatrixCommand, 2> {
        static RGB_BACKLIGHT_MATRIX_COMMAND_CHANNEL: Channel<
            RawMutex,
            RGBBacklightMatrixCommand,
            2,
        > = Channel::new();

        &RGB_BACKLIGHT_MATRIX_COMMAND_CHANNEL
    }

    /// Get a reference to a state object that can be used to notify other tasks about changes to
    /// the underglow configuration. Note that updating the state object will not control the
    /// output of the underglow animator.
    #[inline(always)]
    fn get_state() -> &'static State<'static, RGBBacklightMatrixConfig> {
        static RGB_BACKLIGHT_MATRIX_CONFIG_STATE: State<RGBBacklightMatrixConfig> = State::new(
            RGBBacklightMatrixConfig::default(),
            &[
                #[cfg(feature = "storage")]
                &RGB_BACKLIGHT_MATRIX_CONFIG_STATE_LISTENER,
            ],
        );

        &RGB_BACKLIGHT_MATRIX_CONFIG_STATE
    }

    #[cfg(feature = "storage")]
    #[inline(always)]
    fn get_state_listener() -> &'static embassy_sync::signal::Signal<RawMutex, ()> {
        &RGB_BACKLIGHT_MATRIX_CONFIG_STATE_LISTENER
    }

    #[cfg(feature = "storage")]
    #[inline(always)]
    fn get_save_signal() -> &'static embassy_sync::signal::Signal<RawMutex, ()> {
        &RGB_BACKLIGHT_MATRIX_SAVE_SIGNAL
    }

    #[cfg(feature = "split-central")]
    type CentralDevice: crate::split::central::private::MaybeCentralDevice =
        crate::split::central::private::EmptyCentralDevice;

    rgb_backlight_matrix_effect_items!();
}

pub(crate) mod private {
    use embassy_sync::channel::Channel;

    use crate::hw::platform::RawMutex;
    use crate::lighting::BacklightMatrixDevice;
    use crate::State;

    use super::{
        RGBBacklightMatrixCommand, RGBBacklightMatrixConfig, RGBBacklightMatrixDevice,
        RGBBacklightMatrixEffect,
    };

    pub trait MaybeRGBBacklightMatrixDevice: BacklightMatrixDevice {
        #[inline(always)]
        fn get_command_channel() -> Option<&'static Channel<RawMutex, RGBBacklightMatrixCommand, 2>>
        {
            None
        }

        #[inline(always)]
        fn get_state() -> Option<&'static State<'static, RGBBacklightMatrixConfig>> {
            None
        }

        #[inline(always)]
        fn is_effect_enabled(_effect: RGBBacklightMatrixEffect) -> bool {
            false
        }
    }

    impl<T: RGBBacklightMatrixDevice> MaybeRGBBacklightMatrixDevice for T {
        #[inline(always)]
        fn get_command_channel() -> Option<&'static Channel<RawMutex, RGBBacklightMatrixCommand, 2>>
        {
            Some(T::get_command_channel())
        }

        #[inline(always)]
        fn get_state() -> Option<&'static State<'static, RGBBacklightMatrixConfig>> {
            Some(T::get_state())
        }

        #[inline(always)]
        fn is_effect_enabled(effect: RGBBacklightMatrixEffect) -> bool {
            effect.is_enabled::<T>()
        }
    }
}

/// A trait that a driver must implement in order to support an RGB backlighting matrix scheme.
pub trait RGBBacklightMatrixDriver<K: RGBBacklightMatrixDevice> {
    /// The type of error that the driver will return if [`RGBBacklightMatrixDriver::write`] fails.
    type DriverWriteError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(
        &mut self,
        buf: &[[RGB8; K::LIGHTING_COLS]; K::LIGHTING_ROWS],
    ) -> Result<(), Self::DriverWriteError>;

    /// The type of error that the driver will return if [`RGBBacklightMatrixDriver::turn_on`] fails.
    type DriverEnableError: Debug;

    /// Turn the LEDs on using the driver when the animator gets enabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this, and subsequently [`RGBBacklightMatrixDriver::write`]. So, if your
    /// driver doesn't need do anything special to turn the LEDs on, you may simply return
    /// `Ok(())`.
    async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError>;

    /// The type of error that the driver will return if [`RGBBacklightMatrixDriver::turn_off`] fails.
    type DriverDisableError: Debug;

    /// Turn the LEDs off using the driver when the animator is disabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this. However, the tick method will not call
    /// [`RGBBacklightMatrixDriver::write`] due to the animator being disabled, so you will need to
    /// turn off the LEDs somehow. For example, you can write a brightness of 0 to all LEDs.
    async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError>;
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, MaxSize)]
pub struct RGBBacklightMatrixConfig {
    pub enabled: bool,
    pub effect: RGBBacklightMatrixEffect,
    pub hue: u8,
    pub sat: u8,
    pub val: u8,
    pub speed: u8,
}

impl RGBBacklightMatrixConfig {
    pub const fn default() -> Self {
        RGBBacklightMatrixConfig {
            enabled: true,
            effect: RGBBacklightMatrixEffect::Solid,
            hue: 0,
            sat: 255,
            val: 255,
            speed: 86,
        }
    }
}

impl Default for RGBBacklightMatrixConfig {
    fn default() -> Self {
        Self::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, MaxSize)]
#[non_exhaustive]
#[repr(u8)]
pub enum RGBBacklightMatrixCommand {
    Toggle = 0,
    TurnOn = 1,
    TurnOff = 2,
    NextEffect = 3,
    PrevEffect = 4,
    SetEffect(RGBBacklightMatrixEffect) = 5,
    SetHue(u8) = 6,
    IncreaseHue(u8) = 7,
    DecreaseHue(u8) = 8,
    SetSaturation(u8) = 9,
    IncreaseSaturation(u8) = 10,
    DecreaseSaturation(u8) = 11,
    SetValue(u8) = 12,
    IncreaseValue(u8) = 13,
    DecreaseValue(u8) = 14,
    SetSpeed(u8) = 15,
    IncreaseSpeed(u8) = 16,
    DecreaseSpeed(u8) = 17,
    #[cfg(feature = "storage")]
    SaveConfig = 18,
    ResetTime = 19, // normally used internally for syncing LEDs for split keyboards
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
pub enum RGBBacklightMatrixEffect {
    Solid,
    AlphasMods,
    GradientUpDown,
    GradientLeftRight,

    #[animated]
    Breathing,

    #[animated]
    ColorbandSat,

    #[animated]
    ColorbandVal,

    #[animated]
    ColorbandPinWheelSat,

    #[animated]
    ColorbandPinWheelVal,

    #[animated]
    ColorbandSpiralSat,

    #[animated]
    ColorbandSpiralVal,

    #[animated]
    CycleAll,

    #[animated]
    CycleLeftRight,

    #[animated]
    CycleUpDown,

    #[animated]
    RainbowMovingChevron,

    #[animated]
    CycleOutIn,

    #[animated]
    CycleOutInDual,

    #[animated]
    CyclePinWheel,

    #[animated]
    CycleSpiral,

    #[animated]
    DualBeacon,

    #[animated]
    RainbowBeacon,

    #[animated]
    RainbowPinWheels,

    #[animated]
    Raindrops,

    #[animated]
    JellybeanRaindrops,

    #[animated]
    HueBreathing,

    #[animated]
    HuePendulum,

    #[animated]
    HueWave,

    #[animated]
    PixelRain,

    #[animated]
    PixelFlow,

    #[animated]
    PixelFractal,

    #[animated]
    TypingHeatmap,

    #[animated]
    DigitalRain,

    #[animated]
    #[reactive]
    SolidReactiveSimple,

    #[animated]
    #[reactive]
    SolidReactive,

    #[animated]
    #[reactive]
    SolidReactiveWide,

    #[animated]
    #[reactive]
    SolidReactiveMultiWide,

    #[animated]
    #[reactive]
    SolidReactiveCross,

    #[animated]
    #[reactive]
    SolidReactiveMultiCross,

    #[animated]
    #[reactive]
    SolidReactiveNexus,

    #[animated]
    #[reactive]
    SolidReactiveMultiNexus,

    #[animated]
    #[reactive]
    Splash,

    #[animated]
    #[reactive]
    MultiSplash,

    #[animated]
    #[reactive]
    SolidSplash,

    #[animated]
    #[reactive]
    SolidMultiSplash,

    #[cfg(feature = "vial")]
    #[animated]
    DirectSet,
}

impl RGBBacklightMatrixEffect {
    pub(crate) fn is_enabled<D: RGBBacklightMatrixDevice>(&self) -> bool {
        match self {
            RGBBacklightMatrixEffect::Solid => D::SOLID_ENABLED,
            RGBBacklightMatrixEffect::AlphasMods => D::ALPHAS_MODS_ENABLED,
            RGBBacklightMatrixEffect::GradientUpDown => D::GRADIENT_UP_DOWN_ENABLED,
            RGBBacklightMatrixEffect::GradientLeftRight => D::GRADIENT_LEFT_RIGHT_ENABLED,
            RGBBacklightMatrixEffect::Breathing => D::BREATHING_ENABLED,
            RGBBacklightMatrixEffect::ColorbandSat => D::COLORBAND_SAT_ENABLED,
            RGBBacklightMatrixEffect::ColorbandVal => D::COLORBAND_VAL_ENABLED,
            RGBBacklightMatrixEffect::ColorbandPinWheelSat => D::COLORBAND_PIN_WHEEL_SAT_ENABLED,
            RGBBacklightMatrixEffect::ColorbandPinWheelVal => D::COLORBAND_PIN_WHEEL_VAL_ENABLED,
            RGBBacklightMatrixEffect::ColorbandSpiralSat => D::COLORBAND_SPIRAL_SAT_ENABLED,
            RGBBacklightMatrixEffect::ColorbandSpiralVal => D::COLORBAND_SPIRAL_VAL_ENABLED,
            RGBBacklightMatrixEffect::CycleAll => D::CYCLE_ALL_ENABLED,
            RGBBacklightMatrixEffect::CycleLeftRight => D::CYCLE_LEFT_RIGHT_ENABLED,
            RGBBacklightMatrixEffect::CycleUpDown => D::CYCLE_UP_DOWN_ENABLED,
            RGBBacklightMatrixEffect::RainbowMovingChevron => D::RAINBOW_MOVING_CHEVRON_ENABLED,
            RGBBacklightMatrixEffect::CycleOutIn => D::CYCLE_OUT_IN_ENABLED,
            RGBBacklightMatrixEffect::CycleOutInDual => D::CYCLE_OUT_IN_DUAL_ENABLED,
            RGBBacklightMatrixEffect::CyclePinWheel => D::CYCLE_PIN_WHEEL_ENABLED,
            RGBBacklightMatrixEffect::CycleSpiral => D::CYCLE_SPIRAL_ENABLED,
            RGBBacklightMatrixEffect::DualBeacon => D::DUAL_BEACON_ENABLED,
            RGBBacklightMatrixEffect::RainbowBeacon => D::RAINBOW_BEACON_ENABLED,
            RGBBacklightMatrixEffect::RainbowPinWheels => D::RAINBOW_PIN_WHEELS_ENABLED,
            RGBBacklightMatrixEffect::Raindrops => D::RAINDROPS_ENABLED,
            RGBBacklightMatrixEffect::JellybeanRaindrops => D::JELLYBEAN_RAINDROPS_ENABLED,
            RGBBacklightMatrixEffect::HueBreathing => D::HUE_BREATHING_ENABLED,
            RGBBacklightMatrixEffect::HuePendulum => D::HUE_PENDULUM_ENABLED,
            RGBBacklightMatrixEffect::HueWave => D::HUE_WAVE_ENABLED,
            RGBBacklightMatrixEffect::PixelRain => D::PIXEL_RAIN_ENABLED,
            RGBBacklightMatrixEffect::PixelFlow => D::PIXEL_FLOW_ENABLED,
            RGBBacklightMatrixEffect::PixelFractal => D::PIXEL_FRACTAL_ENABLED,
            RGBBacklightMatrixEffect::TypingHeatmap => D::TYPING_HEATMAP_ENABLED,
            RGBBacklightMatrixEffect::DigitalRain => D::DIGITAL_RAIN_ENABLED,
            RGBBacklightMatrixEffect::SolidReactiveSimple => D::SOLID_REACTIVE_SIMPLE_ENABLED,
            RGBBacklightMatrixEffect::SolidReactive => D::SOLID_REACTIVE_ENABLED,
            RGBBacklightMatrixEffect::SolidReactiveWide => D::SOLID_REACTIVE_WIDE_ENABLED,
            RGBBacklightMatrixEffect::SolidReactiveMultiWide => {
                D::SOLID_REACTIVE_MULTI_WIDE_ENABLED
            }
            RGBBacklightMatrixEffect::SolidReactiveCross => D::SOLID_REACTIVE_CROSS_ENABLED,
            RGBBacklightMatrixEffect::SolidReactiveMultiCross => {
                D::SOLID_REACTIVE_MULTI_CROSS_ENABLED
            }
            RGBBacklightMatrixEffect::SolidReactiveNexus => D::SOLID_REACTIVE_NEXUS_ENABLED,
            RGBBacklightMatrixEffect::SolidReactiveMultiNexus => {
                D::SOLID_REACTIVE_MULTI_NEXUS_ENABLED
            }
            RGBBacklightMatrixEffect::Splash => D::SPLASH_ENABLED,
            RGBBacklightMatrixEffect::MultiSplash => D::MULTI_SPLASH_ENABLED,
            RGBBacklightMatrixEffect::SolidSplash => D::SOLID_SPLASH_ENABLED,
            RGBBacklightMatrixEffect::SolidMultiSplash => D::SOLID_MULTI_SPLASH_ENABLED,
            #[cfg(feature = "vial")]
            RGBBacklightMatrixEffect::DirectSet => D::DIRECT_SET_ENABLED,
        }
    }
}

pub struct RGBBacklightMatrixAnimator<K: RGBBacklightMatrixDevice, D: RGBBacklightMatrixDriver<K>>
where
    [(); K::LIGHTING_COLS]:,
    [(); K::LIGHTING_ROWS]:,
{
    config: RGBBacklightMatrixConfig,
    buf: [[RGB8; K::LIGHTING_COLS]; K::LIGHTING_ROWS], // Stores the brightness/value of each LED
    last_presses: ConstGenericRingBuffer<((u8, u8), u32), 8>, // Stores the row and col of the last 8 key presses, and the time (in ticks) it was pressed
    tick: u32,
    driver: D,
    bounds: LayoutBounds,
    rng: SmallRng,
}

impl<D: RGBBacklightMatrixDevice + 'static, R: RGBBacklightMatrixDriver<D>>
    RGBBacklightMatrixAnimator<D, R>
where
    [(); D::LIGHTING_COLS]:,
    [(); D::LIGHTING_ROWS]:,
{
    pub fn new(config: RGBBacklightMatrixConfig, driver: R) -> Self {
        Self {
            config,
            tick: 0,
            driver,
            last_presses: ConstGenericRingBuffer::new(),
            buf: [[RGB8::new(0, 0, 0); D::LIGHTING_COLS]; D::LIGHTING_ROWS],
            bounds: get_led_layout_bounds::<D>(),
            rng: SmallRng::seed_from_u64(1337),
        }
    }

    pub async fn turn_on(&mut self) {
        if let Err(err) = self.driver.turn_on().await {
            warn!("[BACKLIGHT] Animations have been enabled, but the backlight LEDs could not be turned on: {}", Debug2Format(&err));
        };
    }

    pub async fn turn_off(&mut self) {
        if let Err(err) = self.driver.turn_off().await {
            warn!("[BACKLIGHT] Animations have been disabled, but the backlight LEDs could not be turned off: {}", Debug2Format(&err));
        };
    }

    pub fn process_command(&mut self, command: RGBBacklightMatrixCommand) {
        match command {
            RGBBacklightMatrixCommand::Toggle => {
                self.config.enabled = !self.config.enabled;
            }
            RGBBacklightMatrixCommand::TurnOn => {
                self.config.enabled = true;
            }
            RGBBacklightMatrixCommand::TurnOff => {
                self.config.enabled = false;
            }
            RGBBacklightMatrixCommand::NextEffect => {
                while {
                    self.config.effect.increment();
                    !self.config.effect.is_enabled::<D>()
                } {}
            }
            RGBBacklightMatrixCommand::PrevEffect => {
                while {
                    self.config.effect.decrement();
                    !self.config.effect.is_enabled::<D>()
                } {}
            }
            RGBBacklightMatrixCommand::SetEffect(effect) => {
                self.config.effect = effect;
            }
            RGBBacklightMatrixCommand::SetHue(hue) => {
                self.config.hue = hue;
            }
            RGBBacklightMatrixCommand::IncreaseHue(amount) => {
                self.config.hue = self.config.hue.saturating_add(amount);
            }
            RGBBacklightMatrixCommand::DecreaseHue(amount) => {
                self.config.hue = self.config.hue.saturating_sub(amount);
            }
            RGBBacklightMatrixCommand::SetSaturation(sat) => {
                self.config.sat = sat;
            }
            RGBBacklightMatrixCommand::IncreaseSaturation(amount) => {
                self.config.sat = self.config.sat.saturating_add(amount);
            }
            RGBBacklightMatrixCommand::DecreaseSaturation(amount) => {
                self.config.sat = self.config.sat.saturating_sub(amount);
            }
            RGBBacklightMatrixCommand::SetValue(val) => {
                self.config.val = val;
            }
            RGBBacklightMatrixCommand::IncreaseValue(amount) => {
                self.config.val = self.config.val.saturating_add(amount);
            }
            RGBBacklightMatrixCommand::DecreaseValue(amount) => {
                self.config.val = self.config.val.saturating_sub(amount);
            }
            RGBBacklightMatrixCommand::SetSpeed(speed) => {
                self.config.speed = speed;
            }
            RGBBacklightMatrixCommand::IncreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_add(amount);
            }
            RGBBacklightMatrixCommand::DecreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_sub(amount);
            }
            #[cfg(feature = "storage")]
            RGBBacklightMatrixCommand::SaveConfig => {
                // storage::BACKLIGHT_SAVE_SIGNAL.signal(());
            }
            RGBBacklightMatrixCommand::ResetTime => {
                self.tick = 0;
            }
        };
    }

    pub fn set_brightness_for_each_led(&mut self, calc: impl Fn(&mut Self, f32, u8) -> Hsv) {
        unimplemented!()
    }

    pub fn register_event(&mut self, event: Event) {
        match event {
            Event::Press(row, col) => {
                match self
                    .last_presses
                    .iter_mut()
                    .find(|((pressed_row, pressed_col), _time)| {
                        *pressed_row == row && *pressed_col == col
                    }) {
                    Some(press) => {
                        press.1 = self.tick;
                    }
                    None => {
                        // Check if the matrix position corresponds to a LED position before pushing
                        if D::get_backlight_matrix()
                            .layout
                            .get(row as usize)
                            .and_then(|row| row.get(col as usize))
                            .and_then(|pos| *pos)
                            .is_some()
                        {
                            self.last_presses.push(((row, col), self.tick));
                        }
                    }
                };
            }
            Event::Release(_row, _col) => {} // nothing for now. maybe change some effects to behave depending on the state of a key.
        }
    }

    pub async fn tick(&mut self) {
        if !self.config.enabled {
            return;
        }

        // TODO: animations
        match self.config.effect {
            RGBBacklightMatrixEffect::Solid => todo!(),
            RGBBacklightMatrixEffect::AlphasMods => todo!(),
            RGBBacklightMatrixEffect::GradientUpDown => todo!(),
            RGBBacklightMatrixEffect::GradientLeftRight => todo!(),
            RGBBacklightMatrixEffect::Breathing => todo!(),
            RGBBacklightMatrixEffect::ColorbandSat => todo!(),
            RGBBacklightMatrixEffect::ColorbandVal => todo!(),
            RGBBacklightMatrixEffect::ColorbandPinWheelSat => todo!(),
            RGBBacklightMatrixEffect::ColorbandPinWheelVal => todo!(),
            RGBBacklightMatrixEffect::ColorbandSpiralSat => todo!(),
            RGBBacklightMatrixEffect::ColorbandSpiralVal => todo!(),
            RGBBacklightMatrixEffect::CycleAll => todo!(),
            RGBBacklightMatrixEffect::CycleLeftRight => todo!(),
            RGBBacklightMatrixEffect::CycleUpDown => todo!(),
            RGBBacklightMatrixEffect::RainbowMovingChevron => todo!(),
            RGBBacklightMatrixEffect::CycleOutIn => todo!(),
            RGBBacklightMatrixEffect::CycleOutInDual => todo!(),
            RGBBacklightMatrixEffect::CyclePinWheel => todo!(),
            RGBBacklightMatrixEffect::CycleSpiral => todo!(),
            RGBBacklightMatrixEffect::DualBeacon => todo!(),
            RGBBacklightMatrixEffect::RainbowBeacon => todo!(),
            RGBBacklightMatrixEffect::RainbowPinWheels => todo!(),
            RGBBacklightMatrixEffect::Raindrops => todo!(),
            RGBBacklightMatrixEffect::JellybeanRaindrops => todo!(),
            RGBBacklightMatrixEffect::HueBreathing => todo!(),
            RGBBacklightMatrixEffect::HuePendulum => todo!(),
            RGBBacklightMatrixEffect::HueWave => todo!(),
            RGBBacklightMatrixEffect::PixelRain => todo!(),
            RGBBacklightMatrixEffect::PixelFlow => todo!(),
            RGBBacklightMatrixEffect::PixelFractal => todo!(),
            RGBBacklightMatrixEffect::TypingHeatmap => todo!(),
            RGBBacklightMatrixEffect::DigitalRain => todo!(),
            RGBBacklightMatrixEffect::SolidReactiveSimple => todo!(),
            RGBBacklightMatrixEffect::SolidReactive => todo!(),
            RGBBacklightMatrixEffect::SolidReactiveWide => todo!(),
            RGBBacklightMatrixEffect::SolidReactiveMultiWide => todo!(),
            RGBBacklightMatrixEffect::SolidReactiveCross => todo!(),
            RGBBacklightMatrixEffect::SolidReactiveMultiCross => todo!(),
            RGBBacklightMatrixEffect::SolidReactiveNexus => todo!(),
            RGBBacklightMatrixEffect::SolidReactiveMultiNexus => todo!(),
            RGBBacklightMatrixEffect::Splash => todo!(),
            RGBBacklightMatrixEffect::MultiSplash => todo!(),
            RGBBacklightMatrixEffect::SolidSplash => todo!(),
            RGBBacklightMatrixEffect::SolidMultiSplash => todo!(),
            #[cfg(feature = "vial")]
            RGBBacklightMatrixEffect::DirectSet => {} // We just move onto calling the driver, since the frame buffer is updated by the backlight task
        }

        if let Err(err) = self.driver.write(&self.buf).await {
            error!(
                "[BACKLIGHT] Couldn't update backlight colors: {}",
                Debug2Format(&err)
            );
        };

        self.tick += 1;
    }

    #[cfg(feature = "storage")]
    pub fn create_storage_instance(&self) -> RGBBacklightMatrixStorage<D, R> {
        RGBBacklightMatrixStorage {
            _device_phantom: core::marker::PhantomData,
            _driver_phantom: core::marker::PhantomData,
        }
    }
}

impl<D: RGBBacklightMatrixDevice + 'static, R: RGBBacklightMatrixDriver<D>> Animator
    for RGBBacklightMatrixAnimator<D, R>
where
    [(); D::LIGHTING_COLS]:,
    [(); D::LIGHTING_ROWS]:,
{
    type CommandType = RGBBacklightMatrixCommand;

    type ConfigType = RGBBacklightMatrixConfig;

    type BufferUpdateArgs = (u8, RGB8);

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
                    .send(crate::split::MessageToPeripheral::RGBBacklightMatrix(
                        RGBBacklightMatrixCommand::ResetTime,
                    ))
                    .await;
                channel
                    .send(crate::split::MessageToPeripheral::RGBBacklightMatrix(
                        RGBBacklightMatrixCommand::SetEffect(self.config.effect),
                    ))
                    .await;
                channel
                    .send(crate::split::MessageToPeripheral::RGBBacklightMatrix(
                        RGBBacklightMatrixCommand::SetValue(self.config.val),
                    ))
                    .await;
                channel
                    .send(crate::split::MessageToPeripheral::RGBBacklightMatrix(
                        RGBBacklightMatrixCommand::SetSpeed(self.config.speed),
                    ))
                    .await;
            }
        }
    }

    fn update_buffer(&mut self, (led, color): Self::BufferUpdateArgs) {
        let col = led as usize % D::LIGHTING_COLS;
        let row = led as usize / D::LIGHTING_COLS % D::LIGHTING_ROWS;
        self.buf[row][col] = color;
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
use storage::*;

#[cfg(feature = "storage")]
mod storage {
    use embassy_sync::signal::Signal;

    use crate::hw::platform::RawMutex;

    use super::{RGBBacklightMatrixAnimator, RGBBacklightMatrixDevice, RGBBacklightMatrixDriver};

    pub(super) static RGB_BACKLIGHT_MATRIX_CONFIG_STATE_LISTENER: Signal<RawMutex, ()> =
        Signal::new();
    pub(super) static RGB_BACKLIGHT_MATRIX_SAVE_SIGNAL: Signal<RawMutex, ()> = Signal::new();

    pub struct RGBBacklightMatrixStorage<A, D> {
        pub(super) _driver_phantom: core::marker::PhantomData<A>,
        pub(super) _device_phantom: core::marker::PhantomData<D>,
    }

    impl<D: RGBBacklightMatrixDevice + 'static, R: RGBBacklightMatrixDriver<D>>
        crate::lighting::AnimatorStorage for RGBBacklightMatrixStorage<D, R>
    where
        [(); D::LIGHTING_COLS]:,
        [(); D::LIGHTING_ROWS]:,
    {
        type Animator = RGBBacklightMatrixAnimator<D, R>;

        const STORAGE_KEY: crate::storage::StorageKey =
            crate::storage::StorageKey::RGBBacklightMatrixConfig;

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
