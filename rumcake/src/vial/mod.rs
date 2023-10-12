use core::mem::size_of;

use defmt::{debug, error, warn, Debug2Format};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_usb::class::hid::HidWriter;
use embassy_usb::driver::Driver;
use embedded_storage::nor_flash::NorFlash;
use keyberon::debounce::Debouncer;
use num_derive::FromPrimitive;

use crate::via::{process_via_command, ViaCommandId, ViaKeyboard, VIA_REPORT_HID_SEND_CHANNEL};

pub const VIAL_PROTOCOL_VERSION: u32 = 0x0000000C;

// Packets must be of 32 bytes
pub const VIAL_RAW_EPSIZE: usize = 32;

#[derive(Default)]
pub struct VialState {
    unlocked: bool,
    unlock_in_progress: bool,
    unlock_counter: u8,
    unlock_timer: u32,
}

#[derive(FromPrimitive, Debug)]
pub enum VialCommandId {
    GetKeyboardId = 0x00,
    GetSize,
    GetDef,
    GetEncoder,
    SetEncoder,
    GetUnlockStatus,
    UnlockStart,
    UnlockPoll,
    Lock,
    QmkSettingsQuery,
    QmkSettingsSet,
    QmkSettingsReset,
    DynamicEntryOp,
}

pub trait VialKeyboard: ViaKeyboard {
    const VIAL_ENABLED: bool = true;
    const VIAL_INSECURE: bool = false;
    const VIAL_KEYBOARD_UID: [u8; 8];
    const VIAL_UNLOCK_COMBO: &'static [(u8, u8)]; // Matrix positions to use as the unlock combo for Vial
    const KEYBOARD_DEFINITION: &'static [u8];
    const VIALRGB_ENABLE: bool = true;
}

pub async fn process_vial_command<K: VialKeyboard>(
    debouncer: &'static Mutex<
        ThreadModeRawMutex,
        Debouncer<[[bool; K::LAYOUT_COLS]; K::LAYOUT_ROWS]>,
    >,
    flash: &'static Mutex<ThreadModeRawMutex, impl NorFlash>,
    vial_state: &mut VialState,
    data: &mut [u8],
) {
    debug!("[VIAL] Processing Vial command");
    if let Some(command) = num::FromPrimitive::from_u8(data[0]) {
        debug!("[VIAL] Received command {:?}", Debug2Format(&command));

        match command {
            ViaCommandId::VialPrefix => {
                if K::VIAL_ENABLED {
                    if let Some(cmd) = num::FromPrimitive::from_u8(data[1]) {
                        // Unlike the other normal Via comands, Vial overwrite the data received from the host
                        match cmd {
                            VialCommandId::GetKeyboardId => {
                                data[0..=3].copy_from_slice(&VIAL_PROTOCOL_VERSION.to_be_bytes());
                                data[4..=11].copy_from_slice(&K::VIAL_KEYBOARD_UID);
                                if K::VIALRGB_ENABLE {
                                    data[12] = 1;
                                }
                            }
                            VialCommandId::GetSize => {
                                // get size of compiled keyboard def
                                data[0..=3].copy_from_slice(
                                    &(K::KEYBOARD_DEFINITION.len() * size_of::<u8>()).to_be_bytes(),
                                )
                            }
                            VialCommandId::GetDef => {
                                let page: u16 = u16::from_le_bytes(data[2..=3].try_into().unwrap());
                                let start = page as usize * VIAL_RAW_EPSIZE;
                                let mut end = start + VIAL_RAW_EPSIZE;

                                if !(end < start
                                    || start >= K::KEYBOARD_DEFINITION.len() * size_of::<u8>())
                                {
                                    if end > K::KEYBOARD_DEFINITION.len() * size_of::<u8>() {
                                        end = K::KEYBOARD_DEFINITION.len() * size_of::<u8>()
                                    }
                                    data[0..(end - start)]
                                        .copy_from_slice(&K::KEYBOARD_DEFINITION[start..end])
                                }
                            }
                            VialCommandId::GetEncoder => todo!(), // This is already implemented in the new VIA protool, need to figure out how to handle this
                            VialCommandId::SetEncoder => todo!(), // ditto
                            VialCommandId::GetUnlockStatus => {
                                // There should only be one task with an instance of a ViaCommandHandler, so this should be safe
                                data.fill(0xFF);
                                data[0] = vial_state.unlocked as u8;
                                data[1] = vial_state.unlock_in_progress as u8;

                                if !K::VIAL_INSECURE {
                                    // why do we send the combination to the host in secure mode?
                                    for i in 0..K::VIAL_UNLOCK_COMBO.len() {
                                        data[2 + i * 2] = K::VIAL_UNLOCK_COMBO[i].0;
                                        data[2 + i * 2 + 1] = K::VIAL_UNLOCK_COMBO[i].1;
                                    }
                                }
                            }
                            VialCommandId::UnlockStart => {
                                // There should only be one task with an instance of a ViaCommandHandler, so this should be safe
                                vial_state.unlock_in_progress = true;
                                vial_state.unlock_timer =
                                    embassy_time::Instant::now().as_millis() as u32
                            }
                            VialCommandId::UnlockPoll => {
                                // There should only be one task with an instance of a ViaCommandHandler, so this should be safe
                                if !K::VIAL_INSECURE && vial_state.unlock_in_progress {
                                    let debouncer = debouncer.lock().await;
                                    let state = debouncer.get();
                                    let holding = K::VIAL_UNLOCK_COMBO
                                        .iter()
                                        .all(|(row, col)| state[*row as usize][*col as usize]);

                                    if embassy_time::Instant::now().as_millis() as u32
                                        - vial_state.unlock_timer
                                        > 100
                                        && holding
                                    {
                                        vial_state.unlock_timer =
                                            embassy_time::Instant::now().as_millis() as u32;

                                        vial_state.unlock_counter -= 1;
                                        if vial_state.unlock_counter == 0 {
                                            vial_state.unlock_in_progress = false;
                                            vial_state.unlocked = true;
                                        }
                                    }

                                    data[0] = vial_state.unlocked as u8;
                                    data[1] = vial_state.unlock_in_progress as u8;
                                    data[2] = vial_state.unlock_counter;
                                };
                            }
                            VialCommandId::Lock => {
                                // There should only be one task with an instance of a ViaCommandHandler, so this should be safe
                                vial_state.unlocked = false;
                            }
                            VialCommandId::QmkSettingsQuery => todo!(), // qmk settings is a lot of work
                            VialCommandId::QmkSettingsSet => todo!(),
                            VialCommandId::QmkSettingsReset => todo!(),
                            VialCommandId::DynamicEntryOp => todo!(),
                        }
                    }
                }
            }
            _ => {
                process_via_command::<K>(debouncer, flash, data).await;
            }
        }
    } else {
        warn!("[VIAL] Unknown command received from host {:?}", data[0]);
    }
}

#[rumcake_macros::task]
pub async fn usb_hid_vial_write_task<K: VialKeyboard>(
    _k: K,
    debouncer: &'static Mutex<
        ThreadModeRawMutex,
        Debouncer<[[bool; K::LAYOUT_COLS]; K::LAYOUT_ROWS]>,
    >,
    flash: &'static Mutex<ThreadModeRawMutex, impl NorFlash>,
    mut hid: HidWriter<'static, impl Driver<'static>, 32>,
) {
    let mut vial_state: VialState = Default::default();

    loop {
        let mut report = VIA_REPORT_HID_SEND_CHANNEL.receive().await;

        if K::VIAL_ENABLED && K::VIA_ENABLED {
            process_vial_command::<K>(debouncer, flash, &mut vial_state, &mut report).await;

            debug!("[VIAL] Writing HID raw report {:?}", Debug2Format(&report));
            if let Err(err) = hid.write(&report).await {
                error!(
                    "[VIAL] Couldn't write HID raw report: {:?}",
                    Debug2Format(&err)
                );
            };
        }
    }
}
