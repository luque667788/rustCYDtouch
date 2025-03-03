#![no_std]
extern crate alloc;
use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    watch::Receiver,
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{ascii::FONT_9X18, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, Primitive, RgbColor},
    primitives::{Circle, PrimitiveStyle},
    text::Text,
};
use libm::roundf;

use embedded_graphics::Drawable;


use embedded_hal::spi::{ErrorKind, SpiDevice};
use esp_println::println;

/// A structure responsible for interfacing with a touchscreen via SPI communication.
///
/// The TouchSensor provides methods to initialize, read coordinates from, and manage
/// interactions with a touchscreen controller connected via SPI.
pub struct TouchSensor<T: SpiDevice> {
    /// The SPI device used to communicate with the touchscreen controller
    touch_spi: T,
}

impl<T: SpiDevice> TouchSensor<T> {
    /// Creates a new TouchSensor instance.
    ///
    /// This constructor initializes the touch sensor with an SPI device for communication.
    /// It creates an atomic boolean for tracking touch state and sets up a callback function
    /// that gets triggered on touch events.
    ///
    /// # Parameters
    /// * `spi_device` - An SPI device implementing the `SpiDevice` trait for touch controller communication
    ///
    /// # Returns
    /// A configured TouchSensor instance ready for use
    pub fn new(spi_device: T) -> TouchSensor<T> {



        /* TODO! one day make this work with the light sleep mode and nostd
        unsafe {
            esp_idf_svc::hal::sys::gpio_wakeup_enable(irq_touch.pin(), 0);
            esp_idf_svc::hal::sys::esp_sleep_enable_gpio_wakeup();
        }*/

        let mut touchscreen = TouchSensor {
            touch_spi: spi_device,
        };

        touchscreen.init_touch().unwrap();

        return touchscreen;
    }

    /// Reads the current touch coordinates from the touchscreen controller.
    ///
    /// This method performs an SPI transaction to request and retrieve touch coordinates.
    /// It follows the protocol defined by the touchscreen controller, which involves:
    ///
    /// 1. Sending a series of control bytes to request position data
    /// 2. Reading the response bytes containing X and Y coordinates
    /// 3. Processing the raw data by combining bytes and applying bit shifts to extract the actual coordinates
    ///
    /// The SPI transaction uses a specific protocol where:
    /// - `0xD0` requests touch data with pen interrupt enabled
    /// - The following bytes are structured to request 16-bit X and Y readings
    /// - The received data needs bit-shifting by 3 positions to get the actual coordinates
    ///
    /// # Returns
    /// * `Ok((y, x))` - A tuple containing the Y and X coordinates as 16-bit integers
    /// * `Err` - An error from the SPI transaction if communication fails
    pub fn read_raw_point(&mut self) -> Result<(i16, i16), ErrorKind> {
        /*we can optimize this measurement if there are many measyerement (a lot of samplings and the ncalcualte teh average of somethighn) */

        let mut read_buf = [0; 5];
        let write_buf = [
            0xD0,   // the last bits is PD0 which controls if the pen interuupt is enalbled or not
            0 >> 7, // we send 16bits
            0x90,
            0 >> 7, // we send 16bits every time the second half is 0 and it
            0,      //  is the time the sensor will do its math and give the last 5 bits of data
        ];

        self.touch_spi
            .transfer(&mut read_buf, &write_buf)
            .map_err(|_| ErrorKind::Other)?;

        //println!("Read: {:?}", read_buf);
        // we are ignoring the first byte readbuff[0] (it is the one we actually sent the measurements)
        let mut x = (read_buf[1] as i16) << 8 | read_buf[2] as i16; //merge them together
        let mut y = (read_buf[3] as i16) << 8 | read_buf[4] as i16; // we are ignoring the first byte

        x = x >> 3; // we need to that becasue the 3 last bits are just 0 (datasheet) fig 12 and 14
        y = y >> 3;

        Ok((y, x))
    }

    /// Initializes the touchscreen by performing test measurements.
    ///
    /// This private method performs initial communication with the touch controller to verify operation
    /// and determine baseline measurements. It:
    ///
    /// 1. Performs an initial SPI communication to establish connection
    /// 2. Samples multiple touch readings to calculate average values
    /// 3. Prints diagnostic information to the console
    ///
    /// This initialization process helps detect any communication issues early and provides baseline
    /// values for touch coordinates.
    ///
    /// # Returns
    /// * `Ok(())` - If initialization is successful
    /// * `Err` - An error from the SPI transaction if communication fails
    fn init_touch(&mut self) -> Result<(), ErrorKind> {
        let write_buf: [u8; 5] = [0; 5];
        let mut read_buf: [u8; 5] = [0; 5];

        self.touch_spi
            .transfer(&mut read_buf, &write_buf)
            .map_err(|_| ErrorKind::Other)?;

        println!("Read: {:?}", read_buf);

        println!("going to measure again");

        const NUM_SAMPLES: usize = 2; // tune this value to get a better average
        let mut x_samples = [0i16; NUM_SAMPLES];
        let mut y_samples = [0i16; NUM_SAMPLES];
        let mut x_sum: i16 = 0;
        let mut y_sum: i16 = 0;
        for i in 0..NUM_SAMPLES {
            let (x, y) = self.read_raw_point()?;
            x_samples[i as usize] = x;
            y_samples[i as usize] = y;
            x_sum += x;
            y_sum += y;
        }
        //println!("X samples: {:?}", x_samples);
        //println!("Y samples: {:?}", y_samples);
        println!("X average: {}", x_sum / NUM_SAMPLES as i16);
        println!("Y average: {}", y_sum / NUM_SAMPLES as i16);
        Ok(())
    }

    /* TODO! one day make this work with the light sleep mode and nostd
    pub fn sleep_until_touch(&mut self) -> bool{
        unsafe {
            // Enter light sleep mode (CPU halts until an interrupt occurs) until the user touches the screen
            //esp_idf_svc::hal::sys::esp_light_sleep_start(); TODO!
        }
        self.is_touched()
    }
    */
}

/// A structure for calibrating touchscreen coordinates to match display coordinates.
///
/// TouchCalibration stores transformation parameters (scaling factors and offsets) that convert
/// raw touchscreen readings into display coordinates. It also provides methods to perform
/// the calibration process and apply the transformation to touch coordinates.
pub struct TouchCalibration {
    /// Scaling factor for X coordinates
    pub alpha_x: f32,
    /// Offset adjustment for X coordinates
    pub delta_x: f32,
    /// Scaling factor for Y coordinates
    pub alpha_y: f32,
    /// Offset adjustment for Y coordinates
    pub delta_y: f32,
}

impl TouchCalibration {
    /// Creates a new TouchCalibration instance with default values.
    ///
    /// Initializes a calibration structure with identity transformation (scaling factors of 1.0
    /// and offsets of 0.0), which results in raw touch coordinates being used without adjustment.
    ///
    /// # Returns
    /// A new TouchCalibration instance with default values
    pub fn new() -> TouchCalibration {
        TouchCalibration {
            alpha_x: 1.0,
            delta_x: 0.0,
            alpha_y: 1.0,
            delta_y: 0.0,
        }
    }

    /// Applies the calibration transformation to raw touch coordinates.
    ///
    /// This method converts raw touchscreen coordinates to display coordinates using
    /// the calibration parameters. The transformation involves:
    ///
    /// 1. Converting input values to floating point
    /// 2. Applying offsets (delta_x, delta_y)
    /// 3. Applying scaling factors (alpha_x, alpha_y)
    /// 4. Converting back to integer coordinates for display use
    ///
    /// # Parameters
    /// * `(ix, iy)` - A tuple containing raw X and Y touch coordinates as 16-bit integers
    ///
    /// # Returns
    /// A Point structure containing the calibrated X and Y coordinates in display space
    pub fn apply(&self, (ix, iy): (i16, i16)) -> Point {
        let x = (ix as f32 + self.delta_x) * self.alpha_x;
        let y = (iy as f32 + self.delta_y) * self.alpha_y;
        return Point::new(x as i32, y as i32);
    }

    /// Performs an interactive touchscreen calibration procedure.
    ///
    /// This asynchronous method guides the user through a calibration process by:
    ///
    /// 1. Clearing the display and showing instructions
    /// 2. Drawing calibration points at known screen positions (red dot, then yellow dot)
    /// 3. Waiting for user touches at each point using the provided signal
    /// 4. Reading raw touch coordinates when each point is touched
    /// 5. Calculating calibration parameters (scaling factors and offsets) based on the relation
    ///    between known screen positions and raw touch readings
    /// 6. Averaging multiple calibration points for better accuracy
    /// 7. Updating the calibration parameters in the current instance
    /// 8. Displaying a success message
    ///
    /// The calibration uses two reference points to determine both scaling and offset in both dimensions.
    ///
    /// # Type Parameters
    /// * `D` - A display type that implements DrawTarget with Rgb565 color support
    /// * `T` - An SPI device type used for touchscreen communication
    /// * `M` - A mutex type for the signal
    /// * `A` - A cloneable type for the signal
    /// * `N` - A constant size parameter for the signal buffer
    ///
    /// # Parameters
    /// * `display` - A mutable reference to the display for drawing calibration UI
    /// * `touchscreen` - A mutable reference to the TouchSensor for reading coordinates
    /// * `signal` - A mutable reference to a Receiver signal used to wait for touch events
    ///
    /// # Returns
    /// * `Ok(())` - If calibration completes successfully
    /// * `Err` - An error from display operations if they fail
    pub async fn calibrate<D, T, M, A, const N: usize>(
        &mut self,
        display: &mut D,
        touchscreen: &mut TouchSensor<T>,
        signal: &mut Receiver<'_, M, A, N>,
    ) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Rgb565> + embedded_graphics::geometry::OriginDimensions,
        T: SpiDevice,
        M: RawMutex,
        A: Clone,
    {
        display.clear(Rgb565::BLACK)?;

        // Calibrate logic here, for example, drawing calibration points on the display
        let style = MonoTextStyle::new(&FONT_9X18, Rgb565::WHITE);
        Text::new(
            "Calibrating... click on the red dot",
            Point::new(5, 20),
            style,
        )
        .draw(display)?;

        // Draw a single calibration point
        let x = roundf(display.size().width as f32 / 1.1) as i32;
        let y = roundf(display.size().height as f32 / 1.1) as i32;
        let point_a = Point::new(x, y);

        Circle::new(point_a, 3)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
            .draw(display)?;


        println!("Waiting for touch...");
        signal.changed().await;

        // wait for touch

        //first I need to setup the interrupt or somthign to wait for the touch
        let m_a = touchscreen.read_raw_point().expect("spi error"); //TODO! handle this error better
        println!("First Touch at ({}, {})", m_a.0, m_a.1);
        display.clear(Rgb565::BLACK)?;

        // Calibrate logic here, for example, drawing calibration points on the display
        let style = MonoTextStyle::new(&FONT_9X18, Rgb565::WHITE);
        Text::new(
            "Calibrating...  \n click on the yellow dot",
            Point::new(5, 20),
            style,
        )
        .draw(display)?;

        let mut point_b = Point::new(
            display.size().width as i32 / 2,
            display.size().height as i32 / 2,
        );
        point_b.x = point_b.x - 125;
        point_b.y = point_b.y - 42;

        Circle::new(point_b, 3)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
            .draw(display)?;

        println!("Waiting for second touch...");
        signal.changed().await;

        let m_b = touchscreen.read_raw_point().expect("spi error"); //TODO! handle this error better
        println!("Second Touch at ({}, {})", m_b.0, m_b.1);
        // Calculate calibration values

        let alpha_x = (point_a.x - point_b.x) as f32 / (m_a.0 - m_b.0) as f32;
        let alpha_y = (point_a.y - point_b.y) as f32 / (m_a.1 - m_b.1) as f32;

        let delta_x = point_a.x as f32 / alpha_x - m_a.0 as f32;
        let delta_y = point_a.y as f32 / alpha_y - m_a.1 as f32;

        let d2elta_x = point_b.x as f32 / alpha_x - m_b.0 as f32;
        let d2elta_y = point_b.y as f32 / alpha_y - m_b.1 as f32;

        self.delta_x = (delta_x + d2elta_x) / 2.0;
        self.delta_y = (delta_y + d2elta_y) / 2.0;

        self.alpha_x = alpha_x;
        self.alpha_y = alpha_y;

        println!("Alpha X: {}", self.alpha_x);
        println!("Alpha Y: {}", self.alpha_y);
        println!("Calibration done");

        display.clear(Rgb565::BLACK)?;

        // Create a new text style for the success message
        let success_style = MonoTextStyle::new(&FONT_9X18, Rgb565::GREEN);

        // Calculate the center position of the screen
        let center = Point::new(
            (display.size().width as i32 / 2) - 80,
            display.size().height as i32 / 2,
        );

        // Display a message saying the calibration was successful
        Text::new("Calibration successful!", center, success_style).draw(display)?;
        Timer::after(Duration::from_millis(2000)).await;
        display.clear(Rgb565::BLACK)?;
        Ok(())
    }
}
