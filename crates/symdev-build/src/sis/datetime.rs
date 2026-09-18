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
