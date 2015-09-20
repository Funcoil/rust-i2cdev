// Copyright 2015, Paul Osborne <osbpau@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/license/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option.  This file may not be copied, modified, or distributed
// except according to those terms.

// Reads data from Wii Nunchuck

extern crate i2cdev;
extern crate docopt;

use i2cdev::core::*;
use i2cdev::linux::*;
use std::io::prelude::*;
use std::env::args;
use docopt::Docopt;
use std::thread;
use std::path::Path;

const NUNCHUCK_SLAVE_ADDR: u16 = 0x52;

const USAGE: &'static str = "
Reading Wii Nunchuck data via Linux i2cdev.

Usage:
  nunchuck <device>
  nunchuck (-h | --help)
  nunchuck --version

Options:
  -h --help    Show this help text.
  --version    Show version.
";

// TODO: Move Nunchuck code out to be an actual sensor and add tests

#[derive(Debug)]
struct NunchuckReading {
    joystick_x: u8,
    joystick_y: u8,
    accel_x: u16,  // 10-bit
    accel_y: u16,  // 10-bit
    accel_z: u16,  // 10-bit
    c_button_pressed: bool,
    z_button_pressed: bool,
}

impl NunchuckReading {
    fn from_data(data: &[u8]) -> Option<NunchuckReading> {
        if data.len() < 6 {
            None
        } else {
            Some(NunchuckReading {
                joystick_x: data[0],
                joystick_y: data[1],
                accel_x: (data[2] as u16) << 2 | ((data[5] as u16 >> 6) & 0b11),
                accel_y: (data[3] as u16) << 2 | ((data[5] as u16 >> 4) & 0b11),
                accel_z: (data[4] as u16) << 2 | ((data[5] as u16 >> 2) & 0b11),
                c_button_pressed: (data[5] & 0b10) == 0,
                z_button_pressed: (data[5] & 0b01) == 0,
            })
        }
    }
}

struct Nunchuck {
    i2cdev: LinuxI2CDevice,
}

impl Nunchuck {
    /// Create a new Wii Nunchuck
    ///
    /// This method will open the provide i2c device file and will
    /// send the required init sequence in order to read data in
    /// the future.
    pub fn new<P: AsRef<Path>>(path: P) -> I2CResult<Nunchuck> {
        let i2cdev = try!(LinuxI2CDevice::new(path, NUNCHUCK_SLAVE_ADDR));
        let mut nunchuck = Nunchuck { i2cdev: i2cdev };
        try!(nunchuck.init());
        Ok(nunchuck)
    }

    /// Send the init sequence to the Wii Nunchuck
    pub fn init(&mut self) -> I2CResult<()> {
        // These registers must be written; the documentation is a bit
        // lacking but it appears this is some kind of handshake to
        // perform unencrypted data tranfers
        try!(self.i2cdev.smbus_write_byte_data(0xF0, 0x55));
        try!(self.i2cdev.smbus_write_byte_data(0xFB, 0x00));
        Ok(())
    }

    pub fn read(&mut self) -> I2CResult<NunchuckReading> {
        let mut buf: [u8; 6] = [0; 6];

        // tell the nunchuck to prepare a sample
        try!(self.i2cdev.smbus_write_byte(0x00));

        // now, read it!
        thread::sleep_ms(10);
        try!(self.i2cdev.read(&mut buf));
        match NunchuckReading::from_data(&buf) {
            Some(reading) => Ok(reading),
            None => Err(I2CError::Other("Unable to Parse Data"))
        }
    }
}

fn main() {
    let args = Docopt::new(USAGE)
        .and_then(|d| d.argv(args().into_iter()).parse())
        .unwrap_or_else(|e| e.exit());
    let device = args.get_str("<device>");
    match Nunchuck::new(device) {
        Err(err) => { println!("Unable to open {:?}, {:?}", device, err); }
        Ok(mut nunchuck) => {
            loop {
                match nunchuck.read() {
                    Ok(reading) => println!("{:?}", reading),
                    Err(err)    => println!("Error: {:?}", err),
                };
            }
        },
    }
}
