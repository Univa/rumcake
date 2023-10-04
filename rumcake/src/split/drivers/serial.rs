#[cfg(any(feature = "nrf", feature = "stm32"))]
compile_error!("UART split keyboard driver is not yet working. Please use a different driver.");

#[cfg(feature = "split-central")]
pub mod central {
    use core::fmt::Debug;
    use embedded_io_async::{Read, ReadExactError};

    use crate::split::{MessageToCentral, MessageToPeripheral};

    use super::super::{CentralDeviceDriver, CentralDeviceError};

    pub struct SerialCentralDriver<U> {
        uart: U,
    }

    pub trait SerialCentralDevice {
        fn setup_serial_reader() -> impl Read<Error = impl Debug>;
    }

    pub fn setup_split_central_driver<K: SerialCentralDevice>(
    ) -> SerialCentralDriver<impl Read<Error = impl Debug>> {
        SerialCentralDriver {
            uart: K::setup_serial_reader(),
        }
    }

    impl<E: Debug, U: Read<Error = E>> CentralDeviceDriver for SerialCentralDriver<U> {
        type DriverError = ReadExactError<E>;

        async fn receive_message_from_peripherals(
            &mut self,
        ) -> Result<MessageToCentral, CentralDeviceError<Self::DriverError>> {
            let mut buf = [0; 6];
            self.uart.read_exact(&mut buf).await?;
            let message = postcard::from_bytes_cobs(&mut buf).unwrap();

            Ok(message)
        }

        async fn broadcast_message_to_peripherals(
            &mut self,
            message: MessageToPeripheral,
        ) -> Result<(), CentralDeviceError<Self::DriverError>> {
            todo!()
        }
    }
}

#[cfg(feature = "split-peripheral")]
pub mod peripheral {
    use core::fmt::Debug;
    use embedded_io_async::{Write, WriteAllError};

    use crate::split::{MessageToCentral, MessageToPeripheral};

    use super::super::{PeripheralDeviceDriver, PeripheralDeviceError};

    pub struct SerialPeripheralDriver<U> {
        uart: U,
    }

    pub trait SerialPeripheralDevice {
        fn setup_serial_writer() -> impl Write<Error = impl Debug>;
    }

    pub fn setup_split_peripheral_driver<K: SerialPeripheralDevice>(
    ) -> SerialPeripheralDriver<impl Write<Error = impl Debug>> {
        SerialPeripheralDriver {
            uart: K::setup_serial_writer(),
        }
    }

    impl<E: Debug, U: Write<Error = E>> PeripheralDeviceDriver for SerialPeripheralDriver<U> {
        type DriverError = WriteAllError<E>;

        async fn send_message_to_central(
            &mut self,
            event: MessageToCentral,
        ) -> Result<(), PeripheralDeviceError<Self::DriverError>> {
            let mut buf = [0; 6];
            let data = postcard::to_slice_cobs(&event, &mut buf).unwrap();
            self.uart.write_all(data).await?;

            Ok(())
        }

        async fn receive_message_from_central(
            &mut self,
        ) -> Result<MessageToPeripheral, PeripheralDeviceError<Self::DriverError>> {
            todo!()
        }
    }
}
