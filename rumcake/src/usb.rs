//! USB host communication.
//!
//! To use USB host communication, keyboards must implement [`USBKeyboard`].

use core::marker::PhantomData;

use defmt::{error, info, Debug2Format};
use embassy_futures::select::{self, select};
use embassy_sync::signal::Signal;
use embassy_usb::class::hid::{
    Config, HidReader, HidReaderWriter, HidWriter, ReportId, RequestHandler, State as UsbState,
};
use embassy_usb::control::OutResponse;
use embassy_usb::driver::Driver;
use embassy_usb::{Builder, UsbDevice};
use packed_struct::PackedStruct;
use static_cell::StaticCell;
use usbd_human_interface_device::device::consumer::{
    MultipleConsumerReport, MULTIPLE_CODE_REPORT_DESCRIPTOR,
};
use usbd_human_interface_device::device::keyboard::{
    NKROBootKeyboardReport, NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR,
};

use crate::hw::platform::RawMutex;
use crate::hw::{HIDDevice, HIDOutput, CURRENT_OUTPUT_STATE};
use crate::keyboard::Keyboard;
use crate::{State, StaticArray};

pub(crate) static USB_RUNNING_STATE: State<bool> =
    State::new(false, &[&crate::hw::USB_RUNNING_STATE_LISTENER]);

/// A trait that keyboards must implement to communicate with host devices over USB.
pub trait USBKeyboard: Keyboard + HIDDevice {
    /// Vendor ID for the keyboard.
    const USB_VID: u16;

    /// Product ID for the keyboard.
    const USB_PID: u16;
}

/// Configure the HID report writer, using boot-specification-compatible NKRO keyboard reports.
///
/// The HID writer produced should be passed to [`usb_hid_kb_write_task`].
pub fn setup_usb_hid_nkro_writer(
    b: &mut Builder<'static, impl Driver<'static>>,
) -> HidWriter<
    'static,
    impl Driver<'static>,
    { <<NKROBootKeyboardReport as PackedStruct>::ByteArray as StaticArray>::LEN },
> {
    // Keyboard HID setup
    static KB_STATE: StaticCell<UsbState> = StaticCell::new();
    let kb_state = KB_STATE.init(UsbState::new());
    let kb_hid_config = Config {
        request_handler: None,
        report_descriptor: NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR,
        poll_ms: 1,
        max_packet_size: 64,
    };
    HidWriter::<_, { <<NKROBootKeyboardReport as PackedStruct>::ByteArray as StaticArray>::LEN }>::new(
        b,
        kb_state,
        kb_hid_config,
    )
}

/// Configure the HID report writer, for consumer commands.
///
/// The HID writer produced should be passed to [`usb_hid_consumer_write_task`].
pub fn setup_usb_hid_consumer_writer(
    b: &mut Builder<'static, impl Driver<'static>>,
) -> HidWriter<
    'static,
    impl Driver<'static>,
    { <<MultipleConsumerReport as PackedStruct>::ByteArray as StaticArray>::LEN },
> {
    // Keyboard HID setup
    static CONSUMER_STATE: StaticCell<UsbState> = StaticCell::new();
    let consumer_state = CONSUMER_STATE.init(UsbState::new());
    let consumer_hid_config = Config {
        request_handler: None,
        report_descriptor: MULTIPLE_CODE_REPORT_DESCRIPTOR,
        poll_ms: 1,
        max_packet_size: 64,
    };
    HidWriter::<_, { <<MultipleConsumerReport as PackedStruct>::ByteArray as StaticArray>::LEN }>::new(
        b,
        consumer_state,
        consumer_hid_config,
    )
}

#[rumcake_macros::task]
pub async fn start_usb(mut usb: UsbDevice<'static, impl Driver<'static>>) {
    loop {
        info!("[USB] USB started");
        USB_RUNNING_STATE.set(true).await;
        usb.run_until_suspend().await;
        info!("[USB] USB suspended");
        USB_RUNNING_STATE.set(false).await;
        usb.wait_resume().await;
    }
}

macro_rules! usb_task_inner {
    ($hid:ident, $output_listener:path, $channel:path, $info_log:literal, $error_log:literal) => {
        loop {
            if matches!(CURRENT_OUTPUT_STATE.get().await, Some(HIDOutput::Usb)) {
                match select($output_listener.wait(), $channel.receive()).await {
                    select::Either::First(()) => {}
                    select::Either::Second(report) => {
                        info!($info_log, Debug2Format(&report));
                        if let Err(err) = $hid.write(&report.pack().unwrap()).await {
                            error!($error_log, Debug2Format(&err));
                        };
                    }
                }
            } else {
                $output_listener.wait().await;

                // Ignore any unprocessed reports due to lack of a connection
                while $channel.try_receive().is_ok() {}
            }
        }
    };
}

pub(crate) static KB_CURRENT_OUTPUT_STATE_LISTENER: Signal<RawMutex, ()> = Signal::new();

#[rumcake_macros::task]
pub async fn usb_hid_kb_write_task<K: HIDDevice>(
    _k: K,
    mut hid: HidWriter<
        'static,
        impl Driver<'static>,
        { <<NKROBootKeyboardReport as PackedStruct>::ByteArray as StaticArray>::LEN },
    >,
) {
    let channel = K::get_keyboard_report_send_channel();

    usb_task_inner!(
        hid,
        KB_CURRENT_OUTPUT_STATE_LISTENER,
        channel,
        "[USB] Writing NKRO HID keyboard report to USB: {:?}",
        "[USB] Couldn't write HID keyboard report: {:?}"
    )
}

pub(crate) static CONSUMER_CURRENT_OUTPUT_STATE_LISTENER: Signal<RawMutex, ()> = Signal::new();

#[rumcake_macros::task]
pub async fn usb_hid_consumer_write_task<K: HIDDevice>(
    _k: K,
    mut hid: HidWriter<
        'static,
        impl Driver<'static>,
        { <<MultipleConsumerReport as PackedStruct>::ByteArray as StaticArray>::LEN },
    >,
) {
    let channel = K::get_consumer_report_send_channel();

    usb_task_inner!(
        hid,
        CONSUMER_CURRENT_OUTPUT_STATE_LISTENER,
        channel,
        "[USB] Writing consumer HID report to USB: {:?}",
        "[USB] Couldn't write consumer HID report: {:?}"
    );
}

#[cfg(feature = "via")]
struct ViaCommandHandler<T> {
    _phantom: PhantomData<T>,
}

#[cfg(feature = "via")]
/// Configure the HID report reader and writer for Via/Vial packets.
///
/// The reader should be passed to [`usb_hid_via_read_task`], and the writer should be passed to
/// [`usb_hid_via_write_task`].
pub fn setup_usb_via_hid_reader_writer(
    builder: &mut Builder<'static, impl Driver<'static>>,
) -> HidReaderWriter<'static, impl Driver<'static>, 32, 32> {
    static VIA_STATE: StaticCell<UsbState> = StaticCell::new();
    let via_state = VIA_STATE.init(UsbState::new());
    let via_hid_config = Config {
        request_handler: None,
        report_descriptor: crate::via::VIA_REPORT_DESCRIPTOR,
        poll_ms: 1,
        max_packet_size: 32,
    };
    HidReaderWriter::<_, 32, 32>::new(builder, via_state, via_hid_config)
}

#[cfg(feature = "via")]
impl<T: HIDDevice> RequestHandler for ViaCommandHandler<T> {
    fn get_report(&mut self, _id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn set_report(&mut self, _id: ReportId, buf: &[u8]) -> OutResponse {
        let mut data: [u8; 32] = [0; 32];
        data.copy_from_slice(buf);

        let channel = T::get_via_hid_receive_channel();

        if let Err(err) = channel.try_send(data) {
            error!(
                "[VIA] Could not queue the Via command to be processed: {:?}",
                err
            );
        };

        OutResponse::Accepted
    }

    fn get_idle_ms(&mut self, _id: Option<ReportId>) -> Option<u32> {
        None
    }

    fn set_idle_ms(&mut self, _id: Option<ReportId>, _duration_ms: u32) {}
}

#[cfg(feature = "via")]
#[rumcake_macros::task]
pub async fn usb_hid_via_read_task<T: HIDDevice>(
    _kb: T,
    hid: HidReader<'static, impl Driver<'static>, 32>,
) {
    hid.run(
        false,
        &mut ViaCommandHandler {
            _phantom: PhantomData as PhantomData<T>,
        },
    )
    .await;
}

#[cfg(feature = "via")]
pub(crate) static VIA_CURRENT_OUTPUT_STATE_LISTENER: Signal<RawMutex, ()> = Signal::new();

#[cfg(feature = "via")]
#[rumcake_macros::task]
pub async fn usb_hid_via_write_task<K: HIDDevice>(
    _k: K,
    mut hid: HidWriter<'static, impl Driver<'static>, 32>,
) {
    let channel = K::get_via_hid_send_channel();

    usb_task_inner!(
        hid,
        VIA_CURRENT_OUTPUT_STATE_LISTENER,
        channel,
        "[USB] Writing HID via report: {:?}",
        "[USB] Couldn't write HID via report: {:?}"
    )
}
