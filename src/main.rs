#![feature(llvm_asm, lang_items, abi_avr_interrupt)]
#![no_std]
#![no_main]
//! Example communication between an the PC, the ATmega328p and an LCD.
//!
//! The user can send strings over a serial connection that will be displayed on the
//! connected LCD display. The strings must end with '\0', the zero byte.

use atmega328p_hal::{atmega328p::Peripherals, clock, delay::Delay, prelude::*, usart::Usart0};
use nb::block;

pub mod lcd;

use lcd::Lcd;

type ClkSpeed = clock::MHz16;

#[lang = "eh_personality"]
#[no_mangle]
pub unsafe extern "C" fn rust_eh_personality() -> () {}

#[panic_handler]
fn panic(_info: &::core::panic::PanicInfo) -> ! {
    let io = unsafe { Peripherals::steal() };
    let portb = io.PORTB.split();
    let mut led = portb.pb5.into_output(&portb.ddr);
    let mut delay = Delay::<ClkSpeed>::new();
    loop {
        led.set_high().expect("Error cannot occur");
        delay.delay_ms(500_u16);
        led.set_low().expect("Error cannot occur");
        delay.delay_ms(500_u16);
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    let io = Peripherals::take().expect("Peripherals untaken");
    let portb_parts = io.PORTB.split();
    let portd_parts = io.PORTD.split();
    let register_select = portb_parts.pb2.into_output(&portb_parts.ddr).downgrade();
    let read_write = portb_parts.pb1.into_output(&portb_parts.ddr).downgrade();
    let enable = portb_parts.pb0.into_output(&portb_parts.ddr).downgrade();
    let debug_led = portb_parts.pb5.into_output(&portb_parts.ddr).downgrade();
    let data_pin7 = portd_parts.pd7.into_output(&portd_parts.ddr);
    let data_pin6 = portd_parts.pd6.into_output(&portd_parts.ddr);
    let data_pin5 = portd_parts.pd5.into_output(&portd_parts.ddr);
    let data_pin4 = portd_parts.pd4.into_output(&portd_parts.ddr);
    let usart = io.USART0;
    let rx = portd_parts.pd0.into_pull_up_input(&portd_parts.ddr);
    let tx = portd_parts.pd1.into_output(&portd_parts.ddr);
    let mut lcd = Lcd::new(
        data_pin7,
        data_pin6,
        data_pin5,
        data_pin4,
        portd_parts.ddr,
        register_select,
        read_write,
        enable,
        debug_led,
    );
    let mut serial = Usart0::<ClkSpeed, _>::new(usart, rx, tx, 57600);

    lcd.set_str(b"");
    loop {
        let mut bytes = [0; 64];
        let mut idx = 0;
        let mut last_byte = 1;
        while last_byte != b'\0' && idx < bytes.len() {
            last_byte = block!(serial.read()).unwrap();
            bytes[idx] = last_byte;
            idx += 1;
        }

        lcd.set_str(&bytes[0..idx - 1]);
    }
}
