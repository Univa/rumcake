#[cfg(all(not(feature = "stm32"), not(feature = "nrf")))]
compile_error!("Please enable the appropriate feature flag for the chip you're using.");

#[cfg(all(feature = "stm32", feature = "nrf"))]
compile_error!("Please enable only one chip feature flag.");

#[cfg_attr(feature = "stm32", path = "mcu/stm32.rs")]
#[cfg_attr(feature = "nrf", path = "mcu/nrf.rs")]
pub mod mcu;

use crate::State;
pub static BATTERY_LEVEL_STATE: State<u8> = State::new(
    100,
    &[
        #[cfg(feature = "display")]
        &crate::display::BATTERY_LEVEL_LISTENER,
        #[cfg(feature = "bluetooth")]
        &crate::bluetooth::BATTERY_LEVEL_LISTENER,
    ],
);

use core::cell::{Cell, RefCell};
use defmt::{assert, debug, error};
use embedded_storage_async::nor_flash::NorFlash;
use tickv::FlashController;

extern "C" {
    /// This static value will have an address equal to the `__config_start` address in your
    /// `memory.x` file. You must set this, along with [`__config_end`], if you're using on-chip
    /// flash with the storage task (which is the default). Keep in mind that the start and end
    /// address must be relative to the address of your chip's flash. For example, on STM32F072CBx,
    /// flash memory is located at `0x08000000`, so if you want your config data to start at
    /// `0x08100000`, your start address must be `0x00100000`.
    pub static __config_start: u32;
    /// This static value will have an address equal to the `__config_end` address in your
    /// `memory.x` file. If you want to know what value to set this to in `memory.x`, take
    /// [`__config_start`], and add the size of your config section, in bytes.
    pub static __config_end: u32;
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PendingOperation {
    Read(usize),
    Write(usize, usize),
    Delete(usize),
}

/// Data structure that wraps around an implementor of
/// [`embedded_storage_async::nor_flash::NorFlash`]. This struct is only `pub` in order to set up
/// the storage task, which uses [`tickv`]. If you want to read, write or delete existing data
/// (like [`crate::backlight::animations::BacklightConfig`]), see
/// [`crate::storage::StorageClient`]. Reading, writing or deleting *custom* data using the same
/// storage peripheral used for the storage task is not yet supported.
pub(crate) struct FlashDevice<F: NorFlash>
where
    [(); F::ERASE_SIZE]:,
{
    pub(crate) flash: F,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) pending: Cell<Option<PendingOperation>>,
    pub(crate) op_buf: RefCell<[u8; F::ERASE_SIZE]>,
}

impl<F: NorFlash> FlashDevice<F>
where
    [(); F::ERASE_SIZE]:,
{
    /// Create an instance of [`FlashDevice`], using a provided implementor of
    /// [`embedded_storage_async::nor_flash::NorFlash`].
    pub fn new(driver: F, config_start: usize, config_end: usize) -> Self {
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
            op_buf: RefCell::new([0xFF; F::ERASE_SIZE]),
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

impl<F: NorFlash> FlashController<{ F::ERASE_SIZE }> for FlashDevice<F> {
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
