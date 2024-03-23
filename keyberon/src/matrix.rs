//! Hardware pin switch matrix handling.

use core::ops::Range;

use embedded_hal::digital::v2::{InputPin, OutputPin};
use num_traits::{clamp, SaturatingSub};

/// Describes the hardware-level matrix of switches.
///
/// Generic parameters are in order: The type of column pins,
/// the type of row pins, the number of columns and rows.
/// **NOTE:** In order to be able to put different pin structs
/// in an array they have to be downgraded (stripped of their
/// numbers etc.). Most HAL-s have a method of downgrading pins
/// to a common (erased) struct. (for example see
/// [stm32f0xx_hal::gpio::PA0::downgrade](https://docs.rs/stm32f0xx-hal/0.17.1/stm32f0xx_hal/gpio/gpioa/struct.PA0.html#method.downgrade))
pub struct Matrix<C, R, const CS: usize, const RS: usize>
where
    C: InputPin,
    R: OutputPin,
{
    cols: [C; CS],
    rows: [R; RS],
}

impl<C, R, const CS: usize, const RS: usize> Matrix<C, R, CS, RS>
where
    C: InputPin,
    R: OutputPin,
{
    /// Creates a new Matrix.
    ///
    /// Assumes columns are pull-up inputs,
    /// and rows are output pins which are set high when not being scanned.
    pub fn new<E>(cols: [C; CS], rows: [R; RS]) -> Result<Self, E>
    where
        C: InputPin<Error = E>,
        R: OutputPin<Error = E>,
    {
        let mut res = Self { cols, rows };
        res.clear()?;
        Ok(res)
    }
    fn clear<E>(&mut self) -> Result<(), E>
    where
        C: InputPin<Error = E>,
        R: OutputPin<Error = E>,
    {
        for r in self.rows.iter_mut() {
            r.set_high()?;
        }
        Ok(())
    }
    /// Scans the matrix and checks which keys are pressed.
    ///
    /// Every row pin in order is pulled low, and then each column
    /// pin is tested; if it's low, the key is marked as pressed.
    /// Scans the pins and checks which keys are pressed (state is "low").
    ///
    /// Delay function allows pause to let input pins settle
    pub fn get_with_delay<F: FnMut(), E>(&mut self, mut delay: F) -> Result<[[bool; CS]; RS], E>
    where
        C: InputPin<Error = E>,
        R: OutputPin<Error = E>,
    {
        let mut keys = [[false; CS]; RS];

        for (ri, row) in self.rows.iter_mut().enumerate() {
            row.set_low()?;
            delay();
            for (ci, col) in self.cols.iter().enumerate() {
                if col.is_low()? {
                    keys[ri][ci] = true;
                }
            }
            row.set_high()?;
        }
        Ok(keys)
    }

    /// Scans the matrix and checks which keys are pressed.
    ///
    /// Every row pin in order is pulled low, and then each column
    /// pin is tested; if it's low, the key is marked as pressed.
    /// Scans the pins and checks which keys are pressed (state is "low").
    pub fn get<E>(&mut self) -> Result<[[bool; CS]; RS], E>
    where
        C: InputPin<Error = E>,
        R: OutputPin<Error = E>,
    {
        self.get_with_delay(|| ())
    }
}

/// Matrix-representation of switches directly attached to the pins ("diodeless").
///
/// Generic parameters are in order: The type of column pins,
/// the number of columns and rows.
pub struct DirectPinMatrix<P, const CS: usize, const RS: usize>
where
    P: InputPin,
{
    pins: [[Option<P>; CS]; RS],
}

impl<P, const CS: usize, const RS: usize> DirectPinMatrix<P, CS, RS>
where
    P: InputPin,
{
    /// Creates a new DirectPinMatrix.
    ///
    /// Assumes pins are pull-up inputs. Spots in the matrix that are
    /// not corresponding to any pins use ´None´.
    pub fn new<E>(pins: [[Option<P>; CS]; RS]) -> Result<Self, E>
    where
        P: InputPin<Error = E>,
    {
        let res = Self { pins };
        Ok(res)
    }

    /// Scans the pins and checks which keys are pressed (state is "low").
    pub fn get<E>(&mut self) -> Result<[[bool; CS]; RS], E>
    where
        P: InputPin<Error = E>,
    {
        let mut keys = [[false; CS]; RS];

        for (ri, row) in self.pins.iter_mut().enumerate() {
            for (ci, col_option) in row.iter().enumerate() {
                if let Some(col) = col_option {
                    if col.is_low()? {
                        keys[ri][ci] = true;
                    }
                }
            }
        }
        Ok(keys)
    }
}

/// Matrix where switches generate an analog signal (e.g. hall-effect switches or
/// electrocapacitive). When a key in an analog matrix is pressed, the sampled value returned by
/// the ADC may fall within different ranges. Yielded values can depend on the HAL, and hardware
/// used for the analog-to-digital conversion process. Thus, these values can vary between
/// keyboards and even individual keys. Ranges of values for each key will need to be provided to
/// normalize the analog signal into an 8-bit integer, where 0 represents an unpressed key, and 255
/// represents a fully-pressed key.
///
/// Generic parameters are in order: Raw type returned when sampling your ADC, the number of
/// columns and rows.
pub struct AnalogMatrix<T, const CS: usize, const RS: usize> {
    ranges: [[Range<T>; CS]; RS],
}

impl<T, const CS: usize, const RS: usize> AnalogMatrix<T, CS, RS> {
    /// Create a new AnalogMatrix
    pub fn new(ranges: [[Range<T>; CS]; RS]) -> Self {
        Self { ranges }
    }
}

impl<T: SaturatingSub + PartialOrd, const CS: usize, const RS: usize> AnalogMatrix<T, CS, RS>
where
    u32: From<T>,
{
    /// Scan the matrix, and obtain the analog signal generated by each switch. The
    /// `get_press_value` function should return the raw ADC sample for the given key.
    pub fn get<E>(
        &mut self,
        get_press_value: impl Fn(usize, usize) -> Result<T, E>,
    ) -> Result<[[u8; CS]; RS], E> {
        let mut keys = [[0; CS]; RS];

        keys.iter_mut().enumerate().try_for_each(|(row, cols)| {
            cols.iter_mut().enumerate().try_for_each(|(col, key)| {
                let value = get_press_value(row, col)?;
                let Range { start, end } = &self.ranges[row][col];
                *key = ((u32::from(clamp(&value, start, end).saturating_sub(start)))
                    .saturating_mul(255)
                    / u32::from(end.saturating_sub(start))) as u8;
                Ok(())
            })?;
            Ok(())
        })?;

        Ok(keys)
    }
}
