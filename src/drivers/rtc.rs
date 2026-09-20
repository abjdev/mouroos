use x86_64::instructions::port::Port;

const CMOS_INDEX: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RtcTime {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

unsafe fn cmos_read(reg: u8) -> u8 {
    let mut index = Port::<u8>::new(CMOS_INDEX);
    let mut data = Port::<u8>::new(CMOS_DATA);
    unsafe {
        index.write(reg);
        data.read()
    }
}

unsafe fn is_update_in_progress() -> bool {
    unsafe { cmos_read(0x0A) & 0x80 != 0 }
}

fn bcd_to_bin(bcd: u8) -> u8 {
    (bcd & 0x0F) + ((bcd / 16) * 10)
}

pub fn read_time() -> RtcTime {
    unsafe {
        // Wait until RTC update is complete
        for _ in 0..10_000 {
            if !is_update_in_progress() {
                break;
            }
        }

        let mut seconds = cmos_read(0x00);
        let mut minutes = cmos_read(0x02);
        let mut hours = cmos_read(0x04);
        let mut day = cmos_read(0x07);
        let mut month = cmos_read(0x08);
        let mut year = cmos_read(0x09) as u16;

        let status_b = cmos_read(0x0B);

        // Convert BCD to binary if necessary
        if status_b & 0x04 == 0 {
            seconds = bcd_to_bin(seconds);
            minutes = bcd_to_bin(minutes);
            hours = bcd_to_bin(hours & 0x7F) | (hours & 0x80);
            day = bcd_to_bin(day);
            month = bcd_to_bin(month);
            year = bcd_to_bin(year as u8) as u16;
        }

        // Convert 12 hour clock to 24 hour clock if bit 1 of status_b is clear
        if status_b & 0x02 == 0 && (hours & 0x80) != 0 {
            hours = ((hours & 0x7F) + 12) % 24;
        }

        year += 2000;

        RtcTime {
            hours,
            minutes,
            seconds,
            year,
            month,
            day,
        }
    }
}

pub fn read_seconds() -> u8 {
    unsafe {
        let sec = cmos_read(0x00);
        let status_b = cmos_read(0x0B);
        if status_b & 0x04 == 0 {
            bcd_to_bin(sec)
        } else {
            sec
        }
    }
}
