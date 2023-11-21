use super::drivers::SimpleBacklightMatrixDriver;
use super::{
    get_led_layout_bounds, BacklightDevice, BacklightMatrixDevice, LEDFlags, LayoutBounds,
};
use crate::math::{atan2f, cos, sin, sqrtf};
use crate::{Cycle, LEDEffect};
use rumcake_macros::{generate_items_from_enum_variants, Cycle, LEDEffect};

use core::f32::consts::PI;
use core::u8;
use defmt::{error, warn, Debug2Format};
use keyberon::layout::Event;
use num_derive::FromPrimitive;
use postcard::experimental::max_size::MaxSize;
use rand::rngs::SmallRng;
use rand_core::{RngCore, SeedableRng};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, MaxSize)]
pub struct BacklightConfig {
    pub enabled: bool,
    pub effect: BacklightEffect,
    pub val: u8,
    pub speed: u8,
}

impl BacklightConfig {
    pub const fn default() -> Self {
        BacklightConfig {
            enabled: true,
            effect: BacklightEffect::Solid,
            val: 255,
            speed: 86,
        }
    }
}

impl Default for BacklightConfig {
    fn default() -> Self {
        Self::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum BacklightCommand {
    Toggle,
    TurnOn,
    TurnOff,
    NextEffect,
    PrevEffect,
    SetEffect(BacklightEffect),
    SetValue(u8),
    AdjustValue(i16),
    SetSpeed(u8),
    AdjustSpeed(i16),
    SetConfig(BacklightConfig),
    #[cfg(feature = "storage")]
    SaveConfig,
    SetTime(u32), // normally used internally for syncing LEDs for split keyboards
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
pub enum BacklightEffect {
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
    ReactiveMultiWide,

    #[animated]
    #[reactive]
    ReactiveCross,

    #[animated]
    #[reactive]
    ReactiveMultiCross,

    #[animated]
    #[reactive]
    ReactiveNexus,

    #[animated]
    #[reactive]
    ReactiveMultiNexus,

    #[animated]
    #[reactive]
    ReactiveSplash,

    #[animated]
    #[reactive]
    ReactiveMultiSplash,
}

impl BacklightEffect {
    pub(crate) fn is_enabled<D: BacklightDevice>(&self) -> bool {
        match self {
            BacklightEffect::Solid => D::SOLID_ENABLED,
            BacklightEffect::AlphasMods => D::ALPHAS_MODS_ENABLED,
            BacklightEffect::GradientUpDown => D::GRADIENT_UP_DOWN_ENABLED,
            BacklightEffect::GradientLeftRight => D::GRADIENT_LEFT_RIGHT_ENABLED,
            BacklightEffect::Breathing => D::BREATHING_ENABLED,
            BacklightEffect::Band => D::BAND_ENABLED,
            BacklightEffect::BandPinWheel => D::BAND_PIN_WHEEL_ENABLED,
            BacklightEffect::BandSpiral => D::BAND_SPIRAL_ENABLED,
            BacklightEffect::CycleLeftRight => D::CYCLE_LEFT_RIGHT_ENABLED,
            BacklightEffect::CycleUpDown => D::CYCLE_UP_DOWN_ENABLED,
            BacklightEffect::CycleOutIn => D::CYCLE_OUT_IN_ENABLED,
            BacklightEffect::Raindrops => D::RAINDROPS_ENABLED,
            BacklightEffect::DualBeacon => D::DUAL_BEACON_ENABLED,
            BacklightEffect::WaveLeftRight => D::WAVE_LEFT_RIGHT_ENABLED,
            BacklightEffect::WaveUpDown => D::WAVE_UP_DOWN_ENABLED,
            BacklightEffect::Reactive => D::REACTIVE_ENABLED,
            BacklightEffect::ReactiveWide => D::REACTIVE_WIDE_ENABLED,
            BacklightEffect::ReactiveMultiWide => D::REACTIVE_MULTI_WIDE_ENABLED,
            BacklightEffect::ReactiveCross => D::REACTIVE_CROSS_ENABLED,
            BacklightEffect::ReactiveMultiCross => D::REACTIVE_MULTI_CROSS_ENABLED,
            BacklightEffect::ReactiveNexus => D::REACTIVE_NEXUS_ENABLED,
            BacklightEffect::ReactiveMultiNexus => D::REACTIVE_MULTI_NEXUS_ENABLED,
            BacklightEffect::ReactiveSplash => D::REACTIVE_SPLASH_ENABLED,
            BacklightEffect::ReactiveMultiSplash => D::REACTIVE_MULTI_SPLASH_ENABLED,
        }
    }
}

pub(super) struct BacklightAnimator<'a, K: BacklightMatrixDevice, D: SimpleBacklightMatrixDriver<K>>
where
    [(); K::LIGHTING_COLS]:,
    [(); K::LIGHTING_ROWS]:,
{
    pub(super) config: BacklightConfig,
    pub(super) buf: [[u8; K::LIGHTING_COLS]; K::LIGHTING_ROWS], // Stores the brightness/value of each LED
    pub(super) last_presses: ConstGenericRingBuffer<((u8, u8), u32), 8>, // Stores the row and col of the last 8 key presses, and the time (in ticks) it was pressed
    pub(super) tick: u32,
    pub(super) driver: D,
    pub(super) bounds: LayoutBounds,
    pub(super) rng: SmallRng,
}

impl<K: BacklightMatrixDevice + 'static, D: SimpleBacklightMatrixDriver<K>>
    BacklightAnimator<'_, K, D>
where
    [(); K::LIGHTING_COLS]:,
    [(); K::LIGHTING_ROWS]:,
{
    pub fn new(config: BacklightConfig, driver: D) -> Self {
        Self {
            config,
            tick: 0,
            driver,
            buf: [[0; K::LIGHTING_COLS]; K::LIGHTING_ROWS],
            last_presses: ConstGenericRingBuffer::new(),
            bounds: get_led_layout_bounds::<K>(),
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

    pub async fn process_command(&mut self, command: BacklightCommand) {
        match command {
            BacklightCommand::Toggle => {
                self.config.enabled = !self.config.enabled;
            }
            BacklightCommand::TurnOn => {
                self.config.enabled = true;
            }
            BacklightCommand::TurnOff => {
                self.config.enabled = false;
            }
            BacklightCommand::NextEffect => {
                while {
                    self.config.effect.increment();
                    self.config.effect.is_enabled::<K>()
                } {}
            }
            BacklightCommand::PrevEffect => {
                while {
                    self.config.effect.decrement();
                    self.config.effect.is_enabled::<K>()
                } {}
            }
            BacklightCommand::SetEffect(effect) => {
                self.config.effect = effect;
            }
            BacklightCommand::SetValue(val) => {
                self.config.val = val;
            }
            BacklightCommand::AdjustValue(amount) => {
                self.config.val =
                    (self.config.val as i16 + amount).clamp(u8::MIN as i16, u8::MAX as i16) as u8;
            }
            BacklightCommand::SetSpeed(speed) => {
                self.config.speed = speed;
            }
            BacklightCommand::AdjustSpeed(amount) => {
                self.config.speed =
                    (self.config.speed as i16 + amount).clamp(u8::MIN as i16, u8::MAX as i16) as u8;
            }
            BacklightCommand::SetConfig(config) => {
                self.config = config;
            }
            #[cfg(feature = "storage")]
            BacklightCommand::SaveConfig => {
                super::storage::BACKLIGHT_SAVE_SIGNAL.signal(());
            }
            BacklightCommand::SetTime(time) => {
                self.tick = time;
            }
        };
    }

    pub fn set_brightness_for_each_led(
        &mut self,
        calc: impl Fn(&mut Self, f32, (u8, u8), (u8, u8)) -> u8,
    ) {
        for row in 0..K::LIGHTING_ROWS {
            for col in 0..K::LIGHTING_COLS {
                if let Some(position) = K::get_backlight_matrix().layout[row][col] {
                    let seconds = (self.tick as f32 / K::FPS as f32)
                        * (self.config.speed as f32 * 1.5 / u8::MAX as f32 + 0.5);
                    self.buf[row][col] = (calc(self, seconds, (row as u8, col as u8), position)
                        as u16
                        * self.config.val as u16
                        / u8::MAX as u16) as u8
                }
            }
        }
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
                        if K::get_backlight_matrix()
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

        match self.config.effect {
            BacklightEffect::Solid => {
                if K::SOLID_ENABLED {
                    self.set_brightness_for_each_led(|_animator, _time, _coord, _pos| u8::MAX)
                }
            }
            BacklightEffect::AlphasMods => {
                if K::ALPHAS_MODS_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, (row, col), _pos| {
                        if K::get_backlight_matrix().flags[row as usize][col as usize]
                            .contains(LEDFlags::ALPHA)
                        {
                            u8::MAX
                        } else {
                            animator.config.speed
                        }
                    })
                }
            }
            BacklightEffect::GradientUpDown => {
                if K::GRADIENT_UP_DOWN_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, _coord, (_x, y)| {
                        // Calculate the brightness for each LED based on it's Y position
                        // Speed will be used to determine where the "peak" of the gradient is.
                        let size = animator.bounds.max.1 - animator.bounds.min.1;
                        ((sin((y as f32 + (size as f32 / 2.0)
                            - (animator.config.speed as f32 * size as f32 / u8::MAX as f32))
                            * (PI / size as f32))
                            + 1.0)
                            * 127.0) as u8
                    })
                }
            }
            BacklightEffect::GradientLeftRight => {
                if K::GRADIENT_LEFT_RIGHT_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, _coord, (x, _y)| {
                        // Calculate the brightness for each LED based on it's X position
                        // Speed will be used to determine where the "peak" of the gradient is.
                        let size = animator.bounds.max.0 - animator.bounds.min.0;
                        ((sin((x as f32 + (size as f32 / 2.0)
                            - (animator.config.speed as f32 * size as f32 / u8::MAX as f32))
                            * (PI / size as f32))
                            + 1.0)
                            * 127.0) as u8
                    })
                }
            }
            BacklightEffect::Breathing => {
                if K::BREATHING_ENABLED {
                    self.set_brightness_for_each_led(|_animator, time, _coord, _pos| {
                        ((sin(time) + 1.0) * u8::MAX as f32 / 2.0) as u8
                    })
                }
            }
            BacklightEffect::Band => {
                if K::BAND_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, _y)| {
                        let size = animator.bounds.max.0 - animator.bounds.min.0;
                        let pos = (time * size as f32 % size as f32) as u8;

                        0.max(u8::MAX as i32 - (x as i32 - pos as i32).abs() * 8) as u8
                    })
                }
            }
            BacklightEffect::BandPinWheel => {
                if K::BAND_PIN_WHEEL_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, y)| {
                        // Base speed: 1 half-cycle every second
                        let pos = ((time % 1.0) * u8::MAX as f32) as u8;
                        let dy = y as i32 - animator.bounds.mid.1 as i32;
                        let dx = x as i32 - animator.bounds.mid.0 as i32;
                        ((atan2f(dy as f32, dx as f32) * u8::MAX as f32 / PI) as i32) as u8 - pos
                    })
                }
            }
            BacklightEffect::BandSpiral => {
                if K::BAND_SPIRAL_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, y)| {
                        // Base speed: 1 half-cycle every second
                        let pos = ((time % 1.0) * u8::MAX as f32) as u8;
                        let dy = y as i32 - animator.bounds.mid.1 as i32;
                        let dx = x as i32 - animator.bounds.mid.0 as i32;
                        let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32);
                        ((atan2f(dy as f32, dx as f32) * u8::MAX as f32 / PI) as i32) as u8
                            + dist as u8
                            - pos
                    })
                }
            }
            BacklightEffect::CycleLeftRight => {
                if K::CYCLE_LEFT_RIGHT_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, _y)| {
                        // Base speed: 1 cycle every second
                        let size = animator.bounds.max.0 - animator.bounds.min.0;
                        let pos = ((time % 1.0) * size as f32) as u8;
                        (x - animator.bounds.min.0) + (pos * u8::MAX)
                    })
                }
            }
            BacklightEffect::CycleUpDown => {
                if K::CYCLE_UP_DOWN_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (_x, y)| {
                        // Base speed: 1 cycle every second
                        let size = animator.bounds.max.1 - animator.bounds.min.1;
                        let pos = ((time % 1.0) * size as f32) as u8;
                        (y - animator.bounds.min.1) + (pos * u8::MAX)
                    })
                }
            }
            BacklightEffect::CycleOutIn => {
                if K::CYCLE_OUT_IN_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, y)| {
                        // Base speed: 1 cycle every second
                        let d = sqrtf(
                            ((x as i32 - animator.bounds.mid.0 as i32).pow(2)
                                + (y as i32 - animator.bounds.mid.1 as i32).pow(2))
                                as f32,
                        ) as u8;
                        let pos = ((time % 1.0) * u8::MAX as f32) as u8;

                        u8::MAX - d - pos
                    })
                }
            }
            BacklightEffect::Raindrops => {
                if K::RAINDROPS_ENABLED {
                    // Randomly choose an LED to light up every 0.05 seconds
                    if self.tick
                        % (1.0
                            + 0.05
                                * (K::FPS as f32
                                    / (self.config.speed as f32 * 1.5 / u8::MAX as f32 + 0.5)))
                            as u32
                        == 0
                    {
                        let row = (self.rng.next_u32() as f32 * K::LIGHTING_ROWS as f32
                            / u32::MAX as f32) as u8;
                        let col = (self.rng.next_u32() as f32 * K::LIGHTING_COLS as f32
                            / u32::MAX as f32) as u8;
                        self.buf[row as usize][col as usize] = 255
                    }

                    // Update the rest of the LEDs
                    self.set_brightness_for_each_led(|animator, _time, (row, col), _pos| {
                        animator.buf[row as usize][col as usize].saturating_sub(
                            u8::MAX
                                / (K::FPS as f32
                                    / (animator.config.speed as f32 * 1.5 / u8::MAX as f32 + 0.5))
                                    as u8,
                        )
                    })
                }
            }
            BacklightEffect::DualBeacon => {
                if K::DUAL_BEACON_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, y)| {
                        // Base speed: 1 cycle every second
                        let pos = ((time % 1.0) * u8::MAX as f32) as u8;
                        let dy = y as i32 - animator.bounds.mid.1 as i32;
                        let dx = x as i32 - animator.bounds.mid.0 as i32;
                        let sin = (sin(PI * pos as f32 / 127.0) * 127.0) as i32;
                        let cos = (cos(PI * pos as f32 / 127.0) * 127.0) as i32;
                        ((dy * cos + dx * sin) / 127) as u8
                    })
                }
            }
            BacklightEffect::WaveLeftRight => {
                if K::WAVE_LEFT_RIGHT_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (x, _y)| {
                        // Base speed: 1 cycle every second
                        let size = animator.bounds.max.0 - animator.bounds.min.0;
                        let pos = ((time % 1.0) * size as f32) as u8;
                        ((sin(((x - animator.bounds.min.0) + size - pos) as f32
                            * (2.0 * PI / size as f32))
                            + 1.0)
                            * 127.0) as u8
                    })
                }
            }
            BacklightEffect::WaveUpDown => {
                if K::WAVE_UP_DOWN_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _coord, (_x, y)| {
                        // Base speed: 1 cycle every second
                        let size = animator.bounds.max.1 - animator.bounds.min.1;
                        let pos = ((time % 1.0) * size as f32) as u8;
                        ((sin(((y - animator.bounds.min.1) + size - pos) as f32
                            * (2.0 * PI / size as f32))
                            + 1.0)
                            * 127.0) as u8
                    })
                }
            }
            BacklightEffect::Reactive => {
                if K::REACTIVE_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, (row, col), _pos| {
                        // Base speed: LED fades after one second
                        let time_of_last_press = animator.last_presses.iter().find(
                            |((pressed_row, pressed_col), _time)| {
                                *pressed_row == row && *pressed_col == col
                            },
                        );

                        if let Some((_coord, time)) = time_of_last_press {
                            let pos = (((animator.tick - time) as f32 / K::FPS as f32)
                                * (animator.config.speed as f32 * 1.5 / u8::MAX as f32 + 0.5))
                                .min(1.0);
                            u8::MAX - (u8::MAX as f32 * pos) as u8
                        } else {
                            0
                        }
                    })
                }
            }
            BacklightEffect::ReactiveWide => {
                if K::REACTIVE_WIDE_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, _coord, (led_x, led_y)| {
                        let brightness = animator.last_presses.iter().fold(
                            0,
                            |brightness, ((pressed_row, pressed_col), press_time)| {
                                // Base speed: LED fades after one second
                                if let Some((key_x, key_y)) = K::get_backlight_matrix().layout
                                    [*pressed_row as usize]
                                    [*pressed_col as usize]
                                {
                                    let dx = key_x as i32 - led_x as i32;
                                    let dy = key_y as i32 - led_y as i32;
                                    let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32);

                                    let pos = (((animator.tick - press_time) as f32
                                        / K::FPS as f32)
                                        * (animator.config.speed as f32 * 1.5 / u8::MAX as f32
                                            + 0.5))
                                        .min(1.0);

                                    let brightness_increase = 0.max(
                                        u8::MAX as i32
                                            - ((dist * 5.0) as i32 + (pos * u8::MAX as f32) as i32),
                                    );

                                    (u8::MAX as u16)
                                        .min(brightness as u16 + brightness_increase as u16)
                                        as u8
                                } else {
                                    brightness
                                }
                            },
                        );

                        brightness
                    })
                }
            }
            BacklightEffect::ReactiveMultiWide => todo!(),
            BacklightEffect::ReactiveCross => {
                if K::REACTIVE_CROSS_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, _coord, (led_x, led_y)| {
                        let brightness = animator.last_presses.iter().fold(
                            0,
                            |brightness, ((pressed_row, pressed_col), press_time)| {
                                if let Some((key_x, key_y)) = K::get_backlight_matrix().layout
                                    [*pressed_row as usize]
                                    [*pressed_col as usize]
                                {
                                    let dx = (key_x as i32 - led_x as i32).abs();
                                    let dy = (key_y as i32 - led_y as i32).abs();
                                    let daxis = dx.min(dy);
                                    let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32);

                                    let pos = (((animator.tick - press_time) as f32
                                        / K::FPS as f32)
                                        * (animator.config.speed as f32 * 1.5 / u8::MAX as f32
                                            + 0.5))
                                        .min(1.0);

                                    let brightness_increase = 0.max(
                                        u8::MAX as i32
                                            - ((daxis * 16) + (pos * u8::MAX as f32 + dist) as i32),
                                    );

                                    (u8::MAX as u16)
                                        .min(brightness as u16 + brightness_increase as u16)
                                        as u8
                                } else {
                                    brightness
                                }
                            },
                        );

                        brightness
                    })
                }
            }
            BacklightEffect::ReactiveMultiCross => todo!(),
            BacklightEffect::ReactiveNexus => {
                if K::REACTIVE_NEXUS_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, _coord, (led_x, led_y)| {
                        let brightness = animator.last_presses.iter().fold(
                            0,
                            |brightness, ((pressed_row, pressed_col), press_time)| {
                                if let Some((key_x, key_y)) = K::get_backlight_matrix().layout
                                    [*pressed_row as usize]
                                    [*pressed_col as usize]
                                {
                                    let dx = (key_x as i32 - led_x as i32).abs();
                                    let dy = (key_y as i32 - led_y as i32).abs();
                                    let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32);

                                    let pos = (((animator.tick - press_time) as f32
                                        / K::FPS as f32
                                        * 2.0)
                                        * (animator.config.speed as f32 * 1.5 / u8::MAX as f32
                                            + 0.5))
                                        .min(2.0);

                                    let effect = (pos * u8::MAX as f32) as u16 - dist as u16;

                                    let brightness_increase = if dist as u8 > 72
                                        || (dx > 8 && dy > 8)
                                        || effect > u8::MAX as u16
                                    {
                                        0
                                    } else {
                                        u8::MAX as u16 - effect
                                    };

                                    (u8::MAX as u16)
                                        .min(brightness as u16 + brightness_increase as u16)
                                        as u8
                                } else {
                                    brightness
                                }
                            },
                        );

                        brightness
                    })
                }
            }
            BacklightEffect::ReactiveMultiNexus => todo!(),
            BacklightEffect::ReactiveSplash => {
                if K::REACTIVE_SPLASH_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, _coord, (led_x, led_y)| {
                        let brightness = animator.last_presses.iter().fold(
                            0,
                            |brightness, ((pressed_row, pressed_col), press_time)| {
                                if let Some((key_x, key_y)) = K::get_backlight_matrix().layout
                                    [*pressed_row as usize]
                                    [*pressed_col as usize]
                                {
                                    let dx = (key_x as i32 - led_x as i32).abs();
                                    let dy = (key_y as i32 - led_y as i32).abs();
                                    let dist = sqrtf((dx.pow(2) + dy.pow(2)) as f32);

                                    let pos = (((animator.tick - press_time) as f32
                                        / K::FPS as f32
                                        * 2.0)
                                        * (animator.config.speed as f32 * 1.5 / u8::MAX as f32
                                            + 0.5))
                                        .min(2.0);

                                    let effect = (pos * u8::MAX as f32) as u16 - dist as u16;

                                    let brightness_increase = if effect > u8::MAX as u16 {
                                        0
                                    } else {
                                        u8::MAX as u16 - effect
                                    };

                                    (u8::MAX as u16)
                                        .min(brightness as u16 + brightness_increase as u16)
                                        as u8
                                } else {
                                    brightness
                                }
                            },
                        );

                        brightness
                    })
                }
            }
            BacklightEffect::ReactiveMultiSplash => todo!(),
        }

        if let Err(err) = self.driver.write(&self.buf).await {
            error!(
                "[BACKLIGHT] Couldn't update backlight: {}",
                Debug2Format(&err)
            );
        };

        self.tick += 1;
    }
}
