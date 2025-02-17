#![no_std]
use core::sync::atomic::{AtomicBool, Ordering};
extern crate alloc;
use alloc::{boxed::Box, sync::Arc};
use libm::roundf;
use embedded_graphics::{
    mono_font::{ascii::FONT_9X18, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, Primitive, RgbColor},
    primitives::{Circle, PrimitiveStyle},
    text::Text,
};

use embedded_graphics::Drawable;

use embedded_hal::{delay::DelayNs, spi::SpiDevice};

use embedded_hal::spi::ErrorKind;
use esp_println::println;

pub struct TouchSensor<T: SpiDevice, > {
    touch_spi: T,
    touch_pressed: Arc<AtomicBool>,
    pub callback: Box<dyn FnOnce()>,
}

// i could implement this without diynamci dispatch but the nthe syntax would be strange
// the user would need to provide a function that accepts a notehr funciton and also the atomic bool
// this would defeat the purpose of the abstraction

impl<T: SpiDevice,> TouchSensor< T> {
    pub fn new(
        spi_device: T, // maybe change to a generic spi device later
    ) -> TouchSensor<T> {


        

        let touch_pressed = Arc::new(AtomicBool::new(false));
        let touch_pressed_clone1 = Arc::clone(&touch_pressed);

        let callback = Box::new(move || {
            touch_pressed_clone1.store(true, Ordering::Relaxed);
        });

        /*
        unsafe {
            esp_idf_svc::hal::sys::gpio_wakeup_enable(irq_touch.pin(), 0);
            esp_idf_svc::hal::sys::esp_sleep_enable_gpio_wakeup();
        }*/

        let mut touchscreen = TouchSensor {
            touch_spi: spi_device,
            touch_pressed,
            callback,
        };

        touchscreen.init_touch().unwrap();

        return touchscreen;
    }

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

    fn init_touch(&mut self) -> Result<(), ErrorKind> {
        //end here
        let mut write_buf: [u8; 5] = [0; 5];
        let mut read_buf: [u8; 5] = [0; 5];

        self.touch_spi
            .transfer(&mut read_buf, &write_buf)
            .map_err(|_| ErrorKind::Other)?;

        println!("Read: {:?}", read_buf);

        println!("going to measure again");

        let num_samples = 1;
        let mut x_samples = [0i16; 1];
        let mut y_samples = [0i16; 1];
        let mut x_sum: i16 = 0;
        let mut y_sum: i16 = 0;
        for _ in 0..num_samples {
            let (x, y) = self.read_raw_point()?;
            x_samples[num_samples as usize] = x;
            y_samples[num_samples as usize] = y;
            x_sum += x;
            y_sum += y;
        }
        //println!("X samples: {:?}", x_samples);
        //println!("Y samples: {:?}", y_samples);
        println!("X average: {}", x_sum / num_samples as i16);
        println!("Y average: {}", y_sum / num_samples as i16);
        Ok(())
    }
/*
    fn reenable_interrupt(&mut self) {
        self.irq_touch.enable_interrupt().unwrap();
    }*/

    pub fn is_touched(&mut self) -> bool {
        if self.touch_pressed.swap(false, Ordering::Acquire) {
            //self.reenable_interrupt(); TODO!

            return true;
        } else {
            return false;
        }
    }
    //TODO! use the wait trait of the embedded hal async

    pub fn sleep_until_touch(&mut self) -> bool{
        unsafe {
            // Enter light sleep mode (CPU halts until an interrupt occurs) until the user touches the screen
            //esp_idf_svc::hal::sys::esp_light_sleep_start(); TODO!
        }
        self.is_touched()
    }
}

pub struct TouchCalibration {
    pub alpha_x: f32,

    pub delta_x: f32,
    pub alpha_y: f32,

    pub delta_y: f32,
}

impl TouchCalibration {
    pub fn new() -> TouchCalibration {
        TouchCalibration {
            alpha_x: 1.0,
            delta_x: 0.0,
            alpha_y: 1.0,
            delta_y: 0.0,
        }
    }

    pub fn apply(&self, (ix, iy): (i32, i32)) -> (i32, i32) {
        let x = (ix as f32 + self.delta_x) * self.alpha_x;
        let y = (iy as f32 + self.delta_y) * self.alpha_y;
        return (x as i32, y as i32);
    }

    pub fn calibrate<D, T: SpiDevice, Dl: DelayNs,>(
        &mut self,
        display: &mut D,
        touchscreen: &mut TouchSensor<T>,
        thisdelay: &mut Dl,
    ) -> Result<(), <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Rgb565> + embedded_graphics::geometry::OriginDimensions,
    {
        display
            .clear(Rgb565::BLACK)?;

        // Calibrate logic here, for example, drawing calibration points on the display
        let style = MonoTextStyle::new(&FONT_9X18, Rgb565::WHITE);
        Text::new(
            "Calibrating... click on the red dot",
            Point::new(5, 20),
            style,
        )
        .draw(display)?;

        // Draw a single calibration point
        let x = roundf((display.size().width as f32 / 1.1)) as i32;
        let y = roundf(display.size().height as f32 / 1.1) as i32;
        let point_a = Point::new(
            x,y
        );

        Circle::new(point_a, 3)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
            .draw(display)?;

        /*will implement a blocking behoavir first later i may change this to a thread or somehting pralle
        byut for now i ma just going to wait for a notification signal tirgered form the interrupt
        but i will need also to reenable the interrrupt after it is triggered everytime
         */
        println!("Waiting for touch...");
        touchscreen.sleep_until_touch();
        
        // wait for touch

        //first I need to setup the interrupt or somthign to wait for the touch
        let m_a = touchscreen.read_raw_point().expect("spi error"); //TODO! handle this error better
        println!("First Touch at ({}, {})", m_a.0, m_a.1);
        display
            .clear(Rgb565::BLACK)?;

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

        println!("Waiting for touch...");
        touchscreen.sleep_until_touch();

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

        display
            .clear(Rgb565::BLACK)?;

        // Create a new text style for the success message
        let success_style = MonoTextStyle::new(&FONT_9X18, Rgb565::GREEN);

        // Calculate the center position of the screen
        let center = Point::new(
            (display.size().width as i32 / 2) - 80,
            (display.size().height as i32 / 2),
        );

        // Display a message saying the calibration was successful
        Text::new("Calibration successful!", center, success_style)
            .draw(display)?;
        thisdelay.delay_ms(2000);
        display
            .clear(Rgb565::BLACK)?;
        Ok(())
    }
}
