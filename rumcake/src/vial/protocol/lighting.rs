//! Functions for handling lighting-related features in Vial's protocol.
//!
//! Since Vial does not yet support Via's Custom UIs, this module is implemented to provide
//! conversion methods to convert between Vial's lighting effect IDs to rumcake's lighting effect
//! IDs. This handles conversion for `rgblight`/`underglow`, `backlight`/`simple-backlight`, and
//! `rgb_matrix`/`rgb-backlight-matrix`.

use num_derive::FromPrimitive;

#[repr(u8)]
#[derive(FromPrimitive)]
/// List of QMK rgblight (underglow) effect IDs. This is mainly used for reference
enum QMKRGBLightEffects {
    AllOff,
    SolidColor,
    Breathing1,
    Breathing2,
    Breathing3,
    Breathing4,
    RainbowMood1,
    RainbowMood2,
    RainbowMood3,
    RainbowSwirl1,
    RainbowSwirl2,
    RainbowSwirl3,
    RainbowSwirl4,
    RainbowSwirl5,
    RainbowSwirl6,
    Snake1,
    Snake2,
    Snake3,
    Snake4,
    Snake5,
    Snake6,
    Knight1,
    Knight2,
    Knight3,
    Christmas,
    Gradient1,
    Gradient2,
    Gradient3,
    Gradient4,
    Gradient5,
    Gradient6,
    Gradient7,
    Gradient8,
    Gradient9,
    Gradient10,
    RGBTest,
    Alternating,
}

const UNKNOWN_EFFECT: u8 = 0;

#[cfg(feature = "underglow")]
pub(crate) fn convert_underglow_effect_to_qmk_id(
    config: crate::underglow::animations::UnderglowConfig,
) -> u8 {
    use crate::underglow::animations::UnderglowEffect;
    match config.effect {
        UnderglowEffect::Solid => QMKRGBLightEffects::SolidColor as u8,
        UnderglowEffect::Breathing => match config.speed {
            0..=63 => QMKRGBLightEffects::Breathing1 as u8,
            64..=127 => QMKRGBLightEffects::Breathing2 as u8,
            128..=191 => QMKRGBLightEffects::Breathing3 as u8,
            192..=255 => QMKRGBLightEffects::Breathing4 as u8,
        },
        UnderglowEffect::RainbowMood => match config.speed {
            0..=85 => QMKRGBLightEffects::RainbowMood1 as u8,
            86..=171 => QMKRGBLightEffects::RainbowMood2 as u8,
            172..=255 => QMKRGBLightEffects::RainbowMood3 as u8,
        },
        UnderglowEffect::RainbowSwirl => match config.speed {
            0..=42 => QMKRGBLightEffects::RainbowSwirl1 as u8,
            43..=85 => QMKRGBLightEffects::RainbowSwirl2 as u8,
            86..=128 => QMKRGBLightEffects::RainbowSwirl3 as u8,
            129..=171 => QMKRGBLightEffects::RainbowSwirl4 as u8,
            172..=213 => QMKRGBLightEffects::RainbowSwirl5 as u8,
            214..=255 => QMKRGBLightEffects::RainbowSwirl6 as u8,
        },
        UnderglowEffect::Snake => match config.speed {
            0..=42 => QMKRGBLightEffects::Snake1 as u8,
            43..=85 => QMKRGBLightEffects::Snake2 as u8,
            86..=128 => QMKRGBLightEffects::Snake3 as u8,
            129..=171 => QMKRGBLightEffects::Snake4 as u8,
            172..=213 => QMKRGBLightEffects::Snake5 as u8,
            214..=255 => QMKRGBLightEffects::Snake6 as u8,
        },
        UnderglowEffect::Knight => match config.speed {
            0..=85 => QMKRGBLightEffects::Knight1 as u8,
            86..=171 => QMKRGBLightEffects::Knight2 as u8,
            172..=255 => QMKRGBLightEffects::Knight3 as u8,
        },
        UnderglowEffect::Christmas => QMKRGBLightEffects::Christmas as u8,
        UnderglowEffect::StaticGradient => QMKRGBLightEffects::Gradient1 as u8, // TODO: decide on a parameter to control gradient range
        UnderglowEffect::RGBTest => QMKRGBLightEffects::RGBTest as u8,
        UnderglowEffect::Alternating => QMKRGBLightEffects::Alternating as u8,
        UnderglowEffect::Twinkle => UNKNOWN_EFFECT,
        UnderglowEffect::Reactive => UNKNOWN_EFFECT,
    }
}

#[cfg(feature = "underglow")]
pub(crate) fn convert_qmk_id_to_underglow_effect(
    id: u8,
) -> Option<(crate::underglow::animations::UnderglowEffect, Option<u8>)> {
    use crate::underglow::animations::UnderglowEffect;
    match num::FromPrimitive::from_u8(id) as Option<QMKRGBLightEffects> {
        Some(effect) => match effect {
            QMKRGBLightEffects::AllOff => None, // ID 0 is handled in the protocol by disabling the underglow system
            QMKRGBLightEffects::SolidColor => Some((UnderglowEffect::Solid, None)),
            QMKRGBLightEffects::Breathing1 => Some((UnderglowEffect::Breathing, Some(0))),
            QMKRGBLightEffects::Breathing2 => Some((UnderglowEffect::Breathing, Some(85))),
            QMKRGBLightEffects::Breathing3 => Some((UnderglowEffect::Breathing, Some(171))),
            QMKRGBLightEffects::Breathing4 => Some((UnderglowEffect::Breathing, Some(255))),
            QMKRGBLightEffects::RainbowMood1 => Some((UnderglowEffect::RainbowMood, Some(0))),
            QMKRGBLightEffects::RainbowMood2 => Some((UnderglowEffect::RainbowMood, Some(127))),
            QMKRGBLightEffects::RainbowMood3 => Some((UnderglowEffect::RainbowMood, Some(255))),
            QMKRGBLightEffects::RainbowSwirl1 => Some((UnderglowEffect::RainbowSwirl, Some(0))),
            QMKRGBLightEffects::RainbowSwirl2 => Some((UnderglowEffect::RainbowSwirl, Some(51))),
            QMKRGBLightEffects::RainbowSwirl3 => Some((UnderglowEffect::RainbowSwirl, Some(102))),
            QMKRGBLightEffects::RainbowSwirl4 => Some((UnderglowEffect::RainbowSwirl, Some(153))),
            QMKRGBLightEffects::RainbowSwirl5 => Some((UnderglowEffect::RainbowSwirl, Some(204))),
            QMKRGBLightEffects::RainbowSwirl6 => Some((UnderglowEffect::RainbowSwirl, Some(255))),
            QMKRGBLightEffects::Snake1 => Some((UnderglowEffect::Snake, Some(0))),
            QMKRGBLightEffects::Snake2 => Some((UnderglowEffect::Snake, Some(51))),
            QMKRGBLightEffects::Snake3 => Some((UnderglowEffect::Snake, Some(102))),
            QMKRGBLightEffects::Snake4 => Some((UnderglowEffect::Snake, Some(153))),
            QMKRGBLightEffects::Snake5 => Some((UnderglowEffect::Snake, Some(204))),
            QMKRGBLightEffects::Snake6 => Some((UnderglowEffect::Snake, Some(255))),
            QMKRGBLightEffects::Knight1 => Some((UnderglowEffect::Knight, Some(0))),
            QMKRGBLightEffects::Knight2 => Some((UnderglowEffect::Knight, Some(127))),
            QMKRGBLightEffects::Knight3 => Some((UnderglowEffect::Knight, Some(255))),
            QMKRGBLightEffects::Christmas => Some((UnderglowEffect::Christmas, None)), // TODO: decide on a parameter to control gradient range
            QMKRGBLightEffects::Gradient1 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::Gradient2 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::Gradient3 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::Gradient4 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::Gradient5 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::Gradient6 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::Gradient7 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::Gradient8 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::Gradient9 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::Gradient10 => Some((UnderglowEffect::StaticGradient, None)),
            QMKRGBLightEffects::RGBTest => Some((UnderglowEffect::RGBTest, None)),
            QMKRGBLightEffects::Alternating => Some((UnderglowEffect::Alternating, None)),
        },
        None => None, // Instead of defaulting to the last valid effect like QMK, we will just do nothing
    }
}

#[repr(u8)]
#[derive(FromPrimitive)]
enum QMKBacklightEffects {
    Solid, // Not AllOff, because if we get an effect ID of 0, we should treat it as the solid color effect
    Breathing, // Breathing is represented as effect ID 1 in QMK's implementation of the Via protocol
}

#[cfg(feature = "simple-backlight")]
pub(crate) fn convert_backlight_effect_to_qmk_id(
    effect: crate::backlight::animations::BacklightEffect,
) -> u8 {
    use crate::backlight::animations::BacklightEffect;
    match effect {
        BacklightEffect::Solid => QMKBacklightEffects::Solid as u8,
        BacklightEffect::Breathing => QMKBacklightEffects::Breathing as u8,
        BacklightEffect::Reactive => UNKNOWN_EFFECT,
    }
}

#[cfg(feature = "simple-backlight")]
pub(crate) fn convert_qmk_id_to_backlight_effect(
    id: u8,
) -> Option<crate::backlight::animations::BacklightEffect> {
    use crate::backlight::animations::BacklightEffect;
    match num::FromPrimitive::from_u8(id) as Option<QMKBacklightEffects> {
        Some(effect) => match effect {
            QMKBacklightEffects::Solid => Some(BacklightEffect::Solid),
            QMKBacklightEffects::Breathing => Some(BacklightEffect::Breathing),
        },
        None => None, // Instead of defaulting to the last valid effect like QMK, we will just do nothing
    }
}
