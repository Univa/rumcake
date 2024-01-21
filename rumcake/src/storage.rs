//! Storage related features.
//!
//! This allows other `rumcake` features to store configuration data to a storage peripheral, like
//! your MCU's flash. As a result, a user will be able to configure things like backlight/underglow
//! effect settings, or dynamic keymaps without losing their changes between keyboard restarts.
//!
//! To use this feature, you will need to add a `CONFIG` section, and its start and end address to
//! your `memory.x` file. Refer to [`crate::hw::__config_start`], and the corresponding
//! `feature-storage.md` doc for more information.

use core::cell::{Cell, RefCell};
use core::hash::{Hash, Hasher, SipHasher};

use defmt::{assert, debug};
use defmt::{error, info, warn, Debug2Format};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};
use embedded_storage_async::nor_flash::NorFlash;
use num_derive::FromPrimitive;
use once_cell::sync::OnceCell;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tickv::success_codes::SuccessCode;
use tickv::{AsyncTicKV, ErrorCode, FlashController, MAIN_KEY};

fn get_hashed_key(key: &[u8]) -> u64 {
    let mut hasher = SipHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Keys for data to be stored in the database.
#[derive(Debug, FromPrimitive, Copy, Clone)]
#[repr(u8)]
pub enum StorageKey {
    /// Key to store [`crate::backlight::simple_backlight::animations::BacklightConfig`].
    SimpleBacklightConfig = 0x00,
    /// Key to store [`crate::backlight::simple_backlight_matrix::animations::BacklightConfig`].
    SimpleBacklightMatrixConfig = 0x01,
    /// Key to store [`crate::backlight::rgb_backlight_matrix::animations::BacklightConfig`].
    RGBBacklightMatrixConfig = 0x02,
    /// Key to store [`crate::underglow::animations::UnderglowConfig`].
    UnderglowConfig = 0x10,
    /// Key to store bluetooth profiles, used by the `nrf-ble` implementation of bluetooth host communication.
    BluetoothProfiles = 0x20,
    /// Key to store the currently set Via layout option.
    LayoutOptions = 0x30,
    /// Key to store the current state of the Via dynamic keyboard layout.
    DynamicKeymap = 0x31,
    /// Key to store the current state of the encoders in the Via dynamic keyboard layout.
    DynamicKeymapEncoder = 0x32,
    /// Key to store the current state of the macros in the Via dynamic keyboard layout.
    DynamicKeymapMacro = 0x33,
    /// Key to store the current state of the tap dance keys in the Vial dynamic keyboard layout.
    DynamicKeymapTapDance = 0x40,
    /// Key to store the current state of the combo keys in the Vial dynamic keyboard layout.
    DynamicKeymapCombo = 0x41,
    /// Key to store the current state of the key overrides in the Vial dynamic keyboard layout.
    DynamicKeymapKeyOverride = 0x42,
}

#[repr(u8)]
enum StorageKeyType {
    Data,
    Metadata,
}

/// A wrapper around a TicKV instance which allows you to receive requests to read, write or delete
/// data from a storage peripheral.
pub struct StorageService<'a, F: NorFlash>
where
    [(); F::ERASE_SIZE]:,
{
    database:
        OnceCell<Mutex<ThreadModeRawMutex, AsyncTicKV<'a, FlashDevice<'a, F>, { F::ERASE_SIZE }>>>,
}

impl<'a, F: NorFlash> StorageService<'a, F>
where
    [(); F::ERASE_SIZE]:,
{
    /// Create a new instance of a [`StorageService`]. You should call [`StorageService::setup()`]
    /// before calling any other methods.
    pub const fn new() -> Self {
        StorageService {
            database: OnceCell::new(),
        }
    }

    async fn get_database(
        &self,
    ) -> MutexGuard<ThreadModeRawMutex, AsyncTicKV<'a, FlashDevice<'a, F>, { F::ERASE_SIZE }>> {
        let mutex = self
            .database
            .get()
            .expect("setup() hasn't been called on this storage service yet.");
        mutex.lock().await
    }

    /// Set up the storage service with the provided flash peripheral and buffers. The storage
    /// service will only operate on the flash addresses between `config_start` and `config_end`.
    pub async fn setup(
        &self,
        flash: F,
        config_start: usize,
        config_end: usize,
        read_buf: &'a mut [u8; F::ERASE_SIZE],
        op_buf: &'a mut [u8; F::ERASE_SIZE],
    ) {
        let driver = FlashDevice::new(flash, config_start, config_end, op_buf);
        let flash_size = driver.end - driver.start;
        let mut database = AsyncTicKV::new(driver, read_buf, flash_size);

        // Initialize the database, formatting if needed
        initialise(&mut database).await.unwrap();

        self.database.get_or_init(|| Mutex::new(database));
    }

    /// This function checks the stored metadata for the given key. If the stored metadata differs
    /// from `current_metadata`, then it will invalidate the existing entry for that key, and
    /// update the metadata.
    pub(crate) async fn check_metadata(
        &self,
        buffer: &'static mut [u8],
        key: StorageKey,
        current_metadata: &[u8],
    ) -> Result<(), ()> {
        let mut database = self.get_database().await;

        // Verify if the underlying data type has changed since last boot
        let (will_reset, buf) = match get_key(
            &mut database,
            &[key as u8, StorageKeyType::Metadata as u8],
            buffer,
        )
        .await
        {
            (Ok(_), Some(buf), len) => {
                let changed = current_metadata.len() != len || *current_metadata != *buf;
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

        buf[..current_metadata.len()].copy_from_slice(current_metadata);

        // If the data type has changed, remove the old data from storage, update the metadata
        if will_reset {
            warn!(
                "[STORAGE] Deleting old data and updating stored metadata for {}.",
                Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
            );

            // Invalidate old data
            let _ =
                invalidate_key(&mut database, &[key as u8, StorageKeyType::Metadata as u8]).await;
            garbage_collect(&mut database).await.0.unwrap();

            // Add new metadata
            let length = current_metadata.len();
            append_key(
                &mut database,
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
    pub async fn read<T: DeserializeOwned>(
        &self,
        buffer: &'static mut [u8],
        key: StorageKey,
    ) -> Result<T, ()> {
        let mut database = self.get_database().await;

        info!(
            "[STORAGE] Reading {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        let (result, buf, len) = get_key(
            &mut database,
            &[key as u8, StorageKeyType::Data as u8],
            buffer,
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
                Some(buf) => postcard::from_bytes(&buf[..len]).map_err(|error| {
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
    pub async fn read_raw(
        &self,
        buffer: &'static mut [u8],
        key: StorageKey,
    ) -> Result<(&[u8], usize), ()> {
        let mut database = self.get_database().await;

        info!(
            "[STORAGE] Reading {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        let (result, buf, len) = get_key(
            &mut database,
            &[key as u8, StorageKeyType::Data as u8],
            buffer,
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
    pub async fn write<T: Serialize>(
        &self,
        buffer: &'static mut [u8],
        key: StorageKey,
        data: T,
    ) -> Result<(), ()> {
        let mut database = self.get_database().await;

        info!(
            "[STORAGE] Writing new {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        let result = match postcard::to_slice(&data, buffer) {
            Ok(serialized) => {
                let _ =
                    invalidate_key(&mut database, &[key as u8, StorageKeyType::Data as u8]).await;
                garbage_collect(&mut database).await.0.unwrap();
                append_key(
                    &mut database,
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
    pub async fn write_raw(
        &self,
        buffer: &'static mut [u8],
        key: StorageKey,
        data: &[u8],
    ) -> Result<(), ()> {
        let mut database = self.get_database().await;

        info!(
            "[STORAGE] Writing new {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        buffer[..data.len()].copy_from_slice(data);

        let _ = invalidate_key(&mut database, &[key as u8, StorageKeyType::Data as u8]).await;
        garbage_collect(&mut database).await.0.unwrap();
        let result = append_key(
            &mut database,
            &[key as u8, StorageKeyType::Data as u8],
            buffer,
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
    pub async fn delete(&self, key: StorageKey) -> Result<(), ()> {
        let mut database = self.get_database().await;

        info!(
            "[STORAGE] Deleting {} data.",
            Debug2Format(&<StorageKey as num::FromPrimitive>::from_u8(key as u8).unwrap()),
        );

        let result = invalidate_key(&mut database, &[key as u8, StorageKeyType::Data as u8])
            .await
            .0
            .map_err(|error| {
                error!("[STORAGE] Delete error: {}", Debug2Format(&error));
            });
        garbage_collect(&mut database).await.0.unwrap();

        result.map(|_code| {})
    }
}

async fn perform_pending_flash_op<'a, F: NorFlash>(
    database: &mut AsyncTicKV<'a, FlashDevice<'a, F>, { F::ERASE_SIZE }>,
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
            if FlashDevice::write(&mut database.tickv.controller, address, len)
                .await
                .is_err()
            {
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
    database: &mut AsyncTicKV<'a, FlashDevice<'a, F>, { F::ERASE_SIZE }>,
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
    database: &mut AsyncTicKV<'a, FlashDevice<'a, F>, { F::ERASE_SIZE }>,
) -> Result<SuccessCode, ErrorCode> {
    let mut ret = database.initialise(get_hashed_key(MAIN_KEY));
    if ret.is_err() {
        ret = continue_to_completion(database).await.0;
    }
    ret
}

async fn append_key<'a, F: NorFlash>(
    database: &mut AsyncTicKV<'a, FlashDevice<'a, F>, { F::ERASE_SIZE }>,
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
    database: &mut AsyncTicKV<'a, FlashDevice<'a, F>, { F::ERASE_SIZE }>,
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
    database: &mut AsyncTicKV<'a, FlashDevice<'a, F>, { F::ERASE_SIZE }>,
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
    database: &mut AsyncTicKV<'a, FlashDevice<'a, F>, { F::ERASE_SIZE }>,
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

/// Trait used by storage tasks to obtain a static mutable reference to a buffer.
pub trait StorageDevice {
    /// Obtain a static mutable reference to a buffer to be used with a [`StorageService`]. The
    /// buffer must be large enough to store the largest possible value (in bytes) that will be
    /// stored to the flash peripheral. It will be used to store the result of a TicKV operation.
    ///
    /// By default, this buffer will have a size of 1024 bytes, which should be enough to store
    /// data for most keyboards that use Via. Depending on the data that you'll be storing, you can
    /// either increase the size to allow for larger values to be stored, or decrease the size to
    /// save memory.
    fn get_storage_buffer() -> &'static mut [u8] {
        static mut STORAGE_BUFFER: [u8; 1024] = [0; 1024];
        unsafe { &mut STORAGE_BUFFER }
    }
}

#[derive(Debug, Clone, Copy)]
enum PendingOperation {
    Read(usize),
    Write(usize, usize),
    Delete(usize),
}

/// Data structure that wraps around an implementor of
/// [`embedded_storage_async::nor_flash::NorFlash`]. This struct is only `pub` in order to set up
/// the storage task, which uses [`tickv`]. If you want to read, write or delete existing data
/// (like [`crate::underglow::animations::UnderglowConfig`]), see
/// [`crate::storage::StorageClient`]. Reading, writing or deleting *custom* data using the same
/// storage peripheral used for the storage task is not yet supported.
struct FlashDevice<'a, F: NorFlash>
where
    [(); F::ERASE_SIZE]:,
{
    flash: F,
    start: usize,
    end: usize,
    pending: Cell<Option<PendingOperation>>,
    op_buf: RefCell<&'a mut [u8; F::ERASE_SIZE]>,
}

impl<'a, F: NorFlash> FlashDevice<'a, F>
where
    [(); F::ERASE_SIZE]:,
{
    /// Create an instance of [`FlashDevice`], using a provided implementor of
    /// [`embedded_storage_async::nor_flash::NorFlash`].
    pub fn new(
        driver: F,
        config_start: usize,
        config_end: usize,
        op_buf: &'a mut [u8; F::ERASE_SIZE],
    ) -> Self {
        // Check config partition before moving on
        assert!(
            config_start < config_end,
            "Config end address must be greater than the start address."
        );
        assert!(
            (config_end - config_start) % F::ERASE_SIZE == 0,
            "Config partition size must be a multiple of the page size."
        );
        assert!(
            config_start % F::ERASE_SIZE == 0,
            "Config partition must start on an address that is a multiple of the page size."
        );

        FlashDevice {
            flash: driver,
            start: config_start,
            end: config_end,
            pending: Cell::new(None),
            op_buf: RefCell::new(op_buf),
        }
    }

    pub(crate) async fn read(&mut self, address: usize) -> Result<(), F::Error> {
        debug!(
            "[STORAGE_DRIVER] Reading {} bytes from config page {}, offset {} (address = {:x})",
            F::ERASE_SIZE,
            address / F::ERASE_SIZE,
            address % F::ERASE_SIZE,
            self.start + address
        );

        if let Err(err) = self
            .flash
            .read(
                (self.start + address) as u32,
                self.op_buf.borrow_mut().as_mut(),
            )
            .await
        {
            error!(
                "[STORAGE_DRIVER] Failed to read: {}",
                defmt::Debug2Format(&err)
            );
            return Err(err);
        };

        Ok(())
    }

    pub(crate) async fn write(&mut self, address: usize, len: usize) -> Result<(), F::Error>
    where
        [(); F::ERASE_SIZE]:,
    {
        debug!(
            "[STORAGE_DRIVER] Writing to address {:x} (config page {}, offset {}). data: {}",
            self.start + address,
            address / F::ERASE_SIZE,
            address % F::ERASE_SIZE,
            &self.op_buf.borrow()[..len]
        );

        // In the `write` method in the FlashController trait implementation, we wrote the data to
        // op_buf in the same location as its intended position in the flash page. Now, we read the
        // contents of that page into `op_buf` without overwriting the data that we want to write.
        // This allows us to avoid creating another buffer with a size of F::ERASE_SIZE to store
        // the read results of the page that we're writing to. This is good for MCUs that don't
        // have a lot of RAM (e.g. STM32F072CB).

        // This is the index of the data we're writing in `op_buf`
        let offset = address % F::ERASE_SIZE;

        // Read the existing flash data preceding the write data in op_buf
        if let Err(err) = self
            .flash
            .read(
                (self.start + address - address % F::ERASE_SIZE) as u32,
                &mut self.op_buf.borrow_mut()[..offset],
            )
            .await
        {
            error!(
                "[STORAGE_DRIVER] Failed to read page data before writing (preceding write data): {}",
                defmt::Debug2Format(&err),
            );
            return Err(err);
        };

        // Read the existing flash data succeeding the write data in op_buf
        if let Err(err) = self
            .flash
            .read(
                (self.start + address + len) as u32,
                &mut self.op_buf.borrow_mut()[(offset + len)..],
            )
            .await
        {
            error!(
                "[STORAGE_DRIVER] Failed to read page data before writing (succeeding write data): {}",
                defmt::Debug2Format(&err),
            );
            return Err(err);
        };

        if let Err(err) = self
            .flash
            .erase(
                (self.start + address - address % F::ERASE_SIZE) as u32,
                (self.start + address - address % F::ERASE_SIZE + F::ERASE_SIZE) as u32,
            )
            .await
        {
            error!(
                "[STORAGE_DRIVER] Failed to erase page before writing: {}",
                defmt::Debug2Format(&err),
            );
            return Err(err);
        };

        // Write in chunks of 512 bytes at a time, so that we don't keep interrupts disabled for too long
        // Otherwise, writing a full page at once would cause assertion failures in nrf-softdevice
        for start in (0..F::ERASE_SIZE).step_by(512) {
            if let Err(err) = self
                .flash
                .write(
                    (self.start + ((address / F::ERASE_SIZE) * F::ERASE_SIZE) + start) as u32,
                    &self.op_buf.borrow()[start..(start + 512)],
                )
                .await
            {
                error!(
                    "[STORAGE_DRIVER] Failed to write: {}",
                    defmt::Debug2Format(&err),
                );
                return Err(err);
            }
        }

        Ok(())
    }

    pub(crate) async fn erase(&mut self, address: usize) -> Result<(), F::Error> {
        let start = self.start + address;
        let end = self.start + address + F::ERASE_SIZE;

        debug!(
            "[STORAGE_DRIVER] Erasing config page {} (start addr = {:x}, end addr = {:x}).",
            address / F::ERASE_SIZE,
            start,
            end
        );

        if let Err(err) = self.flash.erase(start as u32, end as u32).await {
            error!(
                "[STORAGE_DRIVER] Failed to erase: {}",
                defmt::Debug2Format(&err)
            );
            return Err(err);
        }

        Ok(())
    }
}

impl<'a, F: NorFlash> FlashController<{ F::ERASE_SIZE }> for FlashDevice<'a, F> {
    fn read_region(
        &self,
        region_number: usize,
        _offset: usize,
        _buf: &mut [u8; F::ERASE_SIZE],
    ) -> Result<(), tickv::ErrorCode> {
        self.pending
            .set(Some(PendingOperation::Read(region_number)));
        Err(tickv::ErrorCode::ReadNotReady(region_number))
    }

    fn write(&self, address: usize, buf: &[u8]) -> Result<(), tickv::ErrorCode> {
        // Write the data to op_buf where the data should be in the page.
        let offset = address % F::ERASE_SIZE;
        self.op_buf.borrow_mut()[offset..(offset + buf.len())].copy_from_slice(buf);
        self.pending
            .set(Some(PendingOperation::Write(address, buf.len())));
        Err(tickv::ErrorCode::WriteNotReady(address))
    }

    fn erase_region(&self, region_number: usize) -> Result<(), tickv::ErrorCode> {
        self.pending
            .set(Some(PendingOperation::Delete(region_number)));
        Err(tickv::ErrorCode::EraseNotReady(region_number))
    }
}
