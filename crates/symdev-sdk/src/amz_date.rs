use std::time::{SystemTime, UNIX_EPOCH};

/// A UTC timestamp in SigV4's basic ISO 8601 form, `YYYYMMDDTHHMMSSZ`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmzDate(String);

impl AmzDate {
    /// The timestamp `secs` seconds after 1970-01-01T00:00:00Z (years up to 9999).
    pub fn from_unix(secs: u64) -> AmzDate {
        let (days, rest) = (secs / 86_400, secs % 86_400);
        let (year, month, day) = civil_from_days(days);
        AmzDate(format!(
            "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
            rest / 3600,
            rest % 3600 / 60,
            rest % 60
        ))
    }

    /// The system clock now; a clock set before 1970 reads as 1970, which the server then
    /// refuses as skewed rather than this crate guessing a time.
    pub fn now() -> AmzDate {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        AmzDate::from_unix(since_epoch.as_secs())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `YYYYMMDD`, the date part of the credential scope.
    pub fn day(&self) -> &str {
        &self.0[..8]
    }
}

/// Proleptic Gregorian (year, month, day) of a day count since 1970-01-01, by Howard
/// Hinnant's `civil_from_days` (shifted to a year that starts on 1 March).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::AmzDate;

    #[test]
    fn formats_the_epoch() {
        assert_eq!(AmzDate::from_unix(0).as_str(), "19700101T000000Z");
    }

    #[test]
    fn formats_the_test_suite_time_and_its_day() {
        let date = AmzDate::from_unix(1_440_938_160);
        assert_eq!(date.as_str(), "20150830T123600Z");
        assert_eq!(date.day(), "20150830");
    }

    #[test]
    fn formats_leap_days_and_century_boundaries() {
        assert_eq!(
            AmzDate::from_unix(1_369_353_600).as_str(),
            "20130524T000000Z"
        );
        assert_eq!(
            AmzDate::from_unix(1_709_251_199).as_str(),
            "20240229T235959Z"
        );
        assert_eq!(AmzDate::from_unix(951_868_800).as_str(), "20000301T000000Z");
        assert_eq!(
            AmzDate::from_unix(4_102_444_799).as_str(),
            "20991231T235959Z"
        );
    }

    #[test]
    fn now_is_a_full_timestamp_after_the_crate_was_written() {
        let now = AmzDate::now();
        assert_eq!(now.as_str().len(), 16);
        assert!(now.as_str() > "20261001T000000Z", "{}", now.as_str());
    }
}
