//! LCD communication library

use atmega328p_hal::{
    atmega328p::PORTD,
    clock,
    delay::Delay,
    port::{mode::*, Pin},
    prelude::*,
};

const LCD_ENABLE_TIME_US: u16 = 1;

pub struct Lcd {
    port: PORTD,
    data_pins: [Pin<Output>; 8],
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
        $lcd.port.ddrd.write(|w| unsafe { w.bits(0b11111111) });
        $lcd.port.portd.write(|w| unsafe { w.bits($data) });
        $lcd.trigger_enable();
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
        port: PORTD,
        register_select: Pin<Output>,
        read_write: Pin<Output>,
        enable: Pin<Output>,
        debug_led: Pin<Output>,
    ) -> Self {
        let mut lcd = Lcd {
            port,
            register_select,
            read_write,
            enable,
            debug_led,
        };
        // Do a manual init, just in case
        lcd.do_manual_init();
        // Setup
        lcd.set_function(DisplayLines::Single);
        lcd.configure_display(DisplayState::On, Cursor::On, Blinking::Off);
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
                Delay::<clock::MHz8>::new().delay_ms(700_u16);
            }
            self.write_byte_to_display(*byte);
            counter += 1;
        }
        self.set_entry_mode(Direction::Right, DisplayShift::Off);
    }

    fn do_manual_init(&mut self) {
        self.register_select.set_low().expect("infallible");
        self.read_write.set_low().expect("infallible");
        self.port.ddrd.write(|w| unsafe { w.bits(0b11111111) });
        // Wait 50ms for display activation
        Delay::<clock::MHz8>::new().delay_ms(50_u16);
        self.send_raw_byte(0b00110000);
        Delay::<clock::MHz8>::new().delay_ms(5_u16);
        self.send_raw_byte(0b00110000);
        Delay::<clock::MHz8>::new().delay_us(150_u16);
        self.send_raw_byte(0b00110000);
    }

    pub fn clear_display(&mut self) {
        implement_lcd_instruction!(self; 0, 0, 0b00000001);
    }

    pub fn return_home(&mut self) {
        implement_lcd_instruction!(self; 0, 0, 0b00000010);
    }

    pub fn set_function(&mut self, display_lines: DisplayLines) {
        let byte = 0b00110000 | (display_lines as u8) << 3;
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

    fn send_raw_byte(&mut self, byte: u8) {
        self.register_select.set_low().expect("infallible");
        self.read_write.set_low().expect("infallible");
        self.port.ddrd.write(|w| unsafe { w.bits(0b11111111) });
        self.port.portd.write(|w| unsafe { w.bits(byte) });
        self.trigger_enable();
    }

    fn wait_while_busy(&mut self) {
        self.debug_led.set_high().expect("infallible");
        // Configure pin 7 as input with connected pull-up
        self.port.ddrd.write(|w| w.pd7().clear_bit());
        self.port.portd.write(|w| w.pd7().set_bit());
        // Configure rs and rw
        self.register_select.set_low().expect("infallible");
        self.read_write.set_high().expect("infallible");
        // Loop while busy flag is not cleared
        let mut busy = true;
        while busy {
            self.enable.set_high().expect("infallible");
            Delay::<clock::MHz8>::new().delay_us(LCD_ENABLE_TIME_US);
            busy = self.port.pind.read().pd7().bit_is_set();
            self.enable.set_low().expect("infallible");
        }
        self.debug_led.set_low().expect("infallible");
        self.port.ddrd.write(|w| w.pd7().set_bit());
    }

    fn trigger_enable(&mut self) {
        self.enable.set_high().expect("infallible");
        Delay::<clock::MHz8>::new().delay_us(LCD_ENABLE_TIME_US);
        self.enable.set_low().expect("infallible");
    }

    fn write_byte_to_display(&mut self, byte: u8) {
        self.wait_while_busy();
        self.register_select.set_high().expect("infallible");
        self.read_write.set_low().expect("infallible");
        self.port.ddrd.write(|w| unsafe { w.bits(0b11111111) });
        self.port.portd.write(|w| unsafe { w.bits(byte) });
        self.trigger_enable();
    }

    fn update_data_pins(&mut self, data: u8) {
        let do_set_bit = |pin: &mut Pin<Output>, bit: u8| if bit == 0 {
            pin.set_low()
        } else {
            pin.set_high()
        };

    }
}
