use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use crate::devices::Device;
use crate::devices::DeviceType;
use crate::hardware::Hardware;
use crate::nets::Net;
use crate::hardware::led::Led;

pub struct QUTy {
    hw: HashMap<String, Box<dyn Hardware>>,
    nets: HashMap<String, Rc<RefCell<Net>>>,
    mcu: Device,
    time: usize
}

impl QUTy {
    pub fn new() -> Self {

        let nets = HashMap::from([
            ("PA1_DISP_LATCH".to_string(), Rc::new(RefCell::new(Net::new("PA1_DISP_LATCH".to_string())))),
            ("PA2_POT".to_string(), Rc::new(RefCell::new(Net::new("PA2_POT".to_string())))),
            ("PA3_CLK".to_string(), Rc::new(RefCell::new(Net::new("PA3_CLK".to_string())))),
            ("PA4_BUTTON0".to_string(), Rc::new(RefCell::new(Net::new("PA4_BUTTON0".to_string())))),
            ("PA5_BUTTON1".to_string(), Rc::new(RefCell::new(Net::new("PA5_BUTTON1".to_string())))),
            ("PA6_BUTTON2".to_string(), Rc::new(RefCell::new(Net::new("PA6_BUTTON2".to_string())))),
            ("PA7_BUTTON3".to_string(), Rc::new(RefCell::new(Net::new("PA7_BUTTON3".to_string())))),
            ("PB0_BUZZER".to_string(), Rc::new(RefCell::new(Net::new("PB0_BUZZER".to_string())))),
            ("PB1_DISP_EN".to_string(), Rc::new(RefCell::new(Net::new("PB1_DISP_EN".to_string())))),
            ("PB2_UART_TX".to_string(), Rc::new(RefCell::new(Net::new("PB2_UART_TX".to_string())))),
            ("PB3_UART_RX".to_string(), Rc::new(RefCell::new(Net::new("PB3_UART_RX".to_string())))),
            ("PB4_UART_RX".to_string(), Rc::new(RefCell::new(Net::new("PB4_UART_RX".to_string())))),
            ("PB5_DISP_DP".to_string(), Rc::new(RefCell::new(Net::new("PB5_DISP_DP".to_string())))),
            ("PC0_SPI_CLK".to_string(), Rc::new(RefCell::new(Net::new("PC0_SPI_CLK".to_string())))),
            ("PC1_SPI_MISO".to_string(), Rc::new(RefCell::new(Net::new("PC1_SPI_MISO".to_string())))),
            ("PC2_SPI_MOSI".to_string(), Rc::new(RefCell::new(Net::new("PC2_SPI_MOSI".to_string())))),
            ("PC3_SPI_CS".to_string(), Rc::new(RefCell::new(Net::new("PC3_SPI_CS".to_string()))))
        ]);

        let mut hw: HashMap::<String, Box<dyn Hardware>> = HashMap::new();
        hw.insert("DS1-DP".to_string(), Box::new(Led::new("DS1-DP".to_string(), false, Rc::clone(nets.get("PB5_DISP_DP").unwrap()))));            


        QUTy { 
            hw, 
            nets, 
            mcu: Device::new(DeviceType::ATtiny1626),
            time: 0
        }
    }

    pub fn step(&mut self) -> bool {
        self.time += 1;
        let result = self.mcu.tick();
        for net in &self.nets {
            net.1.borrow_mut().update();
        }
        for hw in &mut self.hw {
            hw.1.update(self.time);
        }
        result
    }

    pub fn core_debug(&mut self) {
        self.mcu.core.debug(true);
    }

    pub fn core_dumpregs(&self) {
        self.mcu.dump_regs();
    }

    pub fn mcu_dumpstack(&self) {
        self.mcu.dump_stack();
    }

    pub fn mcu_programme(&mut self, filename: &String) {
        self.mcu.load_hex(filename);
    }
}