//! Timing firmware with USB serial output, for a Nano without a debug probe.
//!
//! The timed loops are the same code as `timing`. The USB stack starts only
//! after they end, so USB interrupts cannot disturb the cycle counts. The
//! result text waits in RAM. When the host opens the port (DTR set), the
//! firmware prints the text every second until the host closes the port.
//! `scripts/board-run.sh` flashes, reads and checks the output.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use core::fmt::Write;
use core::sync::atomic::Ordering;

use arduino_nano33ble_kws::{CORE_CLOCK_HZ, TIMED_INVOKES, fatal_blink, run_timing};
use cycle_stats::report::{TextBuffer, write_cache_run, write_header};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::select;
use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::usb::Driver;
use embassy_nrf::usb::vbus_detect::HardwareVbusDetect;
use embassy_nrf::{bind_interrupts, peripherals, usb};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, ControlChanged, Sender, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, UsbDevice};
use iree_embedded::{Arena, singleton};
use kws_frontend::Frontend;
use panic_probe as _;

// newlib's malloc references _sbrk; the IREE runtime allocates from the
// arena instead, so the libc heap can never grow.
iree_embedded::libc_stubs!();

bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<peripherals::USBD>;
    CLOCK_POWER => usb::vbus_detect::InterruptHandler;
});

const USB_PACKET_BYTES: usize = 64;
const RESULT_TEXT_BYTES: usize = 1024;
// Test vendor and product numbers from https://pid.codes (not a product).
const USB_VENDOR_ID: u16 = 0x1209;
const USB_PRODUCT_ID: u16 = 0x0001;

type UsbDriver = Driver<'static, HardwareVbusDetect>;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = Config::default();
    // The USB peripheral needs the external 32 MHz crystal as HFCLK.
    config.hfclk_source = HfclkSource::ExternalXtal;
    let p = embassy_nrf::init(config);
    defmt::info!("iree-embedded KWS timing: {} timed invokes", TIMED_INVOKES);

    let cp = cortex_m::Peripherals::take().unwrap();
    let mut dcb = cp.DCB;
    let mut dwt = cp.DWT;
    dcb.enable_trace();
    dwt.enable_cycle_counter();

    let arena = Arena::new(singleton!([u8; 56 * 1024] = [0; 56 * 1024]));
    let fe = singleton!(Frontend = Frontend::new());
    let cycles = singleton!([u32; TIMED_INVOKES] = [0; TIMED_INVOKES]);
    let mut status = Output::new(p.P0_13, Level::Low, OutputDrive::Standard);

    let mut text = TextBuffer::<RESULT_TEXT_BYTES>::new();
    match run_timing(&arena, fe, cycles) {
        Ok(runs) => {
            write_header(&mut text, TIMED_INVOKES).expect("text buffer holds the header");
            for run in &runs {
                write_cache_run(&mut text, run, CORE_CLOCK_HZ)
                    .expect("text buffer holds the run lines");
            }
            defmt::info!(
                "timing done; result text is {} bytes",
                text.as_bytes().len()
            );
            // Solid orange LED: the run finished and USB serial is next.
            status.set_high();
        }
        Err(e) => {
            writeln!(
                text,
                "ERROR: failed: {:?} (raw {}): {} | largest failed alloc = {} bytes",
                e.code(),
                e.raw_code(),
                e.message(),
                iree_embedded::LAST_ALLOC_FAIL_SIZE.load(Ordering::Relaxed)
            )
            .expect("text buffer holds the error line");
            defmt::error!("timing failed; the error goes to USB serial");
        }
    }
    let failed = text.as_bytes().starts_with(b"ERROR");

    let driver = Driver::new(p.USBD, Irqs, HardwareVbusDetect::new(Irqs));
    let mut usb_config = embassy_usb::Config::new(USB_VENDOR_ID, USB_PRODUCT_ID);
    usb_config.manufacturer = Some("iree-embedded");
    usb_config.product = Some("Nano 33 BLE Sense timing");
    usb_config.serial_number = Some("timing-usb");
    usb_config.max_power = 100;
    usb_config.max_packet_size_0 = 64;

    let mut builder = Builder::new(
        driver,
        usb_config,
        singleton!([u8; 256] = [0; 256]),
        singleton!([u8; 256] = [0; 256]),
        singleton!([u8; 64] = [0; 64]),
        singleton!([u8; 128] = [0; 128]),
    );
    let class = CdcAcmClass::new(
        &mut builder,
        singleton!(State<'static> = State::new()),
        USB_PACKET_BYTES as u16,
    );
    let usb_device = builder.build();
    let (mut sender, _receiver, control) = class.split_with_control();

    join(
        run_usb(usb_device),
        print_when_open(&mut sender, &control, text.as_bytes()),
    )
    .await;
    if failed {
        fatal_blink(&mut status);
    }
}

async fn run_usb(mut usb_device: UsbDevice<'static, UsbDriver>) -> ! {
    usb_device.run().await
}

/// Print the text every second while the host keeps the port open.
async fn print_when_open(
    sender: &mut Sender<'static, UsbDriver>,
    control: &ControlChanged<'static>,
    text: &[u8],
) -> ! {
    loop {
        while !control.dtr() {
            control.control_changed().await;
        }
        sender.wait_connection().await;
        select(send_every_second(sender, text), wait_for_close(control)).await;
    }
}

async fn wait_for_close(control: &ControlChanged<'static>) {
    while control.dtr() {
        control.control_changed().await;
    }
}

/// Returns when the host disables the endpoint. Other write errors cannot happen with a
/// packet that fits the endpoint.
async fn send_every_second(sender: &mut Sender<'static, UsbDriver>, text: &[u8]) {
    loop {
        for packet in text.chunks(USB_PACKET_BYTES) {
            match sender.write_packet(packet).await {
                Ok(()) => {}
                Err(EndpointError::Disabled) => return,
                Err(EndpointError::BufferOverflow) => {
                    unreachable!("packets are at most the endpoint size")
                }
            }
        }
        Timer::after_secs(1).await;
    }
}
