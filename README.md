# Chip Eight

## Background

This project is an attempt at a generic chip 8 emulator implementation that can be plugged in to any front end (kind of). Ultimately it is a personal learning project for me to eventually run the emulator on a Rasberry Pi on a 64x32 LED array. On the way there though, I would like to be able to keep testing, as I don't have all of the hardware set up yet.

The idea here is that there are two traits that need to be implemented to get the emulator working on a system, `Draw` and `ReadInputState`. These simply give the emulator handles to the functions to draw to the screen, and read the current keyboard state.

```rust
pub trait Draw {
    fn draw_buffer(&mut self, screen_buf: &[u8]);
    fn clear_screen(&mut self);
}

pub trait ReadInputState {
    fn read_keys_state(&self) -> Result<[u8; 16], String>;
    fn reset_keys_state(&mut self);
}
```

If interested, see `examples/winit_example.rs` to find a dummy implementation that might work on other platforms than my own.

## Examples
You can also run `cargo run --example winit_example /path/to/chip8/binary.c8` to try it out.

Additionally run `cargo run --example debug_example /path/to/chip8/binary.c8` to step through a program. Hold enter to speed through, type q to quit.

Potentially if you want to see the most cursed implementation check out `cargo run --example debug_example /path/to/chip8/binary.c8`.

## Usage

I have not implemented quirks yet, so usage will change, probably.

Once you have implemented your traits you just need to call `init` and `run` on the emulator. Init loads the program, and will throw an error if the program or font would load out of range. Currently the font is static, so it won't cause a panic, but same can't be said if you try load Bee Movie in as a ch8 program.

```rust
    let drawer = Drawer::init();
    let input_reader = InputReader::init();

    let emulator = chip_eight::Emulator::init(program, drawer, input_reader)
        .expect("The chip 8 program is probably too large")
        .run();
```

The emulator will block the current thread, so do make sure you do the input handling off thread. Maybe this is a place for improving the UX for this library, not really sure.

## TODO
- Ironically, TODO: Populate todo list more
- Implement quirks
- Find out if the flashing is normal

