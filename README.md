# lcd-avr-rust-example

Example communication between an the PC, the ATmega328p and an LCD.

The user can send strings over a serial connection that will be displayed on the
connected LCD display. The strings must end with '\0', the zero byte.
