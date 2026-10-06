use alloc::string::String;
use spin::Mutex;
use x86_64::instructions::port::Port;

const ATA_DATA: u16 = 0x1F0;
#[allow(dead_code)]
const ATA_ERROR_FEATURES: u16 = 0x1F1;
const ATA_SECTOR_COUNT: u16 = 0x1F2;
const ATA_LBA_LOW: u16 = 0x1F3;
const ATA_LBA_MID: u16 = 0x1F4;
const ATA_LBA_HIGH: u16 = 0x1F5;
const ATA_DRIVE_HEAD: u16 = 0x1F6;
const ATA_STATUS_COMMAND: u16 = 0x1F7;
const ATA_DEV_CONTROL: u16 = 0x3F6;

// Status Register Bits
const STATUS_BSY: u8 = 0x80;
const STATUS_DRDY: u8 = 0x40;
const STATUS_DF: u8 = 0x20;
const STATUS_DRQ: u8 = 0x08;
const STATUS_ERR: u8 = 0x01;

// Commands
const CMD_IDENTIFY: u8 = 0xEC;
const CMD_READ_SECTORS: u8 = 0x20;
const CMD_WRITE_SECTORS: u8 = 0x30;
const CMD_FLUSH_CACHE: u8 = 0xE7;

#[derive(Debug, Clone)]
pub struct AtaDriveInfo {
    pub model: String,
    pub sector_count: u64,
    pub capacity_bytes: u64,
}

pub struct AtaPioDriver {
    pub is_present: bool,
    pub info: Option<AtaDriveInfo>,
}

pub static ATA_PRIMARY_MASTER: Mutex<AtaPioDriver> = Mutex::new(AtaPioDriver {
    is_present: false,
    info: None,
});

impl AtaPioDriver {
    /// Initialize primary master ATA drive
    pub fn init(&mut self) -> bool {
        let mut dev_ctrl = Port::<u8>::new(ATA_DEV_CONTROL);
        let mut drive_head = Port::<u8>::new(ATA_DRIVE_HEAD);
        let mut status_port = Port::<u8>::new(ATA_STATUS_COMMAND);
        let mut sector_count_port = Port::<u8>::new(ATA_SECTOR_COUNT);
        let mut lba_low_port = Port::<u8>::new(ATA_LBA_LOW);
        let mut lba_mid_port = Port::<u8>::new(ATA_LBA_MID);
        let mut lba_high_port = Port::<u8>::new(ATA_LBA_HIGH);
        let mut data_port = Port::<u16>::new(ATA_DATA);

        unsafe {
            // Select Master drive (0xA0 = Master, 0xB0 = Slave)
            drive_head.write(0xA0);
            // Disable interrupts (polling mode)
            dev_ctrl.write(0x02);
            Self::io_wait();

            let status = status_port.read();
            if status == 0xFF {
                // Floating bus: no device attached
                self.is_present = false;
                self.info = None;
                return false;
            }

            // Zero LBA registers
            sector_count_port.write(0);
            lba_low_port.write(0);
            lba_mid_port.write(0);
            lba_high_port.write(0);

            // Send IDENTIFY command
            status_port.write(CMD_IDENTIFY);
            Self::io_wait();

            let status_after_cmd = status_port.read();
            if status_after_cmd == 0 {
                // Drive does not exist
                self.is_present = false;
                self.info = None;
                return false;
            }

            // Poll while busy
            let mut timeout = 100_000;
            while (status_port.read() & STATUS_BSY) != 0 && timeout > 0 {
                timeout -= 1;
                Self::io_wait();
            }
            if timeout == 0 {
                self.is_present = false;
                return false;
            }

            // Check if device is ATAPI (LBA mid and high are non-zero)
            let mid = lba_mid_port.read();
            let high = lba_high_port.read();
            if mid != 0 || high != 0 {
                // ATAPI or SATA device, not ATA PIO hard disk
                self.is_present = false;
                return false;
            }

            // Poll until DRQ (data request ready) or ERR
            timeout = 100_000;
            loop {
                let st = status_port.read();
                if (st & STATUS_ERR) != 0 || (st & STATUS_DF) != 0 {
                    self.is_present = false;
                    return false;
                }
                if (st & STATUS_DRQ) != 0 {
                    break;
                }
                timeout -= 1;
                if timeout == 0 {
                    self.is_present = false;
                    return false;
                }
                Self::io_wait();
            }

            // Read 256 16-bit words of IDENTIFY data
            let mut identify_buf = [0u16; 256];
            for word in identify_buf.iter_mut() {
                *word = data_port.read();
            }

            // Extract LBA28 sector count from words 60 and 61
            let sectors_28 = (identify_buf[60] as u32) | ((identify_buf[61] as u32) << 16);

            // Extract LBA48 sector count from words 100..103 if supported
            let sectors_48 = if (identify_buf[83] & (1 << 10)) != 0 {
                (identify_buf[100] as u64)
                    | ((identify_buf[101] as u64) << 16)
                    | ((identify_buf[102] as u64) << 32)
                    | ((identify_buf[103] as u64) << 48)
            } else {
                sectors_28 as u64
            };

            let sector_count = if sectors_48 > 0 {
                sectors_48
            } else {
                sectors_28 as u64
            };

            // Extract model name from words 27..46 (byte-swapped ASCII)
            let mut model_bytes = [0u8; 40];
            for i in 0..20 {
                let w = identify_buf[27 + i];
                model_bytes[i * 2] = (w >> 8) as u8;
                model_bytes[i * 2 + 1] = (w & 0xFF) as u8;
            }
            let model_str = core::str::from_utf8(&model_bytes)
                .unwrap_or("Unknown ATA Drive")
                .trim();

            let capacity_bytes = sector_count * 512;

            self.is_present = true;
            self.info = Some(AtaDriveInfo {
                model: String::from(model_str),
                sector_count,
                capacity_bytes,
            });

            crate::serial_println!(
                "[ATA] Detected Primary Master: '{}' ({} sectors, {} MB)",
                model_str,
                sector_count,
                capacity_bytes / (1024 * 1024)
            );

            true
        }
    }

    /// Read 512-byte sectors starting from LBA
    pub fn read_sectors(&mut self, lba: u64, count: u16, buffer: &mut [u8]) -> Result<(), &'static str> {
        if !self.is_present {
            return Err("ATA drive not present");
        }
        if buffer.len() < count as usize * 512 {
            return Err("Buffer too small for requested sector count");
        }

        let mut drive_head = Port::<u8>::new(ATA_DRIVE_HEAD);
        let mut sector_count_port = Port::<u8>::new(ATA_SECTOR_COUNT);
        let mut lba_low_port = Port::<u8>::new(ATA_LBA_LOW);
        let mut lba_mid_port = Port::<u8>::new(ATA_LBA_MID);
        let mut lba_high_port = Port::<u8>::new(ATA_LBA_HIGH);
        let mut status_port = Port::<u8>::new(ATA_STATUS_COMMAND);
        let mut data_port = Port::<u16>::new(ATA_DATA);

        for i in 0..count {
            let cur_lba = lba + i as u64;
            let offset = i as usize * 512;

            unsafe {
                Self::wait_ready()?;

                // LBA28 mode (drive 0): 0xE0 | top 4 bits of LBA
                drive_head.write(0xE0 | (((cur_lba >> 24) & 0x0F) as u8));
                Self::io_wait();

                sector_count_port.write(1);
                lba_low_port.write((cur_lba & 0xFF) as u8);
                lba_mid_port.write(((cur_lba >> 8) & 0xFF) as u8);
                lba_high_port.write(((cur_lba >> 16) & 0xFF) as u8);

                status_port.write(CMD_READ_SECTORS);
                Self::io_wait();

                Self::wait_drq()?;

                // Read 256 16-bit words
                for w_idx in 0..256 {
                    let word = data_port.read();
                    buffer[offset + w_idx * 2] = (word & 0xFF) as u8;
                    buffer[offset + w_idx * 2 + 1] = (word >> 8) as u8;
                }
            }
        }

        Ok(())
    }

    /// Write 512-byte sectors starting from LBA
    pub fn write_sectors(&mut self, lba: u64, count: u16, buffer: &[u8]) -> Result<(), &'static str> {
        if !self.is_present {
            return Err("ATA drive not present");
        }
        if buffer.len() < count as usize * 512 {
            return Err("Buffer too small for requested write count");
        }

        let mut drive_head = Port::<u8>::new(ATA_DRIVE_HEAD);
        let mut sector_count_port = Port::<u8>::new(ATA_SECTOR_COUNT);
        let mut lba_low_port = Port::<u8>::new(ATA_LBA_LOW);
        let mut lba_mid_port = Port::<u8>::new(ATA_LBA_MID);
        let mut lba_high_port = Port::<u8>::new(ATA_LBA_HIGH);
        let mut status_port = Port::<u8>::new(ATA_STATUS_COMMAND);
        let mut data_port = Port::<u16>::new(ATA_DATA);

        for i in 0..count {
            let cur_lba = lba + i as u64;
            let offset = i as usize * 512;

            unsafe {
                Self::wait_ready()?;

                // LBA28 mode (drive 0): 0xE0 | top 4 bits of LBA
                drive_head.write(0xE0 | (((cur_lba >> 24) & 0x0F) as u8));
                Self::io_wait();

                sector_count_port.write(1);
                lba_low_port.write((cur_lba & 0xFF) as u8);
                lba_mid_port.write(((cur_lba >> 8) & 0xFF) as u8);
                lba_high_port.write(((cur_lba >> 16) & 0xFF) as u8);

                status_port.write(CMD_WRITE_SECTORS);
                Self::io_wait();

                Self::wait_drq()?;

                // Write 256 16-bit words
                for w_idx in 0..256 {
                    let word = (buffer[offset + w_idx * 2] as u16)
                        | ((buffer[offset + w_idx * 2 + 1] as u16) << 8);
                    data_port.write(word);
                }

                // Wait until write cycle finishes
                Self::wait_ready()?;
            }
        }

        // Flush drive write cache
        self.flush_cache()?;
        Ok(())
    }

    /// Flush write cache to physical media
    pub fn flush_cache(&mut self) -> Result<(), &'static str> {
        if !self.is_present {
            return Ok(());
        }
        let mut status_port = Port::<u8>::new(ATA_STATUS_COMMAND);
        unsafe {
            Self::wait_ready()?;
            status_port.write(CMD_FLUSH_CACHE);
            Self::io_wait();
            Self::wait_ready()?;
        }
        Ok(())
    }

    unsafe fn wait_ready() -> Result<(), &'static str> {
        let mut status_port = Port::<u8>::new(ATA_STATUS_COMMAND);
        for _ in 0..200_000 {
            let st = unsafe { status_port.read() };
            if (st & STATUS_BSY) == 0 && (st & STATUS_DRDY) != 0 {
                return Ok(());
            }
            if (st & STATUS_DF) != 0 || (st & STATUS_ERR) != 0 {
                return Err("ATA drive error / fault");
            }
            unsafe { Self::io_wait() };
        }
        Err("ATA wait_ready timeout")
    }

    unsafe fn wait_drq() -> Result<(), &'static str> {
        let mut status_port = Port::<u8>::new(ATA_STATUS_COMMAND);
        for _ in 0..200_000 {
            let st = unsafe { status_port.read() };
            if (st & STATUS_ERR) != 0 || (st & STATUS_DF) != 0 {
                return Err("ATA drive error waiting for DRQ");
            }
            if (st & STATUS_BSY) == 0 && (st & STATUS_DRQ) != 0 {
                return Ok(());
            }
            unsafe { Self::io_wait() };
        }
        Err("ATA wait_drq timeout")
    }

    #[inline(always)]
    unsafe fn io_wait() {
        // Read status 4 times (~400ns delay for ATA controller state transition)
        let mut status_port = Port::<u8>::new(ATA_DEV_CONTROL);
        unsafe {
            status_port.read();
            status_port.read();
            status_port.read();
            status_port.read();
        }
    }
}

pub fn init() -> bool {
    ATA_PRIMARY_MASTER.lock().init()
}
