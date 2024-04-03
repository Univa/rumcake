use core::f32::consts::PI;
use core::fmt::Debug;
use core::u8;

use defmt::{error, warn, Debug2Format};
use embassy_sync::channel::Channel;
use keyberon::layout::Event;
use num_derive::FromPrimitive;
use postcard::experimental::max_size::MaxSize;
use rand::rngs::SmallRng;
use rand_core::{RngCore, SeedableRng};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rumcake_macros::{generate_items_from_enum_variants, Cycle, LEDEffect};
use serde::{Deserialize, Serialize};

use crate::hw::platform::RawMutex;
use crate::lighting::{
    get_led_layout_bounds, Animator, BacklightMatrixDevice, LEDFlags, LayoutBounds,
};
use crate::math::{atan2f, cos, scale, sin, sqrtf};
use crate::{Cycle, LEDEffect, State};

/// A trait that keyboards must implement to use backlight features.
pub trait SimpleBacklightMatrixDevice: BacklightMatrixDevice {
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
    fn get_command_channel() -> &'static Channel<RawMutex, SimpleBacklightMatrixCommand, 2> {
        static SIMPLE_BACKLIGHT_MATRIX_COMMAND_CHANNEL: Channel<
            RawMutex,
            SimpleBacklightMatrixCommand,
            2,
        > = Channel::new();

        &SIMPLE_BACKLIGHT_MATRIX_COMMAND_CHANNEL
    }

    /// Get a reference to a state object that can be used to notify other tasks about changes to
    /// the underglow configuration. Note that updating the state object will not control the
    /// output of the underglow animator.
    #[inline(always)]
    fn get_state() -> &'static State<'static, SimpleBacklightMatrixConfig> {
        static SIMPLE_BACKLIGHT_MATRIX_CONFIG_STATE: State<SimpleBacklightMatrixConfig> =
            State::new(
                SimpleBacklightMatrixConfig::default(),
                &[
                    #[cfg(feature = "storage")]
                    &SIMPLE_BACKLIGHT_MATRIX_CONFIG_STATE_LISTENER,
                ],
            );

        &SIMPLE_BACKLIGHT_MATRIX_CONFIG_STATE
    }

    #[cfg(feature = "storage")]
    #[inline(always)]
    fn get_state_listener() -> &'static embassy_sync::signal::Signal<RawMutex, ()> {
        &SIMPLE_BACKLIGHT_MATRIX_CONFIG_STATE_LISTENER
    }

    #[cfg(feature = "storage")]
    #[inline(always)]
    fn get_save_signal() -> &'static embassy_sync::signal::Signal<RawMutex, ()> {
        &SIMPLE_BACKLIGHT_MATRIX_SAVE_SIGNAL
    }

    #[cfg(feature = "split-central")]
    type CentralDevice: crate::split::central::private::MaybeCentralDevice =
        crate::split::central::private::EmptyCentralDevice;

    simple_backlight_matrix_effect_items!();
}

pub(crate) mod private {
    use embassy_sync::channel::Channel;

    use crate::hw::platform::RawMutex;
    use crate::lighting::BacklightMatrixDevice;
    use crate::State;

    use super::{
        SimpleBacklightMatrixCommand, SimpleBacklightMatrixConfig, SimpleBacklightMatrixDevice,
    };

    pub trait MaybeSimpleBacklightMatrixDevice: BacklightMatrixDevice {
        #[inline(always)]
        fn get_command_channel(
        ) -> Option<&'static Channel<RawMutex, SimpleBacklightMatrixCommand, 2>> {
            None
        }

        #[inline(always)]
        fn get_state() -> Option<&'static State<'static, SimpleBacklightMatrixConfig>> {
            None
        }
    }

    impl<T: SimpleBacklightMatrixDevice> MaybeSimpleBacklightMatrixDevice for T {
        #[inline(always)]
        fn get_command_channel(
        ) -> Option<&'static Channel<RawMutex, SimpleBacklightMatrixCommand, 2>> {
            Some(T::get_command_channel())
        }

        #[inline(always)]
        fn get_state() -> Option<&'static State<'static, SimpleBacklightMatrixConfig>> {
            Some(T::get_state())
        }
    }
}

/// A trait that a driver must implement in order to support a simple (no color) backlighting matrix scheme.
pub trait SimpleBacklightMatrixDriver<K: SimpleBacklightMatrixDevice> {
    /// The type of error that the driver will return if [`SimpleBacklightMatrixDriver::write`] fails.
    type DriverWriteError: Debug;

    /// Render out a frame buffer using the driver.
    async fn write(
        &mut self,
        buf: &[[u8; K::LIGHTING_COLS]; K::LIGHTING_ROWS],
    ) -> Result<(), Self::DriverWriteError>;

    /// The type of error that the driver will return if [`SimpleBacklightMatrixDriver::turn_on`] fails.
    type DriverEnableError: Debug;

    /// Turn the LEDs on using the driver when the animator gets enabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this, and subsequently [`SimpleBacklightMatrixDriver::write`]. So, if your
    /// driver doesn't need do anything special to turn the LEDs on, you may simply return
    /// `Ok(())`.
    async fn turn_on(&mut self) -> Result<(), Self::DriverEnableError>;

    /// The type of error that the driver will return if [`SimpleBacklightMatrixDriver::turn_off`] fails.
    type DriverDisableError: Debug;

    /// Turn the LEDs off using the driver when the animator is disabled.
    ///
    /// The animator's [`tick()`](super::animations::BacklightAnimator::tick) method gets called
    /// directly after this. However, the tick method will not call
    /// [`SimpleBacklightMatrixDriver::write`] due to the animator being disabled, so you will need to
    /// turn off the LEDs somehow. For example, you can write a brightness of 0 to all LEDs.
    async fn turn_off(&mut self) -> Result<(), Self::DriverDisableError>;
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, MaxSize)]
pub struct SimpleBacklightMatrixConfig {
    pub enabled: bool,
    pub effect: SimpleBacklightMatrixEffect,
    pub val: u8,
    pub speed: u8,
}

impl SimpleBacklightMatrixConfig {
    pub const fn default() -> Self {
        SimpleBacklightMatrixConfig {
            enabled: true,
            effect: SimpleBacklightMatrixEffect::Solid,
            val: 255,
            speed: 86,
        }
    }
}

impl Default for SimpleBacklightMatrixConfig {
    fn default() -> Self {
        Self::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, MaxSize)]
#[non_exhaustive]
#[repr(u8)]
pub enum SimpleBacklightMatrixCommand {
    Toggle = 0,
    TurnOn = 1,
    TurnOff = 2,
    NextEffect = 3,
    PrevEffect = 4,
    SetEffect(SimpleBacklightMatrixEffect) = 5,
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
pub enum SimpleBacklightMatrixEffect {
    Solid,
    AlphasMods,
    GradientUpDown,
    GradientLeftRight,

    #[animated]
    Breathing,

    #[animated]
    Band,

    #[animated]
    BandPinWheel,

    #[animated]
    BandSpiral,

    #[animated]
    CycleLeftRight,

    #[animated]
    CycleUpDown,

    #[animated]
    CycleOutIn,

    #[animated]
    Raindrops,

    #[animated]
    DualBeacon,

    #[animated]
    WaveLeftRight,

    #[animated]
    WaveUpDown,

    #[animated]
    #[reactive]
    Reactive,

    #[animated]
    #[reactive]
    ReactiveWide,

    #[animated]
    #[reactive]
    ReactiveCross,

    #[animated]
    #[reactive]
    ReactiveNexus,

    #[animated]
    #[reactive]
    ReactiveSplash,
}

impl SimpleBacklightMatrixEffect {
    pub(crate) fn is_enabled<D: SimpleBacklightMatrixDevice>(&self) -> bool {
        match self {
            SimpleBacklightMatrixEffect::Solid => D::SOLID_ENABLED,
            SimpleBacklightMatrixEffect::AlphasMods => D::ALPHAS_MODS_ENABLED,
            SimpleBacklightMatrixEffect::GradientUpDown => D::GRADIENT_UP_DOWN_ENABLED,
            SimpleBacklightMatrixEffect::GradientLeftRight => D::GRADIENT_LEFT_RIGHT_ENABLED,
            SimpleBacklightMatrixEffect::Breathing => D::BREATHING_ENABLED,
            SimpleBacklightMatrixEffect::Band => D::BAND_ENABLED,
            SimpleBacklightMatrixEffect::BandPinWheel => D::BAND_PIN_WHEEL_ENABLED,
            SimpleBacklightMatrixEffect::BandSpiral => D::BAND_SPIRAL_ENABLED,
            SimpleBacklightMatrixEffect::CycleLeftRight => D::CYCLE_LEFT_RIGHT_ENABLED,
            SimpleBacklightMatrixEffect::CycleUpDown => D::CYCLE_UP_DOWN_ENABLED,
            SimpleBacklightMatrixEffect::CycleOutIn => D::CYCLE_OUT_IN_ENABLED,
            SimpleBacklightMatrixEffect::Raindrops => D::RAINDROPS_ENABLED,
            SimpleBacklightMatrixEffect::DualBeacon => D::DUAL_BEACON_ENABLED,
            SimpleBacklightMatrixEffect::WaveLeftRight => D::WAVE_LEFT_RIGHT_ENABLED,
            SimpleBacklightMatrixEffect::WaveUpDown => D::WAVE_UP_DOWN_ENABLED,
            SimpleBacklightMatrixEffect::Reactive => D::REACTIVE_ENABLED,
            SimpleBacklightMatrixEffect::ReactiveWide => D::REACTIVE_WIDE_ENABLED,
            SimpleBacklightMatrixEffect::ReactiveCross => D::REACTIVE_CROSS_ENABLED,
            SimpleBacklightMatrixEffect::ReactiveNexus => D::REACTIVE_NEXUS_ENABLED,
            SimpleBacklightMatrixEffect::ReactiveSplash => D::REACTIVE_SPLASH_ENABLED,
        }
    }
}

pub struct SimpleBacklightMatrixAnimator<
    D: SimpleBacklightMatrixDevice,
    R: SimpleBacklightMatrixDriver<D>,
> where
    [(); D::LIGHTING_COLS]:,
    [(); D::LIGHTING_ROWS]:,
{
    config: SimpleBacklightMatrixConfig,
    buf: [[u8; D::LIGHTING_COLS]; D::LIGHTING_ROWS], // Stores the brightness/value of each LED
    last_presses: ConstGenericRingBuffer<((u8, u8), u32), 8>, // Stores the row and col of the last 8 key presses, and the time (in ticks) it was pressed
    tick: u32,
    driver: R,
    bounds: LayoutBounds,
    rng: SmallRng,
}

impl<D: SimpleBacklightMatrixDevice + 'static, R: SimpleBacklightMatrixDriver<D>>
    SimpleBacklightMatrixAnimator<D, R>
where
    [(); D::LIGHTING_COLS]:,
    [(); D::LIGHTING_ROWS]:,
{
    pub fn new(config: SimpleBacklightMatrixConfig, driver: R) -> Self {
        Self {
            config,
            tick: 0,
            driver,
            buf: [[0; D::LIGHTING_COLS]; D::LIGHTING_ROWS],
            last_presses: ConstGenericRingBuffer::new(),
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

    pub fn process_command(&mut self, command: SimpleBacklightMatrixCommand) {
        match command {
            SimpleBacklightMatrixCommand::Toggle => {
                self.config.enabled = !self.config.enabled;
            }
            SimpleBacklightMatrixCommand::TurnOn => {
                self.config.enabled = true;
            }
            SimpleBacklightMatrixCommand::TurnOff => {
                self.config.enabled = false;
            }
            SimpleBacklightMatrixCommand::NextEffect => {
                while {
                    self.config.effect.increment();
                    !self.config.effect.is_enabled::<D>()
                } {}
            }
            SimpleBacklightMatrixCommand::PrevEffect => {
                while {
                    self.config.effect.decrement();
                    !self.config.effect.is_enabled::<D>()
                } {}
            }
            SimpleBacklightMatrixCommand::SetEffect(effect) => {
                self.config.effect = effect;
            }
            SimpleBacklightMatrixCommand::SetValue(val) => {
                self.config.val = val;
            }
            SimpleBacklightMatrixCommand::IncreaseValue(amount) => {
                self.config.val = self.config.val.saturating_add(amount);
            }
            SimpleBacklightMatrixCommand::DecreaseValue(amount) => {
                self.config.val = self.config.val.saturating_sub(amount);
            }
            SimpleBacklightMatrixCommand::SetSpeed(speed) => {
                self.config.speed = speed;
            }
            SimpleBacklightMatrixCommand::IncreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_add(amount);
            }
            SimpleBacklightMatrixCommand::DecreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_sub(amount);
            }
            #[cfg(feature = "storage")]
            SimpleBacklightMatrixCommand::SaveConfig => {
                // storage::BACKLIGHT_SAVE_SIGNAL.signal(());
            }
            SimpleBacklightMatrixCommand::ResetTime => {
                self.tick = 0;
            }
        };
    }

    pub fn set_brightness_for_each_led(
        &mut self,
        calc: impl Fn(&mut Self, u32, (u8, u8), (u8, u8)) -> u8,
    ) {
        let time = (self.tick << 8)
            / (((D::FPS as u32) << 8)
                / (self.config.speed as u32 + 128 + (self.config.speed as u32 >> 1))); // `time` should increment by 255 every second

        for row in 0..D::LIGHTING_ROWS {
            for col in 0..D::LIGHTING_COLS {
                if let Some(position) = D::get_backlight_matrix().layout[row][col] {
                    self.buf[row][col] = scale(
                        calc(self, time, (row as u8, col as u8), position),
                        self.config.val,
                    )
                }
            }
        }
    }

    pub fn register_event(&mut self, event: Event) {
        let time = (self.tick << 8)
            / (((D::FPS as u32) << 8)
                / (self.config.speed as u32 + 128 + (self.config.speed as u32 >> 1)));

        match event {
            Event::Press(row, col) => {
                match self
                    .last_presses
                    .iter_mut()
                    .find(|((pressed_row, pressed_col), _time)| {
                        *pressed_row == row && *pressed_col == col
                    }) {
                    Some(press) => {
                        press.1 = time;
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
                            self.last_presses.push(((row, col), time));
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

        match self.config.effect {
            SimpleBacklightMatrixEffect::Solid => {
                if D::SOLID_ENABLED {
                    self.set_brightness_for_each_led(|_animator, _time, _coord, _pos| u8::MAX)
                }
            }
            SimpleBacklightMatrixEffect::AlphasMods => {
                if D::ALPHAS_MODS_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, (row, col), _pos| {
                        if D::get_backlight_matrix().flags[row as usize][col as usize]
                            .contains(LEDFlags::ALPHA)
                        {
                            u8::MAX
                        } else {
                            animator.config.speed
                        }
                    })
                }
            }
            SimpleBacklightMatrixEffect::GradientUpDown => {
                if D::GRADIENT_UP_DOWN_ENABLED {
                    let size = self.bounds.max.1 - self.bounds.min.1;
                    self.set_brightness_for_each_led(|animator, _time, _coord, (_x, y)| {
                        // Calculate the brightness for each LED based on it's Y position
                        // Speed will be used to determine where the "peak" of the gradient is.
                        sin(
                            (((((y - animator.bounds.min.1) as u16) << 7) / size as u16) as u8)
                                .wrapping_add(64)
                                .wrapping_sub(animator.config.speed),
                        )
                    })
                }
            }
            SimpleBacklightMatrixEffect::GradientLeftRight => {
                if D::GRADIENT_LEFT_RIGHT_ENABLED {
                    let size = self.bounds.max.0 - self.bounds.min.0;
                    self.set_brightness_for_each_led(|animator, _time, _coord, (x, _y)| {
                        // Calculate the brightness for each LED based on it's X position
                        // Speed will be used to determine where the "peak" of the gradient is.
                        sin(
                            (((((x - animator.bounds.min.0) as u16) << 7) / size as u16) as u8)
                                .wrapping_add(64)
                                .wrapping_sub(animator.config.speed),
                        )
                    })
                }
            }
            SimpleBacklightMatrixEffect::Breathing => {
                if D::BREATHING_ENABLED {
                    self.set_brightness_for_each_led(|_animator, time, _coord, _pos| {
                        sin((time >> 2) as u8) // 4 seconds for one full cycle
                    })
                }
            }
            SimpleBacklightMatrixEffect::Band => {
                if D::BAND_ENABLED {
                    let size = self.bounds.max.0 - self.bounds.min.0;
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, _y)| {
                        let pos = scale(time as u8, size);
                        u8::MAX.saturating_sub(x.abs_diff(pos).saturating_mul(8))
                    })
                }
            }
            SimpleBacklightMatrixEffect::BandPinWheel => {
                if D::BAND_PIN_WHEEL_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, y)| {
                        // Base speed: 1 half-cycle every second
                        let pos = time as u8;
                        let dy = y as i16 - animator.bounds.mid.1 as i16;
                        let dx = x as i16 - animator.bounds.mid.0 as i16;
                        ((atan2f(dy as f32, dx as f32) * u8::MAX as f32 / PI) as i32) as u8 - pos
                    })
                }
            }
            SimpleBacklightMatrixEffect::BandSpiral => {
                if D::BAND_SPIRAL_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, y)| {
                        // Base speed: 1 half-cycle every second
                        let pos = time as u8;
                        let dy = y as i16 - animator.bounds.mid.1 as i16;
                        let dx = x as i16 - animator.bounds.mid.0 as i16;
                        let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32) as u16;
                        (((atan2f(dy as f32, dx as f32) * u8::MAX as f32 / PI) as i32) as u8)
                            .wrapping_add(dist as u8)
                            .wrapping_sub(pos)
                    })
                }
            }
            SimpleBacklightMatrixEffect::CycleLeftRight => {
                if D::CYCLE_LEFT_RIGHT_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, _y)| {
                        // Base speed: 1 cycle every second
                        (x - animator.bounds.min.0).wrapping_sub(time as u8)
                    })
                }
            }
            SimpleBacklightMatrixEffect::CycleUpDown => {
                if D::CYCLE_UP_DOWN_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (_x, y)| {
                        // Base speed: 1 cycle every second
                        (y - animator.bounds.min.1).wrapping_sub(time as u8)
                    })
                }
            }
            SimpleBacklightMatrixEffect::CycleOutIn => {
                if D::CYCLE_OUT_IN_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, y)| {
                        // Base speed: 1 cycle every second
                        let d = sqrtf(
                            ((x.abs_diff(animator.bounds.mid.0) as u16).pow(2)
                                + (y.abs_diff(animator.bounds.mid.1) as u16).pow(2))
                                as f32,
                        ) as u8;

                        u8::MAX.wrapping_sub(d).wrapping_sub(time as u8)
                    })
                }
            }
            SimpleBacklightMatrixEffect::Raindrops => {
                if D::RAINDROPS_ENABLED {
                    let adjusted_fps = (((D::FPS as u32) << 8)
                        / (self.config.speed as u32 + 128 + (self.config.speed as u32 >> 1)))
                        as u8;

                    // Randomly choose an LED to light up every 0.05 seconds
                    if self.tick % (1 + scale(adjusted_fps, 13)) as u32 == 0 {
                        let rand = self.rng.next_u32();
                        let row = rand as u8 % D::LIGHTING_ROWS as u8;
                        let col = (rand >> 8) as u8 % D::LIGHTING_COLS as u8;
                        self.buf[row as usize][col as usize] = u8::MAX
                    }

                    // Update the rest of the LEDs
                    self.set_brightness_for_each_led(|animator, _time, (row, col), _pos| {
                        animator.buf[row as usize][col as usize]
                            .saturating_sub(u8::MAX / adjusted_fps)
                    })
                }
            }
            SimpleBacklightMatrixEffect::DualBeacon => {
                if D::DUAL_BEACON_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, y)| {
                        // Base speed: 1 cycle every second
                        let pos = time as u8;
                        let dy = y as i16 - animator.bounds.mid.1 as i16;
                        let dx = x as i16 - animator.bounds.mid.0 as i16;
                        let sin = sin(pos) as i16 - 128;
                        let cos = cos(pos) as i16 - 128;
                        ((dy * cos + dx * sin) / 127) as u8
                    })
                }
            }
            SimpleBacklightMatrixEffect::WaveLeftRight => {
                if D::WAVE_LEFT_RIGHT_ENABLED {
                    let size = self.bounds.max.0 - self.bounds.min.0;
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, _y)| {
                        // Base speed: 1 cycle every second
                        sin(
                            (((((x - animator.bounds.min.0) as u16) << 8) / size as u16) as u8)
                                .wrapping_sub(time as u8),
                        )
                    })
                }
            }
            SimpleBacklightMatrixEffect::WaveUpDown => {
                if D::WAVE_UP_DOWN_ENABLED {
                    let size = self.bounds.max.1 - self.bounds.min.1;
                    self.set_brightness_for_each_led(|animator, time, _coord, (_x, y)| {
                        // Base speed: 1 cycle every second
                        sin(
                            (((((y - animator.bounds.min.0) as u16) << 8) / size as u16) as u8)
                                .wrapping_sub(time as u8),
                        )
                    })
                }
            }
            SimpleBacklightMatrixEffect::Reactive => {
                if D::REACTIVE_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, (row, col), _pos| {
                        // Base speed: LED fades after one second
                        let time_of_last_press = animator.last_presses.iter().find(
                            |((pressed_row, pressed_col), _time)| {
                                *pressed_row == row && *pressed_col == col
                            },
                        );

                        if let Some((_coord, press_time)) = time_of_last_press {
                            (u8::MAX as u32).saturating_sub(time - press_time) as u8
                        } else {
                            0
                        }
                    })
                }
            }
            SimpleBacklightMatrixEffect::ReactiveWide => {
                if D::REACTIVE_WIDE_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (led_x, led_y)| {
                        animator.last_presses.iter().fold(
                            0,
                            |brightness: u8, ((pressed_row, pressed_col), press_time)| {
                                // Base speed: LED fades after one second
                                if let Some((key_x, key_y)) = D::get_backlight_matrix().layout
                                    [*pressed_row as usize]
                                    [*pressed_col as usize]
                                {
                                    let dx = key_x.abs_diff(led_x) as u16;
                                    let dy = key_y.abs_diff(led_y) as u16;
                                    let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32) as u16;

                                    let brightness_increase = (u8::MAX as u16).saturating_sub(
                                        dist.saturating_mul(5) + time.abs_diff(*press_time) as u16,
                                    )
                                        as u8;

                                    brightness.saturating_add(brightness_increase)
                                } else {
                                    brightness
                                }
                            },
                        )
                    })
                }
            }
            SimpleBacklightMatrixEffect::ReactiveCross => {
                if D::REACTIVE_CROSS_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (led_x, led_y)| {
                        animator.last_presses.iter().fold(
                            0,
                            |brightness: u8, ((pressed_row, pressed_col), press_time)| {
                                if let Some((key_x, key_y)) = D::get_backlight_matrix().layout
                                    [*pressed_row as usize]
                                    [*pressed_col as usize]
                                {
                                    let dx = key_x.abs_diff(led_x) as u16;
                                    let dy = key_y.abs_diff(led_y) as u16;
                                    let daxis = dx.min(dy);
                                    let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32) as u16;

                                    let brightness_increase = (u8::MAX as u16).saturating_sub(
                                        (daxis * 16) + (time.abs_diff(*press_time) as u16 + dist),
                                    )
                                        as u8;

                                    brightness.saturating_add(brightness_increase)
                                } else {
                                    brightness
                                }
                            },
                        )
                    })
                }
            }
            SimpleBacklightMatrixEffect::ReactiveNexus => {
                if D::REACTIVE_NEXUS_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (led_x, led_y)| {
                        animator.last_presses.iter().fold(
                            0,
                            |brightness: u8, ((pressed_row, pressed_col), press_time)| {
                                if let Some((key_x, key_y)) = D::get_backlight_matrix().layout
                                    [*pressed_row as usize]
                                    [*pressed_col as usize]
                                {
                                    let dx = key_x.abs_diff(led_x) as u16;
                                    let dy = key_y.abs_diff(led_y) as u16;
                                    let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32) as u16;

                                    let effect = (time.abs_diff(*press_time) * 2) as u16 - dist;

                                    let brightness_increase = if dist as u8 > 72
                                        || (dx > 8 && dy > 8)
                                        || effect > u8::MAX as u16
                                    {
                                        0
                                    } else {
                                        (u8::MAX as u16).saturating_sub(effect)
                                    }
                                        as u8;

                                    brightness.saturating_add(brightness_increase)
                                } else {
                                    brightness
                                }
                            },
                        )
                    })
                }
            }
            SimpleBacklightMatrixEffect::ReactiveSplash => {
                if D::REACTIVE_SPLASH_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (led_x, led_y)| {
                        animator.last_presses.iter().fold(
                            0,
                            |brightness: u8, ((pressed_row, pressed_col), press_time)| {
                                if let Some((key_x, key_y)) = D::get_backlight_matrix().layout
                                    [*pressed_row as usize]
                                    [*pressed_col as usize]
                                {
                                    let dx = key_x.abs_diff(led_x) as u16;
                                    let dy = key_y.abs_diff(led_y) as u16;
                                    let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32) as u16;

                                    let effect = (time.abs_diff(*press_time) * 2) as u16 - dist;

                                    let brightness_increase = if effect > u8::MAX as u16 {
                                        0
                                    } else {
                                        (u8::MAX as u16).saturating_sub(effect)
                                    }
                                        as u8;

                                    brightness.saturating_add(brightness_increase)
                                } else {
                                    brightness
                                }
                            },
                        )
                    })
                }
            }
        }

        if let Err(err) = self.driver.write(&self.buf).await {
            error!(
                "[BACKLIGHT] Couldn't update backlight: {}",
                Debug2Format(&err)
            );
        };

        self.tick += 1;
    }

    #[cfg(feature = "storage")]
    pub fn create_storage_instance(&self) -> SimpleBacklightMatrixStorage<D, R> {
        SimpleBacklightMatrixStorage {
            _device_phantom: core::marker::PhantomData,
            _driver_phantom: core::marker::PhantomData,
        }
    }
}

impl<D: SimpleBacklightMatrixDevice + 'static, R: SimpleBacklightMatrixDriver<D>> Animator
    for SimpleBacklightMatrixAnimator<D, R>
where
    [(); D::LIGHTING_COLS]:,
    [(); D::LIGHTING_ROWS]:,
{
    type CommandType = SimpleBacklightMatrixCommand;

    type ConfigType = SimpleBacklightMatrixConfig;

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
                    .send(crate::split::MessageToPeripheral::SimpleBacklightMatrix(
                        SimpleBacklightMatrixCommand::ResetTime,
                    ))
                    .await;
                channel
                    .send(crate::split::MessageToPeripheral::SimpleBacklightMatrix(
                        SimpleBacklightMatrixCommand::SetEffect(self.config.effect),
                    ))
                    .await;
                channel
                    .send(crate::split::MessageToPeripheral::SimpleBacklightMatrix(
                        SimpleBacklightMatrixCommand::SetValue(self.config.val),
                    ))
                    .await;
                channel
                    .send(crate::split::MessageToPeripheral::SimpleBacklightMatrix(
                        SimpleBacklightMatrixCommand::SetSpeed(self.config.speed),
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
use storage::*;

#[cfg(feature = "storage")]
mod storage {
    use embassy_sync::signal::Signal;

    use crate::hw::platform::RawMutex;

    use super::{
        SimpleBacklightMatrixAnimator, SimpleBacklightMatrixDevice, SimpleBacklightMatrixDriver,
    };

    pub(super) static SIMPLE_BACKLIGHT_MATRIX_CONFIG_STATE_LISTENER: Signal<RawMutex, ()> =
        Signal::new();
    pub(super) static SIMPLE_BACKLIGHT_MATRIX_SAVE_SIGNAL: Signal<RawMutex, ()> = Signal::new();

    pub struct SimpleBacklightMatrixStorage<A, D> {
        pub(super) _driver_phantom: core::marker::PhantomData<A>,
        pub(super) _device_phantom: core::marker::PhantomData<D>,
    }

    impl<D: SimpleBacklightMatrixDevice + 'static, R: SimpleBacklightMatrixDriver<D>>
        crate::lighting::AnimatorStorage for SimpleBacklightMatrixStorage<D, R>
    where
        [(); D::LIGHTING_COLS]:,
        [(); D::LIGHTING_ROWS]:,
    {
        type Animator = SimpleBacklightMatrixAnimator<D, R>;

        const STORAGE_KEY: crate::storage::StorageKey =
            crate::storage::StorageKey::SimpleBacklightMatrixConfig;

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
