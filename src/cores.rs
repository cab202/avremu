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
pub enum CoreType {
    AVR,
    AVRe,
    AVReplus,
    AVRxm, 
    AVRxt,
    AVRrc
}

#[allow(dead_code)]
pub struct Core {
    variant: CoreType,
    regs:  [u8; 32],
    sreg: u8,
    pc: u16,
    sp: u16,
    pub ds: Rc<RefCell<dyn MemoryMapped>>,
    pub progmem: Rc<RefCell<dyn MemoryMapped>>,
    busy: u8,
    interrupt_handler: Rc<RefCell<dyn InterruptHandler>>,
    debug: bool
}

impl Core {
    pub fn new(
        variant: CoreType, 
        ds: Rc<RefCell<dyn MemoryMapped>>, 
        progmem: Rc<RefCell<dyn MemoryMapped>>, 
        interrupt_handler: Rc<RefCell<dyn InterruptHandler>>, 
        sp_init: u16
    ) -> Self {
        Self {
            variant,
            regs: [0;32],
            sreg: 0,
            pc: 0,
            sp: sp_init,
            ds,
            progmem,
            interrupt_handler,
            busy: 0,
            debug: false
        }
    }

    pub fn debug(&mut self, on: bool) {
        self.debug = on;
    }

    pub fn get_r(&self, r: u8) -> u8 {
        self.regs[usize::from(r)]
    }
    
    pub fn set_r(&mut self, r: u8, val: u8) {
        self.regs[usize::from(r)] = val;
    }

    fn get_rw(&self, r: u8) -> u16 {
        let rl = usize::from(r);
        u16::from_le_bytes(self.regs[rl..=(rl+1)].try_into().expect("Incorrect word index into working registers."))
    }

    fn set_rw(&mut self, r: u8, val: u16) {
        let rl = usize::from(r);
        let bytes = val.to_le_bytes(); 
        self.regs[rl] = bytes[0];
        self.regs[rl+1] = bytes[1];
    }

    fn get_ior(&self, ioreg: u8) -> u8 {
        match ioreg {
            0x3F => {self.sreg},                //CPU.SREG
            0x3D => {
                //println!("[DEBUG] SP = {:04X}, ret = {:02X}", self.sp, (self.sp & 0xFF) as u8);
                (self.sp & 0xFF) as u8
            },   //CPU.SPL   //TODO: Implement correct 16-bit read
            0x3E => {
                //println!("[DEBUG] SP = {:04X}, ret = {:02X}", self.sp, (self.sp >> 8) as u8);
                (self.sp >> 8) as u8
            },     //CPU.SPH
            _ => self.ds.borrow_mut().read(usize::from(ioreg)).0
        }   
    }

    fn set_ior(&mut self, ioreg: u8, val: u8) {
        match ioreg {
            0x3F => {self.sreg = val},                                      //CPU.SREG
            0x3D => {self.sp = (self.sp & 0xFF00) | (val as u16)},          //CPU.SPL   //TODO: Implement correct 16-bit write
            0x3E => {self.sp = (self.sp & 0x00FF) | ((val as u16) << 8)},   //CPU.SPH
            _ => {self.ds.borrow_mut().write(usize::from(ioreg), val);}
        }
        //println!("[DEBUG] val = {:02X}, SP = {:04X}", val, self.sp);
    }

    fn get_ds(&self, address: u32) -> u8 {
        match address {
            0x0000003F => {self.sreg},                //CPU.SREG
            0x0000003D => {(self.sp & 0xFF) as u8},   //CPU.SPL   //TODO: Implement correct 16-bit read
            0x0000003E => {(self.sp >> 8) as u8},     //CPU.SPH
            _ => self.ds.borrow_mut().read(usize::try_from(address).unwrap()).0
        }
    }

    fn set_ds(&mut self, address: u32, val: u8) {
        match address {
            0x0000003F => {self.sreg = val},                                    //CPU.SREG
            0x0000003D => {self.sp = (self.sp & 0xFF00) | (val as u16)},        //CPU.SPL   //TODO: Implement correct 16-bit write
            0x0000003E => {self.sp = (self.sp & 0x00FF) | ((val as u16) << 8)}, //CPU.SPH
            _ => {self.ds.borrow_mut().write(usize::try_from(address).unwrap(), val);}
        }
        
    }

    fn get_ps(&self, address: u32) -> u8 {
        self.progmem.borrow_mut().read(usize::try_from(address).unwrap()).0
    }
    
    fn get_progmem(&self, pc: u32) -> u16 {
        self.progmem.borrow_mut().read_word(usize::try_from(pc<<1).unwrap()).0
    }

    #[allow(dead_code)]
    fn set_ps(&mut self, address: u32, val: u8) {
        self.progmem.borrow_mut().write(usize::try_from(address).unwrap(), val);
    }

    fn get_sreg_bit(&self, bit: BitSREG) -> bool {
        (self.sreg & (1<<bit as u8)) != 0
    }

    fn set_sreg_bit(&mut self, bit: BitSREG, set: bool) {
        if set {
            self.sreg |= 1 << bit as u8;
        } else {
            self.sreg &= !(1<<bit as u8);
        }
    }

    pub fn get_sp(&self) -> u16 {
        self.sp
    }

    // ARITHMETIC INSTRUCTIONS
    #[allow(non_snake_case)]
    fn adc(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd + Rr + C
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 
        let mut C = self.get_sreg_bit(BitSREG::C);
        let mut r: u8;

        let C1;
        let mut C2 = false;

        (r, C1) = rd.overflowing_add(rr);
        if C {
            (r, C2) = r.overflowing_add(1);
        }
        C = C1 | C2;

        self.set_r(Rd, r);

        let brd = rd.view_bits::<Lsb0>();
        let brr = rr.view_bits::<Lsb0>();
        let br = r.view_bits::<Lsb0>();

        let Z = r == 0;
        let N = br[7];
        let V = (brd[7] & brr[7] & !br[7]) | (!brd[7] & !brr[7] & br[7]);
        let S = N ^ V;
        let H = (brd[3] & brr[3]) | (brr[3] & !br[3]) | (!br[3] | brd[3]);
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn add(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd + Rr
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let (r, C) = rd.overflowing_add(rr);

        self.set_r(Rd, r);

        let brd = rd.view_bits::<Lsb0>();
        let brr = rr.view_bits::<Lsb0>();
        let br = r.view_bits::<Lsb0>();

        let Z = r == 0;
        let N = br[7];
        let V = (brd[7] & brr[7] & !br[7]) | (!brd[7] & !brr[7] & br[7]);
        let S = N ^ V;
        let H = (brd[3] & brr[3]) | (brr[3] & !br[3]) | (!br[3] | brd[3]);
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn adiw(&mut self, Rd: u8, val: u8) {
        // R[d+1]:Rd <- R[d+1]:Rd + val
        self.busy = 1;

        let rdhl = self.get_rw(Rd);
        let (rhl, C) = rdhl.overflowing_add(u16::from(val));
        
        self.set_rw(Rd, rhl);

        let brdhl = rdhl.view_bits::<Lsb0>();
        let brhl = rhl.view_bits::<Lsb0>();

        let Z = rhl == 0;
        let N = brhl[15];
        let V = !brdhl[15] & brhl[15];
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn and(&mut self, Rd: u8, Rr: u8) {
        let rd = self.get_r(Rd);
        let rr = self.get_r(Rr);
        let r = rd & rr;

        self.set_r(Rd, r);

        let Z = r == 0;
        let N = r.view_bits::<Lsb0>()[7];
        let V = false;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn andi(&mut self, Rd: u8, val: u8) {
        let rd = self.get_r(Rd);
        let r = rd & val;

        self.set_r(Rd, r);

        let Z = r == 0;
        let N = r.view_bits::<Lsb0>()[7];
        let V = false;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn com(&mut self, Rd: u8) {
        let rd = self.get_r(Rd);
        let r = !rd;

        self.set_r(Rd, r);

        let C = true;
        let Z = r == 0;
        let N = r.view_bits::<Lsb0>()[7];
        let V = false;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn dec(&mut self, Rd: u8) {
        // Rd <= Rd - 1; (Carry bit unchanged)
        let rd = self.get_r(Rd);
        let (r,_) = rd.overflowing_add(0xFF);    // 0xFF is -1 twos complement
        self.set_r(Rd, r);

        let Z = r == 0;
        let N = r.view_bits::<Lsb0>()[7];
        let V = rd == 0x80;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn eor(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd xor Rr
        let rd = self.get_r(Rd);
        let rr = self.get_r(Rr); 

        let r = rd ^ rr;

        self.set_r(Rd, r);

        let Z = r == 0;
        let N = r.view_bits::<Lsb0>()[7];
        let V = false;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn fmul(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (unsigned) <= Rd (unsigned) x Rr (unsigned) << 1
        self.busy = 1;
        
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let mut word = u16::from(rd)*u16::from(rr);
        let C = word.view_bits::<Lsb0>()[15];
        word <<= 1;

        self.set_rw(0, word);
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn fmuls(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (signed) <= Rd (signed) x Rr (signed) << 1
        self.busy = 1;

        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let mut word = i16::from(rd as i8)*i16::from(rr as i8);
        let C = (word as u16).view_bits::<Lsb0>()[15];
        word <<= 1;

        self.set_rw(0, word as u16);
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn fmulsu(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (signed) <= Rd (signed) x Rr (unsigned) << 1
        self.busy = 1;

        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let mut word = i16::from(rd as i8)*i16::from(rr);
        let C = (word as u16).view_bits::<Lsb0>()[15];
        word <<= 1;

        self.set_rw(0, word as u16);
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn inc(&mut self, Rd: u8) {
        // Rd <= Rd + 1; (Carry bit unchanged)
        let rd = self.get_r(Rd);
        let (r,_) = rd.overflowing_add(1);
        self.set_r(Rd, r);

        let Z = r == 0;
        let N = r.view_bits::<Lsb0>()[7];
        let V = rd == 0x7F;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn mul(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (unsigned) <= Rd (unsigned) x Rr (unsigned)
        self.busy = 1;

        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let word = u16::from(rd)*u16::from(rr);
        let C = word.view_bits::<Lsb0>()[15];

        self.set_rw(0, word);
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn muls(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (signed) <= Rd (signed) x Rr (signed)
        self.busy = 1;

        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let word = i16::from(rd as i8)*i16::from(rr as i8);
        let C = (word as u16).view_bits::<Lsb0>()[15];

        self.set_rw(0, word as u16);
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn mulsu(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (signed) <= Rd (signed) x Rr (unsigned)
        self.busy = 1;

        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let word = i16::from(rd as i8)*i16::from(rr);
        let C = (word as u16).view_bits::<Lsb0>()[15];

        self.set_rw(0, word as u16);
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn neg(&mut self, Rd: u8) {
        // Rd <= ~Rd + 1 (i.e twos-complement)
        let rd = self.get_r(Rd);
        let (r, _) = (!rd).overflowing_add(1);

        self.set_r(Rd, r);

        let brd = rd.view_bits::<Lsb0>();
        let br = r.view_bits::<Lsb0>();

        let C = r != 0;
        let Z = r == 0;
        let N = br[7];
        let V = r == 0x80;
        let S = N ^ V;
        let H = br[3] | brd[3];
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn or(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd or Rr
        let rd = self.get_r(Rd);
        let rr = self.get_r(Rr);
        let r = rd | rr;

        self.set_r(Rd, r);

        let br = r.view_bits::<Lsb0>();

        let Z = r == 0;
        let N = br[7];
        let V = false;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn ori(&mut self, Rd: u8, val: u8) {
        // Rd <- Rd or val
        let rd = self.get_r(Rd);
        let r = rd | val;

        self.set_r(Rd, r);

        let br = r.view_bits::<Lsb0>();

        let Z = r == 0;
        let N = br[7];
        let V = false;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn sbc(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd - Rr - C
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 
        let mut C = self.get_sreg_bit(BitSREG::C);
        let mut r: u8;

        //println!("[DEBUG] sbc, Rd = {}, Rr = {}", rd, rr);

        let C1;
        let mut C2 = false;

        (r, C1) = rd.overflowing_sub(rr);
        if C {
            (r, C2) = r.overflowing_sub(1);
        }
        C = C1 | C2;

        self.set_r(Rd, r);

        let brd = rd.view_bits::<Lsb0>();
        let brr = rr.view_bits::<Lsb0>();
        let br = r.view_bits::<Lsb0>();

        let Z = self.get_sreg_bit(BitSREG::Z) & (r == 0);
        let N = br[7];
        let V = (brd[7] & !brr[7] & !br[7]) | (!brd[7] & brr[7] & br[7]);
        let S = N ^ V;
        let H = (!brd[3] & brr[3]) | (brr[3] & br[3]) | (br[3] | !brd[3]);
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn sbci(&mut self, Rd: u8, val: u8) {
        // Rd <- Rd - val - C
        let rd = self.get_r(Rd); 
        let mut C = self.get_sreg_bit(BitSREG::C);
        let mut r: u8;

        let C1;
        let mut C2 = false;

        (r, C1) = rd.overflowing_sub(val);
        if C {
            (r, C2) = r.overflowing_sub(1);
        }
        C = C1 | C2;

        self.set_r(Rd, r);

        let brd = rd.view_bits::<Lsb0>();
        let brr = val.view_bits::<Lsb0>();
        let br = r.view_bits::<Lsb0>();

        let Z = self.get_sreg_bit(BitSREG::Z) & (r == 0);
        let N = br[7];
        let V = (brd[7] & !brr[7] & !br[7]) | (!brd[7] & brr[7] & br[7]);
        let S = N ^ V;
        let H = (!brd[3] & brr[3]) | (brr[3] & br[3]) | (br[3] | !brd[3]);
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn sbiw(&mut self, Rd: u8, val: u8) {
        // R[d+1]:Rd <- R[d+1]:Rd - val
        self.busy = 1;

        let rd = self.get_rw(Rd); 
        let (r, C) = rd.overflowing_sub(u16::from(val));
        
        self.set_rw(Rd, r);

        let brd = rd.view_bits::<Lsb0>();
        let br = r.view_bits::<Lsb0>();

        let Z = r == 0;
        let N = br[15];
        let V = !br[15] & brd[15];
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }

    #[allow(non_snake_case)]
    fn sub(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd - Rr
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        //println!("[DEBUG] sub, Rd = {}, Rr = {}", rd, rr);

        let (r, C) = rd.overflowing_sub(rr);

        self.set_r(Rd, r);

        let brd = rd.view_bits::<Lsb0>();
        let brr = rr.view_bits::<Lsb0>();
        let br = r.view_bits::<Lsb0>();

        let Z = r == 0;
        let N = br[7];
        let V = (brd[7] & !brr[7] & !br[7]) | (!brd[7] & brr[7] & br[7]);
        let S = N ^ V;
        let H = (!brd[3] & brr[3]) | (brr[3] & br[3]) | (br[3] | !brd[3]);
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn subi(&mut self, Rd: u8, val: u8) {
        // Rd <- Rd - val
        let rd = self.get_r(Rd); 

        let (r, C) = rd.overflowing_sub(val);

        let brd = rd.view_bits::<Lsb0>();
        let brr = val.view_bits::<Lsb0>();
        let br = r.view_bits::<Lsb0>();

        self.set_r(Rd, r);

        let Z = r == 0;
        let N = br[7];
        let V = (brd[7] & !brr[7] & !br[7]) | (!brd[7] & brr[7] & br[7]);
        let S = N ^ V;
        let H = (!brd[3] & brr[3]) | (brr[3] & br[3]) | (br[3] | !brd[3]);
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    // FLOW CONTROL INSTRUCTIONS
    fn brbx(&mut self, bit: u8, offset: i8, set: bool) {
        if !set ^ self.get_sreg_bit(BitSREG::from(bit)) {
            let mut pc = self.pc as i32;
            pc += i32::from(offset);
            self.pc = pc as u16;
            self.busy = 1;
        }
    }

    fn call(&mut self, address: u32) {
        self.busy = 2; // AVRxt, 16-bit PC

        let mut ds = self.ds.borrow_mut();
        ds.write(usize::from(self.sp), (self.pc+1) as u8); self.sp -= 1;
        ds.write(usize::from(self.sp), ((self.pc+1)>>8) as u8); self.sp -= 1;
        self.pc = address as u16;
    }

    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    fn cp(&mut self, Rd: u8, Rr: u8) {
        // Rd - Rr
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let (r, C) = rd.overflowing_sub(rr);

        let br = r.view_bits::<Lsb0>();
        let brd = rd.view_bits::<Lsb0>();
        let brr = rr.view_bits::<Lsb0>();

        let Z = r == 0;
        let N = br[7];
        let V = (brd[7] & !brr[7] & !br[7]) | (!brd[7] & brr[7] & br[7]);
        let S = N ^ V;
        let H = (!brd[3] & brr[3]) | (brr[3] & br[3]) | (br[3] & !brd[3]);
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    fn cpc(&mut self, Rd: u8, Rr: u8) {
        // Rd - Rr - C
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr);
        let mut r: u8;
        let mut C = self.get_sreg_bit(BitSREG::C);
        let Z = self.get_sreg_bit(BitSREG::Z);

        let C1;
        let mut C2 = false;

        (r, C1) = rd.overflowing_sub(rr);
        if C {
            (r, C2) = r.overflowing_sub(1);
        }
        C = C1 | C2;

        let br = r.view_bits::<Lsb0>();
        let brd = rd.view_bits::<Lsb0>();
        let brr = rr.view_bits::<Lsb0>();

        let Z = Z & (r == 0);
        let N = br[7];
        let V = (brd[7] & !brr[7] & !br[7]) | (!brd[7] & brr[7] & br[7]);
        let S = N ^ V;
        let H = (!brd[3] & brr[3]) | (brr[3] & br[3]) | (br[3] & !brd[3]);
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    fn cpi(&mut self, Rd: u8, val: u8) {
        // Rd - val
        let rd = self.get_r(Rd); 

        let (r, C) = rd.overflowing_sub(val);

        let br = r.view_bits::<Lsb0>();
        let brd = rd.view_bits::<Lsb0>();
        let bk = val.view_bits::<Lsb0>();

        let Z = r == 0;
        let N = br[7];
        let V = (brd[7] & !bk[7] & !br[7]) | (!brd[7] & bk[7] & br[7]);
        let S = N ^ V;
        let H = (!brd[3] & bk[3]) | (bk[3] & br[3]) | (br[3] & !brd[3]);
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }

    #[allow(non_snake_case)]
    fn cpse(&mut self, Rd: u8, Rr: u8) {
        use Instruction::*;

        let rd = self.get_r(Rd);
        let rr = self.get_r(Rr);
        
        if rd == rr {
            let opcode = self.get_progmem(self.pc as u32);
            let prefetch = self.get_progmem((self.pc + 1) as u32);
            let op = Instruction::decode(opcode, prefetch);
            match op {
                CALL{..} | JMP{..} | LDS{..} | STS{..} => {self.pc += 2; self.busy = 2},
                _ => {self.pc += 1; self.busy = 1}
            }
        }
    }

    fn icall(&mut self) {
        self.busy = 1; // AVRxt, 16-bit PC

        let mut ds = self.ds.borrow_mut();
        ds.write(usize::from(self.sp), (self.pc) as u8); self.sp -= 1;
        ds.write(usize::from(self.sp), ((self.pc)>>8) as u8); self.sp -= 1;
        self.pc = self.get_rw(30);
    }

    fn ijmp(&mut self) {
        self.busy = 1;

        self.pc = self.get_rw(30);
    }

    fn jmp(&mut self, address: u32) {
        self.busy = 2;

        self.pc = address as u16;
    }

    fn rcall(&mut self, offset: i16) {
        self.busy = 1; // AVRxt, 16-bit PC

        let mut ds = self.ds.borrow_mut();
        ds.write(usize::from(self.sp), (self.pc) as u8); self.sp -= 1;
        ds.write(usize::from(self.sp), ((self.pc)>>8) as u8); self.sp -= 1;
        (self.pc, _) = self.pc.overflowing_add(offset as u16);
    }

    fn ret(&mut self) {
        self.busy = 3; // 16-bit PC, not AVRrc

        let mut ds = self.ds.borrow_mut();
        self.sp += 1; let (bh, _) = ds.read(usize::from(self.sp)); 
        self.sp += 1; let (bl, _) = ds.read(usize::from(self.sp));
        self.pc = ((bh as u16) << 8) | (bl as u16);
    }

    fn reti(&mut self) {
        self.busy = 3; // 16-bit PC, not AVRrc

        let mut ds = self.ds.borrow_mut();
        self.sp += 1; let (bh, _) = ds.read(usize::from(self.sp)); 
        self.sp += 1; let (bl, _) = ds.read(usize::from(self.sp));
        self.pc = ((bh as u16) << 8) | (bl as u16);

        // Ack interrupt
        self.interrupt_handler.borrow_mut().reti();

        //I bit in SREG not set for AVRxt 
    }

    fn rjmp(&mut self, offset: i16) {
        self.busy = 1;

        (self.pc, _) = self.pc.overflowing_add(offset as u16);
    }

    #[allow(non_snake_case)]
    fn sbix(&mut self, ioreg: u8, bit: u8, set: bool) {
        use Instruction::*;
        
        let bitval_n = (self.get_ior(ioreg) & (1<<bit)) == 0;

        if set ^ bitval_n {
            let opcode = self.get_progmem(self.pc as u32);
            let prefetch = self.get_progmem((self.pc + 1) as u32);
            let op = Instruction::decode(opcode, prefetch);
            match op {
                CALL{..} | JMP{..} | LDS{..} | STS{..} => {self.pc += 2; self.busy = 2},
                _ => {self.pc += 1; self.busy = 1},
            }
        }
    }

    #[allow(non_snake_case)]
    fn sbrx(&mut self, Rr: u8, bit: u8, set: bool) {
        use Instruction::*;
        
        let bitval_n = (self.get_r(Rr) & (1<<bit)) == 0;

        if set ^ bitval_n {
            let opcode = self.get_progmem(self.pc as u32);
            let prefetch = self.get_progmem((self.pc + 1) as u32);
            let op = Instruction::decode(opcode, prefetch);
            match op {
                CALL{..} | JMP{..} | LDS{..} | STS{..} => {self.pc += 2; self.busy = 2},
                _ => {self.pc += 1; self.busy = 1},
            }
        } 
    }

    // BIT MANIPULATION INSTRUCTIONS
    #[allow(non_snake_case)]
    fn asr(&mut self, Rd: u8) {
        let vRd = self.get_r(Rd);
        let vR = ((vRd as i8) >> 1) as u8;
        self.set_r(Rd, vR); 
 
        let C = vRd.view_bits::<Lsb0>()[0];
        let Z = vR == 0;
        let N = vR.view_bits::<Lsb0>()[7];
        let V = N ^ C;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }
    
    fn bclr(&mut self, bit: u8) {
        self.set_sreg_bit(BitSREG::from(bit), false);
    }
    
    #[allow(non_snake_case)]
    fn bld(&mut self, Rd: u8, bit: u8) {
        if self.get_sreg_bit(BitSREG::T) {
            self.set_r(Rd, self.get_r(Rd) | (1 << bit));
        } else {
            self.set_r(Rd, self.get_r(Rd) & !(1 << bit));
        }
    }
    
    fn bset(&mut self, bit: u8) {
        self.set_sreg_bit(BitSREG::from(bit), true)
    }
    
    #[allow(non_snake_case)]
    fn bst(&mut self, Rd: u8, bit: u8) {
        self.set_sreg_bit(BitSREG::T, (self.get_r(Rd) & (1 << bit)) != 0);
    }
    
    fn cbi(&mut self, ioreg: u8, bit: u8) {
        match ioreg {
            0x3F => {self.sreg &= !(1u8 << bit)},      //CPU.SREG
            0x3D => {self.sp &= !(1u16 << bit)},       //CPU.SPL
            0x3E => {self.sp &= !(1u16 << (bit+8))},   //CPU.SPH
            _ => {self.ds.borrow_mut().set_bit(usize::from(ioreg), bit, false);}
        }        
    }
    
    #[allow(non_snake_case)]
    fn lsl(&mut self, Rd: u8) {
        let vRd = self.get_r(Rd);
        let vR = vRd << 1;
        self.set_r(Rd, vR);  

        let C = vRd.view_bits::<Lsb0>()[7];
        let Z = vR == 0;
        let N = vR.view_bits::<Lsb0>()[7];
        let V = N ^ C;
        let S = N ^ V;
        let H = vRd.view_bits::<Lsb0>()[3];
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }
    
    #[allow(non_snake_case)]
    fn lsr(&mut self, Rd: u8) {     
        let vRd = self.get_r(Rd);
        let vR = vRd >> 1;
        self.set_r(Rd, vR);  

        let C = vRd.view_bits::<Lsb0>()[0];
        let Z = vR == 0;
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
    fn rol(&mut self, Rd: u8) {
        let rd = self.get_r(Rd);
        let C = self.get_sreg_bit(BitSREG::C);
        
        let mut r = rd << 1;
        if C {
            r |= 0x01;
        }

        self.set_r(Rd, r);  

        let C = rd.view_bits::<Lsb0>()[7];
        let Z = r == 0;
        let N = r.view_bits::<Lsb0>()[7];
        let V = N ^ C;
        let S = N ^ V;
        let H = rd.view_bits::<Lsb0>()[3];
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
        self.set_sreg_bit(BitSREG::H, H);
    }
    
    #[allow(non_snake_case)]
    fn ror(&mut self, Rd: u8) {
        let rd = self.get_r(Rd);
        let C = self.get_sreg_bit(BitSREG::C);

        let mut r = rd >> 1;
        if C {
            r |= 0x80;
        }

        self.set_r(Rd, r);  

        let C = rd.view_bits::<Lsb0>()[0];
        let Z = r == 0;
        let N = r.view_bits::<Lsb0>()[7];
        let V = N ^ C;
        let S = N ^ V;
        
        self.set_sreg_bit(BitSREG::C, C);
        self.set_sreg_bit(BitSREG::Z, Z);
        self.set_sreg_bit(BitSREG::N, N);
        self.set_sreg_bit(BitSREG::V, V);
        self.set_sreg_bit(BitSREG::S, S);
    }
    
    fn sbi(&mut self, ioreg: u8, bit: u8) {
        match ioreg {
            0x3F => {self.sreg |= 1u8 << bit},      //CPU.SREG
            0x3D => {self.sp |= 1u16 << bit},       //CPU.SPL
            0x3E => {self.sp |= 1u16 << (bit+8)},   //CPU.SPH
            _ => {self.ds.borrow_mut().set_bit(usize::from(ioreg), bit, true);}
        }     
        
    }

    #[allow(non_snake_case)]
    fn swap(&mut self, Rd: u8) {
        self.set_r(Rd, self.get_r(Rd).rotate_left(4));
    }

    #[allow(non_snake_case)]
    fn iin(&mut self, Rd: u8, ioreg: u8) {
        self.set_r(Rd, self.get_ior(ioreg));
    }

    #[allow(non_snake_case)]
    fn ld(&mut self, Rd: u8, Rp: u8, offset: u8, dec: bool, inc: bool) {
        self.busy = 2; // AVRxt, SRAM only
        
        let mut address = self.get_rw(Rp);

        if dec {
            address = address.wrapping_sub(1);
            self.set_rw(Rp, address)
        }

        if (address as u32) + (offset as u32) < 0x40 {
            self.set_r(Rd, self.get_ior(((address as u32) + (offset as u32)) as u8));
        } else {
            self.set_r(Rd, self.get_ds((address as u32) + (offset as u32)));
        }
        

        //println!("[DEBUG] R{} <= DS(0x{:04X}) (0x{:04X})", Rd, (address as u32) + (offset as u32), self.get_ds((address as u32) + (offset as u32)));

        if inc {
            address = address.wrapping_add(1);
            self.set_rw(Rp, address)
        }
    }

    #[allow(non_snake_case)]
    fn st(&mut self, Rr: u8, Rp: u8, offset: u8, dec: bool, inc: bool) {
        self.busy = 1;
        
        let mut address = self.get_rw(Rp);

        if dec {
            address = address.wrapping_sub(1);
            self.set_rw(Rp, address)
        }

        if (address as u32) + (offset as u32) < 0x40 {
            self.set_ior(((address as u32) + (offset as u32)) as u8, self.get_r(Rr));
        } else {
            self.set_ds((address as u32) + (offset as u32), self.get_r(Rr));
        }

        //println!("[DEBUG] DS(0x{:04X}) <= R{} (0x{:04X})", (address as u32) + (offset as u32), Rr, self.get_ds((address as u32) + (offset as u32)));

        if inc {
            address = address.wrapping_add(1);
            self.set_rw(Rp, address)
        }
    }

    #[allow(non_snake_case)]
    fn ldi(&mut self, Rd: u8, val: u8) {
        self.set_r(Rd, val);
    }

    #[allow(non_snake_case)]
    fn lds(&mut self, Rd: u8, address: u16) {
        self.busy = 2; // AVRxt, SRAM only

        let val = self.get_ds(address as u32);
        self.set_r(Rd, val);
    }
    
    #[allow(non_snake_case)]
    fn lpm(&mut self, Rd: u8, inc: bool) {
        self.busy = 2;

        let mut address = self.get_rw(30);

        self.set_r(Rd, self.get_ps(address as u32));

        if inc {
            address = address.wrapping_add(1);
            self.set_rw(30, address)
        }
    }

    #[allow(non_snake_case)]
    fn mov(&mut self, Rd: u8, Rr: u8) {
        self.set_r(Rd, self.get_r(Rr))
    }

    #[allow(non_snake_case)]
    fn movw(&mut self, Rd: u8, Rr: u8) {
        self.set_rw(Rd, self.get_rw(Rr))
    }

    #[allow(non_snake_case)]
    fn out(&mut self, Rr: u8, ioreg: u8) {
        self.set_ior(ioreg, self.get_r(Rr));
    }

    #[allow(non_snake_case)]
    fn pop(&mut self, Rd: u8) {
        self.busy = 1; // 2 for AVRrc

        self.sp = self.sp.overflowing_add(1).0;
        self.set_r(Rd, self.get_ds(self.sp as u32));

        //println!("[DEBUG] STACK({:04X}) = {:02X}, SP = {:04X}", self.sp, self.get_r(Rd), self.sp);
    }

    #[allow(non_snake_case)]
    fn push(&mut self, Rr: u8) {
        //println!("[DEBUG] STACK({:04X}) = {:02X}, SP = {:04X}", self.sp, self.get_r(Rr), self.sp.overflowing_sub(1).0);
        self.set_ds(self.sp as u32, self.get_r(Rr));
        self.sp = self.sp.overflowing_sub(1).0;
        
    }

    #[allow(non_snake_case)]
    fn sts(&mut self, Rr: u8, address: u16) {
        self.busy = 1;  // SRAM only

        self.set_ds(address as u32, self.get_r(Rr));
    }

    pub fn tick(&mut self) -> bool {
        use Instruction::*; 

        //for i in 0..32 {
        //    print!("{:02X} ", self.get_r(i));
        //}
        //println!("{:02X} {:04X} {:04X} {:02X}", self.sreg, self.pc, self.sp, self.get_ds(self.sp as u32));

        // Wait for multi-cycle instructions to complete
        if self.busy > 0 {
            self.busy = self.busy - 1;
            if self.debug {
                println!("[0x{:04X}] ...", self.pc<<1);
            }
            return true;
        }

        // HANDLE INTERRUPTS

        // Interrupts enabled
        if self.get_sreg_bit(BitSREG::I) {
            let vector = self.interrupt_handler.borrow_mut().service_pending();
            match vector {
                Some(address) => {
                    let mut ds = self.ds.borrow_mut();
                    ds.write(usize::from(self.sp), (self.pc) as u8); self.sp -= 1;
                    ds.write(usize::from(self.sp), ((self.pc)>>8) as u8); self.sp -= 1;
                    self.pc = address as u16;
                    self.busy = 4;  // 2 cycles to to push PC + 3 cycles for jmp to vector
                    return true;
                }
                None => {}
            }
        }

        let opcode = self.get_progmem(self.pc as u32);
        let prefetch = self.get_progmem((self.pc + 1) as u32);
        let op = Instruction::decode(opcode, prefetch);

        if self.debug {
            println!("[0x{:04X}] {:?}", self.pc<<1, op);
        }

        //Most instructions are single cycle so do this first
        self.pc += 1;

        match op {
            // Control
            //BREAK   => {println!("[END] BREAK instruction encountered."); return false}, // NOP if OCD disabled
            BREAK   => {}, // Break instruction ignored for Tut09
            NOP     => {},
            SLEEP   => {println!("[SLEEP]")}, // Not implemented
            WDR     => {}, // Not implemented
            // Arithmetic
            ADC     {Rd, Rr}    => {self.adc(Rd,Rr)},
            ADD     {Rd, Rr}    => {self.add(Rd,Rr)},
            ADIW    {Rd, val}   => {self.adiw(Rd, val)},
            AND     {Rd, Rr}    => {self.and(Rd, Rr)},
            ANDI    {Rd, val}   => {self.andi(Rd, val)},
            COM     {Rd}            => {self.com(Rd)},
            DEC     {Rd}            => {self.dec(Rd)},
            EOR     {Rd, Rr}    => {self.eor(Rd,Rr)},
            FMUL    {Rd, Rr}    => {self.fmul(Rd,Rr)},
            FMULS   {Rd, Rr}    => {self.fmuls(Rd,Rr)},
            FMULSU  {Rd, Rr}    => {self.fmulsu(Rd,Rr)},
            INC     {Rd}            => {self.inc(Rd)},
            MUL     {Rd, Rr}    => {self.mul(Rd,Rr)},
            MULS    {Rd, Rr}    => {self.muls(Rd,Rr)},
            MULSU   {Rd, Rr}    => {self.mulsu(Rd,Rr)},
            NEG     {Rd}            => {self.neg(Rd)},
            OR      {Rd, Rr}    => {self.or(Rd, Rr)},
            ORI     {Rd, val}   => {self.ori(Rd, val)},
            SBC     {Rd, Rr}    => {self.sbc(Rd, Rr)},
            SBCI    {Rd,val}    => {self.sbci(Rd, val)},
            SBIW    {Rd,val}    => {self.sbiw(Rd, val)},
            SUB     {Rd, Rr}    => {self.sub(Rd, Rr)},
            SUBI    {Rd, val}   => {self.subi(Rd, val)},
            //Flow
            BRBC    {offset, bit}   => {self.brbx(bit, offset, false)},
            BRBS    {offset, bit}   => {self.brbx(bit, offset, true)},
            CALL    {address}          => {self.call(address)},
            CP      { Rd, Rr }      => {self.cp(Rd, Rr)},
            CPC     { Rd, Rr }      => {self.cpc(Rd, Rr)},
            CPI     { Rd, val }     => {self.cpi(Rd, val)},
            CPSE    { Rd, Rr }      => {self.cpse(Rd, Rr)},
            ICALL                           => {self.icall()},
            IJMP                            => {self.ijmp()},
            JMP     { address }        => {self.jmp(address)},
            RCALL   { offset }         => {self.rcall(offset)},
            RET                             => {self.ret()},
            RETI                            => {self.reti()},
            RJMP    { offset }         => {self.rjmp(offset)},
            SBIC    { ioreg, bit }  => {self.sbix(ioreg, bit, false)},
            SBIS    { ioreg, bit }  => {self.sbix(ioreg, bit, true)},
            SBRC    { Rr, bit }     => {self.sbrx(Rr, bit, false)},
            SBRS    { Rr, bit }     => {self.sbrx(Rr, bit, true)},
            //Bit
            ASR     { Rd }              => {self.asr(Rd)},
            BCLR    { bit }             => {self.bclr(bit)},
            BLD     { Rd, bit }     => {self.bld(Rd, bit)},
            BSET    { bit }             => {self.bset(bit)},
            BST     { Rd, bit }     => {self.bst(Rd, bit)},
            CBI     { ioreg, bit }  => {self.cbi(ioreg, bit)},
            LSL     { Rd }              => {self.lsl(Rd)},
            LSR     { Rd }              => {self.lsr(Rd)},
            ROL     { Rd }              => {self.rol(Rd)},
            ROR     { Rd }              => {self.ror(Rd)},
            SBI     { ioreg, bit}   => {self.sbi(ioreg, bit)},
            SWAP    { Rd }              => {self.swap(Rd)},
            //Data transfer
            IN          { Rd, ioreg }       => {self.iin(Rd, ioreg)},
            LDX         { Rd }                  => {self.ld(Rd, 26, 0, false, false)},
            LDXdec      { Rd }                  => {self.ld(Rd, 26, 0, true, false)},
            LDXinc      { Rd }                  => {self.ld(Rd, 26, 0, false, true)},
            LDYdec      { Rd }                  => {self.ld(Rd, 28, 0, true, false)},
            LDYinc      { Rd }                  => {self.ld(Rd, 28, 0, false, true)},
            LDZdec      { Rd }                  => {self.ld(Rd, 30, 0, true, false)},
            LDZinc      { Rd }                  => {self.ld(Rd, 30, 0, false, true)},
            LDDY        { Rd, offset }      => {self.ld(Rd, 28, offset, false, false)},
            LDDZ        { Rd, offset}       => {self.ld(Rd, 30, offset, false, false)},
            LDI         { Rd, val }         => {self.ldi(Rd, val)},
            LDS         { Rd, address }    => {self.lds(Rd, address); self.pc += 1},
            LPM                                     => {self.lpm(0, false)},
            LPMRdZ      { Rd }                  => {self.lpm(Rd, false)},
            LPMRdZinc   { Rd }                  => {self.lpm(Rd, true)},
            MOV         { Rd, Rr }          => {self.mov(Rd, Rr)},
            MOVW        { Rd, Rr }          => {self.movw(Rd, Rr)},
            OUT         { Rr, ioreg }       => {self.out(Rr, ioreg)},
            POP         { Rd }                  => {self.pop(Rd)},
            PUSH        { Rr }                  => {self.push(Rr)},
            STX         { Rr }                  => {self.st(Rr, 26, 0, false, false)},
            STXdec      { Rr }                  => {self.st(Rr, 26, 0, true, false)},
            STXinc      { Rr }                  => {self.st(Rr, 26, 0, false, true)},
            STYdec      { Rr }                  => {self.st(Rr, 28, 0, true, false)},
            STYinc      { Rr }                  => {self.st(Rr, 28, 0, false, true)},
            STZdec      { Rr }                  => {self.st(Rr, 30, 0, true, false)},
            STZinc      { Rr }                  => {self.st(Rr, 30, 0, false, true)},
            STDY        { Rr, offset }      => {self.st(Rr, 28, offset, false, false)},
            STDZ        { Rr, offset }      => {self.st(Rr, 30, offset, false, false)},
            STS         { Rr, address }    => {self.sts(Rr, address); self.pc += 1},
            //Undefined
            UNDEF   => { println!("[ERROR] Undefined opcode: {:b}", opcode) }
        }

        //Return
        true
    }
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Debug)]
enum Instruction {
    ADC{Rd: u8, Rr: u8},
    ADD{Rd: u8, Rr: u8},
    ADIW{Rd: u8, val: u8},
    AND{Rd: u8, Rr: u8},
    ANDI{Rd: u8, val: u8},
    ASR{Rd: u8},
    BCLR{bit: u8},
    BLD{Rd: u8, bit: u8},
    BRBC{offset: i8, bit: u8},
    BRBS{offset: i8, bit: u8},
    BREAK,
    BSET{bit: u8},
    BST{Rd: u8, bit: u8},
    CALL{address: u32},
    CBI{ioreg: u8, bit: u8},
    COM{Rd: u8},
    CP{Rd: u8, Rr: u8},
    CPC{Rd: u8, Rr: u8},
    CPI{Rd: u8, val: u8},
    CPSE{Rd: u8, Rr: u8},
    DEC{Rd: u8},
    EOR{Rd: u8, Rr: u8},
    FMUL{Rd: u8, Rr: u8},
    FMULS{Rd: u8, Rr: u8},
    FMULSU{Rd: u8, Rr: u8},
    ICALL,
    IJMP,
    IN{Rd: u8, ioreg: u8},
    INC{Rd: u8},
    JMP{address: u32},
    LDX{Rd: u8},
    LDXdec{Rd: u8},
    LDXinc{Rd: u8},
    LDYdec{Rd: u8},
    LDYinc{Rd: u8},
    LDZdec{Rd: u8},
    LDZinc{Rd: u8},
    LDDY{Rd: u8, offset: u8},
    LDDZ{Rd: u8, offset: u8},
    LDI{Rd: u8, val: u8},
    LDS{Rd: u8, address: u16},
    LPM,
    LPMRdZ{Rd: u8},
    LPMRdZinc{Rd: u8},
    LSL{Rd: u8},
    LSR{Rd: u8},
    MOV{Rd: u8, Rr: u8},
    MOVW{Rd: u8, Rr: u8},
    MUL{Rd: u8, Rr: u8},
    MULS{Rd: u8, Rr: u8},
    MULSU{Rd: u8, Rr: u8},
    NEG{Rd: u8},
    NOP,
    OR{Rd: u8, Rr: u8},
    ORI{Rd: u8, val: u8},
    OUT{Rr: u8, ioreg: u8},
    POP{Rd: u8},
    PUSH{Rr: u8},
    RCALL{offset: i16},
    RET,
    RETI,
    RJMP{offset: i16},
    ROL{Rd: u8},
    ROR{Rd: u8},
    SBC{Rd: u8, Rr: u8},
    SBCI{Rd: u8, val: u8},
    SBI{ioreg: u8, bit: u8},
    SBIC{ioreg: u8, bit: u8},
    SBIS{ioreg: u8, bit: u8},
    SBIW{Rd: u8, val: u8},
    SBRC{Rr: u8, bit: u8},
    SBRS{Rr: u8, bit: u8},
    SLEEP,
    STX{Rr: u8},
    STXdec{Rr: u8},
    STXinc{Rr: u8},
    STYdec{Rr: u8},
    STYinc{Rr: u8},
    STZdec{Rr: u8},
    STZinc{Rr: u8},
    STDY{Rr: u8, offset: u8},
    STDZ{Rr: u8, offset: u8},
    STS{Rr: u8, address: u16},
    SUB{Rd: u8, Rr: u8},
    SUBI{Rd: u8, val: u8},
    SWAP{Rd: u8},
    WDR,
    UNDEF
}

#[allow(dead_code)]
enum BitSREG {
    C = 0,
    Z = 1,
    N = 2,
    V = 3,
    S = 4,
    H = 5,
    T = 6,
    I = 7
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
            _ => panic!("Invalid SREG bit.")
        }
    }
}

impl Instruction {
    #[bitmatch]
    fn decode (opcode: u16, prefetch: u16) -> Instruction {
        #[bitmatch]
        match opcode {
            "0000_0000_0000_0000" => Instruction::NOP,
            "0000_11rd_dddd_rrrr" => Instruction::ADD { Rd: d as u8, Rr: r as u8 },
            "0001_11rd_dddd_rrrr" if r != d => Instruction::ADC { Rd: d as u8, Rr: r as u8},
            "1001_0110_kkdd_kkkk" => Instruction::ADIW { Rd: ((d as u8) << 1) + 24, val: k as u8 },
            "0010_00rd_dddd_rrrr" => Instruction::AND { Rd: d as u8, Rr: r as u8 },
            "0111_kkkk_dddd_kkkk" => Instruction::ANDI { Rd: (d as u8) + 16, val: k as u8 },
            "1001_010d_dddd_0101" => Instruction::ASR { Rd: d as u8 },
            "1001_0100_1sss_1000" => Instruction::BCLR { bit: s as u8 },
            "1111_100d_dddd_0bbb" => Instruction::BLD { Rd: d as u8, bit: b as u8 },
            "1111_01kk_kkkk_ksss" => Instruction::BRBC { 
                                        offset: {
                                            let mut k = k as u8;
                                            let kb = k.view_bits_mut::<Lsb0>();
                                            kb.set(7, kb[6]);
                                            k as i8
                                        },
                                        bit: s as u8 
                                    },
            "1111_00kk_kkkk_ksss" => Instruction::BRBS { 
                                        offset: {
                                            let mut k = k as u8;
                                            let kb = k.view_bits_mut::<Lsb0>();
                                            kb.set(7, kb[6]);
                                            k as i8
                                        },
                                        bit: s as u8 
                                    },
            "1001_0101_1001_1000" => Instruction::BREAK,
            "1001_0100_0sss_1000" => Instruction::BSET { bit: s as u8 },
            "1111_101d_dddd_0bbb" => Instruction::BST { Rd: d as u8, bit: b as u8 },
            "1001_010k_kkkk_111k" => Instruction::CALL { address: ((k as u32) << 16) | (prefetch as u32) },
            "1001_1000_aaaa_abbb" => Instruction::CBI { ioreg: a as u8, bit: b as u8 },
            "1001_010d_dddd_0000" => Instruction::COM { Rd: d as u8 },
            "0001_01rd_dddd_rrrr" => Instruction::CP { Rd: d as u8, Rr: r as u8 },
            "0000_01rd_dddd_rrrr" => Instruction::CPC { Rd: d as u8, Rr: r as u8 },
            "0011_kkkk_dddd_kkkk" => Instruction::CPI { Rd: d as u8 + 16, val: k as u8 },
            "0001_00rd_dddd_rrrr" => Instruction::CPSE { Rd: d as u8, Rr: r as u8 },
            "1001_010d_dddd_1010" => Instruction::DEC { Rd: d as u8 },
            "0010_01rd_dddd_rrrr" => Instruction::EOR { Rd: d as u8, Rr: r as u8 },
            "0000_0011_0ddd_1rrr" => Instruction::FMUL { Rd: d as u8 + 16, Rr: r as u8 + 16 },
            "0000_0011_1ddd_0rrr" => Instruction::FMULS { Rd: d as u8 + 16, Rr: r as u8 + 16 },
            "0000_0011_1ddd_1rrr" => Instruction::FMULSU { Rd: d as u8 + 16, Rr: r as u8 + 16 },
            "1001_0101_0000_1001" => Instruction::ICALL,
            "1001_0100_0000_1001" => Instruction::IJMP,
            "1011_0aad_dddd_aaaa" => Instruction::IN { Rd: d as u8, ioreg: a as u8 },
            "1001_010d_dddd_0011" => Instruction::INC { Rd: d as u8 },
            "1001_010k_kkkk_110k" => Instruction::JMP { address: ((k as u32) << 16) | (prefetch as u32) },
            "1001_000d_dddd_1100" => Instruction::LDX { Rd: d as u8 },
            "1001_000d_dddd_1110" => Instruction::LDXdec { Rd: d as u8 },
            "1001_000d_dddd_1101" => Instruction::LDXinc { Rd: d as u8 },
            "1001_000d_dddd_1010" => Instruction::LDYdec { Rd: d as u8 },
            "1001_000d_dddd_1001" => Instruction::LDYinc { Rd: d as u8 },
            "1001_000d_dddd_0010" => Instruction::LDZdec { Rd: d as u8 },
            "1001_000d_dddd_0001" => Instruction::LDZinc { Rd: d as u8 },
            "10q0_qq0d_dddd_1qqq" => Instruction::LDDY { Rd: d as u8, offset: q as u8 },
            "10q0_qq0d_dddd_0qqq" => Instruction::LDDZ { Rd: d as u8, offset: q as u8 },
            "1110_kkkk_dddd_kkkk" => Instruction::LDI { Rd: d as u8 + 16, val: k as u8 },
            "1001_000d_dddd_0000" => Instruction::LDS { Rd: d as u8, address: prefetch },
            "1001_0101_1100_1000" => Instruction::LPM,
            "1001_000d_dddd_0100" => Instruction::LPMRdZ { Rd: d as u8 },
            "1001_000d_dddd_0101" => Instruction::LPMRdZinc { Rd: d as u8 },
            "0000_11dd_dddd_dddd" => Instruction::LSL { Rd: d as u8 },
            "1001_010d_dddd_0110" => Instruction::LSR { Rd: d as u8 },
            "0010_11rd_dddd_rrrr" => Instruction::MOV { Rd: d as u8, Rr: r as u8 },
            "0000_0001_dddd_rrrr" => Instruction::MOVW { Rd: (d as u8) << 1, Rr: (r as u8) << 1 },
            "1001_11rd_dddd_rrrr" => Instruction::MUL { Rd: d as u8, Rr: r as u8 },
            "0000_0010_dddd_rrrr" => Instruction::MULS { Rd: d as u8 + 16, Rr: r as u8 + 16 },
            "0000_0011_0ddd_0rrr" => Instruction::MULSU { Rd: d as u8 + 16, Rr: r as u8 + 16 },
            "1001_010d_dddd_0001" => Instruction::NEG { Rd: d as u8 },
            "0000_0000_0000_0000" => Instruction::NOP,
            "0010_10rd_dddd_rrrr" => Instruction::OR { Rd: d as u8, Rr: r as u8 },
            "0110_kkkk_dddd_kkkk" => Instruction::ORI { Rd: d as u8 + 16, val: k as u8 },
            "1011_1aar_rrrr_aaaa" => Instruction::OUT { Rr: r as u8, ioreg: a as u8 },
            "1001_000d_dddd_1111" => Instruction::POP { Rd: d as u8 },
            "1001_001r_rrrr_1111" => Instruction::PUSH { Rr: r as u8 },
            "1101_kkkk_kkkk_kkkk" => Instruction::RCALL { 
                                        offset: {
                                            let mut k = k as u16;
                                            let kb = k.view_bits_mut::<Lsb0>();
                                            kb.set(12, kb[11]);
                                            kb.set(13, kb[11]);
                                            kb.set(14, kb[11]);
                                            kb.set(15, kb[11]);
                                            k as i16
                                        }
                                    },
            "1001_0101_0000_1000" => Instruction::RET,
            "1001_0101_0001_1000" => Instruction::RETI,
            "1100_kkkk_kkkk_kkkk" => Instruction::RJMP { 
                                        offset: {
                                            let mut k = k as u16;
                                            let kb = k.view_bits_mut::<Lsb0>();
                                            kb.set(12, kb[11]);
                                            kb.set(13, kb[11]);
                                            kb.set(14, kb[11]);
                                            kb.set(15, kb[11]);
                                            k as i16
                                        }
                                    },
            "0001_11rd_dddd_rrrr" if r == d => Instruction::ROL { Rd: d as u8 },
            "1001_010d_dddd_0111" => Instruction::ROR { Rd: d as u8 },
            "0000_10rd_dddd_rrrr" => Instruction::SBC { Rd: d as u8, Rr: r as u8 },
            "0100_kkkk_dddd_kkkk" => Instruction::SBCI { Rd: d as u8 + 16, val: k as u8 },
            "1001_1010_aaaa_abbb" => Instruction::SBI { ioreg: a as u8, bit: b as u8 },
            "1001_1001_aaaa_abbb" => Instruction::SBIC { ioreg: a as u8, bit: b as u8 },
            "1001_1011_aaaa_abbb" => Instruction::SBIS { ioreg: a as u8, bit: b as u8 },
            "1001_0111_kkdd_kkkk" => Instruction::SBIW { Rd: ((d as u8) << 1) + 24, val: k as u8 },
            "1111_110r_rrrr_0bbb" => Instruction::SBRC { Rr: r as u8, bit: b as u8 },
            "1111_111r_rrrr_0bbb" => Instruction::SBRS { Rr: r as u8, bit: b as u8 },
            "1001_0101_1000_1000" => Instruction::SLEEP,
            "1001_001r_rrrr_1100" => Instruction::STX { Rr: r as u8 },
            "1001_001r_rrrr_1110" => Instruction::STXdec { Rr: r as u8 },
            "1001_001r_rrrr_1101" => Instruction::STXinc { Rr: r as u8 },
            "1001_001r_rrrr_1010" => Instruction::STYdec { Rr: r as u8 },
            "1001_001r_rrrr_1001" => Instruction::STYinc { Rr: r as u8 },
            "1001_001r_rrrr_0010" => Instruction::STZdec { Rr: r as u8 },
            "1001_001r_rrrr_0001" => Instruction::STZinc { Rr: r as u8 },
            "10q0_qq1r_rrrr_1qqq" => Instruction::STDY { Rr: r as u8, offset: q as u8 },
            "10q0_qq1r_rrrr_0qqq" => Instruction::STDZ { Rr: r as u8, offset: q as u8 },
            "1001_001r_rrrr_0000" => Instruction::STS { Rr: r as u8, address: prefetch },
            "0001_10rd_dddd_rrrr" => Instruction::SUB { Rd: d as u8, Rr: r as u8 },
            "0101_kkkk_dddd_kkkk" => Instruction::SUBI { Rd: d as u8 + 16, val: k as u8 },
            "1001_010d_dddd_0010" => Instruction::SWAP { Rd: d as u8 },
            "1001_0101_1010_1000" => Instruction::WDR,
            _ => Instruction::UNDEF
        }
    }
}