//! LCD communication library.
//!
//! This does not have production quality and there aren't even comments..

use atmega328p_hal::{
    delay::Delay,
    port::{mode::*, portd, Pin},
    prelude::*,
};
use embedded_hal::digital::v2::{InputPin, OutputPin};

use super::ClkSpeed;

const LCD_ENABLE_TIME_US: u8 = 1;
const LCD_TIME_BETWEEN_SCROLLING: u16 = 500;

pub struct Lcd {
    data_pin7: portd::PD7<Output>,
    data_pin6: portd::PD6<Output>,
    data_pin5: portd::PD5<Output>,
    data_pin4: portd::PD4<Output>,
    data_directions: portd::DDR,
    register_select: Pin<Output>,
    read_write: Pin<Output>,
    enable: Pin<Output>,
    debug_led: Pin<Output>,
}

macro_rules! implement_lcd_instruction {
    ($lcd:expr; 0, 0, $data:expr) => {
        implement_lcd_instruction!($lcd; false, false, $data)
    };
    ($lcd:expr; $rs:literal, $rw:literal, $data:expr) => {
        $lcd.wait_while_busy();
        if $rs {
            $lcd.register_select.set_high().expect("infallible");
        } else {
            $lcd.register_select.set_low().expect("infallible");
        }
        if $rw {
            $lcd.read_write.set_high().expect("infallible");
        } else {
            $lcd.read_write.set_low().expect("infallible");
        }
        $lcd.send_two_part_data($data);
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayLines {
    Single = 0,
    Two = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayState {
    Off = 0,
    On = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    Off = 0,
    On = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blinking {
    Off = 0,
    On = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left = 0,
    Right = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayShift {
    Off = 0,
    On = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayOrCursor {
    Cursor = 0,
    Display = 1,
}

impl Lcd {
    pub fn new(
        data_pin7: portd::PD7<Output>,
        data_pin6: portd::PD6<Output>,
        data_pin5: portd::PD5<Output>,
        data_pin4: portd::PD4<Output>,
        data_directions: portd::DDR,
        register_select: Pin<Output>,
        read_write: Pin<Output>,
        enable: Pin<Output>,
        debug_led: Pin<Output>,
    ) -> Self {
        let mut lcd = Lcd {
            data_pin7,
            data_pin6,
            data_pin5,
            data_pin4,
            data_directions,
            register_select,
            read_write,
            enable,
            debug_led,
        };
        // Do a manual init, just in case
        lcd.do_manual_init();
        // Setup
        lcd.set_function(DisplayLines::Single);
        lcd.configure_display(DisplayState::On, Cursor::Off, Blinking::Off);
        lcd.set_entry_mode(Direction::Right, DisplayShift::Off);
        // Return
        lcd
    }

    pub fn set_str(&mut self, content: &[u8]) {
        self.clear_display();
        self.return_home();
        // Write all bytes
        let mut counter = 0_u8;
        for byte in content {
            if counter == 16 {
                self.set_entry_mode(Direction::Right, DisplayShift::On);
            }
            if counter >= 16 {
                Delay::<ClkSpeed>::new().delay_ms(LCD_TIME_BETWEEN_SCROLLING);
            }
            self.write_byte_to_display(*byte);
            counter += 1;
        }
        self.set_entry_mode(Direction::Right, DisplayShift::Off);
    }

    pub fn append_byte(&mut self, byte: u8) {
        self.write_byte_to_display(byte);
    }

    fn do_manual_init(&mut self) {
        self.register_select.set_low().expect("infallible");
        self.read_write.set_low().expect("infallible");
        // Wait 50ms for display activation
        Delay::<ClkSpeed>::new().delay_ms(50_u16);
        self.send_one_part_data(0b0011);
        Delay::<ClkSpeed>::new().delay_ms(5_u16);
        self.send_one_part_data(0b0011);
        Delay::<ClkSpeed>::new().delay_us(150_u16);
        self.send_one_part_data(0b0011);
        // Enable 4 bit mode
        self.send_one_part_data(0b0010);
    }

    pub fn clear_display(&mut self) {
        implement_lcd_instruction!(self; 0, 0, 0b00000001);
    }

    pub fn return_home(&mut self) {
        implement_lcd_instruction!(self; 0, 0, 0b00000010);
    }

    pub fn set_function(&mut self, display_lines: DisplayLines) {
        let byte = 0b00100000 | (display_lines as u8) << 3;
        implement_lcd_instruction!(self; 0, 0, byte);
    }

    pub fn configure_display(&mut self, display: DisplayState, cursor: Cursor, blinking: Blinking) {
        let byte = 0b00001000 | (display as u8) << 2 | (cursor as u8) << 1 | (blinking as u8);
        implement_lcd_instruction!(self; 0, 0, byte);
    }

    pub fn set_entry_mode(&mut self, dir: Direction, shift: DisplayShift) {
        let byte = 0b00000100 | (dir as u8) << 1 | (shift as u8);
        implement_lcd_instruction!(self; 0, 0, byte);
    }

    pub fn turn_display_off(&mut self) {
        implement_lcd_instruction!(self; 0, 0, 0b00001000);
    }

    pub fn shift(&mut self, obj: DisplayOrCursor, dir: Direction) {
        let byte = match (obj, dir) {
            (DisplayOrCursor::Cursor, Direction::Left) => 0b00010000,
            (DisplayOrCursor::Cursor, Direction::Right) => 0b00010100,
            (DisplayOrCursor::Display, Direction::Left) => 0b00011000,
            (DisplayOrCursor::Display, Direction::Right) => 0b00011100,
        };
        implement_lcd_instruction!(self; 0, 0, byte);
    }

    fn wait_while_busy(&mut self) {
        self.debug_led.set_high().expect("infallible");
        // Configure pin 7 as input with connected pull-up
        // This should be safe, since I own this pin and I change it back before the end of this
        // function. I might rething this once interrupts exist...
        let pin7: portd::PD7<Output> = unsafe { ::core::mem::zeroed() };
        let pin7 = pin7.into_pull_up_input(&self.data_directions);
        // Configure rs and rw
        self.register_select.set_low().expect("infallible");
        self.read_write.set_high().expect("infallible");
        // Loop while busy flag is not cleared
        let mut busy = true;
        while busy {
            self.enable.set_high().expect("infallible");
            Delay::<ClkSpeed>::new().delay_us(LCD_ENABLE_TIME_US);
            busy = pin7.is_high().expect("infallible");
            self.enable.set_low().expect("infallible");
            // Skip the next 4 bit
            self.trigger_enable();
        }
        self.debug_led.set_low().expect("infallible");
        let _ = pin7.into_output(&self.data_directions);
    }

    fn trigger_enable(&mut self) {
        self.enable.set_high().expect("infallible");
        Delay::<ClkSpeed>::new().delay_us(LCD_ENABLE_TIME_US);
        self.enable.set_low().expect("infallible");
    }

    fn write_byte_to_display(&mut self, byte: u8) {
        self.wait_while_busy();
        self.register_select.set_high().expect("infallible");
        self.read_write.set_low().expect("infallible");
        self.send_two_part_data(byte);
    }

    fn send_two_part_data(&mut self, byte: u8) {
        self.send_one_part_data((byte & 0b11110000) >> 4);
        self.send_one_part_data(byte & 0b00001111);
    }

    fn send_one_part_data(&mut self, byte: u8) {
        self.data_pin7.set(byte & 0b1000 != 0).expect("infallible");
        self.data_pin6.set(byte & 0b0100 != 0).expect("infallible");
        self.data_pin5.set(byte & 0b0010 != 0).expect("infallible");
        self.data_pin4.set(byte & 0b0001 != 0).expect("infallible");
        self.trigger_enable();
    }
}

trait OutputPinExt {
    type Error;
    fn set(&mut self, state: bool) -> Result<(), Self::Error>;
}

impl<P> OutputPinExt for P
where
    P: OutputPin,
{
    type Error = <P as OutputPin>::Error;
    fn set(&mut self, state: bool) -> Result<(), Self::Error> {
        match state {
            true => self.set_high(),
            false => self.set_low(),
        }
    }
}
