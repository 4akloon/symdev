use std::time::{SystemTime, UNIX_EPOCH};

use super::field::SisEncode;

#[derive(Clone, Copy)]
pub struct SisDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl SisDate {
    pub const KIND: u32 = 6;

    pub fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    pub fn payload(&self) -> [u8; 4] {
        let mut out = [0u8; 4];
        out[..2].copy_from_slice(&self.year.to_le_bytes());
        out[2] = self.month;
        out[3] = self.day;
        out
    }
}

impl SisEncode for SisDate {
    const KIND: u32 = SisDate::KIND;

    fn payload(&self) -> Vec<u8> {
        SisDate::payload(self).to_vec()
    }
}

#[derive(Clone, Copy)]
pub struct SisTime {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl SisTime {
    pub const KIND: u32 = 7;

    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour,
            minute,
            second,
        }
    }

    pub fn payload(&self) -> [u8; 3] {
        [self.hour, self.minute, self.second]
    }
}

impl SisEncode for SisTime {
    const KIND: u32 = SisTime::KIND;

    fn payload(&self) -> Vec<u8> {
        SisTime::payload(self).to_vec()
    }
}

#[derive(Clone, Copy)]
pub struct SisDateTime {
    pub date: SisDate,
    pub time: SisTime,
}

impl SisDateTime {
    pub const KIND: u32 = 8;

    pub fn new(date: SisDate, time: SisTime) -> Self {
        Self { date, time }
    }

    pub fn utc(now: SystemTime) -> Self {
        let secs = now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let days = (secs / 86_400) as i32;
        let rem = (secs % 86_400) as u32;
        let (year, month, day) = civil_from_unix_days(days);
        Self::new(
            SisDate::new(year, month, day),
            SisTime::new(
                (rem / 3600) as u8,
                ((rem % 3600) / 60) as u8,
                (rem % 60) as u8,
            ),
        )
    }
}

fn civil_from_unix_days(z: i32) -> (u16, u8, u8) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + i32::from(m <= 2);
    (year as u16, m as u8, d as u8)
}

impl SisEncode for SisDateTime {
    const KIND: u32 = SisDateTime::KIND;

    fn payload(&self) -> Vec<u8> {
        [self.date.field().bytes(), self.time.field().bytes()].concat()
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;

    #[test]
    fn hello_date_payload_matches_experiment_21() {
        assert_eq!(
            SisDate::new(2026, 8, 17).payload(),
            [0xea, 0x07, 0x08, 0x11]
        );
        assert_eq!(SisDate::KIND, 6);
    }

    #[test]
    fn hello_time_payload_matches_experiment_21() {
        assert_eq!(SisTime::new(15, 18, 24).payload(), [0x0f, 0x12, 0x18]);
        assert_eq!(SisTime::KIND, 7);
    }

    #[test]
    fn hello_datetime_field_matches_experiment_21() {
        let f = SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)).field();
        assert_eq!(
            f.bytes(),
            [
                8, 0, 0, 0, 0x18, 0, 0, 0, 6, 0, 0, 0, 4, 0, 0, 0, 0xea, 0x07, 0x08, 0x11, 7, 0, 0,
                0, 3, 0, 0, 0, 0x0f, 0x12, 0x18, 0
            ]
        );
        assert_eq!(SisDateTime::KIND, 8);
    }
}
