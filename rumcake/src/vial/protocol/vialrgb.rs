//! Functions for handling VialRGB effect ID conversions.

use crate::backlight::animations::BacklightEffect;
use crate::backlight::BacklightDevice;
use num_derive::FromPrimitive;

#[repr(u16)]
#[allow(non_camel_case_types)]
#[derive(FromPrimitive)]
/// List of VialRGB effect IDs. This is mainly used for reference
enum VialRGBEffectIDs {
    VIALRGB_EFFECT_OFF = 0,
    VIALRGB_EFFECT_DIRECT = 1,
    VIALRGB_EFFECT_SOLID_COLOR = 2,
    VIALRGB_EFFECT_ALPHAS_MODS = 3,
    VIALRGB_EFFECT_GRADIENT_UP_DOWN = 4,
    VIALRGB_EFFECT_GRADIENT_LEFT_RIGHT = 5,
    VIALRGB_EFFECT_BREATHING = 6,
    VIALRGB_EFFECT_BAND_SAT = 7,
    VIALRGB_EFFECT_BAND_VAL = 8,
    VIALRGB_EFFECT_BAND_PINWHEEL_SAT = 9,
    VIALRGB_EFFECT_BAND_PINWHEEL_VAL = 10,
    VIALRGB_EFFECT_BAND_SPIRAL_SAT = 11,
    VIALRGB_EFFECT_BAND_SPIRAL_VAL = 12,
    VIALRGB_EFFECT_CYCLE_ALL = 13,
    VIALRGB_EFFECT_CYCLE_LEFT_RIGHT = 14,
    VIALRGB_EFFECT_CYCLE_UP_DOWN = 15,
    VIALRGB_EFFECT_RAINBOW_MOVING_CHEVRON = 16,
    VIALRGB_EFFECT_CYCLE_OUT_IN = 17,
    VIALRGB_EFFECT_CYCLE_OUT_IN_DUAL = 18,
    VIALRGB_EFFECT_CYCLE_PINWHEEL = 19,
    VIALRGB_EFFECT_CYCLE_SPIRAL = 20,
    VIALRGB_EFFECT_DUAL_BEACON = 21,
    VIALRGB_EFFECT_RAINBOW_BEACON = 22,
    VIALRGB_EFFECT_RAINBOW_PINWHEELS = 23,
    VIALRGB_EFFECT_RAINDROPS = 24,
    VIALRGB_EFFECT_JELLYBEAN_RAINDROPS = 25,
    VIALRGB_EFFECT_HUE_BREATHING = 26,
    VIALRGB_EFFECT_HUE_PENDULUM = 27,
    VIALRGB_EFFECT_HUE_WAVE = 28,
    VIALRGB_EFFECT_TYPING_HEATMAP = 29,
    VIALRGB_EFFECT_DIGITAL_RAIN = 30,
    VIALRGB_EFFECT_SOLID_REACTIVE_SIMPLE = 31,
    VIALRGB_EFFECT_SOLID_REACTIVE = 32,
    VIALRGB_EFFECT_SOLID_REACTIVE_WIDE = 33,
    VIALRGB_EFFECT_SOLID_REACTIVE_MULTIWIDE = 34,
    VIALRGB_EFFECT_SOLID_REACTIVE_CROSS = 35,
    VIALRGB_EFFECT_SOLID_REACTIVE_MULTICROSS = 36,
    VIALRGB_EFFECT_SOLID_REACTIVE_NEXUS = 37,
    VIALRGB_EFFECT_SOLID_REACTIVE_MULTINEXUS = 38,
    VIALRGB_EFFECT_SPLASH = 39,
    VIALRGB_EFFECT_MULTISPLASH = 40,
    VIALRGB_EFFECT_SOLID_SPLASH = 41,
    VIALRGB_EFFECT_SOLID_MULTISPLASH = 42,
    VIALRGB_EFFECT_PIXEL_RAIN = 43,
    VIALRGB_EFFECT_PIXEL_FRACTAL = 44,
}

pub(crate) const MAX_VIALRGB_ID: u16 = VialRGBEffectIDs::VIALRGB_EFFECT_PIXEL_FRACTAL as u16;
const UNKNOWN_EFFECT: u16 = 0;

pub(crate) fn convert_effect_to_vialrgb_id(effect: BacklightEffect) -> u16 {
    match effect {
        BacklightEffect::Solid => VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_COLOR as u16,
        BacklightEffect::AlphasMods => VialRGBEffectIDs::VIALRGB_EFFECT_ALPHAS_MODS as u16,
        BacklightEffect::GradientUpDown => VialRGBEffectIDs::VIALRGB_EFFECT_GRADIENT_UP_DOWN as u16,
        BacklightEffect::GradientLeftRight => {
            VialRGBEffectIDs::VIALRGB_EFFECT_GRADIENT_LEFT_RIGHT as u16
        }
        BacklightEffect::Breathing => VialRGBEffectIDs::VIALRGB_EFFECT_BREATHING as u16,
        BacklightEffect::ColorbandSat => VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SAT as u16,
        BacklightEffect::ColorbandVal => VialRGBEffectIDs::VIALRGB_EFFECT_BAND_VAL as u16,
        BacklightEffect::ColorbandPinWheelSat => {
            VialRGBEffectIDs::VIALRGB_EFFECT_BAND_PINWHEEL_SAT as u16
        }
        BacklightEffect::ColorbandPinWheelVal => {
            VialRGBEffectIDs::VIALRGB_EFFECT_BAND_PINWHEEL_VAL as u16
        }
        BacklightEffect::ColorbandSpiralSat => {
            VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SPIRAL_SAT as u16
        }
        BacklightEffect::ColorbandSpiralVal => {
            VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SPIRAL_VAL as u16
        }
        BacklightEffect::CycleAll => VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_ALL as u16,
        BacklightEffect::CycleLeftRight => VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_LEFT_RIGHT as u16,
        BacklightEffect::CycleUpDown => VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_UP_DOWN as u16,
        BacklightEffect::RainbowMovingChevron => {
            VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_MOVING_CHEVRON as u16
        }
        BacklightEffect::CycleOutIn => VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_OUT_IN as u16,
        BacklightEffect::CycleOutInDual => {
            VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_OUT_IN_DUAL as u16
        }
        BacklightEffect::CyclePinWheel => VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_PINWHEEL as u16,
        BacklightEffect::CycleSpiral => VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_SPIRAL as u16,
        BacklightEffect::DualBeacon => VialRGBEffectIDs::VIALRGB_EFFECT_DUAL_BEACON as u16,
        BacklightEffect::RainbowBeacon => VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_BEACON as u16,
        BacklightEffect::RainbowPinWheels => {
            VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_PINWHEELS as u16
        }
        BacklightEffect::Raindrops => VialRGBEffectIDs::VIALRGB_EFFECT_RAINDROPS as u16,
        BacklightEffect::JellybeanRaindrops => {
            VialRGBEffectIDs::VIALRGB_EFFECT_JELLYBEAN_RAINDROPS as u16
        }
        BacklightEffect::HueBreathing => VialRGBEffectIDs::VIALRGB_EFFECT_HUE_BREATHING as u16,
        BacklightEffect::HuePendulum => VialRGBEffectIDs::VIALRGB_EFFECT_HUE_PENDULUM as u16,
        BacklightEffect::HueWave => VialRGBEffectIDs::VIALRGB_EFFECT_HUE_WAVE as u16,
        BacklightEffect::PixelRain => VialRGBEffectIDs::VIALRGB_EFFECT_PIXEL_RAIN as u16,
        BacklightEffect::PixelFlow => UNKNOWN_EFFECT,
        BacklightEffect::PixelFractal => VialRGBEffectIDs::VIALRGB_EFFECT_PIXEL_FRACTAL as u16,
        BacklightEffect::TypingHeatmap => VialRGBEffectIDs::VIALRGB_EFFECT_TYPING_HEATMAP as u16,
        BacklightEffect::DigitalRain => VialRGBEffectIDs::VIALRGB_EFFECT_DIGITAL_RAIN as u16,
        BacklightEffect::SolidReactiveSimple => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_SIMPLE as u16
        }
        BacklightEffect::SolidReactive => VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE as u16,
        BacklightEffect::SolidReactiveWide => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_WIDE as u16
        }
        BacklightEffect::SolidReactiveMultiWide => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTIWIDE as u16
        }
        BacklightEffect::SolidReactiveCross => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_CROSS as u16
        }
        BacklightEffect::SolidReactiveMultiCross => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTICROSS as u16
        }
        BacklightEffect::SolidReactiveNexus => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_NEXUS as u16
        }
        BacklightEffect::SolidReactiveMultiNexus => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTINEXUS as u16
        }
        BacklightEffect::Splash => VialRGBEffectIDs::VIALRGB_EFFECT_SPLASH as u16,
        BacklightEffect::MultiSplash => VialRGBEffectIDs::VIALRGB_EFFECT_MULTISPLASH as u16,
        BacklightEffect::SolidSplash => VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_SPLASH as u16,
        BacklightEffect::SolidMultiSplash => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_MULTISPLASH as u16
        }
        BacklightEffect::DirectSet => VialRGBEffectIDs::VIALRGB_EFFECT_DIRECT as u16,
    }
}

pub(crate) fn convert_vialrgb_id_to_effect(id: u16) -> Option<BacklightEffect> {
    match num::FromPrimitive::from_u16(id) as Option<VialRGBEffectIDs> {
        Some(vialrgb_id) => {
            match vialrgb_id {
                VialRGBEffectIDs::VIALRGB_EFFECT_OFF => None, // ID 0 is handled in the protocol by disabling the rgb matrix system
                VialRGBEffectIDs::VIALRGB_EFFECT_DIRECT => Some(BacklightEffect::DirectSet),
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_COLOR => Some(BacklightEffect::Solid),
                VialRGBEffectIDs::VIALRGB_EFFECT_ALPHAS_MODS => Some(BacklightEffect::AlphasMods),
                VialRGBEffectIDs::VIALRGB_EFFECT_GRADIENT_UP_DOWN => {
                    Some(BacklightEffect::GradientUpDown)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_GRADIENT_LEFT_RIGHT => {
                    Some(BacklightEffect::GradientLeftRight)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BREATHING => Some(BacklightEffect::Breathing),
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SAT => Some(BacklightEffect::ColorbandSat),
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_VAL => Some(BacklightEffect::ColorbandVal),
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_PINWHEEL_SAT => {
                    Some(BacklightEffect::ColorbandPinWheelSat)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_PINWHEEL_VAL => {
                    Some(BacklightEffect::ColorbandPinWheelVal)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SPIRAL_SAT => {
                    Some(BacklightEffect::ColorbandSpiralSat)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SPIRAL_VAL => {
                    Some(BacklightEffect::ColorbandSpiralVal)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_ALL => Some(BacklightEffect::CycleAll),
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_LEFT_RIGHT => {
                    Some(BacklightEffect::CycleLeftRight)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_UP_DOWN => {
                    Some(BacklightEffect::CycleUpDown)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_MOVING_CHEVRON => {
                    Some(BacklightEffect::RainbowMovingChevron)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_OUT_IN => Some(BacklightEffect::CycleOutIn),
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_OUT_IN_DUAL => {
                    Some(BacklightEffect::CycleOutInDual)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_PINWHEEL => {
                    Some(BacklightEffect::CyclePinWheel)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_SPIRAL => Some(BacklightEffect::CycleSpiral),
                VialRGBEffectIDs::VIALRGB_EFFECT_DUAL_BEACON => Some(BacklightEffect::DualBeacon),
                VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_BEACON => {
                    Some(BacklightEffect::RainbowBeacon)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_PINWHEELS => {
                    Some(BacklightEffect::RainbowPinWheels)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_RAINDROPS => Some(BacklightEffect::Raindrops),
                VialRGBEffectIDs::VIALRGB_EFFECT_JELLYBEAN_RAINDROPS => {
                    Some(BacklightEffect::JellybeanRaindrops)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_HUE_BREATHING => {
                    Some(BacklightEffect::HueBreathing)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_HUE_PENDULUM => Some(BacklightEffect::HuePendulum),
                VialRGBEffectIDs::VIALRGB_EFFECT_HUE_WAVE => Some(BacklightEffect::HueWave),
                VialRGBEffectIDs::VIALRGB_EFFECT_TYPING_HEATMAP => {
                    Some(BacklightEffect::TypingHeatmap)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_DIGITAL_RAIN => Some(BacklightEffect::DigitalRain),
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_SIMPLE => {
                    Some(BacklightEffect::SolidReactiveSimple)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE => {
                    Some(BacklightEffect::SolidReactive)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_WIDE => {
                    Some(BacklightEffect::SolidReactiveWide)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTIWIDE => {
                    Some(BacklightEffect::SolidReactiveMultiWide)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_CROSS => {
                    Some(BacklightEffect::SolidReactiveCross)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTICROSS => {
                    Some(BacklightEffect::SolidReactiveMultiCross)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_NEXUS => {
                    Some(BacklightEffect::SolidReactiveNexus)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTINEXUS => {
                    Some(BacklightEffect::SolidReactiveMultiNexus)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SPLASH => Some(BacklightEffect::Splash),
                VialRGBEffectIDs::VIALRGB_EFFECT_MULTISPLASH => Some(BacklightEffect::MultiSplash),
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_SPLASH => Some(BacklightEffect::SolidSplash),
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_MULTISPLASH => {
                    Some(BacklightEffect::SolidMultiSplash)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_PIXEL_RAIN => Some(BacklightEffect::PixelRain),
                VialRGBEffectIDs::VIALRGB_EFFECT_PIXEL_FRACTAL => {
                    Some(BacklightEffect::PixelFractal)
                }
            }
        }
        None => None, // Instead of defaulting to ID 1 (solid color), which Vial's QMK implementation does, we just do nothing
    }
}

pub(crate) fn is_supported<K: BacklightDevice>(id: u16) -> bool {
    match convert_vialrgb_id_to_effect(id) {
        Some(effect) => effect.is_enabled::<K>(),
        None => false,
    }
}
