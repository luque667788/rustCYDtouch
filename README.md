# cyd_touch

A Rust library for interfacing with touchscreens on ESP32 microcontrollers, specifically designed for the "CYD" board (those yellow cheap esp32 boards with touchscreen displays that you can find on aliexpress).

## Features

- Platform-independent SPI communication with touch controllers
- Raw touch coordinate readings
- Touch calibration routines to map touch coordinates to display coordinates
- Coordinate system transformations
- Written in 100% Rust with `no_std` support

## Platform Independence

This crate is designed to be as platform-independent as possible:

- Uses `embedded-hal` traits for hardware abstraction
- SPI communication is platform-agnostic
- Touch interrupt pin handling is delegated to the user application
- Core functionality works on any platform that implements the required `embedded-hal` traits

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