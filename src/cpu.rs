

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
    pub fn set_with_psw(&mut self, value: u8) {
        self.sign = (value & 1 << 7) != 0;
        self.zero = (value & 1 << 6) != 0;
        self.aux_carry = (value & 1 << 4) != 0;
        self.parity = (value & 1 << 2) != 0;
        self.carry = (value & 1) != 0;
    }

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

    fn m(&self) -> u8 {
        self.memory[self.hl.both() as usize]
    }

    fn m_mut(&mut self) -> &mut u8 {
        &mut self.memory[self.hl.both() as usize]
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
    

    fn cmpa(&mut self, operand: u8) {
        self.flags.set_all((self.a as u16)
            .wrapping_sub(operand as u16), (self.a & 0xf)
            .wrapping_sub(operand & 0xf));
    }

    // register pair instructions

    fn push(&mut self, operand: u16) {
        self.sp -= 2;
        self.write_bytes(self.sp, operand);
    }

    fn pop(&mut self) -> u16 {
        self.sp += 2;
        self.read_bytes(self.sp - 2)
    }

    fn dad(&mut self, operand: u16) {
        let result = (self.hl.both() as u32)
            .wrapping_add(operand as u32);

        self.flags.set_carry(result as u16);
        *self.hl.both_mut() = result as u16;
    }

    fn xthl(&mut self) {
        let tmp = self.hl.both();

        *self.hl.both_mut() = self.pop();
        self.push(tmp);
    }

    // immediate iinstructions
       
    fn and(&mut self, operand: u8) {
        self.a &= operand;
        self.flags.set_all_but_aux_carry(self.a as u16);
        self.flags.carry = false;
    }

    fn xor(&mut self, operand: u8) {
        self.a ^= operand;
        self.flags.set_all(self.a as u16, self.a);
        self.flags.carry = false;
    }

    fn or(&mut self, operand: u8) {
        self.a |= operand;
        self.flags.set_all_but_aux_carry(self.a as u16);
        self.flags.carry = false;
    }

    // jump instructions
   
    fn jmp(&mut self, adr: u16) {
        self.pc = adr;
    }

    fn call(&mut self, adr: u16) {
        self.push(self.pc + 3);
        self.pc = adr;
    }

    fn ret(&mut self) {
        self.pc = self.pop();
    }
    

    pub fn emulate(&mut self) {
        let opcode = self.memory[self.pc as usize];

        let (op_size, cycles) = match opcode {
            // NOP
            0x00 | 0x20 => (1, 4),
            // LXB, D16
            0x01 => {
                *self.bc.both_mut() = self.read_next_instruction_bytes();
                (3, 10)
            },
            // STAX B
            0x02 => {
                self.write_byte(self.bc.both(), self.a);
                (1, 7)
            },
            // INX B
            0x03 => {
                *self.bc.both_mut() = self.bc.both().wrapping_add(1);
                (1, 5)
            },
            // INR B
            0x04 => {
                *self.bc.msb_mut() = self.inr(self.bc.msb());
                (1, 5) 
            },
            // DCR B
            0x05 => {
                *self.bc.msb_mut() = self.dec(self.bc.msb());
                (1, 5)
            }
            // MVI B, D8
            0x06 => {
                *self.bc.msb_mut() = self.read_next_instruction_byte();
                (2, 7)
            },
            // RLC
            0x07 => {
                let prev_bit7: u8 = self.a & (1 << 7);
                self.a <<= 1;
                self.flags.carry = prev_bit7 != 0;
                (1, 4)
            },
            // DAD B
            0x09 => {
                self.dad(self.bc.both());
                (1, 10)
            },
            // LDAX B
            0x0a => {
                self.a = self.read_byte(self.bc.both());
                (1, 7)
            },
            // DCX B
            0x0b => {
               *self.bc.both_mut() = self.bc.both().wrapping_sub(1);
               (1, 5)
            },
            // INR C
            0x0c => {
                *self.bc.msb_mut() = self.inr(self.bc.msb());
                (1 ,5)
            },
            // DCR C
            0x0d => {
                *self.bc.msb_mut() = self.dec(self.bc.msb());
                (1, 5)
            },
            // MVI C,D8
            0x0e => {
                *self.bc.lsb_mut() = self.read_next_instruction_byte();
                (2, 7)
            },
            // RRC
            0x0f => {
                let bit0: u8 = self.a & 1;
                self.a >>= 1;
                self.a |= bit0 << 7;
                self.flags.carry = bit0 != 0;
                (1 ,4)
            },
            // LXI D, D16
            0x11 => {
               *self.de.both_mut() = self.read_next_instruction_bytes();
               (3, 10)
            },
            // STAX D
            0x12 => {
                self.write_byte(self.de.both(), self.a);
                (1, 7)
            },
            // INX D
            0x13 => {
                *self.de.both_mut() = self.de.both().wrapping_add(1);
                (1, 5)
            },
            // INR D
            0x14 => {
               *self.de.msb_mut() = self.inr(self.de.msb());
               (1, 5)
            },
            // DCR D
            0x15 => {
                *self.de.msb_mut() = self.dec(self.de.msb());
                (1, 5)
            },

            // MVI D, D8
            0x16 => {
                *self.de.msb_mut() = self.read_next_instruction_byte();
                (2, 7)
            },

            // RAL
            0x17 => {
                let bit7: u8 = self.a & (1 << 7);
                self.a <<= 1;
                self.a |= self.flags.carry as u8;
                self.flags.carry = bit7 != 0;
                (1, 4)
            },
            // DAD D
            0x19 => {
                self.dad(self.de.both());
                (1, 10)
            },
            // LDAX D
            0x1a => {
               self.a =  self.read_byte(self.de.both());
               (1, 7)
            },
            // DCX D
            0x1b => {
                *self.de.both_mut() = self.de.both().wrapping_sub(1);
                (1, 5)
            },
            // INR E
            0x1c => {
                *self.de.lsb_mut() = self.inr(self.de.lsb());
                (1, 5)
            },
            // DCR E
            0x1d => {
                *self.de.lsb_mut() = self.dec(self.de.lsb());
                (1, 5)
            },
            // MVI E, D8
            0x1e => {
                *self.de.lsb_mut() = self.read_next_instruction_byte();
                (2, 7)
            },
            // RAR
            0x1f => {
                let bit0: u8 = self.a * 1;
                self.a >>= 1;
                self.a |= bit0 << 7;
                self.flags.carry = bit0 != 0;
                (1, 4)
            },
            // LXI H, D16
            0x21 => {
               *self.hl.both_mut() = self.read_next_instruction_bytes();
               (3, 10)
            },
            // SHLD adr
            0x22 => {
                self.write_bytes(self.read_next_instruction_bytes(), self.hl.both());
                (3, 16)
            },
            // INX H
            0x23 => {
                *self.hl.both_mut() = self.hl.both().wrapping_add(1);
                (1, 5)
            },
            // INR H
            0x24 => {
                *self.hl.msb_mut() = self.inr(self.hl.msb());
                (1, 5)
            },
            // DCR H
            0x25 => {
                *self.hl.msb_mut() = self.dec(self.hl.msb());
                (1, 5)
            },
            // MVI H, D8
            0x26 => {
                *self.hl.msb_mut() = self.read_next_instruction_byte();
                (2, 7)
            },
            // DAA
            0x27 => {
                self.daa();
                (1, 4)
            },
            // DAD H
            0x29 => {
                self.dad(self.hl.both());
                (3, 10)
            },
            // LHLD adr
            0x2a => {
                *self.hl.both_mut() = self.read_bytes(self.read_next_instruction_bytes());
                (3, 10)
            },
            // DCX H
            0x2b => {
                *self.hl.both_mut() = self.hl.both().wrapping_sub(1);
                (1, 5)
            },
            // INR L
            0x2c => {
                *self.hl.lsb_mut() = self.inr(self.hl.lsb());
                (1, 5)
            },
            // DCR L
            0x2d => {
                *self.hl.lsb_mut() = self.dec(self.hl.lsb());
                (1, 5)
            },
            0x2e => {
                *self.hl.lsb_mut() = self.read_next_instruction_byte();
                (2, 7)
            },
            // CMA
            0x2f => {
                self.a = !self.a;
                (1, 4)
            },
            // LXI SP, D16
            0x31 => {
                self.sp = self.read_next_instruction_bytes();
                (3, 10)
            },
            // STA adr
            0x32 => {
                self.write_byte(self.read_next_instruction_bytes(), self.a);
                (3, 13)
            },
            // INX SP
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
                (1, 5)
            },
            // INR M
            0x34 => {
                *self.m_mut() = self.inr(self.m());
                (1, 10)
            }
            // DCR M
            0x35 => {
                *self.m_mut() = self.dec(self.m());
                (1, 10)
            },
            // MVI M, D8
            0x36 => {
                *self.m_mut() = self.read_next_instruction_byte();
                (2, 10)
            },
            // STC
            0x37 => {
                self.flags.carry = true;
                (1, 4)
            },
            // DAD SP
            0x39 => {
                self.dad(self.sp);
                (3, 10)

            },
            // LDA adr
            0x3a => {
                self.a = self.read_byte(self.read_next_instruction_bytes());
                (3, 13)
            },
            0x3b => {
                self.sp = self.sp.wrapping_sub(1);
                (1, 5)
            },
            // INR A
            0x3c => {
                self.a = self.inr(self.a);
                (1, 5)
            },
            // DCR A
            0x3d => {
                self.a = self.dec(self.a);
                (1, 5)
            },
            // MVI A, D8
            0x3e => {
                self.a = self.read_next_instruction_byte();
                (2, 7)
            },
            // CMC
            0x3f => {
                self.flags.carry = !sel.flags.carry;
                (1, 4)
            },
            // MOV B, B
            0x40 => {
                *self.bc.msb_mut() = self.bc.msb();
                (1, 5)
            }
            // MOV B, C
            0x41 => {
                *self.bc.msb_mut() = self.bc.lsb();
                (1, 5)
            },
            // MOV B, D
            0x42 => {
                *self.bc.msb_mut() = self.de.msb();
                (1, 5)
            },
            // MOV B, E
            0x43 => {
                *self.bc.msb_mut() = self.de.lsb();
                (1, 5)
            },
            // MOV B, H
            0x44 => {
                *self.bc.msb_mut() = self.hl.msb();
                (1, 5)
            },
            // MOV B, L
            0x45 => {
                *self.bc.msb_mut() = self.hl.lsb();
                (1, 5)
            },
            // MOV B, M
            0x46 => {
                *self.bc.msb_mut() = self.m();
                (1, 5)
            },
            // MOV B, A
            0x47 => {
                *self.bc.msb_mut() = self.a;
                (1, 5)
            },
            // MOV C, B
            0x48 => {
                *self.bc.lsb_mut() = self.bc.msb();
                (1, 5)
            },
            // MOV C, C
            0x49 => {
                *self.bc.lsb_mut() = self.bc.lsb();
                (1, 5)
            },
            // MOV C, D
            0x4a => {
                *self.bc.lsb_mut() = self.de.msb();
                (1, 5)
            },
            // MOV C, E
            0x4b => {
                *self.bc.lsb_mut() = self.de.lsb();
                (1, 5)
            },
            // MOV C, H
            0x4c => {
                *self.bc.lsb_mut() = self.hl.msb();
                (1, 5)
            },
            // MOV C, L
            0x4d => {
                *self.bc.lsb_mut() = self.hl.lsb();
                (1, 5)
            },
            // MOV C, M
            0x4e => {
                *self.bc.lsb_mut() = self.m();
                (1, 5)
            },
            // MOV C, A
            0x4f => {
                *self.bc.lsb_mut() = self.a;
                (1, 5)
            },
            // MOV D, B
            0x50 => {
                *self.de.msb_mut() = self.bc.msb();
                (1, 5)
            },
            // MOV D, C
            0x51 => {
                *self.de.msb_mut() = self.bc.lsb();
                (1, 5)
            },
            // MOV D, D
            0x52 => {
                *self.de.msb_mut() = self.de.msb();
                (1, 5)
            },
            // MOV D, E
            0x53 => {
                *self.de.msb_mut() = self.de.lsb();
                (1, 5)
            },
            // MOV D, H
            0x54 => {
                *self.de.msb_mut() = self.hl.msb();
                (1, 5)
            },
            // MOV D, L
            0x55 => {
                *self.de.msb_mut() = self.hl.lsb();
                (1, 5)
            },
            // MOV D, M
            0x56 => {
                *self.de.msb_mut() = self.m();
                (1, 5)
            },
            // MOV D, A
            0x57 => {
                *self.de.msb_mut() = self.a;
                (1, 5)
            },
            // MOV E, B
            0x58 => {
                *self.de.lsb_mut() = self.bc.msb();
                (1, 5)
            },
            // MOV E, C
            0x59 => {
                *self.de.lsb_mut() = self.bc.lsb();
                (1, 5)
            },
            // MOV E, D
            0x5a => {
                *self.de.lsb_mut() = self.de.msb();
                (1, 5)
            },
            // MOV E, E
            0x5b => {
                *self.de.lsb_mut() = self.de.lsb();
                (1, 5)
            },
            // MOV E, H
            0x5c => {
                *self.de.lsb_mut() = self.hl.msb();
                (1, 5)
            },
            // MOV E, L
            0x5d => {
                *self.de.lsb_mut() = self.hl.lsb();
                (1, 5)
            },
            // MOV E, M
            0x5e => {
                *self.de.lsb_mut() = self.m();
                (1, 5)
            },
            // MOV E, A
            0x5f => {
                *self.de.lsb_mut() = self.a;
                (1, 5)
            },
            // MOV H. B
            0x60 => {
                *self.hl.msb_mut() = self.bc.msb();
                (1, 5)
            },
            // MOV H. C
            0x61 => {
                *self.hl.msb_mut() = self.bc.lsb();
                (1, 5)
            },
            // MOV H. D
            0x62 => {
                *self.hl.msb_mut() = self.de.msb();
                (1, 5)
            },
            // MOV H. E
            0x63 => {
                *self.hl.msb_mut() = self.de.lsb();
                (1, 5)
            },
            // MOV H. H
            0x64 => {
                *self.hl.msb_mut() = self.hl.msb();
                (1, 5)
            },
            // MOV H. L
            0x65 => {
                *self.hl.msb_mut() = self.hl.lsb();
                (1, 5)
            },
            // MOV H. M
            0x66 => {
                *self.hl.msb_mut() = self.m();
                (1, 5)
            },
            // MOV H. A
            0x67 => {
                *self.hl.msb_mut() = self.a;
                (1, 5)
            },
            // MOV L. B
            0x68 => {
                *self.hl.lsb_mut() = self.bc.msb();
                (1, 5)
            },
            // MOV L. C
            0x69 => {
                *self.hl.lsb_mut() = self.bc.lsb();
                (1, 5)
            },
            // MOV L. D
            0x6a => {
                *self.hl.lsb_mut() = self.de.msb();
                (1, 5)
            },
            // MOV L. E
            0x6b => {
                *self.hl.lsb_mut() = self.de.lsb();
                (1, 5)
            },
            // MOV L. H
            0x6c => {
                *self.hl.lsb_mut() = self.hl.msb();
                (1, 5)
            },
            // MOV L. L
            0x6d => {
                *self.hl.lsb_mut() = self.hl.lsb();
                (1, 5)
            },
            // MOV L. M
            0x6e => {
                *self.hl.lsb_mut() = self.m();
                (1, 5)
            },
            // MOV L. A
            0x6f => {
                *self.hl.lsb_mut() = self.a;
                (1, 5)
            },
            // MOV M. B
            0x70 => {
                *self.m_mut() = self.bc.msb();
                (1, 5)
            },
            // MOV M. C
            0x71 => {
                *self.m_mut() = self.bc.lsb();
                (1, 5)
            },
            // MOV M. D
            0x72 => {
                *self.m_mut() = self.de.msb();
                (1, 5)
            },
            // MOV M. E
            0x73 => {
                *self.m_mut() = self.de.lsb();
                (1, 5)
            },
            // MOV M. H
            0x74 => {
                *self.m_mut() = self.hl.msb();
                (1, 5)
            },
            // MOV M. L
            0x75 => {
                *self.m_mut() = self.hl.lsb();
                (1, 5)
            },
            // HLT
            0x76 => {
                println!("going into hlt");
                std::process::exit(0);
            },
            // MOV M. A
            0x77 => {
                *self.m_mut() = self.a;
                (1, 5)
            },
            // MOV A. B
            0x78 => {
                self.a = self.bc.msb();
                (1, 5)
            },
            // MOV A. C
            0x79 => {
                self.a = self.bc.lsb();
                (1, 5)
            },
            // MOV A. D
            0x7a => {
                self.a = self.de.msb();
                (1, 5)
            },
            // MOV A. E
            0x7b => {
                self.a = self.de.lsb();
                (1, 5)
            },
            // MOV A. H
            0x7c => {
                self.a = self.hl.msb();
                (1, 5)
            },
            // MOV A. L
            0x7d => {
                self.a = self.hl.lsb();
                (1, 5)
            },
            // MOV A. M
            0x7e => {
                self.a = self.m();
                (1, 5)
            },
            // MOV A. A
            0x7f => {
                self.a = self.a;
                (1, 5)
            },
            _ => panic!("unimplemented instruction {}", opcode),
        };

        self.pc += op_size;
    }
}
