use super::cores::Core;
use super::cores::CoreType;
use super::memory::Memory;
use super::memory::MemoryMap;
use super::memory::MemoryMapped;

use crate::cores::InterruptHandler;
use crate::hardware::Hardware;
use crate::peripherals::adc::Adc;
use crate::peripherals::clkctrl::Clkctrl;
use crate::peripherals::cpu::Cpu;
use crate::peripherals::cpuint::Cpuint;
use crate::peripherals::port::{Port, VirtualPort};
use crate::peripherals::portmux::Portmux;
use crate::peripherals::spi::Spi;
use crate::peripherals::stdio::Stdio;
use crate::peripherals::tca::Tca;
use crate::peripherals::tcb::Tcb;
use crate::peripherals::usart::Usart;
use crate::peripherals::ClockSource;
use crate::peripherals::Clocked;
use crate::peripherals::InterruptSource;

use std::cell::RefCell;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::rc::Rc;

use ihex::Reader;
use ihex::Record;

pub enum DeviceType {
    ATtiny1626,
}

#[allow(non_snake_case)]
pub struct Device {
    pub core: Core,
    pub flash: Rc<RefCell<dyn MemoryMapped>>,
    pub sram: Rc<RefCell<dyn MemoryMapped>>,
    pub mm: Rc<RefCell<dyn MemoryMapped>>,
    pub ports: Vec<Rc<RefCell<Port>>>,
    pub stdio: Rc<RefCell<Stdio>>,
    clock_source: Rc<RefCell<dyn ClockSource>>,
    clocked: Vec<Rc<RefCell<dyn Clocked>>>,
    RAMEND: u16,
}

impl Device {
    pub fn new(dt: DeviceType) -> Self {
        match dt {
            DeviceType::ATtiny1626 => {
                // Constants
                const RAMEND: u16 = 0x3FFF;

                // Clocking
                let clkctrl = Rc::new(RefCell::new(Clkctrl::new()));

                // Cpu
                let cpu = Rc::new(RefCell::new(Cpu::new(vec![clkctrl.clone()])));

                // Memories
                let flash: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new(16 * 1024, 0xFF, 0)));
                let sram: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new(2 * 1024, 0x00, 0)));
                let gpio: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new(4, 0x00, 0)));

                // Read only
                let syscfg: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00, 0x04], 0))); // Rev E (0x04?) is inital release
                let fuse: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00, 0x00, 0x7E], 0)));

                // Placeholder
                let eeprom: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00; 256], 0))); // Should this read 0xFF?
                let userrow: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00; 0x80], 0))); // Should this read 0xFF?

                // Ports
                let porta = Rc::new(RefCell::new(Port::new("PORTA".to_string())));
                let portb = Rc::new(RefCell::new(Port::new("PORTB".to_string())));
                let portc = Rc::new(RefCell::new(Port::new("PORTC".to_string())));

                let vporta = Rc::new(RefCell::new(VirtualPort {
                    port: Rc::clone(&porta),
                }));
                let vportb = Rc::new(RefCell::new(VirtualPort {
                    port: Rc::clone(&portb),
                }));
                let vportc = Rc::new(RefCell::new(VirtualPort {
                    port: Rc::clone(&portc),
                }));

                let portmux = Rc::new(RefCell::new(Portmux::new("PORTMUX".to_string())));

                let ports = vec![Rc::clone(&porta), Rc::clone(&portb), Rc::clone(&portc)];

                let spi0 = Rc::new(RefCell::new(Spi::new(
                    "SPI0".to_string(),
                    Rc::clone(&porta),
                    [1, 2, 3, 4],
                    Rc::clone(&portc),
                    [2, 1, 0, 3],
                )));
                spi0.borrow_mut().mux_alt = true;

                let usart0 = Rc::new(RefCell::new(Usart::new(
                    "USART0".to_string(),
                    Rc::clone(&portb),
                    [3, 2, 1, 0],
                    Rc::clone(&porta),
                    [2, 1, 3, 4],
                )));

                let usart1 = Rc::new(RefCell::new(Usart::new(
                    "USART1".to_string(),
                    Rc::clone(&porta),
                    [2, 1, 3, 4],
                    Rc::clone(&portc),
                    [1, 2, 0, 3],
                )));

                let tca0 = Rc::new(RefCell::new(Tca::new(
                    "TCA0".to_string(),
                    Rc::clone(&portb),
                    [0, 1, 3],
                    [3, 4, 5],
                )));
                let tcb0 = Rc::new(RefCell::new(Tcb::new("TCB0".to_string())));
                let tcb1 = Rc::new(RefCell::new(Tcb::new("TCB1".to_string())));

                let adc0 = Rc::new(RefCell::new(Adc::new(
                    "ADC0".to_string(),
                    [Rc::clone(&porta), Rc::clone(&portb), Rc::clone(&portc)],
                    [
                        (0, 1),
                        (0, 2),
                        (0, 3),
                        (0, 4),
                        (0, 5),
                        (0, 6),
                        (0, 7),
                        (1, 5),
                        (1, 4),
                        (1, 1),
                        (1, 0),
                        (2, 0),
                        (2, 1),
                        (2, 2),
                        (2, 3),
                    ],
                )));

                let clocked = vec![
                    cpu.clone() as Rc<RefCell<dyn Clocked>>,
                    spi0.clone() as Rc<RefCell<dyn Clocked>>,
                    tca0.clone() as Rc<RefCell<dyn Clocked>>,
                    tcb0.clone() as Rc<RefCell<dyn Clocked>>,
                    tcb1.clone() as Rc<RefCell<dyn Clocked>>,
                    adc0.clone() as Rc<RefCell<dyn Clocked>>,
                    usart0.clone() as Rc<RefCell<dyn Clocked>>,
                    usart1.clone() as Rc<RefCell<dyn Clocked>>,
                ];

                let stdio = Rc::new(RefCell::new(Stdio::new(
                    "STDIO".to_string(),
                    "stdout.txt".to_string(),
                )));

                let cpuint = Rc::new(RefCell::new(Cpuint::new()));
                cpuint.borrow_mut().add_source(
                    8,
                    tca0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x01,
                ); // OVF
                cpuint.borrow_mut().add_source(
                    10,
                    tca0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x10,
                ); // CMP0
                cpuint.borrow_mut().add_source(
                    11,
                    tca0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x20,
                ); // CMP1
                cpuint.borrow_mut().add_source(
                    12,
                    tca0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x40,
                ); // CMP2
                cpuint.borrow_mut().add_source(
                    13,
                    tcb0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x03,
                );
                cpuint.borrow_mut().add_source(
                    16,
                    spi0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0xF1,
                );
                cpuint.borrow_mut().add_source(
                    17,
                    usart0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x80,
                ); // RCX
                cpuint.borrow_mut().add_source(
                    18,
                    usart0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x20,
                ); // DRE
                cpuint.borrow_mut().add_source(
                    19,
                    usart0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x40,
                ); // TXC
                cpuint.borrow_mut().add_source(
                    22,
                    adc0.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x01,
                ); // RESRDY
                cpuint.borrow_mut().add_source(
                    25,
                    tcb1.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x03,
                );
                cpuint.borrow_mut().add_source(
                    26,
                    usart1.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x80,
                ); // RCX
                cpuint.borrow_mut().add_source(
                    27,
                    usart1.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x20,
                ); // DRE
                cpuint.borrow_mut().add_source(
                    28,
                    usart1.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0x40,
                ); // TXC
                cpuint.borrow_mut().add_source(
                    6,
                    porta.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0xFF,
                ); // PORTA
                cpuint.borrow_mut().add_source(
                    7,
                    portb.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0xFF,
                ); // PORTB
                cpuint.borrow_mut().add_source(
                    24,
                    portc.clone() as Rc<RefCell<dyn InterruptSource>>,
                    0xFF,
                ); // PORTC

                // Not implemented
                let slpctrl: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00; 0x01], 0)));
                let bod: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00; 0x0C], 0)));
                let twi: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00; 0x0F], 0)));
                let crcscan: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00; 0x03], 0)));
                let ac0: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00; 0x08], 0)));
                let nvmctrl: Rc<RefCell<dyn MemoryMapped>> =
                    Rc::new(RefCell::new(Memory::new_rom(vec![0x00; 0x09], 0)));

                let mut mm = MemoryMap::new();

                // PERIPHERAL MAP
                //  0: [0x0000] VPORTA
                mm.add(0x0000, vporta.clone() as Rc<RefCell<dyn MemoryMapped>>);
                //  1: [0x0004] VPORTB
                mm.add(0x0004, vportb.clone() as Rc<RefCell<dyn MemoryMapped>>);
                //  2: [0x0008] VPORTC
                mm.add(0x0008, vportc.clone() as Rc<RefCell<dyn MemoryMapped>>);
                //  3: [0x001C] GPIO
                mm.add(0x001C, Rc::clone(&gpio));
                //  4: [0x0030] CPU
                mm.add(0x0030, cpu.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // xx: [0x0040] RSTCTRL
                //  5: [0x0050] SLPCTRL
                mm.add(0x0050, Rc::clone(&slpctrl));
                //  6: [0x0060] CLKCTRL
                mm.add(0x0060, clkctrl.clone() as Rc<RefCell<dyn MemoryMapped>>);
                //  7: [0x0080] BOD
                mm.add(0x0080, Rc::clone(&bod));
                // xx: [0x00A0] VREF
                // xx: [0x0100] WDT
                //  8: [0x0110] CPUINT
                mm.add(0x0110, cpuint.clone() as Rc<RefCell<dyn MemoryMapped>>);
                //  9: [0x0120] CRCSCAN (not implemented)
                mm.add(0x0120, Rc::clone(&crcscan));
                // xx: [0x0140] RTC
                // xx: [0x0180] EVSYS
                // xx: [0x01C0] CCL
                // 10: [0x0400] PORTA
                mm.add(0x0400, porta.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 11: [0x0420] PORTB
                mm.add(0x0420, portb.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 12: [0x0440] PORTC
                mm.add(0x0440, portc.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 13: [0x05E0] PORTMUX
                mm.add(0x05E0, portmux.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 14: [0x0600] ADC0
                mm.add(0x0600, adc0.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 15: [0x0680] AC0
                mm.add(0x0680, Rc::clone(&ac0));
                // 16: [0x0800] USART0
                mm.add(0x0800, usart0.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 17: [0x0820] USART1
                mm.add(0x0820, usart1.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 18: [0x08A0] TWI0
                mm.add(0x08A0, Rc::clone(&twi));
                // 19: [0x08C0] SPI0
                mm.add(0x08C0, spi0.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 20: [0x0A00] TCA0
                mm.add(0x0A00, tca0.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 21: [0x0A80] TCB0
                mm.add(0x0A80, tcb0.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 22: [0x0A90] TCB1
                mm.add(0x0A90, tcb1.clone() as Rc<RefCell<dyn MemoryMapped>>);
                // 23: [0x0F00] SYSCFG
                mm.add(0x0F00, Rc::clone(&syscfg));
                // 24: [0x1000] NVMCTRL
                mm.add(0x1000, Rc::clone(&nvmctrl));

                // SYSTEM MEMORY MAP
                // xx: [0x1100] SIGROW
                // xx: [0x1200-127F] RESERVED
                // 25: [0x1280] FUSE
                mm.add(0x1280, Rc::clone(&fuse));
                // xx: [0x128A] LOCKBIT
                // 26: [0x1300] USERROW
                mm.add(0x1300, Rc::clone(&userrow));
                // xx: [0x1380-13FF] RESERVED
                // 27: [0x1400] EEPROM
                mm.add(0x1400, Rc::clone(&eeprom));

                // [0x1500-0x37FF] RESERVED

                // 28: [0x1500]
                mm.add(0x1500, stdio.clone() as Rc<RefCell<dyn MemoryMapped>>);

                // 29: [0x3800] SRAM
                mm.add(0x3800, Rc::clone(&sram));

                // [0x4000-0x7FFF] RESERVED

                // 30: [0x8000] FLASH
                mm.add(0x8000, Rc::clone(&flash));

                // [0xC000-0xFFFF] RESERVED

                let mm: Rc<RefCell<dyn MemoryMapped>> = Rc::new(RefCell::new(mm));

                Device {
                    core: Core::new(
                        CoreType::AVRxt,
                        Rc::clone(&mm),
                        Rc::clone(&flash),
                        cpuint.clone() as Rc<RefCell<dyn InterruptHandler>>,
                        RAMEND,
                    ),
                    flash,
                    sram,
                    mm,
                    ports,
                    clock_source: clkctrl.clone() as Rc<RefCell<dyn ClockSource>>,
                    clocked,
                    stdio,
                    RAMEND,
                }
            }
        }
    }

    pub fn load_hex(&mut self, filename: &str) {
        let path = Path::new(filename);
        let display = path.display();

        // Open the path in read-only mode, returns `io::Result<File>`
        let mut file = match File::open(path) {
            Err(why) => panic!("Couldn't open {}: {}", display, why),
            Ok(file) => file,
        };

        // Read the file contents into a string, returns `io::Result<usize>`
        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(why) => panic!("Couldn't read {}: {}", display, why),
            Ok(_) => {
                let hex = Reader::new(&s);
                for r in hex {
                    if let Record::Data { offset, value } = r.unwrap() {
                        let mut address = usize::from(offset);
                        for b in value {
                            self.flash.borrow_mut().write(address, b);
                            address += 1;
                        }
                    }
                }
            }
        };
    }

    pub fn tick(&mut self, time: u64) -> u64 {
        let result = self.core.tick();

        for dev in &self.clocked {
            dev.borrow_mut().tick(time);
        }

        if result {
            self.clock_source.borrow().clock_period()
        } else {
            0 // Flag termination by core
        }
    }

    pub fn update(&mut self, time: u64) {
        for port in &self.ports {
            port.borrow_mut().update(time);
        }
    }

    pub fn dump_regs(&self) {
        for i in 0..=31 {
            println!("[R{:02}] 0x{:02X}", i, self.core.get_register(i));
        }
    }

    pub fn dump_stack(&self) {
        let mut sp = self.core.get_stack_pointer();
        while sp < self.RAMEND {
            sp += 1;
            println!(
                "[STACK+{:03X}] 0x{:02X}",
                self.RAMEND - sp,
                self.mm.borrow_mut().read(usize::from(sp)).0
            )
        }
    }
}
