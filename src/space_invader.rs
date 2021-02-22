use crate::cpu::{RegisterPair, State8080};

struct GameState {
    cpu: State8080,
    io_state: SpaceInvaderIO,
    instr_count: u64,
    cycles: u64,
    frames: u64,
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
        }
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
    window_state: [u32; 224 * 256],
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
            window_state: [0; 224 * 256],
        }
    }
}
