# EmfrpVM
Emfrp is an FRP language that runs on small-scale embedded systems.
EmfrpVM is an Emfrp execution environment which provides REPL.

# How to build
You need Rust environment for compiling the Emfrp program.
Install in https://www.rust-lang.org/learn/get-started beforehand

## ESP32
You need esp-idf SDK :  https://github.com/espressif/esp-idf 
1. Execute `install.ps1`, or `install.sh` in `emfrp-machine/esp32` directory.
2. Execute `idf.py build` and flash `idf.py flash`, which flashes executable binary on ESP32.
3. Execute `cargo run` in `emfrp-compiler` directory.

## Arduino Uno
You need platformio SDK : https://platformio.org/
1. Execute `platformio run --target upload`, which flashes executable binary on Arduino Uno.
2. Execute `cargo run` in `emfrp-compiler` directory.

## Micro:bit
You need platformio SDK : https://platformio.org/
1. Execute `platformio run --target upload`, which flashes executable binary on Microbit.
2. Execute `cargo run` in `emfrp-compiler` directory.
