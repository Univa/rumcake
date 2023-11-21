//! Storage related features.
//!
//! This allows other `rumcake` features to store configuration data to a storage peripheral, like
//! your MCU's flash. As a result, a user will be able to configure things like backlight/underglow
//! effect settings, or dynamic keymaps without losing their changes between keyboard restarts.
//!
//! To use this feature, you will need to add a `CONFIG` section, and its start and end address to
//! your `memory.x` file. Refer to [`crate::hw::__config_start`], and the corresponding
//! `feature-storage.md` doc for more information.

use core::hash::{Hash, Hasher, SipHasher};

use defmt::{error, info, warn, Debug2Format};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};
use embedded_storage_async::nor_flash::NorFlash;
use num_derive::FromPrimitive;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tickv::success_codes::SuccessCode;
use tickv::{AsyncTicKV, ErrorCode, MAIN_KEY};

use crate::hw::{FlashDevice, PendingOperation};

fn get_hashed_key(key: &[u8]) -> u64 {
    let mut hasher = SipHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Keys for data to be stored in the database. The order of existing keys should not change.
#[derive(Debug, FromPrimitive, Copy, Clone)]
#[repr(u8)]
pub enum StorageKey {
    /// Key to store [`crate::backlight::animations::BacklightConfig`].
    BacklightConfig,
    /// Key to store [`crate::underglow::animations::UnderglowConfig`].
    UnderglowConfig,
    /// Key to store bluetooth profiles, used by the `nrf-ble` implementation of bluetooth host communication.
    BluetoothProfiles,
    /// Key to store the currently set Via layout option.
    LayoutOptions,
    /// Key to store the current state of the Via dynamic keyboard layout.
    DynamicKeymap,
    /// Key to store the current state of the encoders in the Via dynamic keyboard layout.
    DynamicKeymapEncoder,
    /// Key to store the current state of the macros in the Via dynamic keyboard layout.
    DynamicKeymapMacro,
    /// Key to store the current state of the tap dance keys in the Vial dynamic keyboard layout.
    DynamicKeymapTapDance,
    /// Key to store the current state of the combo keys in the Vial dynamic keyboard layout.
    DynamicKeymapCombo,
    /// Key to store the current state of the key overrides in the Vial dynamic keyboard layout.
    DynamicKeymapKeyOverride,
}

#[repr(u8)]
enum StorageKeyType {
    Data,
    Metadata,
}

/// Statically allocated buffers used to read and write data from the storage peripheral. One
/// buffer is used for metadata, the contents of which depend on the data being stored. Another
/// buffer is used for the raw data itself (serialized to bytes). The size of these buffers should
/// be big enough to store the largest possible expected value that they can handle.
pub struct StorageServiceState<const M: usize, const D: usize> {
    metadata_buf: [u8; M],
    data_buf: [u8; D],
}

impl<const M: usize, const D: usize> StorageServiceState<M, D> {
    /// Create new buffers.
    pub const fn new() -> Self {
        Self {
            metadata_buf: [0; M],
            data_buf: [0; D],
        }
    }
}

/// A wrapper around a TicKV instance which allows you to receive requests to read, write or delete
/// `T` from a storage peripheral.
pub struct StorageService<'a, F: NorFlash>
where
    [(); F::ERASE_SIZE]:,
{
    database: tickv::AsyncTicKV<'a, FlashDevice<F>, { F::ERASE_SIZE }>,
}

impl<'a, F: NorFlash> StorageService<'a, F>
where
    [(); F::ERASE_SIZE]:,
{
    pub(crate) const fn new(
        database: tickv::AsyncTicKV<'a, FlashDevice<F>, { F::ERASE_SIZE }>,
    ) -> Self {
        StorageService { database }
    }

    /// This function checks the stored metadata for the given key. If the stored metadata differs
    /// from `current_metadata`, then it will invalidate the existing entry for that key, and
    /// update the metadata.
    pub(crate) async fn initialize<const M: usize, const D: usize>(
        &mut self,
        state: &'static mut StorageServiceState<M, D>,
        key: StorageKey,
        current_metadata: &[u8],
    ) -> Result<(), ()> {
        let buf = &mut state.metadata_buf;

        // Verify if the underlying data type has changed since last boot
        let (will_reset, buf) = match get_key(
            &mut self.database,
            &[key as u8, StorageKeyType::Metadata as u8],
            buf,
        )
        .await
        {
            (Ok(_), Some(buf), _len) => {
                let changed = *current_metadata != *buf;
                if changed {
                    warn!(
                        "[STORAGE] Metadata for {} has changed.",
                        Debug2Format(
                            &<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()
                        ),
                    );
                }
                (changed, buf)
            }
            (Err(error), Some(buf), _len) => {
                warn!(
                    "[STORAGE] Could not read metadata for {}: {}",
                    Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
                    Debug2Format(&error)
                );
                (true, buf)
            }
            _ => unreachable!(),
        };

        buf.copy_from_slice(current_metadata);

        // If the data type has changed, remove the old data from storage, update the metadata
        if will_reset {
            warn!(
                "[STORAGE] Deleting old data and updating stored metadata for {}.",
                Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
            );

            // Invalidate old data
            let _ =
                invalidate_key(&mut self.database, &[key as u8, StorageKeyType::Data as u8]).await;
            let _ = invalidate_key(
                &mut self.database,
                &[key as u8, StorageKeyType::Metadata as u8],
            )
            .await;
            garbage_collect(&mut self.database).await.0.unwrap();

            // Add new metadata
            let length = buf.len();
            append_key(
                &mut self.database,
                &[key as u8, StorageKeyType::Metadata as u8],
                buf,
                length,
            )
            .await
            .0
            .unwrap();
        }

        Ok(())
    }

    /// Read and deserialize data from the storage peripheral, using the given
    /// key to look it up. Uses [`postcard`] for deserialization.
    pub async fn read<T: DeserializeOwned, const M: usize, const D: usize>(
        &mut self,
        state: &'static mut StorageServiceState<M, D>,
        key: StorageKey,
    ) -> Result<T, ()> {
        info!(
            "[STORAGE] Reading {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        let (result, buf, _len) = get_key(
            &mut self.database,
            &[key as u8, StorageKeyType::Data as u8],
            &mut state.data_buf,
        )
        .await;

        result
            .map_err(|error| {
                error!(
                    "[STORAGE] Read error for {}: {}",
                    Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
                    Debug2Format(&error)
                );
            })
            .and_then(|_code| match buf {
                Some(buf) => postcard::from_bytes(buf).map_err(|error| {
                    error!(
                        "[STORAGE] Deserialization error while reading {}: {}",
                        Debug2Format(
                            &<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()
                        ),
                        Debug2Format(&error)
                    );
                }),
                None => unreachable!(),
            })
    }

    /// Read data from the storage peripheral, using the given key to look it up. This skips the
    /// deserialization step, returning raw bytes.
    pub async fn read_raw<const M: usize, const D: usize>(
        &mut self,
        state: &'static mut StorageServiceState<M, D>,
        key: StorageKey,
    ) -> Result<(&[u8], usize), ()> {
        info!(
            "[STORAGE] Reading {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        let (result, buf, len) = get_key(
            &mut self.database,
            &[key as u8, StorageKeyType::Data as u8],
            &mut state.data_buf,
        )
        .await;

        result
            .map_err(|error| {
                error!(
                    "[STORAGE] Read error for {}: {}",
                    Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
                    Debug2Format(&error)
                );
            })
            .map(|_code| (&*buf.unwrap(), len))
    }

    /// Write data to the storage peripheral, at the given key. This will serialize the given data
    /// using [`postcard`] before storing it.
    pub async fn write<T: Serialize, const M: usize, const D: usize>(
        &mut self,
        state: &'static mut StorageServiceState<M, D>,
        key: StorageKey,
        data: T,
    ) -> Result<(), ()> {
        info!(
            "[STORAGE] Writing new {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        let result = match postcard::to_slice(&data, &mut state.data_buf) {
            Ok(serialized) => {
                let _ =
                    invalidate_key(&mut self.database, &[key as u8, StorageKeyType::Data as u8])
                        .await;
                garbage_collect(&mut self.database).await.0.unwrap();
                append_key(
                    &mut self.database,
                    &[key as u8, StorageKeyType::Data as u8],
                    serialized,
                    serialized.len(),
                )
                .await
                .0
                .map_err(|error| {
                    error!(
                        "[STORAGE] Write error for {}: {}",
                        Debug2Format(
                            &<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()
                        ),
                        Debug2Format(&error)
                    );
                })
            }
            Err(error) => {
                error!(
                    "[STORAGE] Serialization error while writing {}: {}",
                    Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
                    Debug2Format(&error)
                );
                Err(())
            }
        };

        result.map(|_code| {})
    }

    /// Write data to the storage peripheral, at the given key. This skips the serialization step,
    /// allowing you to write raw bytes to storage.
    pub async fn write_raw<const M: usize, const D: usize>(
        &mut self,
        state: &'static mut StorageServiceState<M, D>,
        key: StorageKey,
        data: &[u8],
    ) -> Result<(), ()> {
        info!(
            "[STORAGE] Writing new {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        state.data_buf.copy_from_slice(data);

        let _ = invalidate_key(&mut self.database, &[key as u8, StorageKeyType::Data as u8]).await;
        garbage_collect(&mut self.database).await.0.unwrap();
        let result = append_key(
            &mut self.database,
            &[key as u8, StorageKeyType::Data as u8],
            &mut state.data_buf,
            data.len(),
        )
        .await
        .0
        .map_err(|error| {
            error!(
                "[STORAGE] Write error for {}: {}",
                Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
                Debug2Format(&error)
            );
        });

        result.map(|_code| {})
    }

    /// Deletes the data at a given key.
    pub async fn delete(&mut self, key: StorageKey) -> Result<(), ()> {
        info!(
            "[STORAGE] Deleting {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        let result = invalidate_key(&mut self.database, &[key as u8, StorageKeyType::Data as u8])
            .await
            .0
            .map_err(|error| {
                error!("[STORAGE] Delete error: {}", Debug2Format(&error));
            });
        garbage_collect(&mut self.database).await.0.unwrap();

        result.map(|_code| {})
    }
}

async fn perform_pending_flash_op<'a, F: NorFlash>(
    database: &mut AsyncTicKV<'a, FlashDevice<F>, { F::ERASE_SIZE }>,
) -> Result<(), ErrorCode> {
    let operation = database.tickv.controller.pending.get();
    database.tickv.controller.pending.set(None);
    match operation {
        Some(PendingOperation::Read(page)) => {
            if database
                .tickv
                .controller
                .read(page * F::ERASE_SIZE)
                .await
                .is_err()
            {
                return Err(ErrorCode::ReadFail);
            }
            database.set_read_buffer(database.tickv.controller.op_buf.borrow_mut().as_mut());
        }
        Some(PendingOperation::Write(address, len)) => {
            // Data should already by contained in `op_buf`, so we just need to pass the length of
            // the data and the address to write to.
            if database.tickv.controller.write(address, len).await.is_err() {
                return Err(ErrorCode::WriteFail);
            }
        }
        Some(PendingOperation::Delete(page)) => {
            if database
                .tickv
                .controller
                .erase(page * F::ERASE_SIZE)
                .await
                .is_err()
            {
                return Err(ErrorCode::EraseFail);
            }
        }
        _ => {}
    }
    Ok(())
}

async fn continue_to_completion<'a, F: NorFlash>(
    database: &mut AsyncTicKV<'a, FlashDevice<F>, { F::ERASE_SIZE }>,
) -> (
    Result<SuccessCode, ErrorCode>,
    Option<&'static mut [u8]>,
    usize,
) {
    let ret = loop {
        // Perform the last called AsyncTicKV operation to completion
        if let Err(e) = perform_pending_flash_op(database).await {
            break (Err(e), None, 0);
        };
        let (result, buf, len) = database.continue_operation();
        match result {
            // These errors occur when we want to call an async flash operation.
            // We continue the loop to handle them with `perform_pending_flash_op`
            Err(ErrorCode::ReadNotReady(_))
            | Err(ErrorCode::WriteNotReady(_))
            | Err(ErrorCode::EraseNotReady(_)) => {}
            _ => {
                break (result, buf, len);
            }
        }
    };

    // Take care of any leftover pending flash operations (usually a write) when the TicKV operation is complete
    perform_pending_flash_op(database).await.unwrap();

    ret
}

async fn initialise<'a, F: NorFlash>(
    database: &mut AsyncTicKV<'a, FlashDevice<F>, { F::ERASE_SIZE }>,
) -> Result<SuccessCode, ErrorCode> {
    let mut ret = database.initialise(get_hashed_key(MAIN_KEY));
    if ret.is_err() {
        ret = continue_to_completion(database).await.0;
    }
    ret
}

async fn append_key<'a, F: NorFlash>(
    database: &mut AsyncTicKV<'a, FlashDevice<F>, { F::ERASE_SIZE }>,
    key: &[u8],
    value: &'static mut [u8],
    length: usize,
) -> (
    Result<SuccessCode, ErrorCode>,
    Option<&'static mut [u8]>,
    usize,
) {
    let ret = database.append_key(get_hashed_key(key), value, length);
    match ret {
        Ok(SuccessCode::Queued) => continue_to_completion(database).await,
        _ => unreachable!(),
    }
}

async fn get_key<'a, F: NorFlash>(
    database: &mut AsyncTicKV<'a, FlashDevice<F>, { F::ERASE_SIZE }>,
    key: &[u8],
    buf: &'static mut [u8],
) -> (
    Result<SuccessCode, ErrorCode>,
    Option<&'static mut [u8]>,
    usize,
) {
    let ret = database.get_key(get_hashed_key(key), buf);
    match ret {
        Ok(SuccessCode::Queued) => continue_to_completion(database).await,
        _ => unreachable!(),
    }
}

async fn invalidate_key<'a, F: NorFlash>(
    database: &mut AsyncTicKV<'a, FlashDevice<F>, { F::ERASE_SIZE }>,
    key: &[u8],
) -> (
    Result<SuccessCode, ErrorCode>,
    Option<&'static mut [u8]>,
    usize,
) {
    let ret = database.invalidate_key(get_hashed_key(key));
    match ret {
        Ok(SuccessCode::Queued) => continue_to_completion(database).await,
        _ => unreachable!(),
    }
}

async fn garbage_collect<'a, F: NorFlash>(
    database: &mut AsyncTicKV<'a, FlashDevice<F>, { F::ERASE_SIZE }>,
) -> (
    Result<SuccessCode, ErrorCode>,
    Option<&'static mut [u8]>,
    usize,
) {
    let ret = database.garbage_collect();
    match ret {
        Ok(SuccessCode::Queued) => continue_to_completion(database).await,
        _ => unreachable!(),
    }
}

/// A mutex-guarded [`StorageService`], which you can use to read, and write data to flash.
pub struct Database<'a, F: NorFlash>
where
    [(); F::ERASE_SIZE]:,
{
    db: once_cell::sync::OnceCell<Mutex<ThreadModeRawMutex, StorageService<'a, F>>>,
}

impl<'a, F: NorFlash> Database<'a, F>
where
    [(); F::ERASE_SIZE]:,
{
    /// Create a new instance of a storage service. You will need to call [`setup`] with your
    /// desired [`NorFlash`] implementor before using
    pub const fn new() -> Self {
        Self {
            db: once_cell::sync::OnceCell::new(),
        }
    }

    /// Initialize the database. You must provide a [`NorFlash`] implementor, along with the start
    /// and end addresses of the flash region that will be used to store data. A statically
    /// allocated read buffer must also be provided, which is used by TicKV internally.
    pub async fn setup(
        &self,
        flash: F,
        config_start: usize,
        config_end: usize,
        read_buf: &'a mut [u8; F::ERASE_SIZE],
    ) {
        let driver = FlashDevice::new(flash, config_start, config_end);
        let flash_size = driver.end - driver.start;
        let mut database = tickv::AsyncTicKV::new(driver, read_buf, flash_size);

        // Initialize the database, formatting if needed
        initialise(&mut database).await.unwrap();

        self.db
            .get_or_init(|| Mutex::new(StorageService::new(database)));
    }

    /// Obtain a lock on the mutex on the storage service, so that you can use it.
    pub async fn lock(&self) -> MutexGuard<ThreadModeRawMutex, StorageService<'a, F>> {
        self.db.get().unwrap().lock().await
    }
}
