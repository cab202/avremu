use std::cell::RefCell;
use std::rc::Rc;

use super::memory::MemoryMapped;

use bitmatch::bitmatch;

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
    busy: u8
}

impl Core {
    pub fn new(variant: CoreType, ds: Rc<RefCell<dyn MemoryMapped>>, progmem: Rc<RefCell<dyn MemoryMapped>>) -> Self {
        Self {
            variant,
            regs: [0;32],
            sreg: 0,
            pc: 0,
            sp: 2047,
            ds,
            progmem,
            busy: 0
        }
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

    fn spm(&mut self, address: u16, val: u16) {
        self.progmem.borrow_mut().write_word(usize::from(address)<<1, val);
    }

    #[allow(non_snake_case)]
    fn adc(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd + Rr + C
        let mut rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 
        let mut c = self.get_sreg_bit(BitSREG::C);

        if c {
            (rd, c) = rd.overflowing_add(1);
            if c {
                // We overflowed so cant overflow again
                rd += rr;
            } else {
                // We could still overflow
                (rd, c) = rd.overflowing_add(rr);
            }
        } else {
            // Carry bit is not set, normal overflowing add
            (rd, c) = rd.overflowing_add(rr);
        }

        self.set_r(Rd, rd);

        self.set_sreg_bit(BitSREG::C, c);
    }

    #[allow(non_snake_case)]
    fn add(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd + Rr
        let mut rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 
        let c: bool;

        (rd, c) = rd.overflowing_add(rr);

        self.set_r(Rd, rd);

        self.set_sreg_bit(BitSREG::C, c)
    }

    #[allow(non_snake_case)]
    fn adiw(&mut self, Rd: u8, val: u8) {
        // R[d+1]:Rd <- R[d+1]:Rd + val
        let mut word = self.get_rw(Rd);
        let c: bool;
        (word, c) = word.overflowing_add(u16::from(val));
        
        self.set_rw(Rd, word);

        self.set_sreg_bit(BitSREG::C, c)
    }

    #[allow(non_snake_case)]
    fn andi(&mut self, Rd: u8, val: u8) {
        let mut r = self.get_r(Rd);
        r &= val;
        self.set_r(Rd, r);
    }

    #[allow(non_snake_case)]
    fn com(&mut self, Rd: u8) {
        let mut r = self.get_r(Rd);
        r = !r;
        self.set_r(Rd, r);
    }

    #[allow(non_snake_case)]
    fn dec(&mut self, Rd: u8) {
        // Rd <= Rd - 1; (Carry bit unchanged)
        let mut r = self.get_r(Rd);
        (r,_) = r.overflowing_add(0xFF);    // 0xFF is -1 twos complement
        self.set_r(Rd, r);
    }

    #[allow(non_snake_case)]
    fn eor(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd xor Rr
        let mut rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        rd ^= rr;

        self.set_r(Rd, rd);
    }

    #[allow(non_snake_case)]
    fn fmul(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (unsigned) <= Rd (unsigned) x Rr (unsigned) << 1
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let mut word = u16::from(rd)*u16::from(rr);
        let c = ((1 << 15) & word) != 0;
        word <<= 1;

        self.set_rw(0, word);
        self.set_sreg_bit(BitSREG::C, c);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn fmuls(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (signed) <= Rd (signed) x Rr (signed) << 1
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let mut word = i16::from(rd as i8)*i16::from(rr as i8);
        let c = ((1 << 15) & word) != 0;
        word <<= 1;

        self.set_rw(0, word as u16);
        self.set_sreg_bit(BitSREG::C, c);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn fmulsu(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (signed) <= Rd (signed) x Rr (unsigned) << 1
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let mut word = i16::from(rd as i8)*i16::from(rr);
        let c = ((1 << 15) & word) != 0;
        word <<= 1;

        self.set_rw(0, word as u16);
        self.set_sreg_bit(BitSREG::C, c);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn inc(&mut self, Rd: u8) {
        // Rd <= Rd + 1; (Carry bit unchanged)
        let mut r = self.get_r(Rd);
        (r,_) = r.overflowing_add(1);
        self.set_r(Rd, r);
    }

    #[allow(non_snake_case)]
    fn mul(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (unsigned) <= Rd (unsigned) x Rr (unsigned)
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let word = u16::from(rd)*u16::from(rr);
        let c = ((1 << 15) & word) != 0;

        self.set_rw(0, word);
        self.set_sreg_bit(BitSREG::C, c);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn muls(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (signed) <= Rd (signed) x Rr (signed)
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let word = i16::from(rd as i8)*i16::from(rr as i8);
        let c = ((1 << 15) & word) != 0;

        self.set_rw(0, word as u16);
        self.set_sreg_bit(BitSREG::C, c);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn mulsu(&mut self, Rd: u8, Rr: u8) {
        // R1:R0 (signed) <= Rd (signed) x Rr (unsigned)
        let rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        let word = i16::from(rd as i8)*i16::from(rr);
        let c = ((1 << 15) & word) != 0;

        self.set_rw(0, word as u16);
        self.set_sreg_bit(BitSREG::C, c);
        self.set_sreg_bit(BitSREG::Z, word == 0);
    }

    #[allow(non_snake_case)]
    fn neg(&mut self, Rd: u8) {
        // Rd <= ~Rd + 1 (i.e twos-complement)
        let mut r = self.get_r(Rd);
        r = !r;
        (r,_) = r.overflowing_add(1);
        self.set_r(Rd, r);
    }

    #[allow(non_snake_case)]
    fn or(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd or Rr
        let mut rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 

        rd |= rr;

        self.set_r(Rd, rd);
    }

    #[allow(non_snake_case)]
    fn ori(&mut self, Rd: u8, val: u8) {
        // Rd <- Rd or val
        let mut rd = self.get_r(Rd); 

        rd |= val;

        self.set_r(Rd, rd);
    }

    #[allow(non_snake_case)]
    fn sbc(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd - Rr - C
        let mut rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 
        let mut c = self.get_sreg_bit(BitSREG::C);

        if c {
            (rd, c) = rd.overflowing_sub(1);
            if c {
                // We overflowed so cant overflow again
                rd -= rr;
            } else {
                // We could still overflow
                (rd, c) = rd.overflowing_sub(rr);
            }
        } else {
            // Carry bit is not set, normal overflowing add
            (rd, c) = rd.overflowing_sub(rr);
        }

        self.set_r(Rd, rd);

        self.set_sreg_bit(BitSREG::C, c);
    }

    #[allow(non_snake_case)]
    fn sbci(&mut self, Rd: u8, val: u8) {
        // Rd <- Rd - val - C
        let mut rd = self.get_r(Rd); 
        let mut c = self.get_sreg_bit(BitSREG::C);

        if c {
            (rd, c) = rd.overflowing_sub(1);
            if c {
                // We overflowed so cant overflow again
                rd -= val;
            } else {
                // We could still overflow
                (rd, c) = rd.overflowing_sub(val);
            }
        } else {
            // Carry bit is not set, normal overflowing add
            (rd, c) = rd.overflowing_sub(val);
        }

        self.set_r(Rd, rd);

        self.set_sreg_bit(BitSREG::C, c);
    }

    #[allow(non_snake_case)]
    fn sbiw(&mut self, Rd: u8, val: u8) {
        // R[d+1]:Rd <- R[d+1]:Rd - val
        let mut word = self.get_rw(Rd);
        let c: bool;
        (word, c) = word.overflowing_sub(u16::from(val));
        
        self.set_rw(Rd, word);

        self.set_sreg_bit(BitSREG::C, c)
    }

    #[allow(non_snake_case)]
    fn sub(&mut self, Rd: u8, Rr: u8) {
        // Rd <- Rd - Rr
        let mut rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 
        let c: bool;

        (rd, c) = rd.overflowing_sub(rr);

        self.set_r(Rd, rd);

        self.set_sreg_bit(BitSREG::C, c)
    }

    #[allow(non_snake_case)]
    fn subi(&mut self, Rd: u8, val: u8) {
        // Rd <- Rd + Rr
        let mut rd = self.get_r(Rd); 
        let c: bool;

        (rd, c) = rd.overflowing_sub(val);

        self.set_r(Rd, rd);

        self.set_sreg_bit(BitSREG::C, c)
    }

    fn brbx(&mut self, bit: u8, offset: i8, set: bool) -> u8 {
        if !set ^ self.get_sreg_bit(BitSREG::from(bit)) {
            let mut pc = self.pc as i32;
            pc += i32::from(offset);
            self.pc = pc as u16;
            return 1;
        } else {
            return 0;
        } 
    }

    fn call(&mut self, address: u32) {
        let mut ds = self.ds.borrow_mut();
        ds.write(usize::from(self.sp), (self.pc+1) as u8); self.sp -= 1;
        ds.write(usize::from(self.sp), ((self.pc+1)>>8) as u8); self.sp -= 1;
        self.pc = address as u16;
    }

    #[allow(non_snake_case)]
    fn cp(&mut self, Rd: u8, Rr: u8) {
        // Rd - Rr
        let mut rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 
        let c: bool;

        (rd, c) = rd.overflowing_sub(rr);

        self.set_sreg_bit(BitSREG::C, c)
    }

    #[allow(non_snake_case)]
    fn cpc(&mut self, Rd: u8, Rr: u8) {
        // Rd - Rr - C
        let mut rd = self.get_r(Rd); 
        let rr = self.get_r(Rr); 
        let mut c = self.get_sreg_bit(BitSREG::C);

        if c {
            (rd, c) = rd.overflowing_sub(1);
            if c {
                // We overflowed so cant overflow again
                rd -= rr;
            } else {
                // We could still overflow
                (rd, c) = rd.overflowing_sub(rr);
            }
        } else {
            // Carry bit is not set, normal overflowing sub
            (rd, c) = rd.overflowing_sub(rr);
        }

        self.set_sreg_bit(BitSREG::C, c);
    }

    #[allow(non_snake_case)]
    fn cpi(&mut self, Rd: u8, val: u8) {
        // Rd - val
        let mut rd = self.get_r(Rd); 
        let c: bool;

        (rd, c) = rd.overflowing_sub(val);

        self.set_sreg_bit(BitSREG::C, c)
    }

    fn cpse(&mut self, Rd: u8, Rr: u8) -> u8 {
        use Instruction::*;

        if Rd == Rr {
            let opcode = self.progmem.borrow().read_word(usize::from(self.pc << 1)).0;
            let op = Instruction::decode(opcode);
            match op {
                CALL{..} | JMP{..} | LDS{..} | STS{..} => {self.pc += 2; 2},
                _ => {self.pc += 1; 1},
            }
        } else {
            0
        }
    }

    pub fn tick(&mut self) -> bool {
        use Instruction::*; 

        let opcode = self.progmem.borrow().read_word(usize::from(self.pc << 1)).0;
        let op = Instruction::decode(opcode);

        println!("[0x{:04X}] {:?}", self.pc, op);

        //Most instructions are single cycle so do this first
        self.pc += 1;

        match op {
            // Control
            BREAK   => {return false}, // NOP if OCD disabled
            NOP     => {},
            SLEEP   => {}, // Not implemented
            WDR     => {}, // Not implemented
            // Arithmetic
            ADC     {Rd, Rr}    => {self.adc(Rd,Rr)},
            ADD     {Rd, Rr}    => {self.add(Rd,Rr)},
            ADIW    {Rd, val}   => {self.adiw(Rd, val); self.busy = 1},
            ANDI    {Rd, val}   => {self.andi(Rd, val)},
            COM     {Rd}            => {self.com(Rd)},
            DEC     {Rd}            => {self.dec(Rd)},
            EOR     {Rd, Rr}    => {self.eor(Rd,Rr)},
            FMUL    {Rd, Rr}    => {self.fmul(Rd,Rr); self.busy = 1},
            FMULS   {Rd, Rr}    => {self.fmuls(Rd,Rr); self.busy = 1},
            FMULSU  {Rd, Rr}    => {self.fmulsu(Rd,Rr); self.busy = 1},
            INC     {Rd}            => {self.inc(Rd)},
            MUL     {Rd, Rr}    => {self.mul(Rd,Rr); self.busy = 1},
            MULS    {Rd, Rr}    => {self.muls(Rd,Rr); self.busy = 1},
            MULSU   {Rd, Rr}    => {self.mulsu(Rd,Rr); self.busy = 1},
            NEG     {Rd}            => {self.neg(Rd)},
            OR      {Rd, Rr}    => {self.or(Rd, Rr)},
            ORI     {Rd, val}   => {self.ori(Rd, val)},
            SBC     {Rd, Rr}    => {self.sbc(Rd, Rr)},
            SBCI    {Rd,val}    => {self.sbci(Rd, val)},
            SBIW    {Rd,val}    => {self.sbiw(Rd, val); self.busy = 1},
            SUB     {Rd, Rr}    => {self.sub(Rd, Rr)},
            SUBI    {Rd, val}   => {self.subi(Rd, val)},
            //Flow
            BRBC    {offset, bit}   => {self.busy = self.brbx(bit, offset, false)},
            BRBS    {offset, bit}   => {self.busy = self.brbx(bit, offset, true)},
            CALL    {address}          => {self.call(address); self.busy = 2},
            CP      { Rd, Rr }      => {self.cp(Rd, Rr)},
            CPC     { Rd, Rr }      => {self.cpc(Rd, Rr)},
            CPI     { Rd, val }     => {self.cpi(Rd, val)},
            CPSE    { Rd, Rr }      => {self.busy  = self.cpse(Rd, Rr)},
            ICALL                           => {},
            IJMP                            => {},
            JMP { address }            => {},
            RCALL { offset }           => {},
            RET                             => {},
            RETI                            => {},
            RJMP { offset }            => {},
            SBIC { ioreg, bit }     => {},
            SBIS { ioreg, bit }     => {},
            SBRC { Rr, bit }        => {},
            SBRS { Rr, bit }        => {},

            
            UNDEF   => { panic!("[0x{:04X}] Undefined opcode: {:b}", self.pc, opcode) },
            _       => { panic!("Unhandled instruction!") }
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
    fn decode (opcode: u16) -> Instruction {
        #[bitmatch]
        match opcode {
            "0000_0000_0000_0000" => Instruction::NOP,
            "0000_11rd_dddd_rrrr" => Instruction::ADD { Rd: d as u8, Rr: r as u8 },
            "0001_11rd_dddd_rrrr" if r != d => Instruction::ADC { Rd: d as u8, Rr: r as u8},
            "1001_0110_kkdd_kkkk" => Instruction::ADIW { 
                                        Rd: {
                                            ((d as u8)<<1)+24
                                        }, 
                                        val: k as u8 
                                    },
            "0010_00rd_dddd_rrrr" => Instruction::AND { Rd: d as u8, Rr: r as u8 },
            "0111_kkkk_dddd_kkkk" => Instruction::ANDI { Rd: d as u8, val: k as u8 },
            "1001_010d_dddd_0101" => Instruction::ASR { Rd: d as u8 },
            "1001_0100_1sss_1000" => Instruction::BCLR { bit: s as u8 },
            "1111_100d_dddd_0bbb" => Instruction::BLD { Rd: d as u8, bit: b as u8 },
            "1111_01kk_kkkk_ksss" => Instruction::BRBC { offset: k as i8, bit: s as u8 },
            "1111_00kk_kkkk_ksss" => Instruction::BRBS { offset: k as i8, bit: s as u8 },
            "1001_0101_1001_1000" => Instruction::BREAK,
            "1001_0100_0sss_1000" => Instruction::BSET { bit: s as u8 },
            "1111_101d_dddd_0bbb" => Instruction::BST { Rd: d as u8, bit: b as u8 },
            "1001_010k_kkkk_111k" => Instruction::CALL { 
                                        address: {
                                            // Fetch next word in programme memory
                                            (k as u32) << 16 
                                        }
                                    },
            "1001_1000_aaaa_abbb" => Instruction::CBI { ioreg: a as u8, bit: b as u8 },
            "1001_010d_dddd_0000" => Instruction::COM { Rd: d as u8 },
            "0001_01rd_dddd_rrrr" => Instruction::CP { Rd: d as u8, Rr: r as u8 },
            "0000_01rd_dddd_rrrr" => Instruction::CPC { Rd: d as u8, Rr: r as u8 },
            "0011_kkkk_dddd_kkkk" => Instruction::CPI { Rd: d as u8, val: k as u8 },
            "0001_00rd_dddd_rrrr" => Instruction::CPSE { Rd: d as u8, Rr: r as u8 },
            "1001_010d_dddd_1010" => Instruction::DEC { Rd: d as u8 },
            "0010_01rd_dddd_rrrr" => Instruction::EOR { Rd: d as u8, Rr: r as u8 },
            "0000_0011_0ddd_1rrr" => Instruction::FMUL { Rd: d as u8, Rr: r as u8 },
            "0000_0011_1ddd_0rrr" => Instruction::FMULS { Rd: d as u8, Rr: r as u8 },
            "0000_0011_1ddd_1rrr" => Instruction::FMULSU { Rd: d as u8, Rr: r as u8 },
            "1001_0101_0000_1001" => Instruction::ICALL,
            "1001_0100_0000_1001" => Instruction::IJMP,
            "1011_0aad_dddd_aaaa" => Instruction::IN { Rd: d as u8, ioreg: a as u8 },
            "1001_010d_dddd_0011" => Instruction::INC { Rd: d as u8 },
            "1001_010k_kkkk_110k" => Instruction::JMP { address: k as u32}, //FIX
            "1001_000d_dddd_1100" => Instruction::LDX { Rd: d as u8 },
            "1001_000d_dddd_1110" => Instruction::LDXdec { Rd: d as u8 },
            "1001_000d_dddd_1101" => Instruction::LDXinc { Rd: d as u8 },
            "1001_000d_dddd_1010" => Instruction::LDYdec { Rd: d as u8 },
            "1001_000d_dddd_1001" => Instruction::LDYinc { Rd: d as u8 },
            "1001_000d_dddd_0010" => Instruction::LDZdec { Rd: d as u8 },
            "1001_000d_dddd_0001" => Instruction::LDZinc { Rd: d as u8 },
            "10q0_qq0d_dddd_1qqq" => Instruction::LDDY { Rd: d as u8, offset: q as u8 },
            "10q0_qq0d_dddd_0qqq" => Instruction::LDDZ { Rd: d as u8, offset: q as u8 },
            "1110_kkkk_dddd_kkkk" => Instruction::LDI { Rd: d as u8, val: k as u8 },
            "1001_000d_dddd_0000" => Instruction::LDS { Rd: d as u8, address: 0 as u16 }, //FIX
            "1001_0101_1100_1000" => Instruction::LPM,
            "1001_000d_dddd_0100" => Instruction::LPMRdZ { Rd: d as u8 },
            "1001_000d_dddd_0101" => Instruction::LPMRdZinc { Rd: d as u8 },
            "0000_11dd_dddd_dddd" => Instruction::LSL { Rd: d as u8 },
            "1001_010d_dddd_0110" => Instruction::LSR { Rd: d as u8 },
            "0010_11rd_dddd_rrrr" => Instruction::MOV { Rd: d as u8, Rr: r as u8 },
            "0000_0001_dddd_rrrr" => Instruction::MOVW { Rd: d as u8, Rr: r as u8 },
            "1001_11rd_dddd_rrrr" => Instruction::MUL { Rd: d as u8, Rr: r as u8 },
            "0000_0010_dddd_rrrr" => Instruction::MULS { Rd: d as u8, Rr: r as u8 },
            "0000_0011_0ddd_0rrr" => Instruction::MULSU { Rd: d as u8, Rr: r as u8 },
            "1001_010d_dddd_0001" => Instruction::NEG { Rd: d as u8 },
            "0000_0000_0000_0000" => Instruction::NOP,
            "0010_10rd_dddd_rrrr" => Instruction::OR { Rd: d as u8, Rr: r as u8 },
            "0110_kkkk_dddd_kkkk" => Instruction::ORI { Rd: d as u8, val: k as u8 },
            "1011_1aar_rrrr_aaaa" => Instruction::OUT { Rr: r as u8, ioreg: a as u8 },
            "1001_000d_dddd_1111" => Instruction::POP { Rd: d as u8 },
            "1001_001r_rrrr_1111" => Instruction::PUSH { Rr: r as u8 },
            "1101_kkkk_kkkk_kkkk" => Instruction::RCALL { offset: k as i16 }, //CHECK?
            "1001_0101_0000_1000" => Instruction::RET,
            "1001_0101_0001_1000" => Instruction::RETI,
            "1100_kkkk_kkkk_kkkk" => Instruction::RJMP { offset: k as i16 }, //CHECK
            "0001_11rd_dddd_rrrr" if r == d => Instruction::ROL { Rd: d as u8 },
            "1001_010d_dddd_0111" => Instruction::ROR { Rd: d as u8 },
            "0000_10rd_dddd_rrrr" => Instruction::SBC { Rd: d as u8, Rr: r as u8 },
            "0100_kkkk_dddd_kkkk" => Instruction::SBCI { Rd: d as u8, val: k as u8 },
            "1001_1010_aaaa_abbb" => Instruction::SBI { ioreg: a as u8, bit: b as u8 },
            "1001_1001_aaaa_abbb" => Instruction::SBIC { ioreg: a as u8, bit: b as u8 },
            "1001_1011_aaaa_abbb" => Instruction::SBIS { ioreg: a as u8, bit: b as u8 },
            "1001_0111_kkdd_kkkk" => Instruction::SBIW { Rd: d as u8, val: k as u8 },
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
            "1001_001r_rrrr_0000" => Instruction::STS { Rr: r as u8, address: 0 as u16 }, //FIX
            "0001_10rd_dddd_rrrr" => Instruction::SUB { Rd: d as u8, Rr: r as u8 },
            "0101_kkkk_dddd_kkkk" => Instruction::SUBI { Rd: d as u8, val: k as u8 },
            "1001_010d_dddd_0010" => Instruction::SWAP { Rd: d as u8 },
            "1001_0101_1010_1000" => Instruction::WDR,
            _ => Instruction::UNDEF
        }
    }
}