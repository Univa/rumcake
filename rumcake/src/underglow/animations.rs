use super::drivers::UnderglowDriver;
use super::UnderglowDevice;
use crate::math::{scale, sin};
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, MaxSize)]
pub enum UnderglowCommand {
    Toggle,
    TurnOn,
    TurnOff,
    NextEffect,
    PrevEffect,
    SetEffect(UnderglowEffect),
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

impl UnderglowEffect {
    fn is_enabled<D: UnderglowDevice>(&self) -> bool {
        match self {
            UnderglowEffect::Solid => D::SOLID_ENABLED,
            UnderglowEffect::Breathing => D::BREATHING_ENABLED,
            UnderglowEffect::RainbowMood => D::RAINBOW_MOOD_ENABLED,
            UnderglowEffect::RainbowSwirl => D::RAINBOW_SWIRL_ENABLED,
            UnderglowEffect::Snake => D::SNAKE_ENABLED,
            UnderglowEffect::Knight => D::KNIGHT_ENABLED,
            UnderglowEffect::Christmas => D::CHRISTMAS_ENABLED,
            UnderglowEffect::StaticGradient => D::STATIC_GRADIENT_ENABLED,
            UnderglowEffect::RGBTest => D::RGB_TEST_ENABLED,
            UnderglowEffect::Alternating => D::ALTERNATING_ENABLED,
            UnderglowEffect::Twinkle => D::TWINKLE_ENABLED,
            UnderglowEffect::Reactive => D::REACTIVE_ENABLED,
        }
    }
}

pub(super) struct UnderglowAnimator<R: UnderglowDriver<D>, D: UnderglowDevice>
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

impl<R: UnderglowDriver<D>, D: UnderglowDevice> UnderglowAnimator<R, D>
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
            UnderglowCommand::TurnOn => {
                self.config.enabled = true;
            }
            UnderglowCommand::TurnOff => {
                self.config.enabled = false;
            }
            UnderglowCommand::NextEffect => {
                // We assume that there is always at least one effect enabled
                while {
                    self.config.effect.increment();
                    !self.config.effect.is_enabled::<D>()
                } {}
            }
            UnderglowCommand::PrevEffect => {
                while {
                    self.config.effect.decrement();
                    !self.config.effect.is_enabled::<D>()
                } {}
            }
            UnderglowCommand::SetEffect(effect) => {
                self.config.effect = effect;
            }
            UnderglowCommand::SetHue(hue) => {
                self.config.hue = hue;
            }
            UnderglowCommand::IncreaseHue(amount) => {
                self.config.hue = self.config.hue.saturating_add(amount);
            }
            UnderglowCommand::DecreaseHue(amount) => {
                self.config.hue = self.config.hue.saturating_sub(amount);
            }
            UnderglowCommand::SetSaturation(sat) => {
                self.config.sat = sat;
            }
            UnderglowCommand::IncreaseSaturation(amount) => {
                self.config.sat = self.config.sat.saturating_add(amount);
            }
            UnderglowCommand::DecreaseSaturation(amount) => {
                self.config.sat = self.config.sat.saturating_sub(amount);
            }
            UnderglowCommand::SetValue(val) => {
                self.config.val = val;
            }
            UnderglowCommand::IncreaseValue(amount) => {
                self.config.val = self.config.val.saturating_add(amount);
            }
            UnderglowCommand::DecreaseValue(amount) => {
                self.config.val = self.config.val.saturating_sub(amount);
            }
            UnderglowCommand::SetSpeed(speed) => {
                self.config.speed = speed;
            }
            UnderglowCommand::IncreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_add(amount);
            }
            UnderglowCommand::DecreaseSpeed(amount) => {
                self.config.speed = self.config.speed.saturating_sub(amount);
            }
            #[cfg(feature = "storage")]
            UnderglowCommand::SaveConfig => {
                super::storage::UNDERGLOW_SAVE_SIGNAL.signal(());
            }
            UnderglowCommand::ResetTime => {
                self.tick = 0;
            }
        };
    }

    pub fn set_brightness_for_each_led(&mut self, calc: impl Fn(&mut Self, u32, u8) -> Hsv) {
        let time = (self.tick << 8)
            / (((D::FPS as u32) << 8)
                / (self.config.speed as u32 + 128 + (self.config.speed as u32 >> 1))); // `time` should increment by 255 every second

        for led in 0..D::NUM_LEDS {
            let mut hsv = calc(self, time, led as u8);
            hsv.val = scale(hsv.val, self.config.val);
            self.buf[led] = hsv2rgb(hsv);
        }
    }

    pub fn register_event(&mut self, event: Event) {
        match event {
            Event::Press(_x, _y) => {
                self.time_of_last_press = (self.tick << 8)
                    / (((D::FPS as u32) << 8)
                        / (self.config.speed as u32 + 128 + (self.config.speed as u32 >> 1)));
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
                        val: sin((time >> 2) as u8), // 4 seconds for one full cycle
                    })
                }
            }
            UnderglowEffect::RainbowMood => {
                if D::RAINBOW_MOOD_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _led| Hsv {
                        hue: (time >> 4) as u8, // 16 seconds for a full cycle
                        sat: animator.config.sat,
                        val: u8::MAX,
                    })
                }
            }
            UnderglowEffect::RainbowSwirl => {
                if D::RAINBOW_SWIRL_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, led| Hsv {
                        hue: ((((led as u16) << 8) / D::NUM_LEDS as u16) as u8)
                            .wrapping_add((time >> 4) as u8), // 16 seconds for a full cycle
                        sat: animator.config.sat,
                        val: u8::MAX,
                    })
                }
            }
            UnderglowEffect::Snake => {
                if D::SNAKE_ENABLED {
                    let length = 4;

                    self.set_brightness_for_each_led(|animator, time, led| {
                        let pos = scale(time as u8, D::NUM_LEDS as u8); // 1 second for a full cycle

                        for j in 0..length {
                            let lit = (pos + j) % D::NUM_LEDS as u8;

                            if led == lit {
                                return Hsv {
                                    hue: animator.config.hue,
                                    sat: animator.config.sat,
                                    val: (u8::MAX as u16 * (j + 1) as u16 / length as u16) as u8,
                                };
                            }
                        }

                        Hsv::default()
                    })
                }
            }
            UnderglowEffect::Knight => {
                if D::KNIGHT_ENABLED {
                    let length: u32 = 4;

                    self.set_brightness_for_each_led(|animator, time, led| {
                        let pos = ((time * D::NUM_LEDS as u32) >> 8)
                            % ((D::NUM_LEDS as u32 + length - 1) * 2); // 1 second to traverse a length of NUM_LEDS

                        let direction = if pos >= (D::NUM_LEDS as u32 + length - 1) {
                            1 // going back
                        } else {
                            0 // going forward
                        };

                        let start = if direction == 1 {
                            2 * D::NUM_LEDS as u32 - pos + length - 2
                        } else {
                            pos - length + 1
                        } as i32;

                        let end = if direction == 1 {
                            2 * D::NUM_LEDS as u32 - pos + 2 * length - 3
                        } else {
                            pos
                        } as i32;

                        Hsv {
                            hue: animator.config.hue,
                            sat: animator.config.sat,
                            val: if start <= led as i32 && led as i32 <= end {
                                u8::MAX
                            } else {
                                0
                            },
                        }
                    })
                }
            }
            UnderglowEffect::Christmas => {
                if D::CHRISTMAS_ENABLED {
                    // 85 is the hue value corresponding to green.
                    self.set_brightness_for_each_led(|animator, time, led| {
                        let pos = (((time * 32) >> 8) % 64).abs_diff(32); // 1 second to transition colors
                        let hue = 85 * pos.pow(3) / (pos.pow(3) + (32 - pos).pow(3)); // Cubic bezier curve transition from QMK

                        Hsv {
                            hue: if led % 2 == 1 {
                                hue as u8
                            } else {
                                (85 - hue) as u8
                            },
                            sat: animator.config.sat,
                            // val calculation modified from QMK to use animator's val setting
                            val: (u8::MAX - (3 * (42 - (hue % 85).abs_diff(42)) as u8) / 2),
                        }
                    })
                }
            }
            UnderglowEffect::StaticGradient => {
                if D::STATIC_GRADIENT_ENABLED {
                    // TODO: decide on a parameter to control gradient range
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
                    self.set_brightness_for_each_led(|animator, time, _led| {
                        let pos = (time >> 8) % 3; // Change colors every second

                        // Test red
                        if pos == 0 {
                            return Hsv {
                                hue: 0,
                                sat: animator.config.sat,
                                val: u8::MAX,
                            };
                        }

                        // Test green
                        if pos == 1 {
                            return Hsv {
                                hue: 85,
                                sat: animator.config.sat,
                                val: u8::MAX,
                            };
                        }

                        // Test blue
                        if pos == 2 {
                            return Hsv {
                                hue: 170,
                                sat: animator.config.sat,
                                val: u8::MAX,
                            };
                        }

                        Hsv::default()
                    })
                }
            }
            UnderglowEffect::Alternating => {
                if D::ALTERNATING_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, led| {
                        let pos = (time >> 8) % 2; // Flip every second
                        let threshold = (D::NUM_LEDS / 2) as u16;
                        let led = led as u16;

                        Hsv {
                            hue: animator.config.hue,
                            sat: animator.config.sat,
                            val: if (pos == 1 && led < threshold) || (pos == 0 && led >= threshold)
                            {
                                u8::MAX
                            } else {
                                0
                            },
                        }
                    })
                }
            }
            UnderglowEffect::Twinkle => {
                if D::TWINKLE_ENABLED {
                    let adjusted_fps = (((D::FPS as u32) << 8)
                        / (self.config.speed as u32 + 128 + (self.config.speed as u32 >> 1)))
                        as u8;

                    self.set_brightness_for_each_led(|animator, _time, led| {
                        // we will dissect the bits of this random number to set some parameters
                        let rand = animator.rng.next_u32();
                        let data = animator.twinkle_state.get_mut(led as usize).unwrap();

                        // 5% chance of being selected
                        // check if the upper 8 bits correspond to a u8 that is less than 13
                        if (rand as u8) < 13
                            && data.1 == 0
                            && animator.tick % (1 + scale(adjusted_fps, 13) as u32) == 0
                        {
                            // use the next 8 bits for hue
                            data.0.hue = (rand >> 8) as u8;
                            // use the next 8 bits for saturation
                            data.0.sat = (rand >> 16) as u8;
                            data.1 = u8::MAX;
                        }

                        // update the rest
                        data.1 = data.1.saturating_sub(u8::MAX / adjusted_fps);

                        Hsv {
                            hue: data.0.hue,
                            sat: scale(data.0.sat, animator.config.sat),
                            val: sin(data.1.wrapping_sub(64)),
                        }
                    })
                }
            }
            UnderglowEffect::Reactive => {
                if D::REACTIVE_ENABLED {
                    self.set_brightness_for_each_led(|animator, time, _led| Hsv {
                        hue: animator.config.hue,
                        sat: animator.config.sat,
                        val: (u8::MAX as u32).saturating_sub(time - animator.time_of_last_press)
                            as u8, // LED fades after one second
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
