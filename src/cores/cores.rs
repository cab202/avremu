pub mod cores {
    pub enum CoreType {
        AVR,
        AVRe,
        AVReplus,
        AVRxm, 
        AVRxt,
        AVRrc
    }

    struct Core {
        regs:  [u8; 32],
        sreg: u8,
        pc: u16,
        sp: u16,
        busy: u8
    }

}