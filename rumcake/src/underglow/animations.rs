use core::f32::consts::PI;

use super::drivers::UnderglowDriver;
use super::UnderglowDevice;
use crate::math::sin;
use crate::{Cycle, LEDEffect};
use postcard::experimental::max_size::MaxSize;
use rumcake_macros::{generate_items_from_enum_variants, Cycle, LEDEffect};

use defmt::{error, warn, Debug2Format};
use keyberon::layout::Event;
use num_derive::FromPrimitive;
use rand::rngs::SmallRng;
use rand_core::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::RGB8;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, MaxSize)]
pub struct UnderglowConfig {
    pub enabled: bool,
    pub effect: UnderglowEffect,
    pub hue: u8,
    pub sat: u8,
    pub val: u8,
    pub speed: u8,
}

impl UnderglowConfig {
    pub const fn default() -> Self {
        UnderglowConfig {
            enabled: true,
            effect: UnderglowEffect::Solid,
            hue: 0,
            sat: 255,
            val: 255,
            speed: 86,
        }
    }
}

impl Default for UnderglowConfig {
    fn default() -> Self {
        Self::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum UnderglowCommand {
    Toggle,
    NextEffect,
    PrevEffect,
    SetEffect(UnderglowEffect),
    SetHue(u8),
    AdjustHue(i16),
    SetSaturation(u8),
    AdjustSaturation(i16),
    SetValue(u8),
    AdjustValue(i16),
    SetSpeed(u8),
    AdjustSpeed(i16),
    SetConfig(UnderglowConfig),
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
pub enum UnderglowEffect {
    Solid,

    #[animated]
    Breathing,

    #[animated]
    RainbowMood,

    #[animated]
    RainbowSwirl,

    #[animated]
    Snake,

    #[animated]
    Knight,

    #[animated]
    Christmas,
    StaticGradient,

    #[animated]
    RGBTest,

    #[animated]
    Alternating,

    #[animated]
    Twinkle,

    #[animated]
    #[reactive]
    Reactive,
}

pub(super) struct UnderglowAnimator<R: UnderglowDriver<D, Color = RGB8>, D: UnderglowDevice>
where
    [(); D::NUM_LEDS]:,
{
    pub(super) config: UnderglowConfig,
    pub(super) buf: [RGB8; D::NUM_LEDS],
    pub(super) twinkle_state: [(Hsv, u8); D::NUM_LEDS], // For the twinkle effect specifically, tracks the lifespan of lit LEDs.
    pub(super) tick: u32,
    pub(super) time_of_last_press: u32,
    pub(super) driver: R,
    pub(super) rng: SmallRng,
}

impl<R: UnderglowDriver<D, Color = RGB8>, D: UnderglowDevice> UnderglowAnimator<R, D>
where
    [(); D::NUM_LEDS]:,
{
    pub fn new(config: UnderglowConfig, driver: R) -> Self {
        Self {
            config,
            tick: 0,
            driver,
            time_of_last_press: 0,
            twinkle_state: [(
                Hsv {
                    hue: 0,
                    sat: 0,
                    val: 0,
                },
                0,
            ); D::NUM_LEDS],
            buf: [RGB8::new(0, 0, 0); D::NUM_LEDS],
            rng: SmallRng::seed_from_u64(239810),
        }
    }

    pub async fn turn_on(&mut self) {
        if let Err(err) = self.driver.turn_on().await {
            warn!("[UNDERGLOW] Animations have been enabled, but the underglow LEDs could not be turned on: {}", Debug2Format(&err));
        };
    }

    pub async fn turn_off(&mut self) {
        if let Err(err) = self.driver.turn_off().await {
            warn!("[UNDERGLOW] Animations have been disabled, but the underglow LEDs could not be turned off: {}", Debug2Format(&err));
        };
    }

    pub async fn process_command(&mut self, command: UnderglowCommand) {
        match command {
            UnderglowCommand::Toggle => {
                self.config.enabled = !self.config.enabled;
            }
            UnderglowCommand::NextEffect => {
                self.config.effect.increment();
            }
            UnderglowCommand::PrevEffect => {
                self.config.effect.decrement();
            }
            UnderglowCommand::SetEffect(effect) => {
                self.config.effect = effect;
            }
            UnderglowCommand::SetHue(hue) => {
                self.config.hue = hue;
            }
            UnderglowCommand::AdjustHue(amount) => {
                self.config.hue =
                    (self.config.hue as i16 + amount).clamp(u8::MIN as i16, u8::MAX as i16) as u8;
            }
            UnderglowCommand::SetSaturation(sat) => {
                self.config.sat = sat;
            }
            UnderglowCommand::AdjustSaturation(amount) => {
                self.config.sat =
                    (self.config.sat as i16 + amount).clamp(u8::MIN as i16, u8::MAX as i16) as u8;
            }
            UnderglowCommand::SetValue(val) => {
                self.config.val = val;
            }
            UnderglowCommand::AdjustValue(amount) => {
                self.config.val =
                    (self.config.val as i16 + amount).clamp(u8::MIN as i16, u8::MAX as i16) as u8;
            }
            UnderglowCommand::SetSpeed(speed) => {
                self.config.speed = speed;
            }
            UnderglowCommand::AdjustSpeed(amount) => {
                self.config.speed =
                    (self.config.speed as i16 + amount).clamp(u8::MIN as i16, u8::MAX as i16) as u8;
            }
            UnderglowCommand::SetConfig(config) => {
                self.config = config;
            }
            #[cfg(feature = "storage")]
            UnderglowCommand::SaveConfig => {
                super::UNDERGLOW_CONFIG_STORAGE_CLIENT
                    .request(crate::storage::StorageRequest::Write(
                        super::UNDERGLOW_CONFIG_STATE.get().await,
                    ))
                    .await;
            }
            UnderglowCommand::SetTime(time) => {
                self.tick = time;
            }
        };
    }

    pub fn set_brightness_for_each_led(&mut self, calc: impl Fn(&mut Self, f32, u8) -> Hsv) {
        for led in 0..D::NUM_LEDS {
            let seconds = (self.tick as f32 / D::FPS as f32)
                * (self.config.speed as f32 * 1.5 / u8::MAX as f32 + 0.5);
            let mut hsv = calc(self, seconds, led as u8);
            hsv.val = (hsv.val as u16 * self.config.val as u16 / u8::MAX as u16) as u8;
            self.buf[led] = hsv2rgb(hsv);
        }
    }

    pub fn register_event(&mut self, event: Event) {
        match event {
            Event::Press(_x, _y) => {
                self.time_of_last_press = self.tick;
            }
            Event::Release(_x, _y) => {} // nothing for now. maybe change some effects to behave depending on the state of a key.
        }
    }

    pub async fn tick(&mut self) {
        if !self.config.enabled {
            return;
        }

        match self.config.effect {
            UnderglowEffect::Solid => {
                if D::SOLID_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, _led| Hsv {
                        hue: animator.config.hue,
                        sat: animator.config.sat,
                        val: u8::MAX,
                    })
                }
            }
            UnderglowEffect::Breathing => {
                if D::BREATHING_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _led| Hsv {
                        hue: animator.config.hue,
                        sat: animator.config.sat,
                        val: (sin(time) * u8::MAX as f32 / 2.0 + u8::MAX as f32 / 2.0) as u8,
                    })
                }
            }
            UnderglowEffect::RainbowMood => {
                if D::RAINBOW_MOOD_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _led| Hsv {
                        hue: (time * 15.0 % u8::MAX as f32) as u8,
                        sat: animator.config.sat,
                        val: u8::MAX,
                    })
                }
            }
            UnderglowEffect::RainbowSwirl => {
                if D::RAINBOW_SWIRL_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, led| Hsv {
                        hue: (led as u16 * u8::MAX as u16 / D::NUM_LEDS as u16
                            + (time * 15.0) as u16) as u8,
                        sat: animator.config.sat,
                        val: u8::MAX,
                    })
                }
            }
            UnderglowEffect::Snake => {
                if D::SNAKE_ENABLED {
                    // Base speed: 1 second to do a full cycle.
                    let length = 4;

                    self.set_brightness_for_each_led(|animator, time, led| {
                        let pos = (time * D::NUM_LEDS as f32) as u32;
                        let mut hsv: Hsv = Hsv::default();

                        for j in 0..length {
                            let lit = (pos + j) % D::NUM_LEDS as u32;

                            if led as u32 == lit {
                                hsv = Hsv {
                                    hue: animator.config.hue,
                                    sat: animator.config.sat,
                                    val: (u8::MAX as f32 * (j + 1) as f32 / length as f32) as u8,
                                };
                            }
                        }

                        hsv
                    })
                }
            }
            UnderglowEffect::Knight => {
                if D::KNIGHT_ENABLED {
                    // Base speed: 1 second to traverse a length of NUM_LEDS
                    let length: u32 = 4;

                    self.set_brightness_for_each_led(|animator, time, led| {
                        let pos = (time * D::NUM_LEDS as f32) as u32
                            % ((D::NUM_LEDS as u32 + length - 1) * 2);

                        let direction = if pos >= (D::NUM_LEDS as u32 + length - 1) {
                            -1
                        } else {
                            1
                        };

                        let start: i32 = if direction == -1 {
                            2 * D::NUM_LEDS as u32 - pos + length - 2
                        } else {
                            pos - length + 1
                        } as i32;

                        let end: i32 = if direction == -1 {
                            2 * D::NUM_LEDS as u32 - pos + 2 * length - 3
                        } else {
                            pos
                        } as i32;

                        if start <= led as i32 && led as i32 <= end {
                            Hsv {
                                hue: animator.config.hue,
                                sat: animator.config.sat,
                                val: if (led as usize == D::NUM_LEDS - 1 && direction == 1)
                                    || (led == 0 && direction == -1)
                                {
                                    u8::MAX
                                } else if direction == 1 {
                                    (u8::MAX as f32 * (led as i32 - start + 1) as f32
                                        / length as f32) as u8
                                } else {
                                    (u8::MAX as f32 * (end - led as i32 + 1) as f32 / length as f32)
                                        as u8
                                },
                            }
                        } else {
                            Hsv::default()
                        }
                    })
                }
            }
            UnderglowEffect::Christmas => {
                if D::CHRISTMAS_ENABLED {
                    // Base speed: 1 second to transition colors
                    // 85 is the hue value corresponding to green.
                    self.set_brightness_for_each_led(|animator, time, led| {
                        let pos = ((time * 32.0) as i32 % 64 - 32).abs();
                        let hue = 85 * pos.pow(3) / (pos.pow(3) + (32 - pos).pow(3)); // Cubic bezier curve transition from QMK

                        Hsv {
                            hue: if led % 2 == 1 {
                                hue as u8
                            } else {
                                (85 - hue) as u8
                            },
                            sat: animator.config.sat,
                            // val calculation modified from QMK to use animator's val setting
                            val: (u8::MAX - (3 * (42 - (hue % 85 - 42).abs()) as u8) / 2),
                        }
                    })
                }
            }
            UnderglowEffect::StaticGradient => {
                if D::STATIC_GRADIENT_ENABLED {
                    const GRADIENT_RANGES: [u16; 5] = [255, 170, 127, 85, 64];

                    self.set_brightness_for_each_led(|animator, _time, led| {
                        let hue = led as u16 * GRADIENT_RANGES[4] / D::NUM_LEDS as u16;
                        Hsv {
                            hue: animator.config.hue + hue as u8,
                            sat: animator.config.sat,
                            val: u8::MAX,
                        }
                    })
                }
            }
            UnderglowEffect::RGBTest => {
                if D::RGB_TEST_ENABLED {
                    // Base speed: change colors every second
                    self.set_brightness_for_each_led(|animator, time, _led| {
                        let pos = (time as u32) % 3;
                        let mut hsv = Hsv::default();

                        // Test red
                        if pos == 0 {
                            hsv = Hsv {
                                hue: 0,
                                sat: animator.config.sat,
                                val: u8::MAX,
                            }
                        }

                        // Test green
                        if pos == 1 {
                            hsv = Hsv {
                                hue: 85,
                                sat: animator.config.sat,
                                val: u8::MAX,
                            }
                        }

                        // Test blue
                        if pos == 2 {
                            hsv = Hsv {
                                hue: 170,
                                sat: animator.config.sat,
                                val: u8::MAX,
                            }
                        }

                        hsv
                    })
                }
            }
            UnderglowEffect::Alternating => {
                if D::ALTERNATING_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, led| {
                        let pos = (time as u32) % 2;
                        if (led as usize) < D::NUM_LEDS / 2 && pos == 1
                            || (led as usize) >= D::NUM_LEDS / 2 && pos == 0
                        {
                            Hsv {
                                hue: animator.config.hue,
                                sat: animator.config.sat,
                                val: u8::MAX,
                            }
                        } else {
                            Hsv {
                                hue: animator.config.hue,
                                sat: animator.config.sat,
                                val: 0,
                            }
                        }
                    })
                }
            }
            UnderglowEffect::Twinkle => {
                if D::TWINKLE_ENABLED {
                    self.set_brightness_for_each_led(|animator, _time, led| {
                        // if selected
                        if ((animator.rng.next_u32() as f32 * u8::MAX as f32 / u32::MAX as f32)
                            as u8)
                            < ((0.05 * u8::MAX as f32) as u8)
                            && animator.twinkle_state[led as usize].1 == 0
                            && animator.tick % (0.05 * D::FPS as f32) as u32 == 0
                        {
                            animator.twinkle_state[led as usize].0.hue =
                                (animator.rng.next_u32() as f32 * u8::MAX as f32 / u32::MAX as f32)
                                    as u8;
                            animator.twinkle_state[led as usize].0.sat =
                                (animator.rng.next_u32() as f32 * u8::MAX as f32 / u32::MAX as f32)
                                    as u8;
                            animator.twinkle_state[led as usize].1 = u8::MAX;
                        }

                        // update the rest
                        if animator.twinkle_state[led as usize].1 > 0 {
                            animator.twinkle_state[led as usize].1 = animator.twinkle_state
                                [led as usize]
                                .1
                                .saturating_sub((u8::MAX as f32 / D::FPS as f32) as u8);

                            Hsv {
                                hue: animator.twinkle_state[led as usize].0.hue,
                                sat: (animator.twinkle_state[led as usize].0.sat as u16
                                    * animator.config.sat as u16
                                    / u8::MAX as u16) as u8,
                                val: (sin((animator.twinkle_state[led as usize].1 as f32 - 64.0)
                                    * PI
                                    / 127.0)
                                    * u8::MAX as f32
                                    / 2.0
                                    + u8::MAX as f32 / 2.0)
                                    as u8,
                            }
                        } else {
                            Hsv::default()
                        }
                    })
                }
            }
            UnderglowEffect::Reactive => {
                if D::REACTIVE_ENABLED {
                    let pos = (((self.tick - self.time_of_last_press) as f32 / D::FPS as f32)
                        * (self.config.speed as f32 * 1.5 / u8::MAX as f32 + 0.5))
                        .min(1.0);

                    self.set_brightness_for_each_led(|animator, _time, _led| Hsv {
                        hue: animator.config.hue,
                        sat: animator.config.sat,
                        val: u8::MAX - (animator.config.val as f32 * pos) as u8,
                    })
                }
            }
        }

        if let Err(err) = self.driver.write(self.buf.iter().cloned()).await {
            error!(
                "[UNDERGLOW] Couldn't update underglow colors: {}",
                Debug2Format(&err)
            );
        };

        self.tick += 1;
    }
}
