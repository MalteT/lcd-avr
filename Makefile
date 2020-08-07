
PORT=/dev/ttyACM0

run: build
	avrdude -pm328p -carduino -b115200 -Uflash:w:target/avr-atmega328p/release/lcd-avr-rust-example.elf:e -P${PORT}

build:
	cargo build --release

