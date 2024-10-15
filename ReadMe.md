# Game Boy Emulator

Team members:

- Chris Ding
- Sipei Zhou
- Xingwei Lin

## Summary Description

The goal of this project is to develop a GameBoy emulator using Rust. The emulator will be capable of running GameBoy ROMs and providing accurate emulation of the hardware, including the CPU, memory, graphics, and input systems. [Demo Video](https://drive.google.com/file/d/1XNJIJFE0hJnn-z0U1Fg7RxfDNSbZZIJ6/view?usp=sharing).

## Project Summary

### Progress Made

Finished:

- Implemented separate modules for the cartridge, CPU, graphics, clock, and memory.

- Wrote unit tests for memory, CPU instructions, and Joypad inputs.

- Passed all blargg test roms for CPU instructions, CPU timing, as well as interrupts.

- Set up logging for debugging purposes.

- Able to run ROM-only game.

- Emulate the same cpu frequency as the Gameboy system.

Incomplete:

- Fully implemented audio system: we've implemented one of the pulse channels (out of four channels), but encountered the issue with the audio queue not filling in fast enough, causing choppy audio.

- Implement complex memory bank modes (MBC3), which some roms (like Pokemon) use.

- Integrated the blargg test roms into `cargo test`.

### Lessons Learned

The lessons learned are:
- How the GameBoy system works, thanks to many resources such as [gbdev](https://gbdev.io/pandocs/) and [talks online](https://www.youtube.com/watch?v=HyzD8pNlpwI). We also gained an appreciation for older hardware.
- Using flamegraph to profile a Rust program. We used flamegraph because event polling was consuming a lot of cpu cycles, before optimizing, the framegraph looked as such:

![](fig/flamegraph_og.svg)
The program was running slower than the original Gameboy! Also, adding `std::thread::sleep` ignores any keys pressed during sleep. To solve this problem, we added a 0.1s delay between every polling, and used `sdl2`'s `delay` instead of `sleep`.

![](fig/flamegraph_poll_every.svg)
- A nice review of Assembly instructions with Gameboy's ASM instructions.

## Additional Details

### External Rust Crates

- `log` and `env_logger`: Used for logging messages for debugging purposes.
- `clap`: Used to parse the command line arguments to specify which rom file to read.
- `sdl2`: Used to display the GameBoy graphics.

***
***

## Details of Our Code

### Main
Main program logic

1. Logging System Initialization

    env_logger crate

2. Command Line Argument Parsing

    Defines and parses command-line arguments using the clap library. 

```
Parameters:

rom_file (required): Specifies the ROM file to load.

boot_bin (optional, with default): Specifies the boot ROM file to load, defaulting to assets/dmg_boot.bin if not provided.
```
3. Boot and ROM File Reading

    Read the boot and ROM files specified via command-line arguments.

4. Graphics Display Control

    Determines whether to enable graphics.

5. Initializing GameBoy Instance and Loading ROM

    Creates an instance of GameBoy, loading the boot and game ROMs based on the files read. It also controls whether to enable the graphical interface. Start the emulation with gameboy.run(). This method is responsible for executing the GameBoy's main loop, handling CPU instructions, graphics, sound, and other functionalities.

```rust
use std::{fs, path::Path};

use clap::{App, Arg};
use gb_rs::gb::GameBoy;
use log::{debug, info};

fn main() -> Result<(), String> {
    // Step 1:
    env_logger::init();

    // Step 2:
    let matches = App::new("gb-rs")
        .version("1.0")
        .about("A simple program to read a ROM file and emulate it")
        .arg(
            Arg::with_name("rom_file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("Sets the ROM file to read")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("boot_bin")
                .short('b')
                .long("boot binary")
                .value_name("BOOT")
                .help("Sets the Boot ROM file to read")
                .default_value(Path::new("assets").join("dmg_boot.bin").to_str().unwrap()),
        )
        .arg(
            Arg::with_name("no_graphics")
                .long("no-graphics")
                .help("Disables graphics")
                .takes_value(false)
                .required(false), // Set default value to true
        )
        .arg(
            Arg::with_name("no_audio")
                .long("no-audio")
                .help("Disables audio")
                .takes_value(false)
                .required(false), // Set default value to true
        )
        .get_matches();

    // Step 3:
    let boot_bin = matches.value_of("boot_bin").unwrap();
    info!("Loading boot bin {}", boot_bin);
    let contents = fs::read(boot_bin);
    let boot_bin = match contents {
        Ok(fs) => fs,
        Err(e) => {
            debug!("Unable to read file {} due to {}", boot_bin, e.to_string());
            return Err(String::from("Unable to read file"));
        }
    };

    let rom_file = matches.value_of("rom_file").unwrap();
    info!("Running rom file {}", rom_file);
    let contents = fs::read(rom_file);
    let rom_file = match contents {
        Ok(fs) => fs,
        Err(e) => {
            debug!("Unable to read file {} due to {}", rom_file, e.to_string());
            return Err(String::from("Unable to read file"));
        }
    };

    // Step 4:
    let graphics_enabled = !matches.is_present("no_graphics");

    // Step 5:
    let mut gameboy = GameBoy::new(graphics_enabled);
    gameboy.load_boot(boot_bin);
    gameboy.load_rom(rom_file);
    gameboy.run();

    Ok(())
}

```

### CPU

Architecture: The Game Boy's CPU is an 8-bit processor with a 16-bit address bus, allowing access to up to 64KB of memory. It operates at around 4.19 MHz.

Registers: 8-bit Registers: It features eight 8-bit registers (A, B, C, D, E, H, L, and F), with the F register used for storing flags like zero, carry, etc. 16-bit Registers: Additionally, it includes four 16-bit registers (AF, BC, DE, HL), which can be paired for various operations. We represent the registers inside the struct in the cpu:
```rust
pub struct CPU {
    pub a: Byte,
    pub b: Byte,
    pub c: Byte,
    pub d: Byte,
    pub e: Byte,
    pub h: Byte,
    pub l: Byte,
    pub f: Byte,                    // flag
    pub sp: Word,                   // stack pointer
    pub pc: Word,                   // program counter
    pub ime: (Option<usize>, bool), // Interrupt Master Enable Flag, left is countdown (if exists), right is the flag
    pub halt: bool,                 // Halt flag
}
```
as well as provide an enum for the CPU registers and register pairs.
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    HL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register16 {
    BC,
    DE,
    HL,
    SP,
    AF,
}
```

Instructions: The instruction set is similar to the Intel 8080 processor but with some variations. It covers data movement, arithmetic and logic operations, branching, and interrupts.  The CPU has assembly instruction (opcode) table as such:

![](fig/opcode.png)

Because similar operations often differ by only a few bits, for example, `LD r1,r2` loads values from `r2` to `r1`, this opcode is represented by the byte `0b01xxxyyy`, where xxx yyy specify `r1` and `r2`. Therefore, we represents an opcode with its effective fields. 

```rust
/// OpCode template with its effective fields
#[derive(Debug, PartialEq, Eq)]
pub struct OpCode(Byte, Byte);
```
It has a matches method to check if a given opcode matches its pattern.
```rust
impl OpCode {
    /// Check if the give opcode `code` matches self, considering mask
    fn matches(&self, code: Byte) -> bool {
        code.mask(self.1) == self.0
    }
}
```

We then decode the Byte instructions into the Instruction Enum:
```rust
#[derive(Debug, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum Instruction {
    ...
    /// Add (register)
    ADD_R(Register),
    /// Add (indirect HL)
    ADD_HL,
    /// Add (immediate)
    ADD_N(Byte),
    /// Subtract (register)
    SUB_R(Register),
    /// Subtract (indirect HL)
    SUB_HL,
    /// Subtract (immediate)
    SUB_N(Byte),
    /// And (register)
    AND_R(Register),
    /// And (indirect HL)
    AND_HL,
    /// And (immediate)
    AND_N(Byte),
    /// Or (register)
    OR_R(Register),
    ...
}
```

Interrupts: The CPU supports interrupts, allowing it to respond to external events such as button presses or timer overflows. It has five interrupt sources: V-Blank, LCD, Timer, Serial, and Joypad.

The way the cpu runs is it first decodes the intruction under the pc, then given the decoded instruction, it then executes the corresponding instruction, for example, `RET` return from subroutine by popping the pc from the address pointed to in the sp, similar to intel x86 instructions. 

## `clock.rs` Module

### Overview
The `clock.rs` module is an integral part of our Game Boy emulator, simulating the console's internal clock to manage timing and synchronization for game logic and animations. This module oversees the DIV register and the timing system involving TIMA, TMA, and TAC registers, all critical for replicating the original Game Boy's timing features.

### Introduction to Machine Cycles and Clock Cycles
Understanding machine cycles and clock cycles is crucial for emulating the Game Boy's timing mechanisms:

- **Clock Cycles**: These are the basic units of time in computing, defined by the frequency of the system’s clock. The Game Boy's CPU operates at approximately 4.19 MHz, translating to about 4.19 million cycles per second.
- **Machine Cycles**: On the Game Boy, a machine cycle consists of four clock cycles. These cycles are used to measure the duration it takes to perform operations, with each operation consuming a certain number of machine cycles.

### Functionalities
- **DIV Register**: Continuously increments based on the number of machine cycles elapsed since the last update. It provides a basic timing mechanism for operations that do not require precise timing, such as periodic updates in games.
- **TIMA, TMA, TAC Registers**:
  - **TIMA (Timer Counter)**: This timer counter increments according to the frequency set by the TAC register. When the TIMA register overflows (exceeding its maximum value of 255), it reloads from the TMA register and triggers an interrupt.
  - **TMA (Timer Modulo)**: This register stores the value that TIMA should reload from when it overflows.
  - **TAC (Timer Control)**: This control register enables or disables the TIMA timer and selects its frequency of operation.

### How `clock.rs` Serves the Game Boy Emulator
- **Example 1: Animation Timing**: Many games use the DIV register to control the frame update rate of animations, ensuring smooth transitions to the next frame at appropriate times.
- **Example 2: Gameplay Mechanics**: Certain gameplay mechanics, such as specific actions of characters or the generation of enemies, may rely on the precise timing of timers. The TIMA timer is set to trigger game events at specific intervals, such as spawning an enemy every few seconds or starting a countdown timer for the next game phase.

### Comparison with Traditional Computer Clocks
While both the Game Boy's internal clock and traditional computer clocks are used for measuring time, they have distinct characteristics and usage scenarios:
- **Purpose**: The Game Boy's internal clock is primarily used to synchronize in-game events and timing, controlling animations, game mechanics, and other time-sensitive operations. In contrast, traditional computer clocks are used for general timekeeping, task scheduling, and system operation coordination.
- **Granularity**: The granularity of timing may differ between the two systems. While both clocks operate at specific frequencies, the Game Boy's clock might have coarser granularity due to its lower clock speed and simpler timing requirements compared to modern computer clocks.
- **Control**: Programmers can directly control the Game Boy's timing through registers like DIV, TIMA, TMA, and TAC, allowing them to configure timers to trigger events or manage timing intervals as needed for the game. In contrast, while programmers can interact with traditional computer clocks through APIs or system calls, they have less direct control over its timing mechanisms.

## `memory.rs` Module

### Overview
The `memory.rs` module is foundational to the operation of our Game Boy emulator, handling all aspects of memory management crucial for replicating the functionality of the original Game Boy system. It simulates the memory architecture, including the interaction between the CPU, ROM, RAM, and various types of Memory Bank Controllers (MBCs).

### Key Features

#### Memory Layout Management
- **Fixed and Switchable Memory Banks**: The module defines and simulates fixed ROM areas and switchable banks for both ROM and RAM, which are essential for supporting games that exceed the Game Boy's native memory capacity.
- **Memory Size Constants**: Defines the size of the memory map, boot ROM area, and typical ROM and RAM bank sizes, setting up the framework for memory operations.

#### Cartridge Support
- **Multiple Cartridge Types**: Implements support for several types of Game Boy cartridges, including ROM-only, MBC1, and MBC3. This allows the emulator to load and run a wide range of Game Boy games, from simple early titles to more complex games that use enhanced features.
- **Dynamic Memory Banking**: Depending on the cartridge type loaded, the module can dynamically switch between different memory banks. This is crucial for accessing different parts of the game data stored on larger cartridges.

#### Memory Access
- **Read and Write Operations**: Provides methods for reading and writing bytes directly to the emulated memory. Special handling is included for memory-mapped I/O operations, such as DMA transfers, which are important for managing sprite data and other fast memory operations.
- **Boot ROM Handling**: Manages the simulation of the Game Boy's initial boot process, which is critical for setting up the correct initial state of the emulator.

### Implementation Details

#### Memory Structures
- **Memory Array**: Simulates the Game Boy's complete memory layout from 0x0000 to 0xFFFF.
- **Boot ROM and Cartridge ROM/RAM**: Separate arrays simulate the non-volatile memory present in the boot ROM and switchable cartridge ROM/RAM banks.

#### Special Function Handlers
- **Boot ROM Unloading**: Includes functionality to disable the boot ROM and switch to normal cartridge operation, mimicking the behavior of the actual hardware after startup.
- **Direct Memory Access (DMA)**: Handles DMA operations that allow for rapid data transfer within memory, essential for graphic rendering and animation.

### Usage Examples

#### Loading a Game
To load a game, the cartridge data is read into the appropriate memory structures, initializing the state based on the type of cartridge (e.g., ROM-only, MBC1).

#### Accessing Memory
During gameplay, CPU requests to read or write memory are managed by the module, ensuring that accesses to memory-mapped I/O and switchable banks are correctly handled.

### Importance in Emulation
`memory.rs` is not just a static component; it actively participates in the emulation process by simulating the memory-related operations that are integral to game functionality. This includes managing how games store and retrieve data, how they interact with hardware features, and how they execute game logic that depends on specific memory configurations.

### Limitations

#### Lack of Full MMU Support
While the `memory.rs` module effectively simulates various Memory Bank Controllers and basic memory operations necessary for most Game Boy games, it does not fully implement a Memory Management Unit (MMU). This limitation means that more complex memory management tasks, such as virtual memory handling, protection, and more advanced mapping features seen in modern computing systems, are not simulated. This is generally in line with the scope of classic Game Boy hardware but may restrict the ability to extend the emulator to simulate other advanced hardware features or newer console variants that require detailed MMU functionalities.

#### Limited MBC Support
Currently, the emulator primarily supports ROM-only games with limited functionality for games that require memory banking via MBCs (MBC1 and MBC3). This support is incomplete and may not accurately reflect all features needed for games that utilize extensive memory banking and advanced MBC features.


### Graphics

The Gameboy screen has resolution of 160x144 pixels, and has the ability to display up to 4 shades of gray. Emulators often support custom color palettes for user configuration.

Gameboy can only display 8x8 pixel tiles. These tiles are fixed textures that can be arranged on the screen grid. Each pixel is represented in 2 bits (2BPP format). Each tile is stored as 16 bytes, with each byte representing a row of 8 pixels. For example, the texture `A` is represented:

```
  Tile:                                     Image:

  .33333..                     .33333.. -> 01111100 -> $7C
  22...22.                                 01111100 -> $7C
  11...11.                     22...22. -> 00000000 -> $00
  2222222. <-- digits                      11000110 -> $C6
  33...33.     represent       11...11. -> 11000110 -> $C6
  22...22.     color                       00000000 -> $00
  11...11.     numbers         2222222. -> 00000000 -> $00
  ........                                 11111110 -> $FE
                               33...33. -> 11000110 -> $C6
                                           11000110 -> $C6
                               22...22. -> 00000000 -> $00
                                           11000110 -> $C6
                               11...11. -> 11000110 -> $C6
                                           00000000 -> $00
                               ........ -> 00000000 -> $00
                                           00000000 -> $00
```

We use a texture struct to represent this:

```rust
#[derive(Clone, Copy)]
struct Tile {
    tile: [[Pixel; 8]; 8],
}
```

and Pixel is defined as
```rust
#[derive(Clone, Copy, Debug, PartialEq)]
enum PixelSource {
    /// When background is disabled
    Background {
        enabled: bool,
    },
    Object {
        number: usize,
    }, // object number
}

#[derive(Clone, Copy)]
pub struct Pixel {
    color_ref: u8, // should be u2
    pixel_source: PixelSource,
}
```
We will see why PixelSource is important to remember later.

The GameBoy graphics represents the scene with background sprites and object sprites, both using textures references. The background scene uses 32x32 textures, giving a total 256x256 pixel size, thats why GameBoy requires a viewport to specify which 160x144 pixels to view into:

![](fig/background.png)
this also allows for dynamic scrolling effects, such as in mario, where the offscreen textures are swapped out before it is viewed to give a dynamic effect.

The pixels are rendered onscreen using scanlines, that means rendering each line left to right and top to bottom. Due to limitations to the hardware, the GameBoy can only render 10 objects in a single line

![](fig/scanline.png)

To render the line, GameBoy fetches the pixel values from an ObjectFIFO and a BackgroundFIFO, we generalize this using a trait:
```rust
pub trait FIFO {
    fn next_line(&mut self, memory: &Memory);
    fn pop(&mut self, memory: &Memory) -> Pixel;
}
```
and then mixes the Background pixels with the Object pixels, and puts it on screen. Lasty, to enable on screen effects, the GameBoy allows interrupts before rendering specific lines, this allows for effects like this:

![](fig/scanline_effect.png)

Gameboy graphics also has some specific timings that it must satisfy:

![](fig/ppu_mode_timing_basic.png)

### GameBoy

Emulates the core functionalities of a GameBoy console, including the CPU, memory, graphics, joypad, clock, and a debugger. 

#### Structure and Enum Definitions

GameBoy: The main structure containing all components required to emulate a GameBoy.

Debugger: Implementing Traits for Custom Behavior, such as pausing execution, stepping through execution, and setting breakpoints.

Breakpoint: An enum that defines breakpoints which can either be an instruction or an address.

#### run method

Main loop for our emulator.

```rust
pub fn run(mut self) {
    ... // Initialize timestamps and time

    loop {
        // enable events
        if let Some(ref mut graphics) = self.graphics {
            if last_poll_time.elapsed().as_millis() > 100 {
                for event in graphics.event_pump.poll_iter() {
                    match event {
                        Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape | Keycode::Q), .. } => return,
                        Event::KeyDown { keycode: Some(Keycode::P), .. } => self.dbg.toggle_pause(),
                        ...
                    }
                }
                last_poll_time = std::time::Instant::now();
            }
        }

        ... // CPU Execution and Interrupt Handling

        // render graphics
        if let Some(ref mut graphics) = self.graphics {
            graphics.render(&mut self.memory, self.clock.get_timestamp());
            if self.clock.get_timestamp() - last_timestamp > 17476 {
                while last_time.elapsed().as_millis() < 16 {
                    graphics.timer.delay(1);
                }
                last_timestamp = self.clock.get_timestamp();
                last_time = std::time::Instant::now();
            }
        }

        ... // audio(unimplemented)
    }
}
```

Run() implements the main execution loop of an emulator. It includes handling events, updating states, processing input, executing CPU instructions, handling interrupts, outputting debug information, and rendering graphics.

- Event Handling: Uses SDL2 to manage graphical and keyboard events. Specifically, it handles quit events and keypresses (e.g., the 'P' key for pausing, the right bracket key for stepping, and other game control keys).

- Pause and Step Control: If the debugger's state is set to pause or step, the main loop will pause accordingly or execute the next step.

- CPU Execution: If the CPU is not in a halt state, it executes an instruction.

- Interrupt Handling: The CPU checks for and processes any possible interrupts.

- Graphics Rendering: If graphics are enabled, the graphics module's render method is called.

To ensure timing is correct, we check how long the process above took, and make sure it runs at the same frequency as a Gameboy cpu.

### Joypad

Handling gamepad inputs.

Structure:

- Joypad: This structure manages gamepad input handling, containing two main fields:

    `last_keys`: A `HashSet<Keycode>` that stores the currently pressed keys.

    `code_keys`: A `HashMap<Byte, Keycode>` that maps GameBoy controller buttons to corresponding keyboard keys.

Constants: Define the memory addresses for the GameBoy controller register and specific bit flags related to button inputs

```rust
// ----- joypad controls -----
pub const JOYPAD_REGISTER_ADDRESS: Address = 0xFF00;
pub const DPAD_FLAG: Byte = 0b0001_0000;
pub const BUTTONS_FLAG: Byte = 0b0010_0000;

pub const RIGHT_BUTTON: Byte = 0b1110_1110;
pub const LEFT_BUTTON: Byte = 0b1110_1101;
pub const UP_BUTTON: Byte = 0b1110_1011;
pub const DOWN_BUTTON: Byte = 0b1110_0111;
pub const A_BUTTON: Byte = 0b1101_1110;
pub const B_BUTTON: Byte = 0b1101_1101;
pub const SELECT_BUTTON: Byte = 0b1101_1011;
pub const START_BUTTON: Byte = 0b1101_0111;
```

Two methods in Joypad struct:

- `update()`: Update button register

    Reads the current state of the joypad register.

    Determines which group of buttons (directional or action) to check based on the DPAD_FLAG and BUTTONS_FLAG.

    Updates the register value to reflect the current state of the pressed buttons. If a specific button is pressed, the corresponding bit in the button register is updated.

- `handle_button()`: Handle button press

    Handles the pressing (down = true) or releasing (down = false) of an individual button.

    Depending on the button type (directional or action), if the button is pressed for the first time and the 
    corresponding button group is active (checked via register flags), the joypad interrupt flag is set.

    Updates the last_keys set to track which keys are pressed or released.

Interesting thing to note, only the upper nibble of the joypad register is written to by the rom, when written to, it is used to check which buttons are selected.

|     | 7              | 6           | 5            | 4          | 3         | 2           | 1            | 0           |
|-----|----------------|-------------|--------------|------------|-----------|-------------|--------------|-------------|
| P1  | | | Select buttons | Select d-pad| Start / Down | Select / Up| B / Left  | A / Right   |              |             |

For example, if 0x20 is written to the Joypad register, it means the rom is trying to access which the d-pad buttons, a 0x23 means the down and up buttons are written.

## Rust Specific Features

### Implementing Graphics using Enum and State Machines

As stated above in the graphics section, the PPU (pixel prossessing unit) has specific modes, we represent these using Enums:
```rust
/// PPU Mode with corresponding line number
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PPUMode {
    /// Horizontal BLANK
    Mode0 { line: usize },
    /// Vertical BLANK
    Mode1 { line: usize },
    /// OAM Scan
    Mode2 { line: usize },
    /// Drawing Pixels
    Mode3 { line: usize },
}
```
and going from one mode to another requires actions, for example, going from Mode2 -> Mode3 requires drawing the pixels onscreen, we accomplish this using match and a state machine pattern:
```rust
match (self.last_ppu_mode, current_ppu_mode) {
    (PPUMode::Mode1 { line: l1 }, PPUMode::Mode2 { line: l2 })
        if l1 == 153 && l2 == 0 =>
    {
        // new frame
        self.set_lyc(memory);
    }
    (PPUMode::Mode2 { line: l1 }, PPUMode::Mode3 { line: l2 }) if l1 == l2 => {
        // draw scanline
        self.draw_scanline(memory);
    }
    (PPUMode::Mode3 { line: l1 }, PPUMode::Mode0 { line: l2 }) if l1 == l2 => {
        // finish draw pixel to hblank
    }
    (PPUMode::Mode0 { line: l1 }, PPUMode::Mode2 { line: l2 }) if l1 + 1 == l2 => {
        // newline
        self.set_lyc(memory);
    }
    (PPUMode::Mode0 { line: l1 }, PPUMode::Mode1 { line: l2 }) if l1 + 1 == l2 => {
        // render to screen if vblank
        self.set_lyc(memory);
        self.set_vblank_int(memory);
        let mut texture = self
            .texture_creator
            .create_texture_target(
                PixelFormatEnum::RGB24,
                SCREEN_WIDTH as u32,
                SCREEN_HEIGHT as u32,
            )
            .unwrap();
        texture
            .update(None, &self.screen_buffer, SCREEN_WIDTH * 3)
            .unwrap();
        self.canvas.copy(&texture, None, None).unwrap();
        self.canvas.present();
    }
    (PPUMode::Mode1 { line: l1 }, PPUMode::Mode1 { line: l2 }) if l1 + 1 == l2 => {
        // newline in vblank mode
        self.set_lyc(memory);
    }
    _ => panic!(
        "PPU Transition Error {:?} {:?}, Clock Diff {:?} at line {:?}",
        self.last_ppu_mode, current_ppu_mode, clock_diff, self.line_y
    ),
}
```

### Liberal use of Block Returns

We make liberal use of returns from blocks, whether its `if` blocks or `match` blocks, for example:
```rust
let palette = if get_flag(obj_flag, OBJ_PALETTE_FLAG) {
    memory.read_byte(OBP1_ADDRESS)
} else {
    memory.read_byte(OBP0_ADDRESS)
};
```
gets the palette color depending on the bit value.

### Rust and Bit Manipulation

After working with bit manipulation, I've started to enjoy how Rust differentiates between `a as Type` and `Type::from(a)`. 

The most important difference can be seen when converting `u32` to `i32`: When you treat a `u32` as an `i32` directly without explicit conversion, Rust will interpret the bits in memory as if they represent a signed integer, you can do this using the `as` keyword:
```rust
let unsigned_value: u32 = 4294967295; // Maximum value of u32
let signed_value = unsigned_value as i32;
```
this will cast the underlying bit representation into `i32`, so the result is `-1`.

To convert the value, you can use `into`/`from`, for example:
```rust
let unsigned_value: u32 = 4294967295; // Maximum value of u32
let signed_value = i32::try_from(unsigned_value);
```
will return an Err because the underlying value is too big. For example, in the cpu implementation, there is a command `ADD SP,e8` which takes an intermediate value, interprets it as a signed integer, and adds it to the stack pointer, we can accomplish this using `as`:
```rust
let e = memory.read_byte(address + 1) as SignedByte;
(Instruction::ADD_SP_E(e), 2)
```

### Entry API for HashMap

In the PPU, the rom is allowed to modify the texture while drawing scanlines, but the old texture should still appear on screen, this is why we require a `tile_cache` that caches the texture. However, due to rust's ownership model, getting and setting from a HashMap is quite painful, that is why there is the Entry API that helps with this. 
```rust
let tile = match self.tile_cache.entry(tile_pos) {
    Entry::Occupied(occ) => occ.into_mut(),
    Entry::Vacant(vacant) => {
        let tile_idx = tile_pos.i + tile_pos.j * 32;
        let tile_num_address = map_address + (tile_idx as Address);
        let tile_num = memory.read_byte(tile_num_address);
        let tile_start_address = if get_flag(lcdc, BGW_TILES_DATA_FLAG) {
            0x8000 + BYTES_PER_TILE * (tile_num as Address)
        } else {
            let res = 0x9000 + (BYTES_PER_TILE as i32) * ((tile_num as i8) as i32);
            res as Address
        };

        let tile = Tile::fetch_tile(
            memory,
            PixelSource::Background {
                enabled: window_enabled,
            },
            tile_start_address,
        );
        vacant.insert(tile)
    }
};
```
The code operates on a HashMap called tile_cache, and it uses the entry method to access or insert a value into the map. This provides efficient access to map entries while avoiding unnecessary double lookups and unwraps.
We also use pattern matching on the Enum, and into_mut() method is called on the Occupied variant, consuming it and returning a mutable reference to the entry's value.

### Type Aliasing

Rust allows type aliasing, making the code much more readable. For example, we soft differentiate between `Word`, which are simple values, and `Addresses`, even though they are both `u16`s:
```rust
pub type Byte = u8;
pub type SignedByte = i8;
pub type Address = u16;
pub type Word = u16;
```

## Rust Specific Challenges & Abandoned Approaches

### Memory and Mutable References

Throughout the codebase, accessing memory is essential. For instance, the graphic module requires access to VRAM, and the joypad module needs access to the joypad registers. In C++, it would be natural to store a pointer to the memory object in both structs:

```cpp
class Graphics {
private:
    Memory* memoryPtr; // Pointer to memory object
public:
    Graphics(Memory* mem) : memoryPtr(mem) {}
    // Other methods
};

class Joypad {
private:
    Memory* memoryPtr; // Pointer to memory object
public:
    Joypad(Memory* mem) : memoryPtr(mem) {}
    // Other methods
};
```

However, due to Rust's ownership rules, storing a reference to memory would require multiple mutable references to the same object, which Rust doesn't allow. Instead of storing it directly in the Graphics and Joypad structs, we pass a reference when calling the function:

```rust
pub fn render(&mut self, memory: &mut Memory, timestamp: u128) {
    let clock_diff = timestamp - self.last_timestamp;
    // Code for rendering
    // Access memory using the passed reference
    // More code...
}
```

By following Rust's ownership model, we guarantee that memory access is available when and where it's needed. This approach fits seamlessly into the main emulation loop:

```rust
// update joypad
self.joypad.update(&mut self.memory);

// start executing gb
if self.cpu.halt {
    self.clock.tick(1, &mut self.memory);
} else {
    self.cpu.execute(&mut self.memory, &mut self.clock);
}

self.cpu.handle_interrupts(&mut self.memory);

self.cpu.ime_step();

// serial output debug
if self.memory.read_byte(0xff02) != 0 {
    let c = self.memory.read_byte(0xff01) as char;
    print!("{}", c);
    self.memory.write_byte(0xff02, 0);
}

// render graphics
if let Some(ref mut graphics) = self.graphics {
    // non gb related keydowns
    graphics.render(&mut self.memory, self.clock.get_timestamp());
    if self.clock.get_timestamp() - last_timestamp > 17476 {
        while last_time.elapsed().as_millis() < 16 {
            graphics.timer.delay(1);
        }
        last_timestamp = self.clock.get_timestamp();
        last_time = std::time::Instant::now();
    }
}
```
If we didn't have the Game Boy loop serving as the foundation for everything, it might be simpler to store an `Arc<RefCell>` of the Memory object.

### CPU Opcodes and Instruction Enums

At first, we planned to combine decoding and executing in one function. But Prof. Fluet persuaded us otherwise. It turned out to be a smart move. As we added more instructions, separating decode and execute made it so its easier to spot and fix bugs. Keeping decoding and execution separate proved to be very handy.

Initially, we thought about using bit matching directly in the if statements. But this approach made bugs really difficult to spot. That's when we decided to introduce another struct called Opcode. It has a template and an effective field, and we implemented the match function with it. This not only improved readability but also made debugging much easier.
```rust
let (instruction, size) = if Self::NOP.matches(opcode) {
    (Instruction::NOP, 1)
} else if Self::PUSH_POP.matches(opcode) {
    let rr = Register16::get_rr(opcode >> 4, false);
    if opcode & (1 << 2) != 0 {
        (Instruction::PUSH(rr), 1)
    } else {
        (Instruction::POP(rr), 1)
    }
} else if Self::ARITH_OP_R.matches(opcode) {
    let r = Register::get_r(opcode);
    let instruction = match (opcode.get_high_nibble(), r) {
        (8, Register::HL) => Instruction::ADD_HL,
        (8, r) => Instruction::ADD_R(r),
        (9, Register::HL) => Instruction::SUB_HL,
        (9, r) => Instruction::SUB_R(r),
        (0xa, Register::HL) => Instruction::AND_HL,
        (0xa, r) => Instruction::AND_R(r),
        (0xb, Register::HL) => Instruction::OR_HL,
        (0xb, r) => Instruction::OR_R(r),
        _ => panic!("Unknown combination, should never happen"),
    };
    (instruction, 1)
} else if Self::ARITH_OP_C_R.matches(opcode) {
    ...
}
```