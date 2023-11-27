//! USB host communication.
//!
//! To use USB host communication, keyboards must implement [`USBKeyboard`].

use defmt::{error, info, Debug2Format};
use embassy_futures::select::{self, select};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_usb::class::hid::{Config, HidWriter, State as UsbState};
use embassy_usb::driver::Driver;
use embassy_usb::{Builder, UsbDevice};
use packed_struct::PackedStruct;
use static_cell::StaticCell;
use usbd_human_interface_device::device::keyboard::{
    NKROBootKeyboardReport, NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR,
};

use crate::keyboard::{Keyboard, KeyboardLayout, KEYBOARD_REPORT_HID_SEND_CHANNEL};
use crate::{State, StaticArray};

static USB_STATE_LISTENER: Signal<ThreadModeRawMutex, ()> = Signal::new();
/// State that indicates whether HID reports are being sent out via USB. If this is `false`, it is
/// assumed that HID reports are being sent out via Bluetooth instead.
pub static USB_STATE: State<bool> = State::new(
    cfg!(not(feature = "bluetooth")),
    &[
        &USB_STATE_LISTENER,
        #[cfg(feature = "display")]
        &crate::display::USB_STATE_LISTENER,
        #[cfg(feature = "bluetooth")]
        &crate::bluetooth::USB_STATE_LISTENER,
    ],
);

/// A trait that keyboards must implement to communicate with host devices over USB.
pub trait USBKeyboard: Keyboard + KeyboardLayout {
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

#[rumcake_macros::task]
pub async fn start_usb(mut usb: UsbDevice<'static, impl Driver<'static>>) {
    loop {
        info!("[USB] USB started");
        usb.run_until_suspend().await;
        info!("[USB] USB suspended");
        usb.wait_resume().await;
    }
}

// TODO: media keys
#[rumcake_macros::task]
pub async fn usb_hid_kb_write_task(
    mut hid: HidWriter<
        'static,
        impl Driver<'static>,
        { <<NKROBootKeyboardReport as PackedStruct>::ByteArray as StaticArray>::LEN },
    >,
) {
    loop {
        if USB_STATE.get().await {
            match select(
                USB_STATE_LISTENER.wait(),
                KEYBOARD_REPORT_HID_SEND_CHANNEL.receive(),
            )
            .await
            {
                select::Either::First(()) => {
                    info!("[USB] USB HID reports enabled = {}", USB_STATE.get().await);
                }
                select::Either::Second(report) => {
                    info!(
                        "[USB] Writing HID keyboard report to USB: {:?}",
                        Debug2Format(&report)
                    );
                    if let Err(err) = hid.write(&report.pack().unwrap()).await {
                        error!(
                            "[USB] Couldn't write HID keyboard report: {:?}",
                            Debug2Format(&err)
                        );
                    };
                }
            }
        } else {
            USB_STATE_LISTENER.wait().await;
            info!("[USB] USB HID reports enabled = {}", USB_STATE.get().await);

            // Ignore any unprocessed reports due to lack of a connection
            while KEYBOARD_REPORT_HID_SEND_CHANNEL.try_receive().is_ok() {}
        }
    }
}
