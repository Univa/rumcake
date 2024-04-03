//! Functions for handling VialRGB effect ID conversions.

use crate::lighting::rgb_backlight_matrix::private::MaybeRGBBacklightMatrixDevice;
use crate::lighting::rgb_backlight_matrix::RGBBacklightMatrixEffect;
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

pub(crate) fn convert_effect_to_vialrgb_id(effect: RGBBacklightMatrixEffect) -> u16 {
    match effect {
        RGBBacklightMatrixEffect::Solid => VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_COLOR as u16,
        RGBBacklightMatrixEffect::AlphasMods => VialRGBEffectIDs::VIALRGB_EFFECT_ALPHAS_MODS as u16,
        RGBBacklightMatrixEffect::GradientUpDown => {
            VialRGBEffectIDs::VIALRGB_EFFECT_GRADIENT_UP_DOWN as u16
        }
        RGBBacklightMatrixEffect::GradientLeftRight => {
            VialRGBEffectIDs::VIALRGB_EFFECT_GRADIENT_LEFT_RIGHT as u16
        }
        RGBBacklightMatrixEffect::Breathing => VialRGBEffectIDs::VIALRGB_EFFECT_BREATHING as u16,
        RGBBacklightMatrixEffect::ColorbandSat => VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SAT as u16,
        RGBBacklightMatrixEffect::ColorbandVal => VialRGBEffectIDs::VIALRGB_EFFECT_BAND_VAL as u16,
        RGBBacklightMatrixEffect::ColorbandPinWheelSat => {
            VialRGBEffectIDs::VIALRGB_EFFECT_BAND_PINWHEEL_SAT as u16
        }
        RGBBacklightMatrixEffect::ColorbandPinWheelVal => {
            VialRGBEffectIDs::VIALRGB_EFFECT_BAND_PINWHEEL_VAL as u16
        }
        RGBBacklightMatrixEffect::ColorbandSpiralSat => {
            VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SPIRAL_SAT as u16
        }
        RGBBacklightMatrixEffect::ColorbandSpiralVal => {
            VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SPIRAL_VAL as u16
        }
        RGBBacklightMatrixEffect::CycleAll => VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_ALL as u16,
        RGBBacklightMatrixEffect::CycleLeftRight => {
            VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_LEFT_RIGHT as u16
        }
        RGBBacklightMatrixEffect::CycleUpDown => {
            VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_UP_DOWN as u16
        }
        RGBBacklightMatrixEffect::RainbowMovingChevron => {
            VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_MOVING_CHEVRON as u16
        }
        RGBBacklightMatrixEffect::CycleOutIn => {
            VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_OUT_IN as u16
        }
        RGBBacklightMatrixEffect::CycleOutInDual => {
            VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_OUT_IN_DUAL as u16
        }
        RGBBacklightMatrixEffect::CyclePinWheel => {
            VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_PINWHEEL as u16
        }
        RGBBacklightMatrixEffect::CycleSpiral => {
            VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_SPIRAL as u16
        }
        RGBBacklightMatrixEffect::DualBeacon => VialRGBEffectIDs::VIALRGB_EFFECT_DUAL_BEACON as u16,
        RGBBacklightMatrixEffect::RainbowBeacon => {
            VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_BEACON as u16
        }
        RGBBacklightMatrixEffect::RainbowPinWheels => {
            VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_PINWHEELS as u16
        }
        RGBBacklightMatrixEffect::Raindrops => VialRGBEffectIDs::VIALRGB_EFFECT_RAINDROPS as u16,
        RGBBacklightMatrixEffect::JellybeanRaindrops => {
            VialRGBEffectIDs::VIALRGB_EFFECT_JELLYBEAN_RAINDROPS as u16
        }
        RGBBacklightMatrixEffect::HueBreathing => {
            VialRGBEffectIDs::VIALRGB_EFFECT_HUE_BREATHING as u16
        }
        RGBBacklightMatrixEffect::HuePendulum => {
            VialRGBEffectIDs::VIALRGB_EFFECT_HUE_PENDULUM as u16
        }
        RGBBacklightMatrixEffect::HueWave => VialRGBEffectIDs::VIALRGB_EFFECT_HUE_WAVE as u16,
        RGBBacklightMatrixEffect::PixelRain => VialRGBEffectIDs::VIALRGB_EFFECT_PIXEL_RAIN as u16,
        RGBBacklightMatrixEffect::PixelFlow => UNKNOWN_EFFECT,
        RGBBacklightMatrixEffect::PixelFractal => {
            VialRGBEffectIDs::VIALRGB_EFFECT_PIXEL_FRACTAL as u16
        }
        RGBBacklightMatrixEffect::TypingHeatmap => {
            VialRGBEffectIDs::VIALRGB_EFFECT_TYPING_HEATMAP as u16
        }
        RGBBacklightMatrixEffect::DigitalRain => {
            VialRGBEffectIDs::VIALRGB_EFFECT_DIGITAL_RAIN as u16
        }
        RGBBacklightMatrixEffect::SolidReactiveSimple => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_SIMPLE as u16
        }
        RGBBacklightMatrixEffect::SolidReactive => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE as u16
        }
        RGBBacklightMatrixEffect::SolidReactiveWide => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_WIDE as u16
        }
        RGBBacklightMatrixEffect::SolidReactiveMultiWide => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTIWIDE as u16
        }
        RGBBacklightMatrixEffect::SolidReactiveCross => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_CROSS as u16
        }
        RGBBacklightMatrixEffect::SolidReactiveMultiCross => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTICROSS as u16
        }
        RGBBacklightMatrixEffect::SolidReactiveNexus => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_NEXUS as u16
        }
        RGBBacklightMatrixEffect::SolidReactiveMultiNexus => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTINEXUS as u16
        }
        RGBBacklightMatrixEffect::Splash => VialRGBEffectIDs::VIALRGB_EFFECT_SPLASH as u16,
        RGBBacklightMatrixEffect::MultiSplash => {
            VialRGBEffectIDs::VIALRGB_EFFECT_MULTISPLASH as u16
        }
        RGBBacklightMatrixEffect::SolidSplash => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_SPLASH as u16
        }
        RGBBacklightMatrixEffect::SolidMultiSplash => {
            VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_MULTISPLASH as u16
        }
        RGBBacklightMatrixEffect::DirectSet => VialRGBEffectIDs::VIALRGB_EFFECT_DIRECT as u16,
    }
}

pub(crate) fn convert_vialrgb_id_to_effect(id: u16) -> Option<RGBBacklightMatrixEffect> {
    match num::FromPrimitive::from_u16(id) as Option<VialRGBEffectIDs> {
        Some(vialrgb_id) => {
            match vialrgb_id {
                VialRGBEffectIDs::VIALRGB_EFFECT_OFF => None, // ID 0 is handled in the protocol by disabling the rgb matrix system
                VialRGBEffectIDs::VIALRGB_EFFECT_DIRECT => {
                    Some(RGBBacklightMatrixEffect::DirectSet)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_COLOR => {
                    Some(RGBBacklightMatrixEffect::Solid)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_ALPHAS_MODS => {
                    Some(RGBBacklightMatrixEffect::AlphasMods)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_GRADIENT_UP_DOWN => {
                    Some(RGBBacklightMatrixEffect::GradientUpDown)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_GRADIENT_LEFT_RIGHT => {
                    Some(RGBBacklightMatrixEffect::GradientLeftRight)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BREATHING => {
                    Some(RGBBacklightMatrixEffect::Breathing)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SAT => {
                    Some(RGBBacklightMatrixEffect::ColorbandSat)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_VAL => {
                    Some(RGBBacklightMatrixEffect::ColorbandVal)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_PINWHEEL_SAT => {
                    Some(RGBBacklightMatrixEffect::ColorbandPinWheelSat)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_PINWHEEL_VAL => {
                    Some(RGBBacklightMatrixEffect::ColorbandPinWheelVal)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SPIRAL_SAT => {
                    Some(RGBBacklightMatrixEffect::ColorbandSpiralSat)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_BAND_SPIRAL_VAL => {
                    Some(RGBBacklightMatrixEffect::ColorbandSpiralVal)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_ALL => {
                    Some(RGBBacklightMatrixEffect::CycleAll)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_LEFT_RIGHT => {
                    Some(RGBBacklightMatrixEffect::CycleLeftRight)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_UP_DOWN => {
                    Some(RGBBacklightMatrixEffect::CycleUpDown)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_MOVING_CHEVRON => {
                    Some(RGBBacklightMatrixEffect::RainbowMovingChevron)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_OUT_IN => {
                    Some(RGBBacklightMatrixEffect::CycleOutIn)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_OUT_IN_DUAL => {
                    Some(RGBBacklightMatrixEffect::CycleOutInDual)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_PINWHEEL => {
                    Some(RGBBacklightMatrixEffect::CyclePinWheel)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_CYCLE_SPIRAL => {
                    Some(RGBBacklightMatrixEffect::CycleSpiral)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_DUAL_BEACON => {
                    Some(RGBBacklightMatrixEffect::DualBeacon)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_BEACON => {
                    Some(RGBBacklightMatrixEffect::RainbowBeacon)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_RAINBOW_PINWHEELS => {
                    Some(RGBBacklightMatrixEffect::RainbowPinWheels)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_RAINDROPS => {
                    Some(RGBBacklightMatrixEffect::Raindrops)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_JELLYBEAN_RAINDROPS => {
                    Some(RGBBacklightMatrixEffect::JellybeanRaindrops)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_HUE_BREATHING => {
                    Some(RGBBacklightMatrixEffect::HueBreathing)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_HUE_PENDULUM => {
                    Some(RGBBacklightMatrixEffect::HuePendulum)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_HUE_WAVE => {
                    Some(RGBBacklightMatrixEffect::HueWave)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_TYPING_HEATMAP => {
                    Some(RGBBacklightMatrixEffect::TypingHeatmap)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_DIGITAL_RAIN => {
                    Some(RGBBacklightMatrixEffect::DigitalRain)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_SIMPLE => {
                    Some(RGBBacklightMatrixEffect::SolidReactiveSimple)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE => {
                    Some(RGBBacklightMatrixEffect::SolidReactive)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_WIDE => {
                    Some(RGBBacklightMatrixEffect::SolidReactiveWide)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTIWIDE => {
                    Some(RGBBacklightMatrixEffect::SolidReactiveMultiWide)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_CROSS => {
                    Some(RGBBacklightMatrixEffect::SolidReactiveCross)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTICROSS => {
                    Some(RGBBacklightMatrixEffect::SolidReactiveMultiCross)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_NEXUS => {
                    Some(RGBBacklightMatrixEffect::SolidReactiveNexus)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_REACTIVE_MULTINEXUS => {
                    Some(RGBBacklightMatrixEffect::SolidReactiveMultiNexus)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SPLASH => Some(RGBBacklightMatrixEffect::Splash),
                VialRGBEffectIDs::VIALRGB_EFFECT_MULTISPLASH => {
                    Some(RGBBacklightMatrixEffect::MultiSplash)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_SPLASH => {
                    Some(RGBBacklightMatrixEffect::SolidSplash)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_SOLID_MULTISPLASH => {
                    Some(RGBBacklightMatrixEffect::SolidMultiSplash)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_PIXEL_RAIN => {
                    Some(RGBBacklightMatrixEffect::PixelRain)
                }
                VialRGBEffectIDs::VIALRGB_EFFECT_PIXEL_FRACTAL => {
                    Some(RGBBacklightMatrixEffect::PixelFractal)
                }
            }
        }
        None => None, // Instead of defaulting to ID 1 (solid color), which Vial's QMK implementation does, we just do nothing
    }
}

pub(crate) fn is_supported<K: MaybeRGBBacklightMatrixDevice>(id: u16) -> bool {
    match convert_vialrgb_id_to_effect(id) {
        Some(effect) => K::is_effect_enabled(effect),
        None => false,
    }
}
