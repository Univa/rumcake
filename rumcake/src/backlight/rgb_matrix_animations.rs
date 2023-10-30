use super::drivers::RGBBacklightMatrixDriver;
use super::{get_led_layout_bounds, BacklightMatrixDevice, LayoutBounds};
use crate::{Cycle, LEDEffect};
use postcard::experimental::max_size::MaxSize;
use rumcake_macros::{generate_items_from_enum_variants, Cycle, LEDEffect};

use defmt::{error, warn, Debug2Format};
use keyberon::layout::Event;
use num_derive::FromPrimitive;
use rand::rngs::SmallRng;
use rand_core::SeedableRng;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use serde::{Deserialize, Serialize};
use smart_leds::hsv::Hsv;
use smart_leds::RGB8;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, MaxSize)]
pub struct BacklightConfig {
    pub enabled: bool,
    pub effect: BacklightEffect,
    pub hue: u8,
    pub sat: u8,
    pub val: u8,
    pub speed: u8,
}

impl BacklightConfig {
    pub const fn default() -> Self {
        BacklightConfig {
            enabled: true,
            effect: BacklightEffect::Solid,
            hue: 0,
            sat: 255,
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
    NextEffect,
    PrevEffect,
    SetEffect(BacklightEffect),
    SetHue(u8),
    AdjustHue(i16),
    SetSaturation(u8),
    AdjustSaturation(i16),
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

pub(super) struct BacklightAnimator<K: BacklightMatrixDevice, D: RGBBacklightMatrixDriver<K>>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    pub(super) config: BacklightConfig,
    pub(super) buf: [[RGB8; K::MATRIX_COLS]; K::MATRIX_ROWS], // Stores the brightness/value of each LED
    pub(super) last_presses: ConstGenericRingBuffer<((u8, u8), u32), 8>, // Stores the row and col of the last 8 key presses, and the time (in ticks) it was pressed
    pub(super) tick: u32,
    pub(super) driver: D,
    pub(super) bounds: LayoutBounds,
    pub(super) rng: SmallRng,
}

impl<K: BacklightMatrixDevice, D: RGBBacklightMatrixDriver<K>> BacklightAnimator<K, D>
where
    [(); K::MATRIX_COLS]:,
    [(); K::MATRIX_ROWS]:,
{
    pub fn new(config: BacklightConfig, driver: D) -> Self {
        Self {
            config,
            tick: 0,
            driver,
            last_presses: ConstGenericRingBuffer::new(),
            buf: [[RGB8::new(0, 0, 0); K::MATRIX_COLS]; K::MATRIX_ROWS],
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
            BacklightCommand::NextEffect => {
                self.config.effect.increment();
            }
            BacklightCommand::PrevEffect => {
                self.config.effect.decrement();
            }
            BacklightCommand::SetEffect(effect) => {
                self.config.effect = effect;
            }
            BacklightCommand::SetHue(hue) => {
                self.config.hue = hue;
            }
            BacklightCommand::AdjustHue(amount) => {
                self.config.hue =
                    (self.config.hue as i16 + amount).clamp(u8::MIN as i16, u8::MAX as i16) as u8;
            }
            BacklightCommand::SetSaturation(sat) => {
                self.config.sat = sat;
            }
            BacklightCommand::AdjustSaturation(amount) => {
                self.config.sat =
                    (self.config.sat as i16 + amount).clamp(u8::MIN as i16, u8::MAX as i16) as u8;
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
                super::BACKLIGHT_CONFIG_STORAGE_CLIENT
                    .request(crate::storage::StorageRequest::Write(
                        super::BACKLIGHT_CONFIG_STATE.get().await,
                    ))
                    .await;
            }
            BacklightCommand::SetTime(time) => {
                self.tick = time;
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
                        self.last_presses.push(((row, col), self.tick));
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
            BacklightEffect::Solid => todo!(),
            BacklightEffect::AlphasMods => todo!(),
            BacklightEffect::GradientUpDown => todo!(),
            BacklightEffect::GradientLeftRight => todo!(),
            BacklightEffect::Breathing => todo!(),
            BacklightEffect::Band => todo!(),
            BacklightEffect::BandPinWheel => todo!(),
            BacklightEffect::BandSpiral => todo!(),
            BacklightEffect::CycleLeftRight => todo!(),
            BacklightEffect::CycleUpDown => todo!(),
            BacklightEffect::CycleOutIn => todo!(),
            BacklightEffect::Raindrops => todo!(),
            BacklightEffect::DualBeacon => todo!(),
            BacklightEffect::WaveLeftRight => todo!(),
            BacklightEffect::WaveUpDown => todo!(),
            BacklightEffect::Reactive => todo!(),
            BacklightEffect::ReactiveWide => todo!(),
            BacklightEffect::ReactiveMultiWide => todo!(),
            BacklightEffect::ReactiveCross => todo!(),
            BacklightEffect::ReactiveMultiCross => todo!(),
            BacklightEffect::ReactiveNexus => todo!(),
            BacklightEffect::ReactiveMultiNexus => todo!(),
            BacklightEffect::ReactiveSplash => todo!(),
            BacklightEffect::ReactiveMultiSplash => todo!(),
        }

        if let Err(err) = self.driver.write(&self.buf).await {
            error!(
                "[BACKLIGHT] Couldn't update backlight colors: {}",
                Debug2Format(&err)
            );
        };

        self.tick += 1;
    }
}
