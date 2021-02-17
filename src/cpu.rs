

#[derive(Clone, Copy)]
#[repr(C)]
pub union RegisterPair {
    both: u16,
    one: (u8, u8),
}

impl RegisterPair {
    pub fn new() -> RegisterPair {
        RegisterPair { both: 0 } 
    } 

    pub fn both(self) -> u16 {
        unsafe  { self.both }
    }

    pub fn both_mut(&mut self) -> &mut u16 {
        unsafe { &mut self.both }
    }

    pub fn lsb(self) -> u8 {
        unsafe { self.one.0 } 
    }

    pub fn lsb_mut(&mut self) -> &mut u8 {
        unsafe { &mut self.one.0 }
    }

    pub fn msb(self) -> u8 {
        unsafe { self.one.1 }
    }

    pub fn msb_mut(&mut self) -> &mut u8 {
        unsafe { &mut self.one.1 }
    }
}

pub struct Flags {
    zero: bool,
    sign: bool,
    parity: bool,
    carry: bool,
    aux_carry: bool,
}

const MEMORY_SIZE: usize = 0x4000;

pub struct State8080 {
    a: u8,
    bc: RegisterPair,
    de: RegisterPair,
    hl: RegisterPair,
    sp: u16,
    pc: u16,
    memory: [u8; MEMORY_SIZE],
    flags: Flags,
    interupts_enabled: bool,
}


impl State8080 {
    pub fn new() -> State8080 {
        State8080 { 
            a: 0,
            bc: RegisterPair::new(),
            de: RegisterPair::new(),
            hl: RegisterPair::new(),
            sp: 0,
            pc: 0,
            memory: [0; MEMORY_SIZE],
            flags: Flags {
                zero: false,
                sign: false,
                parity: false,
                carry: false,
                aux_carry: false,
            },
            interupts_enabled: false,
        }
    }


    fn read_byte(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }  

    fn read_bytes(&self, address: u16) -> u16 {
        ((self.read_byte(address + 1) as u16) << 8) | self.read_byte(address) as u16
    }

    fn read_next_instruction_byte(&self) -> u8 {
        self.read_byte(self.pc + 1)
    }

    fn read_next_instruction_bytes(&self) -> u16 {
        self.read_bytes(self.pc + 1)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        let mut stored_value = self.memory[address as usize];
        stored_value = value;
    }

    fn write_bytes(&mut self, address: u16, value: u16) {
        self.write_byte(address, value as u8);
        self.write_byte(address, (value >> 8) as u8);
    }

    pub fn emulate(&mut self, ) {
        let opcode = self.memory[self.pc as usize];

        let size = match opcode {
            0x00 | 0x20 => 1,
            0x01 => {
                *self.bc.both_mut() = self.read_next_instruction_bytes();
                3
            },
            0x02 => {
                self.write_byte(self.bc.both(), self.a);
                1
            },
            0x03 => {
                *self.bc.both_mut() = self.bc.both().wrapping_add(1);
                1 
            },
            0x04 => {
                
            }
            _ => panic!("unimplemented instruction {}", opcode),
        };

        self.pc += 1;
    }
}
