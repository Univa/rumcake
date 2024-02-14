use crate::backlight::drivers::RGBBacklightMatrixDriver;
use crate::backlight::{
    get_led_layout_bounds, BacklightDevice, BacklightMatrixDevice, LayoutBounds,
};
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, MaxSize)]
pub enum BacklightCommand {
    Toggle,
    TurnOn,
    TurnOff,
    NextEffect,
    PrevEffect,
    SetEffect(BacklightEffect),
    SetHue(u8),
    IncreaseHue(u8),
    DecreaseHue(u8),
    SetSaturation(u8),
    IncreaseSaturation(u8),
    DecreaseSaturation(u8),
    SetValue(u8),
    IncreaseValue(u8),
    DecreaseValue(u8),
    SetSpeed(u8),
    IncreaseSpeed(u8),
    DecreaseSpeed(u8),
    #[cfg(feature = "storage")]
    SaveConfig,
    ResetTime, // normally used internally for syncing LEDs for split keyboards
}

#[generate_items_from_enum_variants(
    "const RGB_BACKLIGHT_MATRIX_{variant_shouty_snake_case}_ENABLED: bool = true"
)]
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

impl BacklightEffect {
    pub(crate) fn is_enabled<D: BacklightDevice>(&self) -> bool {
        match self {
            BacklightEffect::Solid => D::RGB_BACKLIGHT_MATRIX_SOLID_ENABLED,
            BacklightEffect::AlphasMods => D::RGB_BACKLIGHT_MATRIX_ALPHAS_MODS_ENABLED,
            BacklightEffect::GradientUpDown => D::RGB_BACKLIGHT_MATRIX_GRADIENT_UP_DOWN_ENABLED,
            BacklightEffect::GradientLeftRight => {
                D::RGB_BACKLIGHT_MATRIX_GRADIENT_LEFT_RIGHT_ENABLED
            }
            BacklightEffect::Breathing => D::RGB_BACKLIGHT_MATRIX_BREATHING_ENABLED,
            BacklightEffect::ColorbandSat => D::RGB_BACKLIGHT_MATRIX_COLORBAND_SAT_ENABLED,
            BacklightEffect::ColorbandVal => D::RGB_BACKLIGHT_MATRIX_COLORBAND_VAL_ENABLED,
            BacklightEffect::ColorbandPinWheelSat => {
                D::RGB_BACKLIGHT_MATRIX_COLORBAND_PIN_WHEEL_SAT_ENABLED
            }
            BacklightEffect::ColorbandPinWheelVal => {
                D::RGB_BACKLIGHT_MATRIX_COLORBAND_PIN_WHEEL_VAL_ENABLED
            }
            BacklightEffect::ColorbandSpiralSat => {
                D::RGB_BACKLIGHT_MATRIX_COLORBAND_SPIRAL_SAT_ENABLED
            }
            BacklightEffect::ColorbandSpiralVal => {
                D::RGB_BACKLIGHT_MATRIX_COLORBAND_SPIRAL_VAL_ENABLED
            }
            BacklightEffect::CycleAll => D::RGB_BACKLIGHT_MATRIX_CYCLE_ALL_ENABLED,
            BacklightEffect::CycleLeftRight => D::RGB_BACKLIGHT_MATRIX_CYCLE_LEFT_RIGHT_ENABLED,
            BacklightEffect::CycleUpDown => D::RGB_BACKLIGHT_MATRIX_CYCLE_UP_DOWN_ENABLED,
            BacklightEffect::RainbowMovingChevron => {
                D::RGB_BACKLIGHT_MATRIX_RAINBOW_MOVING_CHEVRON_ENABLED
            }
            BacklightEffect::CycleOutIn => D::RGB_BACKLIGHT_MATRIX_CYCLE_OUT_IN_ENABLED,
            BacklightEffect::CycleOutInDual => D::RGB_BACKLIGHT_MATRIX_CYCLE_OUT_IN_DUAL_ENABLED,
            BacklightEffect::CyclePinWheel => D::RGB_BACKLIGHT_MATRIX_CYCLE_PIN_WHEEL_ENABLED,
            BacklightEffect::CycleSpiral => D::RGB_BACKLIGHT_MATRIX_CYCLE_SPIRAL_ENABLED,
            BacklightEffect::DualBeacon => D::RGB_BACKLIGHT_MATRIX_DUAL_BEACON_ENABLED,
            BacklightEffect::RainbowBeacon => D::RGB_BACKLIGHT_MATRIX_RAINBOW_BEACON_ENABLED,
            BacklightEffect::RainbowPinWheels => D::RGB_BACKLIGHT_MATRIX_RAINBOW_PIN_WHEELS_ENABLED,
            BacklightEffect::Raindrops => D::RGB_BACKLIGHT_MATRIX_RAINDROPS_ENABLED,
            BacklightEffect::JellybeanRaindrops => {
                D::RGB_BACKLIGHT_MATRIX_JELLYBEAN_RAINDROPS_ENABLED
            }
            BacklightEffect::HueBreathing => D::RGB_BACKLIGHT_MATRIX_HUE_BREATHING_ENABLED,
            BacklightEffect::HuePendulum => D::RGB_BACKLIGHT_MATRIX_HUE_PENDULUM_ENABLED,
            BacklightEffect::HueWave => D::RGB_BACKLIGHT_MATRIX_HUE_WAVE_ENABLED,
            BacklightEffect::PixelRain => D::RGB_BACKLIGHT_MATRIX_PIXEL_RAIN_ENABLED,
            BacklightEffect::PixelFlow => D::RGB_BACKLIGHT_MATRIX_PIXEL_FLOW_ENABLED,
            BacklightEffect::PixelFractal => D::RGB_BACKLIGHT_MATRIX_PIXEL_FRACTAL_ENABLED,
            BacklightEffect::TypingHeatmap => D::RGB_BACKLIGHT_MATRIX_TYPING_HEATMAP_ENABLED,
            BacklightEffect::DigitalRain => D::RGB_BACKLIGHT_MATRIX_DIGITAL_RAIN_ENABLED,
            BacklightEffect::SolidReactiveSimple => {
                D::RGB_BACKLIGHT_MATRIX_SOLID_REACTIVE_SIMPLE_ENABLED
            }
            BacklightEffect::SolidReactive => D::RGB_BACKLIGHT_MATRIX_SOLID_REACTIVE_ENABLED,
            BacklightEffect::SolidReactiveWide => {
                D::RGB_BACKLIGHT_MATRIX_SOLID_REACTIVE_WIDE_ENABLED
            }
            BacklightEffect::SolidReactiveMultiWide => {
                D::RGB_BACKLIGHT_MATRIX_SOLID_REACTIVE_MULTI_WIDE_ENABLED
            }
            BacklightEffect::SolidReactiveCross => {
                D::RGB_BACKLIGHT_MATRIX_SOLID_REACTIVE_CROSS_ENABLED
            }
            BacklightEffect::SolidReactiveMultiCross => {
                D::RGB_BACKLIGHT_MATRIX_SOLID_REACTIVE_MULTI_CROSS_ENABLED
            }
            BacklightEffect::SolidReactiveNexus => {
                D::RGB_BACKLIGHT_MATRIX_SOLID_REACTIVE_NEXUS_ENABLED
            }
            BacklightEffect::SolidReactiveMultiNexus => {
                D::RGB_BACKLIGHT_MATRIX_SOLID_REACTIVE_MULTI_NEXUS_ENABLED
            }
            BacklightEffect::Splash => D::RGB_BACKLIGHT_MATRIX_SPLASH_ENABLED,
            BacklightEffect::MultiSplash => D::RGB_BACKLIGHT_MATRIX_MULTI_SPLASH_ENABLED,
            BacklightEffect::SolidSplash => D::RGB_BACKLIGHT_MATRIX_SOLID_SPLASH_ENABLED,
            BacklightEffect::SolidMultiSplash => D::RGB_BACKLIGHT_MATRIX_SOLID_MULTI_SPLASH_ENABLED,
            #[cfg(feature = "vial")]
            BacklightEffect::DirectSet => D::RGB_BACKLIGHT_MATRIX_DIRECT_SET_ENABLED,
        }
    }
}

pub(super) struct BacklightAnimator<K: BacklightMatrixDevice, D: RGBBacklightMatrixDriver<K>>
where
    [(); K::LIGHTING_COLS]:,
    [(); K::LIGHTING_ROWS]:,
{
    pub(super) config: BacklightConfig,
    pub(super) buf: [[RGB8; K::LIGHTING_COLS]; K::LIGHTING_ROWS], // Stores the brightness/value of each LED
    pub(super) last_presses: ConstGenericRingBuffer<((u8, u8), u32), 8>, // Stores the row and col of the last 8 key presses, and the time (in ticks) it was pressed
    pub(super) tick: u32,
    pub(super) driver: D,
    pub(super) bounds: LayoutBounds,
    pub(super) rng: SmallRng,
}

impl<K: BacklightMatrixDevice + 'static, D: RGBBacklightMatrixDriver<K>> BacklightAnimator<K, D>
where
    [(); K::LIGHTING_COLS]:,
    [(); K::LIGHTING_ROWS]:,
{
    pub fn new(config: BacklightConfig, driver: D) -> Self {
        Self {
            config,
            tick: 0,
            driver,
            last_presses: ConstGenericRingBuffer::new(),
            buf: [[RGB8::new(0, 0, 0); K::LIGHTING_COLS]; K::LIGHTING_ROWS],
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
                    !self.config.effect.is_enabled::<K>()
                } {}
            }
            BacklightCommand::PrevEffect => {
                while {
                    self.config.effect.decrement();
                    !self.config.effect.is_enabled::<K>()
                } {}
            }
            BacklightCommand::SetEffect(effect) => {
                self.config.effect = effect;
            }
            BacklightCommand::SetHue(hue) => {
                self.config.hue = hue;
            }
            BacklightCommand::IncreaseHue(amount) => {
                self.config.hue = self.config.hue.saturating_add(amount);
            }
            BacklightCommand::DecreaseHue(amount) => {
                self.config.hue = self.config.hue.saturating_sub(amount);
            }
            BacklightCommand::SetSaturation(sat) => {
                self.config.sat = sat;
            }
            BacklightCommand::IncreaseSaturation(amount) => {
                self.config.sat = self.config.sat.saturating_add(amount);
            }
            BacklightCommand::DecreaseSaturation(amount) => {
                self.config.sat = self.config.sat.saturating_sub(amount);
            }
            BacklightCommand::SetValue(val) => {
                self.config.val = val;
            }
            BacklightCommand::IncreaseValue(amount) => {
                self.config.val = self.config.val.saturating_add(amount);
            }
            BacklightCommand::DecreaseValue(amount) => {
                self.config.val = self.config.val.saturating_sub(amount);
            }
            BacklightCommand::SetSpeed(speed) => {
                self.config.speed = speed;
            }
            BacklightCommand::IncreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_add(amount);
            }
            BacklightCommand::DecreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_sub(amount);
            }
            #[cfg(feature = "storage")]
            BacklightCommand::SaveConfig => {
                super::storage::BACKLIGHT_SAVE_SIGNAL.signal(());
            }
            BacklightCommand::ResetTime => {
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

        // TODO: animations
        match self.config.effect {
            BacklightEffect::Solid => todo!(),
            BacklightEffect::AlphasMods => todo!(),
            BacklightEffect::GradientUpDown => todo!(),
            BacklightEffect::GradientLeftRight => todo!(),
            BacklightEffect::Breathing => todo!(),
            BacklightEffect::ColorbandSat => todo!(),
            BacklightEffect::ColorbandVal => todo!(),
            BacklightEffect::ColorbandPinWheelSat => todo!(),
            BacklightEffect::ColorbandPinWheelVal => todo!(),
            BacklightEffect::ColorbandSpiralSat => todo!(),
            BacklightEffect::ColorbandSpiralVal => todo!(),
            BacklightEffect::CycleAll => todo!(),
            BacklightEffect::CycleLeftRight => todo!(),
            BacklightEffect::CycleUpDown => todo!(),
            BacklightEffect::RainbowMovingChevron => todo!(),
            BacklightEffect::CycleOutIn => todo!(),
            BacklightEffect::CycleOutInDual => todo!(),
            BacklightEffect::CyclePinWheel => todo!(),
            BacklightEffect::CycleSpiral => todo!(),
            BacklightEffect::DualBeacon => todo!(),
            BacklightEffect::RainbowBeacon => todo!(),
            BacklightEffect::RainbowPinWheels => todo!(),
            BacklightEffect::Raindrops => todo!(),
            BacklightEffect::JellybeanRaindrops => todo!(),
            BacklightEffect::HueBreathing => todo!(),
            BacklightEffect::HuePendulum => todo!(),
            BacklightEffect::HueWave => todo!(),
            BacklightEffect::PixelRain => todo!(),
            BacklightEffect::PixelFlow => todo!(),
            BacklightEffect::PixelFractal => todo!(),
            BacklightEffect::TypingHeatmap => todo!(),
            BacklightEffect::DigitalRain => todo!(),
            BacklightEffect::SolidReactiveSimple => todo!(),
            BacklightEffect::SolidReactive => todo!(),
            BacklightEffect::SolidReactiveWide => todo!(),
            BacklightEffect::SolidReactiveMultiWide => todo!(),
            BacklightEffect::SolidReactiveCross => todo!(),
            BacklightEffect::SolidReactiveMultiCross => todo!(),
            BacklightEffect::SolidReactiveNexus => todo!(),
            BacklightEffect::SolidReactiveMultiNexus => todo!(),
            BacklightEffect::Splash => todo!(),
            BacklightEffect::MultiSplash => todo!(),
            BacklightEffect::SolidSplash => todo!(),
            BacklightEffect::SolidMultiSplash => todo!(),
            #[cfg(feature = "vial")]
            BacklightEffect::DirectSet => {} // We just move onto calling the driver, since the frame buffer is updated by the backlight task
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
