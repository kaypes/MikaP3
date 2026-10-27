#![no_std]

//! Hardware-independent application, content and rendering for MikaP3.

#[cfg(test)]
extern crate std;

pub mod app;
pub mod arts;
pub mod audio;
pub mod controls;
pub mod flappy;
pub mod snake;
pub mod songs;
pub mod state;
pub mod visual;

// Exercise the driver's pure wire encoder on the host, without the HAL.
#[cfg(test)]
#[path = "leds/encoding.rs"]
mod led_encoding;
