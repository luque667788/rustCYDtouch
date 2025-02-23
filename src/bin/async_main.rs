#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel;
use embassy_sync::pubsub::PubSubChannel;
use embassy_sync::watch::Watch;
use embassy_time::{Delay, Duration, Timer};
use embedded_hal::digital::{InputPin, OutputPin};
use esp_backtrace as _;

use esp_hal::interrupt::InterruptConfigurable;
use esp_hal::{
    clock::CpuClock, gpio::Input, handler, peripheral::Peripheral, time::RateExtU32, touch,
};
use esp_hal_embassy::main;
use log::info;
use static_cell::StaticCell;

use core::borrow::Borrow;
use core::cell::RefCell;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_sync::channel::TrySendError;

use embedded_graphics::{
    mono_font::{ascii::FONT_9X18, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
    text::{renderer::CharacterStyle, Text},
};

use cyd_touch::{TouchCalibration, TouchSensor};

static SPI_BUFFER: StaticCell<[u8; 1024]> = StaticCell::new(); // Adjust size as needed
extern crate alloc;
const NUM_SUBSCRIBERS: usize = 4;
// Define a static array of channels
use embassy_sync::signal::Signal;

static SIGNAL: Watch<CriticalSectionRawMutex, bool, 4> = Watch::new();

/// Initializes an ILI9341 display with RGB565 color format using SPI communication.
///
/// # Arguments
/// * `sclk` - Serial Clock pin
/// * `miso` - Master Input Slave Output pin
/// * `mosi` - Master Output Slave Input pin
/// * `cs` - Chip Select pin
/// * `rst` - Reset pin
/// * `dc` - Data/Command pin
/// * `spi` - SPI peripheral instance
/// * `bl` - Backlight control pin
///
/// # Returns
/// A drawable target implementing the embedded-graphics trait system with Rgb565 color support
fn init_display<'a, BL, DC, RSTpin, SCK, MISO, CS, MOSI>(
    sclk: impl Peripheral<P = SCK> + 'a,
    miso: impl Peripheral<P = MISO> + 'a,
    mosi: impl Peripheral<P = MOSI> + 'a,
    cs: impl Peripheral<P = CS> + 'a,
    rst: impl Peripheral<P = RSTpin> + 'a,
    dc: impl Peripheral<P = DC> + 'a,
    spi: impl Peripheral<P = impl esp_hal::spi::master::PeripheralInstance> + 'a,
    bl: impl Peripheral<P = BL> + 'a,
) -> impl 'a + DrawTarget<Color = Rgb565> + OriginDimensions
where
    SCK: esp_hal::gpio::OutputPin,
    MISO: esp_hal::gpio::InputPin,
    MOSI: esp_hal::gpio::OutputPin,
    CS: esp_hal::gpio::OutputPin,
    RSTpin: esp_hal::gpio::OutputPin,
    DC: esp_hal::gpio::OutputPin,
    BL: esp_hal::gpio::OutputPin,
{
    let mut cs_embedded_hal: esp_hal::gpio::Output<'_> =
        esp_hal::gpio::Output::new(cs, esp_hal::gpio::Level::High);

    let mut spi_display = esp_hal::spi::master::Spi::new(
        spi,
        esp_hal::spi::master::Config::default()
            .with_frequency(40.MHz())
            .with_mode(esp_hal::spi::Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(mosi)
    .with_miso(miso)
    .into_async();

    let mut delay = Delay.clone();

    let dc = esp_hal::gpio::Output::new(dc, esp_hal::gpio::Level::High);

    let spi_dev1 =
        embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(spi_display, cs_embedded_hal).unwrap();

    let buffer = SPI_BUFFER.init([0; 1024]);
    let dsp_interface = mipidsi::interface::SpiInterface::new(spi_dev1, dc, buffer); //change the value of the buffer if you like

    let mut display = mipidsi::Builder::new(mipidsi::models::ILI9341Rgb565, dsp_interface)
        .invert_colors(mipidsi::options::ColorInversion::Normal)
        .orientation(mipidsi::options::Orientation::new().rotate(mipidsi::options::Rotation::Deg90)) // Mirror on text
        .reset_pin(esp_hal::gpio::Output::new(rst, esp_hal::gpio::Level::High))
        .init(&mut delay)
        .unwrap();

    let mut bl = esp_hal::gpio::Output::new(bl, esp_hal::gpio::Level::High); // backlight
    bl.set_high();
    core::mem::forget(bl);

    display.clear(Rgb565::BLACK).unwrap();
    display
}

/// Initializes a touch sensor using SPI communication for the CYD touch panel.
///
/// # Arguments
/// * `miso` - Master Input Slave Output pin
/// * `mosi` - Master Output Slave Input pin
/// * `sclk` - Serial Clock pin
/// * `cs` - Chip Select pin
/// * `spi` - SPI peripheral instance
///
/// # Returns
/// A `TouchSensor` instance configured for the CYD touch panel
///
/// # Type Parameters
/// * `'d` - Lifetime of the peripheral references
/// * `MISO` - Type implementing `esp_hal::gpio::InputPin`
/// * `MOSI` - Type implementing `esp_hal::gpio::OutputPin`
/// * `SCK` - Type implementing `esp_hal::gpio::OutputPin`
/// * `CS` - Type implementing `esp_hal::gpio::OutputPin`
///
/// # Example
/// ```no_run
/// let mut touch = init_touch(
///     peripherals.GPIO39, // MISO
///     peripherals.GPIO32, // MOSI
///     peripherals.GPIO25, // SCLK
///     peripherals.GPIO33, // CS
///     peripherals.SPI3,   // SPI
/// );
/// ```
fn init_touch<'d, MISO, MOSI, SCK, CS>(
    miso: impl Peripheral<P = MISO> + 'd,
    mosi: impl Peripheral<P = MOSI> + 'd,
    sclk: impl Peripheral<P = SCK> + 'd,
    cs: impl Peripheral<P = CS> + 'd,
    spi: impl Peripheral<P = impl esp_hal::spi::master::PeripheralInstance> + 'd,
) -> cyd_touch::TouchSensor<impl embedded_hal::spi::SpiBus + 'd>
where
    MISO: esp_hal::gpio::InputPin,
    MOSI: esp_hal::gpio::OutputPin,
    SCK: esp_hal::gpio::OutputPin,
    CS: esp_hal::gpio::OutputPin,
{
    let mut spi_touch = esp_hal::spi::master::Spi::new(
        spi,
        esp_hal::spi::master::Config::default()
            .with_frequency(2.MHz())
            .with_mode(esp_hal::spi::Mode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(mosi)
    .with_miso(miso)
    .with_cs(cs)
    .into_async(); // Ensure CS is included for SPI communication

    let mut touch = cyd_touch::TouchSensor::new(spi_touch);
    touch
}

#[main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    esp_alloc::heap_allocator!(72 * 1024);

    esp_println::logger::init_logger_from_env();

    let timer0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    

    let mut display = init_display(
        peripherals.GPIO14, // SCLK
        peripherals.GPIO12, // MISO
        peripherals.GPIO13, // MOSI
        peripherals.GPIO15, // CS
        peripherals.GPIO4,  // Reset
        peripherals.GPIO2,  // Data/Command
        peripherals.SPI2,   // SPI
        peripherals.GPIO21, // Backlight
    );

    let mut touch = init_touch(
        peripherals.GPIO39, // MISO
        peripherals.GPIO32, // MOSI
        peripherals.GPIO25, // SCLK
        peripherals.GPIO33, // CS
        peripherals.SPI3,   // SPI
    );
    let touchpin = peripherals.GPIO36;
    let mut touchpin: Input<'_> = Input::new(touchpin, esp_hal::gpio::Pull::Up);

    spawner.spawn(button_task(touchpin)).unwrap(); // Spawn the task that waits for the TOUCHPIN "interrupt"

    let mut calibration = TouchCalibration::new();
    calibration
        .calibrate(&mut display, &mut touch, &mut SIGNAL.receiver().unwrap())
        .await
        .unwrap_or_else(|_| {
            log::error!("Touch calibration failed");
            panic!("Touch calibration failed")
        });

    let mut style = MonoTextStyle::new(&FONT_9X18, Rgb565::WHITE);

    // Position x:5, y: 10
    Text::new("Hello", Point::new(5, 10), style)
        .draw(&mut display)
        .unwrap_or_else(|_| panic!("Failed to draw text"));

    // Turn text to blue
    style.set_text_color(Some(Rgb565::WHITE));
    Text::new("World", Point::new(160, 26), style)
        .draw(&mut display)
        .unwrap_or_else(|_| panic!("Failed to draw text"));

    // Spawn a task that waits for the interrupt
    let mut recv = SIGNAL.receiver().unwrap();

    loop {
        recv.changed().await;
        let rawpoint = touch.read_raw_point().unwrap();
        let calibrated_point = calibration.apply(rawpoint);
        info!("Touch at {:?}", calibrated_point);
        Text::new("here", calibrated_point, style)
        .draw(&mut display)
        .unwrap_or_else(|_| panic!("Failed to draw text"));

    }
}

#[embassy_executor::task]
async fn button_task(mut pin: Input<'static>) {
    let sender = SIGNAL.sender();
    loop {
        pin.wait_for(esp_hal::gpio::Event::FallingEdge).await;
        // Wait for the interrupt notification
        sender.send(true);
        
        // Handle touch press
        info!("touch boring task: Touch was pressed");
        Timer::after(Duration::from_millis(100)).await;
    }
}
