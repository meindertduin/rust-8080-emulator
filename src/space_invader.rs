use crate::cpu::{RegisterPair, State8080};
use minifb::Window;

pub struct GameState {
    cpu: State8080,
    io_state: SpaceInvaderIO,
    instr_count: u64,
    cycles: u64,
    frames: u64,
    window_state: [u32; 224 * 256],
}

impl GameState {
    const SCREEN_WIDTH: u64 = 224;
    const SCREEN_HEIGHT: u64 = 256;
    const CYCLES_PER_FRAME: u64 = 4_000_000 / 60;

    pub fn new_game() -> Self {
        Self {
            cpu: State8080::load_from_rom(include_bytes!("invaders.rom"), 0, 0),
            io_state: SpaceInvaderIO::new(),
            instr_count: 0,
            cycles: 0,
            frames: 0,
            window_state: [0; 224 * 256],
        }
    }

    pub fn next_frame(&mut self, window: &mut Window) {
        self.handle_half_render(window, true);
        self.handle_half_render(window, false);

        self.frames += 1;
        self.handle_input(&window);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    fn handle_half_render(&mut self, window: &mut Window, is_top: bool) {
        let mut cycles_spent = 0;
        while cycles_spent < Self::CYCLES_PER_FRAME / 2 {
            let cycles = self.cpu.emulate(&mut self.io_state);
            
            cycles_spent += cycles_spent;
            self.instr_count += 1;
            self.cycles += cycles;
        }

        let (mem_start, pix_start) = if is_top {
            (0x2400, 0)
        } else {
            (0x3200, 0x7000)
        };

        for offset in 0..0xE00 {
            let byte = self.cpu.memory()[mem_start + offset];

            for bit in 0..8 {
                let color: u32 = if byte & (1 << bit) == 0 {
                    0x00_00_00_00
                } else {
                    0xff_ff_ff_ff
                };

                let x = (pix_start + 8 * offset + bit) as u64 / Self::SCREEN_HEIGHT;
                let y = Self::SCREEN_HEIGHT -1 - (pix_start + 8 * offset + bit) as u64 % Self::SCREEN_HEIGHT;

                self.window_state[(x + y * Self::SCREEN_WIDTH) as usize] = color;
            }
        }

        window.update_with_buffer(&self.window_state, Self::SCREEN_WIDTH as usize, Self::SCREEN_HEIGHT as usize)
            .unwrap_or_else(|e| println!("Error while updating window: {}", e));

        self.cpu.interrupt(if is_top { 1 } else { 2 });
    }

    fn handle_input(&mut self, window: &Window) {
    
    }
}


pub trait IOState {
    fn input(&self, port: u8) -> u8;
    fn output(&mut self, port: u8, value: u8);
}

pub struct SpaceInvaderIO {
    port0: u8,
    port1: u8,
    port2: u8,
    shift_register: RegisterPair,
    shift_offset: u8,
}

impl IOState for SpaceInvaderIO {
    fn input(&self, port: u8) -> u8 {
        match port {
            1 => self.port1,
            2 => self.port2,
            3 => (self.shift_register.both() >> (8 - self.shift_offset)) as u8,
            _ => panic!("port {} is not readable", port),
        }
    }

    fn output(&mut self, port: u8, value: u8) {
        match port {
            2 => self.shift_offset = value & 0b111,
            4 => {
                *self.shift_register.lsb_mut() = self.shift_register.msb();
                *self.shift_register.msb_mut() = value;
            },
            3 | 5 | 6 => {

            },
            _ => panic!("port {} is not writable", port)
        }
    }
} 

impl SpaceInvaderIO {
    pub fn new() -> Self {
        Self {
            shift_register: RegisterPair::new(),
            shift_offset: 0,
            port0: 0b0111_0000,
            port1: 0b0001_0000,
            port2: 0b0000_0000,
        }
    }
}
