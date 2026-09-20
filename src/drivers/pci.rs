use x86_64::instructions::port::Port;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
}

impl PciDevice {
    pub fn read_u32(&self, offset: u8) -> u32 {
        unsafe { pci_read_32(self.bus, self.slot, self.func, offset) }
    }

    pub fn write_u32(&self, offset: u8, val: u32) {
        unsafe { pci_write_32(self.bus, self.slot, self.func, offset, val) }
    }

    pub fn read_bar(&self, bar_index: u8) -> u32 {
        assert!(bar_index < 6);
        let offset = 0x10 + (bar_index * 4);
        self.read_u32(offset)
    }

    pub fn enable_bus_master_and_memory(&self) {
        let cmd = self.read_u32(0x04);
        // Bit 0: I/O Space, Bit 1: Memory Space, Bit 2: Bus Master
        self.write_u32(0x04, cmd | 0x07);
    }
}

pub unsafe fn pci_read_32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC)
        | 0x8000_0000;

    let mut config_address = Port::<u32>::new(PCI_CONFIG_ADDRESS);
    let mut config_data = Port::<u32>::new(PCI_CONFIG_DATA);

    unsafe {
        config_address.write(address);
        config_data.read()
    }
}

pub unsafe fn pci_write_32(bus: u8, slot: u8, func: u8, offset: u8, val: u32) {
    let address = ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC)
        | 0x8000_0000;

    let mut config_address = Port::<u32>::new(PCI_CONFIG_ADDRESS);
    let mut config_data = Port::<u32>::new(PCI_CONFIG_DATA);

    unsafe {
        config_address.write(address);
        config_data.write(val);
    }
}

/// Scans PCI bus to find the primary display device (Class 0x03 or QEMU/Bochs BGA).
pub fn find_vga_device() -> Option<PciDevice> {
    for bus in 0..=1 {
        for slot in 0..32 {
            let vendor_and_device = unsafe { pci_read_32(bus, slot, 0, 0) };
            let vendor_id = (vendor_and_device & 0xFFFF) as u16;
            let device_id = ((vendor_and_device >> 16) & 0xFFFF) as u16;

            if vendor_id == 0xFFFF {
                continue;
            }

            let class_rev = unsafe { pci_read_32(bus, slot, 0, 0x08) };
            let class_code = ((class_rev >> 24) & 0xFF) as u8;
            let subclass = ((class_rev >> 16) & 0xFF) as u8;

            if class_code == 0x03 || (vendor_id == 0x1234 && device_id == 0x1111) {
                return Some(PciDevice {
                    bus,
                    slot,
                    func: 0,
                    vendor_id,
                    device_id,
                    class_code,
                    subclass,
                });
            }
        }
    }
    None
}
