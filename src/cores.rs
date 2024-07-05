use std::cell::RefCell;
use std::rc::Rc;

use super::memory::MemoryMapped;

use bitmatch::bitmatch;
use bitvec::prelude::*;

pub trait InterruptHandler {
    fn service_pending(&mut self) -> Option<u16> {
        Option::None
    }

    fn reti(&mut self) {}
}

#[allow(dead_code)]
#[allow(clippy::upper_case_acronyms)]
pub enum CoreType {
    AVR,
    AVRe,
    AVReplus,
    AVRxm,
    AVRxt,
    AVRrc,
}

#[allow(dead_code)]
pub struct Core {
    variant: CoreType,
    regs: [u8; 32],
    sreg: u8,
    pc: u16,
    sp: u16,
    pub ds: Rc<RefCell<dyn MemoryMapped>>,
    pub progmem: Rc<RefCell<dyn MemoryMapped>>,
    busy: u8,
    interrupt_handler: Rc<RefCell<dyn InterruptHandler>>,
    interupt_inhibit: bool,
    debug: bool,
}

impl Core {
    pub fn new(
        variant: CoreType,
        ds: Rc<RefCell<dyn MemoryMapped>>,
        progmem: Rc<RefCell<dyn MemoryMapped>>,
        interrupt_handler: Rc<RefCell<dyn InterruptHandler>>,
        sp: u16,
    ) -> Self {
        Self {
            variant,
            regs: [0; 32],
            sreg: 0,
            pc: 0,
            sp,
            ds,
            progmem,
            interrupt_handler,
            interupt_inhibit: false,
            busy: 0,
            debug: false,
        }
    }

    pub fn debug(&mut self, on: bool) {
        self.debug = on;
    }

    pub fn get_register(&self, register: u8) -> u8 {
        self.regs[usize::from(register)]
    }

    pub fn set_register(&mut self, register: u8, value: u8) {
        self.regs[usize::from(register)] = value;
    }

    fn get_register_word(&self, register_low: u8) -> u16 {
        let register_low = usize::from(register_low);
        u16::from_le_bytes(
            self.regs[register_low..=(register_low + 1)]
                .try_into()
                .expect("Incorrect word index into working registers."),
        )
    }

    fn set_register_word(&mut self, register_low: u8, value: u16) {
        let register_low = usize::from(register_low);
        let bytes = value.to_le_bytes();
        self.regs[register_low] = bytes[0];
        self.regs[register_low + 1] = bytes[1];
    }

    fn get_io_register(&self, register: u8) -> u8 {
        match register {
            0x3F => self.sreg,              // CPU.SREG
            0x3E => (self.sp >> 8) as u8,   // CPU.SPH
            0x3D => (self.sp & 0xFF) as u8, // CPU.SPL
            _ => self.ds.borrow_mut().read(usize::from(register)).0,
        }
    }

    fn set_io_register(&mut self, register: u8, value: u8) {
        match register {
            0x3F => self.sreg = value,                                    // CPU.SREG
            0x3E => self.sp = (self.sp & 0x00FF) | ((value as u16) << 8), // CPU.SPH
            0x3D => self.sp = (self.sp & 0xFF00) | (value as u16),        // CPU.SPL
            _ => {
                self.ds.borrow_mut().write(usize::from(register), value);
            }
        }
    }

    fn get_data_space(&self, address: u32) -> u8 {
        match address {
            0x0000003F => self.sreg,              // CPU.SREG
            0x0000003E => (self.sp >> 8) as u8,   // CPU.SPH
            0x0000003D => (self.sp & 0xFF) as u8, // CPU.SPL
            _ => {
                self.ds
                    .borrow_mut()
                    .read(usize::try_from(address).unwrap())
                    .0
            }
        }
    }

    fn set_data_space(&mut self, address: u32, value: u8) {
        match address {
            0x0000003F => self.sreg = value, // CPU.SREG
            0x0000003E => self.sp = (self.sp & 0x00FF) | ((value as u16) << 8), // CPU.SPH
            0x0000003D => self.sp = (self.sp & 0xFF00) | (value as u16), // CPU.SPL
            _ => {
                self.ds
                    .borrow_mut()
                    .write(usize::try_from(address).unwrap(), value);
            }
        }
    }

    fn get_ps(&self, address: u32) -> u8 {
        self.progmem
            .borrow_mut()
            .read(usize::try_from(address).unwrap())
            .0
    }

    fn get_progmem(&self, pc: u32) -> u16 {
        self.progmem
            .borrow_mut()
            .read_word(usize::try_from(pc << 1).unwrap())
            .0
    }

    fn get_sreg_bit(&self, bit: BitSREG) -> bool {
        (self.sreg & (1 << bit as u8)) != 0
    }

    fn set_sreg_bit(&mut self, bit: BitSREG, set: bool) {
        if set {
            self.sreg |= 1 << bit as u8;
        } else {
            self.sreg &= !(1 << bit as u8);
        }
    }

    pub fn get_stack_pointer(&self) -> u16 {
        self.sp
    }

    // ARITHMETIC INSTRUCTIONS
    #[allow(non_snake_case)]
    fn adc(&mut self, d: u8, r: u8) {
        // Rd <- Rd + Rr + C
        let Rd = self.get_register(d);
        let Rr = self.get_register(r);
        let mut C = self.get_sreg_bit(BitSREG::C);
        let mut R: u8;

        let C1;
        let mut C2 = false;

        (R, C1) = Rd.overflowing_add(Rr);
        if C {
            (R, C2) = R.overflowing_add(1);
        }
        C = C1 | C2;

        self.set_register(d, R);

        let bRd = Rd.view_bits::<Lsb0>();
        let bRr = Rr.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        let Z = R == 0;
        let N = bR[7];
        let V = bRd[7] & bRr[7] & !bR[7] | !bRd[7] & !bRr[7] & bR[7];
        let S = N ^ V;
        let H = bRd[3] & bRr[3] | bRr[3] & !bR[3] | !bR[3] & bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn add(&mut self, d: u8, r: u8) {
        // Rd <- Rd + Rr
        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let (R, C) = Rd.overflowing_add(Rr);

        self.set_register(d, R);

        let bRd = Rd.view_bits::<Lsb0>();
        let bRr = Rr.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        let Z = R == 0;
        let N = bR[7];
        let V = bRd[7] & bRr[7] & !bR[7] | !bRd[7] & !bRr[7] & bR[7];
        let S = N ^ V;
        let H = bRd[3] & bRr[3] | bRr[3] & !bR[3] | !bR[3] & bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn adiw(&mut self, d: u8, K: u8) {
        // R[d+1]:Rd <- R[d+1]:Rd + K
        self.busy = 1;

        let Rd_hl = self.get_register_word(d);
        let (R_hl, C) = Rd_hl.overflowing_add(u16::from(K));

        self.set_register_word(d, R_hl);

        let bRd_hl = Rd_hl.view_bits::<Lsb0>();
        let bR_hl = R_hl.view_bits::<Lsb0>();

        let Z = R_hl == 0;
        let N = bR_hl[15];
        let V = !bRd_hl[7] & bR_hl[15];
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn and(&mut self, d: u8, r: u8) {
        // Rd <- Rd & Rr
        let Rd = self.get_register(d);
        let Rr = self.get_register(r);
        let R = Rd & Rr;

        self.set_register(d, R);

        let Z = R == 0;
        let N = R.view_bits::<Lsb0>()[7];
        let V = false;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn andi(&mut self, d: u8, K: u8) {
        // Rd <- Rd & K
        let Rd = self.get_register(d);
        let R = Rd & K;

        self.set_register(d, R);

        let Z = R == 0;
        let N = R.view_bits::<Lsb0>()[7];
        let V = false;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn com(&mut self, d: u8) {
        // Rd <- 0xFF - Rd
        let Rd = self.get_register(d);
        let R = !Rd;

        self.set_register(d, R);

        let C = true;
        let Z = R == 0;
        let N = R.view_bits::<Lsb0>()[7];
        let V = false;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn dec(&mut self, d: u8) {
        // Rd <- Rd - 1
        let Rd = self.get_register(d);
        let R = Rd.overflowing_sub(1).0;

        self.set_register(d, R);

        let Z = R == 0;
        let N = R.view_bits::<Lsb0>()[7];
        let V = Rd == 0x80;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn eor(&mut self, d: u8, r: u8) {
        // Rd <- Rd ^ Rr
        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let R = Rd ^ Rr;

        self.set_register(d, R);

        let Z = R == 0;
        let N = R.view_bits::<Lsb0>()[7];
        let V = false;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn fmul(&mut self, d: u8, r: u8) {
        // R1:R0 (unsigned) <- Rd (unsigned) x Rr (unsigned) << 1
        self.busy = 1;

        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let mut word = u16::from(Rd) * u16::from(Rr);
        let C = word.view_bits::<Lsb0>()[15];
        word <<= 1;

        self.set_register_word(0, word);

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn fmuls(&mut self, d: u8, r: u8) {
        // R1:R0 (signed) <- Rd (signed) x Rr (signed) << 1
        self.busy = 1;

        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let mut word = i16::from(Rd as i8) * i16::from(Rr as i8);
        let C = (word as u16).view_bits::<Lsb0>()[15];
        word <<= 1;

        self.set_register_word(0, word as u16);

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn fmulsu(&mut self, d: u8, r: u8) {
        // R1:R0 (signed) <- Rd (signed) x Rr (unsigned) << 1
        self.busy = 1;

        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let mut word = i16::from(Rd as i8) * i16::from(Rr);
        let C = (word as u16).view_bits::<Lsb0>()[15];
        word <<= 1;

        self.set_register_word(0, word as u16);

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn inc(&mut self, d: u8) {
        // Rd <- Rd + 1
        let Rd = self.get_register(d);
        let R = Rd.overflowing_add(1).0;

        self.set_register(d, R);

        let Z = R == 0;
        let N = R.view_bits::<Lsb0>()[7];
        let V = Rd == 0x7F;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn mul(&mut self, d: u8, r: u8) {
        // R1:R0 (unsigned) <- Rd (unsigned) x Rr (unsigned)
        self.busy = 1;

        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let word = u16::from(Rd) * u16::from(Rr);
        let C = word.view_bits::<Lsb0>()[15];

        self.set_register_word(0, word);

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn muls(&mut self, d: u8, r: u8) {
        // R1:R0 (signed) <- Rd (signed) x Rr (signed)
        self.busy = 1;

        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let word = i16::from(Rd as i8) * i16::from(Rr as i8);
        let C = (word as u16).view_bits::<Lsb0>()[15];

        self.set_register_word(0, word as u16);

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn mulsu(&mut self, d: u8, r: u8) {
        // R1:R0 (signed) <- Rd (signed) x Rr (unsigned)
        self.busy = 1;

        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let word = i16::from(Rd as i8) * i16::from(Rr);
        let C = (word as u16).view_bits::<Lsb0>()[15];

        self.set_register_word(0, word as u16);

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn neg(&mut self, d: u8) {
        // Rd <- ~Rd + 1
        let Rd = self.get_register(d);
        let R = (!Rd).overflowing_add(1).0;

        self.set_register(d, R);

        let bRd = Rd.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        let C = R != 0;
        let Z = R == 0;
        let N = bR[7];
        let V = R == 0x80;
        let S = N ^ V;
        let H = bR[3] | bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn or(&mut self, d: u8, r: u8) {
        // Rd <- Rd | Rr
        let Rd = self.get_register(d);
        let Rr = self.get_register(r);
        let R = Rd | Rr;

        self.set_register(d, R);

        let bR = R.view_bits::<Lsb0>();

        let Z = R == 0;
        let N = bR[7];
        let V = false;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn ori(&mut self, d: u8, val: u8) {
        // Rd <- Rd | val
        let Rd = self.get_register(d);
        let R = Rd | val;

        self.set_register(d, R);

        let bR = R.view_bits::<Lsb0>();

        let Z = R == 0;
        let N = bR[7];
        let V = false;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn sbc(&mut self, d: u8, r: u8) {
        // Rd <- Rd - Rr - C
        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let mut C = self.get_sreg_bit(BitSREG::C);
        let mut R: u8;

        let C1;
        let mut C2 = false;

        (R, C1) = Rd.overflowing_sub(Rr);
        if C {
            (R, C2) = R.overflowing_sub(1);
        }
        C = C1 | C2;

        self.set_register(d, R);

        let bRd = Rd.view_bits::<Lsb0>();
        let bRr = Rr.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        let Z = self.get_sreg_bit(BitSREG::Z) & (R == 0);
        let N = bR[7];
        let V = bRd[7] & !bRr[7] & !bR[7] | !bRd[7] & bRr[7] & bR[7];
        let S = N ^ V;
        let H = !bRd[3] & bRr[3] | bRr[3] & bR[3] | bR[3] & !bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn sbci(&mut self, d: u8, val: u8) {
        // Rd <- Rd - val - C
        let Rd = self.get_register(d);
        let mut C = self.get_sreg_bit(BitSREG::C);
        let mut R: u8;

        let C1;
        let mut C2 = false;

        (R, C1) = Rd.overflowing_sub(val);
        if C {
            (R, C2) = R.overflowing_sub(1);
        }
        C = C1 | C2;

        self.set_register(d, R);

        let bRd = Rd.view_bits::<Lsb0>();
        let bK = val.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        let Z = self.get_sreg_bit(BitSREG::Z) & (R == 0);
        let N = bR[7];
        let V = bRd[7] & !bK[7] & !bR[7] | !bRd[7] & bK[7] & bR[7];
        let S = N ^ V;
        let H = !bRd[3] & bK[3] | bK[3] & bR[3] | bR[3] & !bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn sbiw(&mut self, d: u8, K: u8) {
        // R[d+1]:Rd <- R[d+1]:Rd - K
        self.busy = 1;

        let Rd_hl = self.get_register_word(d);

        let (R_hl, C) = Rd_hl.overflowing_sub(u16::from(K));

        self.set_register_word(d, R_hl);

        let bRd_hl = Rd_hl.view_bits::<Lsb0>();
        let bR_hl = R_hl.view_bits::<Lsb0>();

        let Z = R_hl == 0;
        let N = bR_hl[15];
        let V = !bR_hl[15] & bRd_hl[15];
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn sub(&mut self, d: u8, r: u8) {
        // Rd <- Rd - Rr
        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let (R, C) = Rd.overflowing_sub(Rr);

        self.set_register(d, R);

        let bRd = Rd.view_bits::<Lsb0>();
        let bRr = Rr.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        let Z = R == 0;
        let N = bR[7];
        let V = bRd[7] & !bRr[7] & !bR[7] | !bRd[7] & bRr[7] & bR[7];
        let S = N ^ V;
        let H = !bRd[3] & bRr[3] | bRr[3] & bR[3] | bR[3] & !bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn subi(&mut self, d: u8, K: u8) {
        // Rd <- Rd - val
        let Rd = self.get_register(d);

        let (R, C) = Rd.overflowing_sub(K);

        let bRd = Rd.view_bits::<Lsb0>();
        let bK = K.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        self.set_register(d, R);

        let Z = R == 0;
        let N = bR[7];
        let V = bRd[7] & !bK[7] & !bR[7] | !bRd[7] & bK[7] & bR[7];
        let S = N ^ V;
        let H = !bRd[3] & bK[3] | bK[3] & bR[3] | bR[3] & !bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    // FLOW CONTROL INSTRUCTIONS
    fn brbx(&mut self, s: u8, k: i8, set: bool) {
        if !set ^ self.get_sreg_bit(BitSREG::from(s)) {
            let mut pc = self.pc as i32;
            pc += i32::from(k);
            self.pc = pc as u16;
            self.busy = 1;
        }
    }

    fn call(&mut self, k: u32) {
        self.busy = 2; // AVRxt, 16-bit PC

        // PC + 1 because we already incremented the PC
        let mut ds = self.ds.borrow_mut();
        ds.write(usize::from(self.sp), (self.pc + 1) as u8);
        self.sp -= 1;
        ds.write(usize::from(self.sp), ((self.pc + 1) >> 8) as u8);
        self.sp -= 1;
        self.pc = k as u16;
    }

    #[allow(non_snake_case)]
    fn cp(&mut self, d: u8, r: u8) {
        // Rd - Rr
        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        let (R, C) = Rd.overflowing_sub(Rr);

        let bRd = Rd.view_bits::<Lsb0>();
        let bRr = Rr.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        let Z = R == 0;
        let N = bR[7];
        let V = bRd[7] & !bRr[7] & !bR[7] | !bRd[7] & bRr[7] & bR[7];
        let S = N ^ V;
        let H = !bRd[3] & bRr[3] | bRr[3] & bR[3] | bR[3] & !bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn cpc(&mut self, d: u8, r: u8) {
        // Rd - Rr - C
        let Rd = self.get_register(d);
        let Rr = self.get_register(r);
        let mut R: u8;

        let mut C = self.get_sreg_bit(BitSREG::C);
        let Z = self.get_sreg_bit(BitSREG::Z);

        let C1;
        let mut C2 = false;

        (R, C1) = Rd.overflowing_sub(Rr);
        if C {
            (R, C2) = R.overflowing_sub(1);
        }
        C = C1 | C2;

        let bRd = Rd.view_bits::<Lsb0>();
        let bRr = Rr.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        let Z = Z & (R == 0);
        let N = bR[7];
        let V = bRd[7] & !bRr[7] & !bR[7] | !bRd[7] & bRr[7] & bR[7];
        let S = N ^ V;
        let H = !bRd[3] & bRr[3] | bRr[3] & bR[3] | bR[3] & !bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn cpi(&mut self, d: u8, K: u8) {
        // Rd - K
        let Rd = self.get_register(d);

        let (R, C) = Rd.overflowing_sub(K);

        let bRd = Rd.view_bits::<Lsb0>();
        let bK = K.view_bits::<Lsb0>();
        let bR = R.view_bits::<Lsb0>();

        let Z = R == 0;
        let N = bR[7];
        let V = bRd[7] & !bK[7] & !bR[7] | !bRd[7] & bK[7] & bR[7];
        let S = N ^ V;
        let H = !bRd[3] & bK[3] | bK[3] & bR[3] | bR[3] & !bRd[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn cpse(&mut self, d: u8, r: u8) {
        use Instruction::*;

        let Rd = self.get_register(d);
        let Rr = self.get_register(r);

        if Rd == Rr {
            let opcode = self.get_progmem(self.pc as u32);
            let prefetch = self.get_progmem((self.pc + 1) as u32);
            let op = Instruction::decode(opcode, prefetch);
            match op {
                CALL { .. } | JMP { .. } | LDS { .. } | STS { .. } => {
                    self.pc += 2;
                    self.busy = 2
                }
                _ => {
                    self.pc += 1;
                    self.busy = 1
                }
            }
        }
    }

    fn icall(&mut self) {
        self.busy = 1; // AVRxt, 16-bit PC

        // PC + 0 because we already incremented the PC
        let mut ds = self.ds.borrow_mut();
        ds.write(usize::from(self.sp), (self.pc) as u8);
        self.sp -= 1;
        ds.write(usize::from(self.sp), ((self.pc) >> 8) as u8);
        self.sp -= 1;
        self.pc = self.get_register_word(30);
    }

    fn ijmp(&mut self) {
        self.busy = 1;

        self.pc = self.get_register_word(30);
    }

    fn jmp(&mut self, k: u32) {
        self.busy = 2;

        self.pc = k as u16;
    }

    fn rcall(&mut self, k: i16) {
        self.busy = 1; // AVRxt, 16-bit PC

        // PC + 0 because we already incremented the PC
        let mut ds = self.ds.borrow_mut();
        ds.write(usize::from(self.sp), (self.pc) as u8);
        self.sp -= 1;
        ds.write(usize::from(self.sp), ((self.pc) >> 8) as u8);
        self.sp -= 1;
        self.pc = self.pc.overflowing_add(k as u16).0;
    }

    fn ret(&mut self) {
        self.busy = 3; // 16-bit PC, not AVRrc

        let mut ds = self.ds.borrow_mut();
        self.sp += 1;
        let (bh, _) = ds.read(usize::from(self.sp));
        self.sp += 1;
        let (bl, _) = ds.read(usize::from(self.sp));
        self.pc = ((bh as u16) << 8) | (bl as u16);
    }

    fn reti(&mut self) {
        self.busy = 3; // 16-bit PC, not AVRrc

        let mut ds = self.ds.borrow_mut();
        self.sp += 1;
        let (bh, _) = ds.read(usize::from(self.sp));
        self.sp += 1;
        let (bl, _) = ds.read(usize::from(self.sp));
        self.pc = ((bh as u16) << 8) | (bl as u16);

        // Ack interrupt
        self.interrupt_handler.borrow_mut().reti();

        // I bit in SREG not set for AVRxt
        self.interupt_inhibit = true; // prevent immediate servicing of another interrupt
    }

    fn rjmp(&mut self, k: i16) {
        self.busy = 1;

        self.pc = self.pc.overflowing_add(k as u16).0;
    }

    #[allow(non_snake_case)]
    fn sbix(&mut self, A: u8, b: u8, set: bool) {
        use Instruction::*;

        let bitval_n = (self.get_io_register(A) & (1 << b)) == 0;

        if set ^ bitval_n {
            let opcode = self.get_progmem(self.pc as u32);
            let prefetch = self.get_progmem((self.pc + 1) as u32);
            let op = Instruction::decode(opcode, prefetch);
            match op {
                CALL { .. } | JMP { .. } | LDS { .. } | STS { .. } => {
                    self.pc += 2;
                    self.busy = 2
                }
                _ => {
                    self.pc += 1;
                    self.busy = 1
                }
            }
        }
    }

    #[allow(non_snake_case)]
    fn sbrx(&mut self, r: u8, b: u8, set: bool) {
        use Instruction::*;

        let bitval_n = (self.get_register(r) & (1 << b)) == 0;

        if set ^ bitval_n {
            let opcode = self.get_progmem(self.pc as u32);
            let prefetch = self.get_progmem((self.pc + 1) as u32);
            let op = Instruction::decode(opcode, prefetch);
            match op {
                CALL { .. } | JMP { .. } | LDS { .. } | STS { .. } => {
                    self.pc += 2;
                    self.busy = 2
                }
                _ => {
                    self.pc += 1;
                    self.busy = 1
                }
            }
        }
    }

    // BIT MANIPULATION INSTRUCTIONS
    #[allow(non_snake_case)]
    fn asr(&mut self, d: u8) {
        let Rd = self.get_register(d);
        let R = ((Rd as i8) >> 1) as u8;

        self.set_register(d, R);

        let C = Rd.view_bits::<Lsb0>()[0];
        let Z = R == 0;
        let N = R.view_bits::<Lsb0>()[7];
        let V = N ^ C;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    fn bclr(&mut self, b: u8) {
        self.set_sreg_bit(BitSREG::from(b), false);
    }

    #[allow(non_snake_case)]
    fn bld(&mut self, d: u8, b: u8) {
        if self.get_sreg_bit(BitSREG::T) {
            self.set_register(d, self.get_register(d) | (1 << b));
        } else {
            self.set_register(d, self.get_register(d) & !(1 << b));
        }
    }

    fn bset(&mut self, b: u8) {
        self.set_sreg_bit(BitSREG::from(b), true)
    }

    #[allow(non_snake_case)]
    fn bst(&mut self, d: u8, bit: u8) {
        self.set_sreg_bit(BitSREG::T, (self.get_register(d) & (1 << bit)) != 0);
    }

    #[allow(non_snake_case)]
    fn cbi(&mut self, A: u8, b: u8) {
        match A {
            0x3F => self.sreg &= !(1u8 << b),      // CPU.SREG
            0x3D => self.sp &= !(1u16 << b),       // CPU.SPL
            0x3E => self.sp &= !(1u16 << (b + 8)), // CPU.SPH
            _ => {
                self.ds.borrow_mut().set_bit(usize::from(A), b, false);
            }
        }
    }

    #[allow(non_snake_case)]
    fn lsl(&mut self, d: u8) {
        let Rd = self.get_register(d);
        let R = Rd << 1;

        self.set_register(d, R);

        let C = Rd.view_bits::<Lsb0>()[7];
        let Z = R == 0;
        let N = R.view_bits::<Lsb0>()[7];
        let V = N ^ C;
        let S = N ^ V;
        let H = Rd.view_bits::<Lsb0>()[3];

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn lsr(&mut self, d: u8) {
        let Rd = self.get_register(d);
        let R = Rd >> 1;

        self.set_register(d, R);

        let C = Rd.view_bits::<Lsb0>()[0];
        let Z = R == 0;
        let N = false;
        let V = N ^ C;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn ror(&mut self, d: u8) {
        let Rd = self.get_register(d);
        let C = self.get_sreg_bit(BitSREG::C);

        let mut R = Rd >> 1;
        if C {
            R |= 0x80;
        }

        self.set_register(d, R);

        let C = Rd.view_bits::<Lsb0>()[0];
        let Z = R == 0;
        let N = R.view_bits::<Lsb0>()[7];
        let V = N ^ C;
        let S = N ^ V;

        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn sbi(&mut self, A: u8, b: u8) {
        match A {
            0x3F => self.sreg |= 1u8 << b,      // CPU.SREG
            0x3D => self.sp |= 1u16 << b,       // CPU.SPL
            0x3E => self.sp |= 1u16 << (b + 8), // CPU.SPH
            _ => {
                self.ds.borrow_mut().set_bit(usize::from(A), b, true);
            }
        }
    }

    #[allow(non_snake_case)]
    fn swap(&mut self, d: u8) {
        self.set_register(d, self.get_register(d).rotate_left(4));
    }

    #[allow(non_snake_case)]
    fn in_(&mut self, d: u8, A: u8) {
        self.set_register(d, self.get_io_register(A));
    }

    #[allow(non_snake_case)]
    fn ld(&mut self, d: u8, pointer: PointerRegister, q: u8, dec: bool, inc: bool) {
        self.busy = 2; // AVRxt, SRAM only

        let mut address = self.get_register_word(pointer as u8);

        if dec {
            address = address.wrapping_sub(1);
            self.set_register_word(pointer as u8, address)
        }

        if (address as u32) + (q as u32) < 64 {
            self.set_register(
                d,
                self.get_io_register(((address as u32) + (q as u32)) as u8),
            );
        } else {
            self.set_register(d, self.get_data_space((address as u32) + (q as u32)));
        }

        if inc {
            address = address.wrapping_add(1);
            self.set_register_word(pointer as u8, address)
        }
    }

    #[allow(non_snake_case)]
    fn st(&mut self, r: u8, pointer: PointerRegister, q: u8, dec: bool, inc: bool) {
        self.busy = 1;

        let mut address = self.get_register_word(pointer as u8);

        if dec {
            address = address.wrapping_sub(1);
            self.set_register_word(pointer as u8, address)
        }

        if (address as u32) + (q as u32) < 64 {
            self.set_io_register(((address as u32) + (q as u32)) as u8, self.get_register(r));
        } else {
            self.set_data_space((address as u32) + (q as u32), self.get_register(r));
        }

        if inc {
            address = address.wrapping_add(1);
            self.set_register_word(pointer as u8, address)
        }
    }

    #[allow(non_snake_case)]
    fn ldi(&mut self, d: u8, K: u8) {
        self.set_register(d, K);
    }

    #[allow(non_snake_case)]
    fn lds(&mut self, d: u8, k: u16) {
        self.busy = 2; // AVRxt, SRAM only

        self.set_register(d, self.get_data_space(k as u32));
    }

    #[allow(non_snake_case)]
    fn lpm(&mut self, d: u8, inc: bool) {
        self.busy = 2;

        let mut address = self.get_register_word(PointerRegister::Z as u8);

        self.set_register(d, self.get_ps(address as u32));

        if inc {
            address = address.wrapping_add(1);
            self.set_register_word(PointerRegister::Z as u8, address)
        }
    }

    #[allow(non_snake_case)]
    fn mov(&mut self, d: u8, r: u8) {
        self.set_register(d, self.get_register(r))
    }

    #[allow(non_snake_case)]
    fn movw(&mut self, d: u8, r: u8) {
        self.set_register_word(d, self.get_register_word(r))
    }

    #[allow(non_snake_case)]
    fn out(&mut self, A: u8, r: u8) {
        self.set_io_register(A, self.get_register(r));
    }

    #[allow(non_snake_case)]
    fn pop(&mut self, d: u8) {
        self.busy = 1; // 2 for AVRrc

        self.sp = self.sp.overflowing_add(1).0;
        self.set_register(d, self.get_data_space(self.sp as u32));
    }

    #[allow(non_snake_case)]
    fn push(&mut self, r: u8) {
        self.set_data_space(self.sp as u32, self.get_register(r));
        self.sp = self.sp.overflowing_sub(1).0;
    }

    #[allow(non_snake_case)]
    fn sts(&mut self, r: u8, k: u16) {
        self.busy = 1; // SRAM only

        self.set_data_space(k as u32, self.get_register(r));
    }

    pub fn tick(&mut self) -> bool {
        use Instruction::*;

        // Wait for multi-cycle instructions to complete
        if self.busy > 0 {
            self.busy -= 1;
            if self.debug {
                println!("[0x{:04X}] ...", self.pc << 1);
            }
            return true;
        }

        // HANDLE INTERRUPTS
        if self.interupt_inhibit {
            // After reti, core will always execute one instruction before another interrupt
            self.interupt_inhibit = false;
        } else {
            // Interrupts enabled
            if self.get_sreg_bit(BitSREG::I) {
                let vector = self.interrupt_handler.borrow_mut().service_pending();
                if let Some(address) = vector {
                    let mut ds = self.ds.borrow_mut();
                    ds.write(usize::from(self.sp), (self.pc) as u8);
                    self.sp -= 1;
                    ds.write(usize::from(self.sp), ((self.pc) >> 8) as u8);
                    self.sp -= 1;
                    self.pc = address;
                    self.busy = 4; // 2 cycles to to push PC + 3 cycles for jmp to vector
                    return true;
                }
            }
        }

        let opcode = self.get_progmem(self.pc as u32);
        let prefetch = self.get_progmem((self.pc as u32) + 1);
        let op = Instruction::decode(opcode, prefetch);

        if self.debug {
            println!("[0x{:04X}] {:?}", self.pc << 1, op);
        }

        // Most instructions are single cycle so do this first
        // Terminate if PC overflows to prevent program from restarting
        if let Some(result) = self.pc.checked_add(1) {
            self.pc = result;
        } else {
            return false;
        }

        #[allow(non_snake_case)]
        match op {
            // Arithmetic and Logic Instructions
            ADC { d, r } => self.adc(d, r),
            ADD { d, r } => self.add(d, r),
            ADIW { d, K } => self.adiw(d, K),
            AND { d, r } => self.and(d, r),
            ANDI { d, K } => self.andi(d, K),
            COM { d } => self.com(d),
            DEC { d } => self.dec(d),
            EOR { d, r } => self.eor(d, r),
            FMUL { d, r } => self.fmul(d, r),
            FMULS { d, r } => self.fmuls(d, r),
            FMULSU { d, r } => self.fmulsu(d, r),
            INC { d } => self.inc(d),
            MUL { d, r } => self.mul(d, r),
            MULS { d, r } => self.muls(d, r),
            MULSU { d, r } => self.mulsu(d, r),
            NEG { d } => self.neg(d),
            OR { d, r } => self.or(d, r),
            ORI { d, K } => self.ori(d, K),
            SBC { d, r } => self.sbc(d, r),
            SBCI { d, K } => self.sbci(d, K),
            SBIW { d, K } => self.sbiw(d, K),
            SUB { d, r } => self.sub(d, r),
            SUBI { d, K } => self.subi(d, K),
            // Change of Flow Instructions
            BRBC { k, s } => self.brbx(s, k, false),
            BRBS { k, s } => self.brbx(s, k, true),
            CALL { k } => self.call(k),
            CP { d, r } => self.cp(d, r),
            CPC { d, r } => self.cpc(d, r),
            CPI { d, K } => self.cpi(d, K),
            CPSE { d, r } => self.cpse(d, r),
            ICALL => self.icall(),
            IJMP => self.ijmp(),
            JMP { k } => self.jmp(k),
            RCALL { k } => self.rcall(k),
            RET => self.ret(),
            RETI => self.reti(),
            RJMP { k } => self.rjmp(k),
            SBIC { A, b } => self.sbix(A, b, false),
            SBIS { A, b } => self.sbix(A, b, true),
            SBRC { r, b } => self.sbrx(r, b, false),
            SBRS { r, b } => self.sbrx(r, b, true),
            // Data Transfer Instructions
            IN { d, A } => self.in_(d, A),
            LDX { d } => self.ld(d, PointerRegister::X, 0, false, false),
            LDXinc { d } => self.ld(d, PointerRegister::X, 0, false, true),
            LDXdec { d } => self.ld(d, PointerRegister::X, 0, true, false),
            LDYinc { d } => self.ld(d, PointerRegister::Y, 0, false, true),
            LDYdec { d } => self.ld(d, PointerRegister::Y, 0, true, false),
            LDZinc { d } => self.ld(d, PointerRegister::Z, 0, false, true),
            LDZdec { d } => self.ld(d, PointerRegister::Z, 0, true, false),
            LDDY { d, q } => self.ld(d, PointerRegister::Y, q, false, false),
            LDDZ { d, q } => self.ld(d, PointerRegister::Z, q, false, false),
            LDI { d, K } => self.ldi(d, K),
            LDS { d, k } => {
                self.lds(d, k);
                self.pc += 1
            }
            LPM => self.lpm(0, false),
            LPMZ { d } => self.lpm(d, false),
            LPMZinc { d } => self.lpm(d, true),
            MOV { d, r } => self.mov(d, r),
            MOVW { d, r } => self.movw(d, r),
            OUT { A, r } => self.out(A, r),
            POP { d } => self.pop(d),
            PUSH { d } => self.push(d),
            STX { r } => self.st(r, PointerRegister::X, 0, false, false),
            STXdec { r } => self.st(r, PointerRegister::X, 0, true, false),
            STXinc { r } => self.st(r, PointerRegister::X, 0, false, true),
            STYdec { r } => self.st(r, PointerRegister::Y, 0, true, false),
            STYinc { r } => self.st(r, PointerRegister::Y, 0, false, true),
            STZdec { r } => self.st(r, PointerRegister::Z, 0, true, false),
            STZinc { r } => self.st(r, PointerRegister::Z, 0, false, true),
            STDY { r, q } => self.st(r, PointerRegister::Y, q, false, false),
            STDZ { r, q } => self.st(r, PointerRegister::Z, q, false, false),
            STS { r, k: address } => {
                self.sts(r, address);
                self.pc += 1
            }
            // Bit and Bit-Test Instructions
            ASR { d } => self.asr(d),
            BCLR { s } => self.bclr(s),
            BLD { d, b } => self.bld(d, b),
            BSET { s } => self.bset(s),
            BST { d, b } => self.bst(d, b),
            CBI { A, b } => self.cbi(A, b),
            LSL { d } => self.lsl(d),
            LSR { d } => self.lsr(d),
            ROR { d } => self.ror(d),
            SBI { A, b } => self.sbi(A, b),
            SWAP { d } => self.swap(d),
            // MCU Control Instructions
            BREAK => {
                println!("[END] BREAK instruction encountered.");
                return false;
            }
            NOP => {}
            SLEEP => println!("[SLEEP]"), // Not implemented
            WDR => println!("[WDR]"),     // Not implemented
            // Undefined
            UNDEF => {
                println!("[ERROR] Undefined opcode: {:b}", opcode)
            }
        }

        true
    }
}

#[allow(non_snake_case)]
#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
// Equivalent instructions omitted
// d: destination register
// r: source register
// K: constant data
// k: constant address
// b: bit position in register file or I/O register
// s: bit position in SREG
// A: I/O memory address
// q: displacement for direct addressing
enum Instruction {
    ADC { d: u8, r: u8 },
    ADD { d: u8, r: u8 },
    ADIW { d: u8, K: u8 },
    AND { d: u8, r: u8 },
    ANDI { d: u8, K: u8 },
    ASR { d: u8 },
    BCLR { s: u8 },
    BLD { d: u8, b: u8 },
    BRBC { s: u8, k: i8 },
    BRBS { s: u8, k: i8 },
    BREAK,
    BSET { s: u8 },
    BST { d: u8, b: u8 },
    CALL { k: u32 },
    CBI { A: u8, b: u8 },
    COM { d: u8 },
    CP { d: u8, r: u8 },
    CPC { d: u8, r: u8 },
    CPI { d: u8, K: u8 },
    CPSE { d: u8, r: u8 },
    DEC { d: u8 },
    EOR { d: u8, r: u8 },
    FMUL { d: u8, r: u8 },
    FMULS { d: u8, r: u8 },
    FMULSU { d: u8, r: u8 },
    ICALL,
    IJMP,
    IN { d: u8, A: u8 },
    INC { d: u8 },
    JMP { k: u32 },
    LDX { d: u8 },
    LDXinc { d: u8 },
    LDXdec { d: u8 },
    LDYinc { d: u8 },
    LDYdec { d: u8 },
    LDZinc { d: u8 },
    LDZdec { d: u8 },
    LDDY { d: u8, q: u8 },
    LDDZ { d: u8, q: u8 },
    LDI { d: u8, K: u8 },
    LDS { d: u8, k: u16 },
    LPM,
    LPMZ { d: u8 },
    LPMZinc { d: u8 },
    LSL { d: u8 },
    LSR { d: u8 },
    MOV { d: u8, r: u8 },
    MOVW { d: u8, r: u8 },
    MUL { d: u8, r: u8 },
    MULS { d: u8, r: u8 },
    MULSU { d: u8, r: u8 },
    NEG { d: u8 },
    NOP,
    OR { d: u8, r: u8 },
    ORI { d: u8, K: u8 },
    OUT { A: u8, r: u8 },
    POP { d: u8 },
    PUSH { d: u8 },
    RCALL { k: i16 },
    RET,
    RETI,
    RJMP { k: i16 },
    ROR { d: u8 },
    SBC { d: u8, r: u8 },
    SBCI { d: u8, K: u8 },
    SBI { A: u8, b: u8 },
    SBIC { A: u8, b: u8 },
    SBIS { A: u8, b: u8 },
    SBIW { d: u8, K: u8 },
    SBRC { r: u8, b: u8 },
    SBRS { r: u8, b: u8 },
    SLEEP,
    STX { r: u8 },
    STXdec { r: u8 },
    STXinc { r: u8 },
    STYdec { r: u8 },
    STYinc { r: u8 },
    STZdec { r: u8 },
    STZinc { r: u8 },
    STDY { q: u8, r: u8 },
    STDZ { q: u8, r: u8 },
    STS { k: u16, r: u8 },
    SUB { d: u8, r: u8 },
    SUBI { d: u8, K: u8 },
    SWAP { d: u8 },
    WDR,
    UNDEF,
}

enum BitSREG {
    C = 0,
    Z = 1,
    N = 2,
    V = 3,
    S = 4,
    H = 5,
    T = 6,
    I = 7,
}

impl BitSREG {
    fn from(bit: u8) -> BitSREG {
        match bit {
            0 => BitSREG::C,
            1 => BitSREG::Z,
            2 => BitSREG::N,
            3 => BitSREG::V,
            4 => BitSREG::S,
            5 => BitSREG::H,
            6 => BitSREG::T,
            7 => BitSREG::I,
            _ => panic!("Invalid SREG bit."),
        }
    }
}

#[derive(Copy, Clone)]
enum PointerRegister {
    X = 26,
    Y = 28,
    Z = 30,
}

impl Instruction {
    #[bitmatch]
    #[allow(non_snake_case)]
    fn decode(opcode: u16, prefetch: u16) -> Instruction {
        #[bitmatch]
        match opcode {
            "0000_0000_0000_0000" => Instruction::NOP,
            "0001_11rd_dddd_rrrr" => Instruction::ADC {
                d: d as u8,
                r: r as u8,
            },
            "0000_11rd_dddd_rrrr" => Instruction::ADD {
                d: d as u8,
                r: r as u8,
            },
            "1001_0110_KKdd_KKKK" => Instruction::ADIW {
                d: ((d as u8) << 1) + 24,
                K: K as u8,
            },
            "0010_00rd_dddd_rrrr" => Instruction::AND {
                d: d as u8,
                r: r as u8,
            },
            "0111_KKKK_dddd_KKKK" => Instruction::ANDI {
                d: (d as u8) + 16,
                K: K as u8,
            },
            "1001_010d_dddd_0101" => Instruction::ASR { d: d as u8 },
            "1001_0100_1sss_1000" => Instruction::BCLR { s: s as u8 },
            "1111_100d_dddd_0bbb" => Instruction::BLD {
                d: d as u8,
                b: b as u8,
            },
            "1111_01kk_kkkk_ksss" => Instruction::BRBC {
                k: {
                    let mut k = k as u8;
                    let bk = k.view_bits_mut::<Lsb0>();
                    bk.set(7, bk[6]);
                    k as i8
                },
                s: s as u8,
            },
            "1111_00kk_kkkk_ksss" => Instruction::BRBS {
                k: {
                    let mut k = k as u8;
                    let bk = k.view_bits_mut::<Lsb0>();
                    bk.set(7, bk[6]);
                    k as i8
                },
                s: s as u8,
            },
            "1001_0101_1001_1000" => Instruction::BREAK,
            "1001_0100_0sss_1000" => Instruction::BSET { s: s as u8 },
            "1111_101d_dddd_0bbb" => Instruction::BST {
                d: d as u8,
                b: b as u8,
            },
            "1001_010k_kkkk_111k" => Instruction::CALL {
                k: ((k as u32) << 16) | (prefetch as u32),
            },
            "1001_1000_AAAA_Abbb" => Instruction::CBI {
                A: A as u8,
                b: b as u8,
            },
            "1001_010d_dddd_0000" => Instruction::COM { d: d as u8 },
            "0001_01rd_dddd_rrrr" => Instruction::CP {
                d: d as u8,
                r: r as u8,
            },
            "0000_01rd_dddd_rrrr" => Instruction::CPC {
                d: d as u8,
                r: r as u8,
            },
            "0011_KKKK_dddd_KKKK" => Instruction::CPI {
                d: d as u8 + 16,
                K: K as u8,
            },
            "0001_00rd_dddd_rrrr" => Instruction::CPSE {
                d: d as u8,
                r: r as u8,
            },
            "1001_010d_dddd_1010" => Instruction::DEC { d: d as u8 },
            "0010_01rd_dddd_rrrr" => Instruction::EOR {
                d: d as u8,
                r: r as u8,
            },
            "0000_0011_0ddd_1rrr" => Instruction::FMUL {
                d: d as u8 + 16,
                r: r as u8 + 16,
            },
            "0000_0011_1ddd_0rrr" => Instruction::FMULS {
                d: d as u8 + 16,
                r: r as u8 + 16,
            },
            "0000_0011_1ddd_1rrr" => Instruction::FMULSU {
                d: d as u8 + 16,
                r: r as u8 + 16,
            },
            "1001_0101_0000_1001" => Instruction::ICALL,
            "1001_0100_0000_1001" => Instruction::IJMP,
            "1011_0AAd_dddd_AAAA" => Instruction::IN {
                d: d as u8,
                A: A as u8,
            },
            "1001_010d_dddd_0011" => Instruction::INC { d: d as u8 },
            "1001_010k_kkkk_110k" => Instruction::JMP {
                k: ((k as u32) << 16) | (prefetch as u32),
            },
            "1001_000d_dddd_1100" => Instruction::LDX { d: d as u8 },
            "1001_000d_dddd_1101" => Instruction::LDXinc { d: d as u8 },
            "1001_000d_dddd_1110" => Instruction::LDXdec { d: d as u8 },
            "1001_000d_dddd_1001" => Instruction::LDYinc { d: d as u8 },
            "1001_000d_dddd_1010" => Instruction::LDYdec { d: d as u8 },
            "1001_000d_dddd_0001" => Instruction::LDZinc { d: d as u8 },
            "1001_000d_dddd_0010" => Instruction::LDZdec { d: d as u8 },
            "10q0_qq0d_dddd_1qqq" => Instruction::LDDY {
                d: d as u8,
                q: q as u8,
            },
            "10q0_qq0d_dddd_0qqq" => Instruction::LDDZ {
                d: d as u8,
                q: q as u8,
            },
            "1110_KKKK_dddd_KKKK" => Instruction::LDI {
                d: d as u8 + 16,
                K: K as u8,
            },
            "1001_000d_dddd_0000" => Instruction::LDS {
                d: d as u8,
                k: prefetch,
            },
            "1001_0101_1100_1000" => Instruction::LPM,
            "1001_000d_dddd_0100" => Instruction::LPMZ { d: d as u8 },
            "1001_000d_dddd_0101" => Instruction::LPMZinc { d: d as u8 },
            "0000_11dd_dddd_dddd" => Instruction::LSL { d: d as u8 },
            "1001_010d_dddd_0110" => Instruction::LSR { d: d as u8 },
            "0010_11rd_dddd_rrrr" => Instruction::MOV {
                d: d as u8,
                r: r as u8,
            },
            "0000_0001_dddd_rrrr" => Instruction::MOVW {
                d: (d as u8) << 1,
                r: (r as u8) << 1,
            },
            "1001_11rd_dddd_rrrr" => Instruction::MUL {
                d: d as u8,
                r: r as u8,
            },
            "0000_0010_dddd_rrrr" => Instruction::MULS {
                d: d as u8 + 16,
                r: r as u8 + 16,
            },
            "0000_0011_0ddd_0rrr" => Instruction::MULSU {
                d: d as u8 + 16,
                r: r as u8 + 16,
            },
            "1001_010d_dddd_0001" => Instruction::NEG { d: d as u8 },
            "0010_10rd_dddd_rrrr" => Instruction::OR {
                d: d as u8,
                r: r as u8,
            },
            "0110_KKKK_dddd_KKKK" => Instruction::ORI {
                d: d as u8 + 16,
                K: K as u8,
            },
            "1011_1AAr_rrrr_AAAA" => Instruction::OUT {
                A: A as u8,
                r: r as u8,
            },
            "1001_000d_dddd_1111" => Instruction::POP { d: d as u8 },
            "1001_001d_dddd_1111" => Instruction::PUSH { d: d as u8 },
            "1101_kkkk_kkkk_kkkk" => Instruction::RCALL {
                k: {
                    let mut k = k;
                    let bk = k.view_bits_mut::<Lsb0>();
                    bk.set(12, bk[11]);
                    bk.set(13, bk[11]);
                    bk.set(14, bk[11]);
                    bk.set(15, bk[11]);
                    k as i16
                },
            },
            "1001_0101_0000_1000" => Instruction::RET,
            "1001_0101_0001_1000" => Instruction::RETI,
            "1100_kkkk_kkkk_kkkk" => Instruction::RJMP {
                k: {
                    let mut k = k;
                    let bk = k.view_bits_mut::<Lsb0>();
                    bk.set(12, bk[11]);
                    bk.set(13, bk[11]);
                    bk.set(14, bk[11]);
                    bk.set(15, bk[11]);
                    k as i16
                },
            },
            "1001_010d_dddd_0111" => Instruction::ROR { d: d as u8 },
            "0000_10rd_dddd_rrrr" => Instruction::SBC {
                d: d as u8,
                r: r as u8,
            },
            "0100_KKKK_dddd_KKKK" => Instruction::SBCI {
                d: d as u8 + 16,
                K: K as u8,
            },
            "1001_1010_AAAA_Abbb" => Instruction::SBI {
                A: A as u8,
                b: b as u8,
            },
            "1001_1001_AAAA_Abbb" => Instruction::SBIC {
                A: A as u8,
                b: b as u8,
            },
            "1001_1011_AAAA_Abbb" => Instruction::SBIS {
                A: A as u8,
                b: b as u8,
            },
            "1001_0111_KKdd_KKKK" => Instruction::SBIW {
                d: ((d as u8) << 1) + 24,
                K: K as u8,
            },
            "1111_110r_rrrr_0bbb" => Instruction::SBRC {
                r: r as u8,
                b: b as u8,
            },
            "1111_111r_rrrr_0bbb" => Instruction::SBRS {
                r: r as u8,
                b: b as u8,
            },
            "1001_0101_1000_1000" => Instruction::SLEEP,
            "1001_001r_rrrr_1100" => Instruction::STX { r: r as u8 },
            "1001_001r_rrrr_1110" => Instruction::STXdec { r: r as u8 },
            "1001_001r_rrrr_1101" => Instruction::STXinc { r: r as u8 },
            "1001_001r_rrrr_1010" => Instruction::STYdec { r: r as u8 },
            "1001_001r_rrrr_1001" => Instruction::STYinc { r: r as u8 },
            "1001_001r_rrrr_0010" => Instruction::STZdec { r: r as u8 },
            "1001_001r_rrrr_0001" => Instruction::STZinc { r: r as u8 },
            "10q0_qq1r_rrrr_1qqq" => Instruction::STDY {
                q: q as u8,
                r: r as u8,
            },
            "10q0_qq1r_rrrr_0qqq" => Instruction::STDZ {
                q: q as u8,
                r: r as u8,
            },
            "1001_001r_rrrr_0000" => Instruction::STS {
                k: prefetch,
                r: r as u8,
            },
            "0001_10rd_dddd_rrrr" => Instruction::SUB {
                d: d as u8,
                r: r as u8,
            },
            "0101_KKKK_dddd_KKKK" => Instruction::SUBI {
                d: d as u8 + 16,
                K: K as u8,
            },
            "1001_010d_dddd_0010" => Instruction::SWAP { d: d as u8 },
            "1001_0101_1010_1000" => Instruction::WDR,
            _ => {
                println!("[ERROR] Undefined opcode: {:b}", opcode);
                Instruction::UNDEF
            }
        }
    }
}
