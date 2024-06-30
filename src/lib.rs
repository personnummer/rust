#[macro_use]
extern crate lazy_static;

use chrono::{Datelike, NaiveDate, Utc};
use regex::{Match, Regex};

use std::{convert::TryFrom, error::Error, fmt};

lazy_static! {
    static ref PNR_REGEX: Regex = Regex::new(
        r"(?x)
        ^                    # Starts with
        (?P<century>\d{2})?  # Maybe the century
        (?P<year>\d{2})      # Year with two digits
        (?P<month>\d{2})     # Month
        (?P<day>\d{2})       # Day
        (?P<divider>[-+]?)?  # Divider can be - or +
        (?P<number>\d{3})    # At least three digits
        (?P<control>\d?)     # And an optional control digit
        $"
    )
    .unwrap();
}

/// The extra value added to coordination numbers.
const COORDINATION_NUMBER: u32 = 60;

#[derive(Debug)]
pub enum PersonnummerError {
    InvalidInput,
    InvalidDate,
}

impl fmt::Display for PersonnummerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PersonnummerError::InvalidInput => write!(f, "Invalid format"),
            PersonnummerError::InvalidDate => write!(f, "Invalid date"),
        }
    }
}

impl Error for PersonnummerError {}

#[allow(dead_code)]
/// [Personnummer] holds relevant data to check for valid personal identity numbers.
pub struct Personnummer {
    date: chrono::NaiveDate,
    serial: u32,
    control: u8,
    divider: char,
    coordination: bool,
}

/// [FormattedPersonnummer] holds two formats of a normalized personal identity number, one long and
/// one short format. The long format displays the full century while the short format only
/// displays the year.
pub struct FormattedPersonnummer {
    long: String,
    short: String,
}

impl FormattedPersonnummer {
    /// Returns the long format of a formatted personal identity number as a [String].
    pub fn long(&self) -> String {
        self.long.clone()
    }

    /// Returns the short format of a formatted personal identity number as a [String].
    pub fn short(&self) -> String {
        self.short.clone()
    }
}

impl TryFrom<&str> for Personnummer {
    type Error = PersonnummerError;

    fn try_from(pnr: &str) -> Result<Self, PersonnummerError> {
        let caps = PNR_REGEX
            .captures(pnr)
            .ok_or(PersonnummerError::InvalidInput)?;

        let century = match caps.name("century") {
            Some(m) => m.as_str().parse::<u32>().unwrap_or(19) * 100,
            None => 1900,
        };

        let match_to_u32 =
            |m: Option<Match<'_>>| -> u32 { m.unwrap().as_str().parse::<u32>().unwrap_or(0) };

        let year = match_to_u32(caps.name("year"));
        let month = match_to_u32(caps.name("month"));
        let day = match_to_u32(caps.name("day"));
        let serial = match_to_u32(caps.name("number"));

        let control = caps
            .name("control")
            .unwrap()
            .as_str()
            .parse::<u8>()
            .unwrap_or(0);

        let divider = caps
            .name("divider")
            .unwrap()
            .as_str()
            .parse::<char>()
            .unwrap_or('\0');

        let date = match NaiveDate::from_ymd_opt(
            (century + year) as i32,
            month,
            day % COORDINATION_NUMBER,
        ) {
            Some(date) => date,
            None => return Err(PersonnummerError::InvalidDate),
        };

        Ok(Personnummer {
            date,
            serial,
            control,
            divider,
            coordination: (day > 31),
        })
    }
}

impl Personnummer {
    /// Returns a new instance of a [Personnummer]. Panics for invalid dates but not for invalid
    /// personal identity numbers. Use [Personnummer::valid()] to check validity.
    pub fn new(pnr: &str) -> Result<Personnummer, PersonnummerError> {
        Personnummer::try_from(pnr)
    }

    /// Same as [Personnummer::new()] but returns an [Option] instead of panicing on invalid dates.
    pub fn parse(pnr: &str) -> Result<Personnummer, PersonnummerError> {
        Personnummer::try_from(pnr)
    }

    /// Returns a [FormattedPersonnummer] from a [Personnummer] which can be used to display a
    /// normalized version of the [Personnummer].
    pub fn format(&self) -> FormattedPersonnummer {
        let day = self.date.day();
        let day_or_coordination = if self.coordination {
            day + COORDINATION_NUMBER
        } else {
            day
        };

        let long = format!(
            "{}{:02}{:02}-{:03}{}",
            self.date.year(),
            self.date.month(),
            day_or_coordination,
            self.serial,
            self.control
        );

        let short = String::from(&long[2..]);

        FormattedPersonnummer { long, short }
    }

    /// Validate a [Personnummer]. The validation requires a valid date and that the Luhn checksum
    /// matches the control digit.
    pub fn valid(&self) -> bool {
        let ymd = format!(
            "{:02}{:02}{:02}",
            self.date.year() % 100,
            self.date.month(),
            self.date.day()
        );

        let to_control = format!("{:06}{:03}", ymd, self.serial);

        self.serial > 0 && luhn(to_control) == self.control
    }

    /// Return the age of the person holding the personal identity number. The dates used for the
    /// person and the current date are naive dates.
    pub fn get_age(&self) -> i32 {
        let now = Utc::now();

        if self.date.month() > now.month()
            || self.date.month() == now.month() && self.date.day() > now.day()
        {
            now.year() - self.date.year() - 1
        } else {
            now.year() - self.date.year()
        }
    }

    /// Check if the person holding the personal identity number is a female.
    pub fn is_female(&self) -> bool {
        (self.serial % 10) % 2 == 0
    }

    /// Check if the person holding the personal identity number is a male.
    pub fn is_male(&self) -> bool {
        !self.is_female()
    }

    /// Check if the personal identity number is a coordination number.
    pub fn is_coordination_number(&self) -> bool {
        self.coordination
    }

    /// Year of birth date.
    pub fn year(&self) -> i32 {
        self.date.year()
    }

    /// Month of birth date.
    pub fn month(&self) -> u32 {
        self.date.month()
    }

    /// Day of birth date.
    pub fn day(&self) -> u32 {
        self.date.day()
    }

    /// Serial part of personal identity number.
    pub fn serial(&self) -> u32 {
        self.serial
    }
}

/// Calculate the checksum based on luhn algorithm. See more information here:
/// https://en.wikipedia.org/wiki/Luhn_algorithm.
fn luhn(value: String) -> u8 {
    let checksum = value
        .chars()
        .map(|c| c.to_digit(10).unwrap_or(0))
        .enumerate()
        .fold(0, |acc, (idx, v)| {
            let value = if idx % 2 == 0 { v * 2 } else { v };
            acc + if value > 9 { value - 9 } else { value }
        });

    (10 - (checksum as u8 % 10)) % 10
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::collections::HashMap;

    #[test]
    fn test_invalid_date() {
        let cases = vec!["19901301-1111", "2017-02-29", "", "not-a-date"];

        for tc in cases {
            assert!(Personnummer::parse(tc).is_err());
        }
    }

    #[test]
    fn test_valid_date_invalid_digits() {
        let cases = vec![
            "19900101-1111",
            "20160229-1111",
            "6403273814",
            "20150916-0006",
        ];

        for tc in cases {
            assert!(!Personnummer::new(tc).unwrap().valid());
        }
    }

    #[test]
    fn test_valid_personal_identity_number() {
        let cases = vec![
            "19900101-0017",
            "196408233234",
            "000101-0107",
            "510818-9167",
            "19130401+2931",
        ];

        for tc in cases {
            assert!(Personnummer::new(tc).unwrap().valid());
        }
    }

    #[test]
    fn test_age() {
        let now = Utc::now();

        let days_in_a_year = 365;
        let leap_years_in_20_years = 20 / 4;
        let twenty_years_ago = (days_in_a_year * 20) + leap_years_in_20_years;

        let leap_years_in_100_years = 100 / 4;
        let hundred_years_ago = (days_in_a_year * 100) + leap_years_in_100_years;

        let twenty_tomorrow_date = (now - Duration::days(twenty_years_ago - 1)).date();
        let twenty_tomorrow = format!(
            "{}{:02}{:02}-1111",
            twenty_tomorrow_date.year(),
            twenty_tomorrow_date.month(),
            twenty_tomorrow_date.day()
        );

        let twenty_yesterday_date = (now - Duration::days(twenty_years_ago + 1)).date();
        let twenty_yesterday = format!(
            "{}{:02}{:02}-1111",
            twenty_yesterday_date.year(),
            twenty_yesterday_date.month(),
            twenty_yesterday_date.day()
        );

        let hundred_years_ago_date = (now - Duration::days(hundred_years_ago)).date();
        let hundred_years_age = format!(
            "{}{:02}{:02}-1111",
            hundred_years_ago_date.year(),
            hundred_years_ago_date.month(),
            hundred_years_ago_date.day()
        );

        let mut cases: HashMap<&str, i32> = HashMap::new();

        cases.insert(twenty_tomorrow.as_str(), 19);
        cases.insert(twenty_yesterday.as_str(), 20);
        cases.insert(hundred_years_age.as_str(), 100);

        for (pnr, age) in cases {
            assert_eq!(Personnummer::new(pnr).unwrap().get_age(), age);
        }
    }

    #[test]
    fn test_gender() {
        let mut cases: HashMap<&str, bool> = HashMap::new();

        cases.insert("19090903-6600", true);
        cases.insert("19900101-0017", false);
        cases.insert("800101-3294", false);
        cases.insert("000903-6609", true);
        cases.insert("800101+3294", false);

        for (pnr, is_female) in cases {
            let p = Personnummer::new(pnr).unwrap();

            assert!(p.valid());
            assert_eq!(p.is_female(), is_female);
            assert_eq!(p.is_male(), !is_female);
        }
    }

    #[test]
    fn test_coordination() {
        let mut cases: HashMap<&str, bool> = HashMap::new();

        cases.insert("800161-3294", true);
        cases.insert("800101-3294", false);
        cases.insert("640327-3813", false);

        for (pnr, is_coordination) in cases {
            let p = Personnummer::new(pnr).unwrap();

            assert!(p.valid());
            assert_eq!(p.is_coordination_number(), is_coordination);
        }
    }
}
