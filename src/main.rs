#![feature(llvm_asm, lang_items, abi_avr_interrupt)]
#![no_std]
#![no_main]

use atmega328p_hal::{
    atmega328p::Peripherals,
    clock,
    delay::Delay,
    prelude::*,
};

pub mod lcd;

use lcd::Lcd;

#[lang = "eh_personality"]
#[no_mangle]
pub unsafe extern "C" fn rust_eh_personality() -> () {}

#[panic_handler]
fn panic(_info: &::core::panic::PanicInfo) -> ! {
    let io = unsafe { Peripherals::steal() };
    let portb = io.PORTB.split();
    let mut led = portb.pb5.into_output(&portb.ddr);
    let mut delay = Delay::<clock::MHz8>::new();
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
    let register_select = portb_parts.pb2.into_output(&portb_parts.ddr).downgrade();
    let read_write = portb_parts.pb1.into_output(&portb_parts.ddr).downgrade();
    let enable = portb_parts.pb0.into_output(&portb_parts.ddr).downgrade();
    let debug_led = portb_parts.pb5.into_output(&portb_parts.ddr).downgrade();
    let mut lcd = Lcd::new(io.PORTD, register_select, read_write, enable, debug_led);

    lcd.set_str(b"Hallo, Malte!");
    Delay::<clock::MHz8>::new().delay_ms(5000_u16);
    lcd.set_str(b"Ich habe dich bereits erwartet...");
    Delay::<clock::MHz8>::new().delay_ms(5000_u16);
    lcd.set_str(b"Wie geht es dir?");
    Delay::<clock::MHz8>::new().delay_ms(5000_u16);
    lcd.turn_display_off();
    loop {}
}

