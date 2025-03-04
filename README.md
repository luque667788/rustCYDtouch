# cyd_touch

A Rust library for interfacing with touchscreens on ESP32 microcontrollers, specifically designed for the "CYD" board (those yellow cheap esp32 boards with touchscreen displays that you can find on aliexpress).

## Features

- Platform-independent SPI communication with touch controllers using embedded-hal abstractions
- Raw touch coordinate readings
- Touch calibration routines to map touch coordinates to display coordinates
- Written in 100% Rust with `no_std` support

## Platform Independence

I've tried to keep this library as platform-independent as possible with my level of knowledge:

- Uses `embedded-hal` traits for hardware abstraction where applicable
- SPI communication aims to be platform-agnostic using `SpiDevice` trait from `embedded-hal`
- Touch interrupt handling is left to the user application for flexibility
- The core functionality should work on platforms that implement the required `embedded-hal` traits (though you'll need to handle interrupts in your application)

## Current Limitations

- Basic functionality currently supports single-touch only (no multi-touch)
- No drag gesture support (only tap/click events)
- This is a personal project that works for my specific use case and may require modifications for your needs. Feel free to use it as a reference example for interfacing with the touch sensor on CYD boards via SPI.


## Example

The repository includes an Embassy async example demonstrating how to use the library with async Rust.

To run the example:

```bash
cd examples/embassy_async
cargo run (this will already upload to the esp32)
```

The example showcases:
- Initializing an ILI9341 display over SPI
- Setting up a touchscreen sensor
- Running a calibration procedure
- Processing touch events asynchronously
- Displaying touch events on screen

## License

MIT License