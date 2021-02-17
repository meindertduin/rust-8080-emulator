

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

impl Flags {
    fn set_sign(&mut self, value: u8) {
       self.sign = value & (1 << 7) != 0; 
    }

    fn set_zero(&mut self, value: u8) {
        self.zero = value == 0;
    }

    fn set_aux_carry(&mut self, value: u8) {
        self.aux_carry = value > 0xf;
    }

    fn set_pariry(&mut self, value: u8) {
        self.parity = value.count_ones() % 2 == 0;
    }

    pub fn set_carry(&mut self, value: u16) {
        self.carry = value > 0xff;
    }

    pub fn set_all_but_carry(&mut self, value: u8) {
        self.set_zero(value);
        self.set_sign(value);
        self.set_all_but_carry(value);
        self.set_pariry(value);
    }

    pub fn set_all_but_aux_carry(&mut self, value: u16) {
        self.set_zero(value as u8);
        self.set_pariry(value as u8);
        self.set_sign(value as u8);
        self.set_carry(value);
    }

    pub fn set_all(&mut self, value: u16, aux_value: u8) {
        self.set_carry(value);
        self.set_pariry(value as u8);
        self.set_sign(value as u8);
        self.set_carry(value);
        self.set_aux_carry(aux_value);
    }
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

    // single register instructions

    fn inr(&mut self, operand: u8) -> u8 {
        let result = operand.wrapping_add(1);
        self.flags.set_all_but_carry(result);
        result 
    }

    fn dec(&mut self, operand: u8) -> u8 {
        let result = operand.wrapping_sub(1);
        self.flags.set_all_but_carry(result);
        result
    }

    fn cma(&mut self) {
        self.a = ! self.a;
    }

    fn daa(&mut self) {
        let mut result = self.a as u16;

        let lsb = result & 0xf;

        if self.flags.aux_carry || lsb > 9 {
            result += 6;

            if result & 0xf < lsb {
                self.flags.aux_carry = true;
            }
        }

        let lsb = result & 0xf;
        let mut msb = (result >> 4) & 0xf;

        if self.flags.carry || msb > 9 {
            msb += 6;
        }

        let result = (msb << 4) | lsb;
        self.flags.set_all_but_aux_carry(result);

        self.a = result as u8;
    }

    fn dad(&mut self, operand: u16) {
        let result = (self.hl.both() as u32)
            .wrapping_add(operand as u32);

        self.flags.set_carry(result as u16);
        *self.hl.both_mut() = result as u16;
    }

    // register or memory to accumulator instructions
    
    fn add(&mut self, operand: u8) {
        let result = (self.a as u16)
            .wrapping_add(operand as u16);

        self.flags
            .set_all(result, (self.a & 0xf)
            .wrapping_add(operand & 0xf));
        self.a = result as u8;
    }

    fn adc(&mut self, operand: u8) {
        let result = (self.a as u16)
            .wrapping_add(operand as u16)
            .wrapping_add(self.flags.carry as u16);

        self.flags
            .set_all(result, (self.a & 0xf)
            .wrapping_add(operand & 0xf)
            .wrapping_add((self.flags.carry as u8) & 0xf));
        self.a = result as u8;
    }

    fn sub(&mut self, operand: u8) {
        let result = (self.a as u16)
            .wrapping_sub(operand as u16);

        self.flags.set_all(result, (self.a & 0xf)
            .wrapping_sub(operand & 0xf));
        self.a = result as u8;
    }

    fn sbb(&mut self, operand: u8) {
        let result = (self.a as u16)
            .wrapping_sub(operand as u16)
            .wrapping_sub(self.flags.carry as u16);

        self.flags.set_all(result, (self.a & 0xf)
                .wrapping_sub(operand & 0xf)
                .wrapping_sub((self.flags.carry as u8) & 0xf));
        self.a = result as u8;
    }
    
    fn ana(&mut self, operand: u8) {
        self.a &= operand;
        self.flags.set_all_but_aux_carry(self.a as u16);
        self.flags.carry = false;
    }

    fn xra(&mut self, operand: u8) {
        self.a ^= operand;
        self.flags.set_all(self.a as u16, self.a);
        self.flags.carry = false;
    }

    fn ora(&mut self, operand: u8) {
        self.a |= operand;
        self.flags.set_all_but_aux_carry(self.a as u16);
        self.flags.carry = false;
    }
    
    fn cmp(&mut self, operand: u8) {
        self.flags.set_all((self.a as u16).wrapping_sub(operand as u16), (self.a & 0xf).wrapping_sub(operand & 0xf));
    }
    
    pub fn emulate(&mut self, ) {
        let opcode = self.memory[self.pc as usize];

        let op_size = match opcode {
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
                *self.bc.msb_mut() = self.inr(self.bc.msb());
                1 
            },
            0x05 => {
                *self.bc.msb_mut() = self.dec(self.bc.msb());
                1
            }
            0x06 => {
                *self.bc.msb_mut() = self.read_next_instruction_byte();
                2
            },
            0x07 => {
                let prev_bit7: u8 = self.a & (1 << 7);
                self.a <<= 1;
                self.flags.carry = prev_bit7 != 0;
                1
            },
            _ => panic!("unimplemented instruction {}", opcode),
        };

        self.pc += op_size;
    }
}
