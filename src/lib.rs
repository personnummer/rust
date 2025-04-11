#[macro_use]
extern crate lazy_static;

use chrono::{Datelike, NaiveDate, Utc};
use regex::{Match, Regex};

use std::{
    convert::TryFrom,
    error::Error,
    fmt::{self, Display},
    str::FromStr,
};

lazy_static! {
    static ref PNR_REGEX: Regex = Regex::new(
        r"(?x)
        ^                    # Starts with
        (?P<century>\d{2})?  # Maybe the century
        (?P<year>\d{2})      # Year with two digits
        (?P<month>\d{2})     # Month
        (?P<day>\d{2})       # Day
        (?P<sep>[-+]?)?      # Seperator can be - or +
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

/// [Personnummer] holds relevant data to check for valid personal identity numbers.
pub struct Personnummer {
    date: chrono::NaiveDate,
    serial: u32,
    control: u8,
    seperator: Seperator,
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
    /// Returns the [long format](https://github.com/personnummer/meta/tree/master?tab=readme-ov-file#long-format)
    /// of a formatted personal identity number as a [String].
    pub fn long(&self) -> String {
        self.long.clone()
    }

    /// Returns the [short format](https://github.com/personnummer/meta/tree/master?tab=readme-ov-file#short-format)
    /// of a formatted personal identity number as a [String].
    pub fn short(&self) -> String {
        self.short.clone()
    }
}

/// [Seperator] is used by the short format to seperate birth date and serial information.
/// A plus sign (+) indicates that the age is over 100 years.
#[derive(Debug, Copy, Clone)]
pub enum Seperator {
    Plus,
    Minus,
}
impl FromStr for Seperator {
    type Err = PersonnummerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "-" => Ok(Self::Minus),
            "+" => Ok(Self::Plus),
            _ => Err(PersonnummerError::InvalidInput),
        }
    }
}

impl Display for Seperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Seperator::Plus => write!(f, "+"),
            Seperator::Minus => write!(f, "-"),
        }
    }
}

impl TryFrom<&str> for Personnummer {
    type Error = PersonnummerError;

    fn try_from(pnr: &str) -> Result<Self, PersonnummerError> {
        let caps = PNR_REGEX
            .captures(pnr)
            .ok_or(PersonnummerError::InvalidInput)?;

        let match_to_u32 =
            |m: Option<Match<'_>>| -> u32 { m.unwrap().as_str().parse::<u32>().unwrap_or(0) };

        let century = caps
            .name("century")
            .and_then(|v| v.as_str().parse::<u32>().ok());
        let year = match_to_u32(caps.name("year"));
        let month = match_to_u32(caps.name("month"));
        let day = match_to_u32(caps.name("day"));
        let seperator = caps.name("sep").unwrap().as_str().parse::<Seperator>().ok();
        let serial = match_to_u32(caps.name("number"));
        let control = caps
            .name("control")
            .unwrap()
            .as_str()
            .parse::<u8>()
            .unwrap_or(0);

        let current_year = Utc::now().year() as u32;
        let (century, seperator) = match century {
            Some(century) => {
                if current_year - (century * 100 + year) >= 100 {
                    (century, Seperator::Plus)
                } else {
                    (century, Seperator::Minus)
                }
            }
            None => {
                let (base_year, seperator) = match seperator {
                    Some(Seperator::Plus) => (current_year - 100, Seperator::Plus),
                    _ => (current_year, Seperator::Minus),
                };

                let century = (base_year - ((base_year - year) % 100)) / 100;
                (century, seperator)
            }
        };

        let date = match NaiveDate::from_ymd_opt(
            (century * 100 + year) as i32,
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
            seperator,
            coordination: day > COORDINATION_NUMBER,
        })
    }
}

impl Personnummer {
    /// Returns a new instance of a [Personnummer]. Returns an error for invalid dates but not
    /// for invalid personal identity numbers. Use [Personnummer::valid()] to check validity.
    pub fn new(pnr: &str) -> Result<Personnummer, PersonnummerError> {
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
            "{:04}{:02}{:02}{:03}{}",
            self.date.year(),
            self.date.month(),
            day_or_coordination,
            self.serial,
            self.control
        );

        let short = format!(
            "{:02}{:02}{:02}{}{:03}{}",
            self.date.year() % 100,
            self.date.month(),
            day_or_coordination,
            self.seperator,
            self.serial,
            self.control
        );

        FormattedPersonnummer { long, short }
    }

    /// Validate a [Personnummer]. The validation requires a valid date and that the Luhn checksum
    /// matches the control digit.
    pub fn valid(&self) -> bool {
        let day = self.date.day();
        let day_or_coordination = if self.coordination {
            day + COORDINATION_NUMBER
        } else {
            day
        };

        let to_control = format!(
            "{:02}{:02}{:02}{:03}",
            self.date.year() % 100,
            self.date.month(),
            day_or_coordination,
            self.serial
        );

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

    /// The seperator between birthdate and serial.
    pub fn seperator(&self) -> Seperator {
        self.seperator
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
            assert!(Personnummer::new(tc).is_err());
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
    fn test_valid_pn_invalid_seperator() {
        let cases = vec![
            "19900101/0017",
            "19640823_3234",
            "000101 0107",
            "510818S9167",
            "1913040102931",
        ];

        for tc in cases {
            assert!(Personnummer::new(tc).is_err());
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

        let twenty_tomorrow_date = (now - Duration::days(twenty_years_ago - 1)).date_naive();
        let twenty_tomorrow = format!(
            "{}{:02}{:02}-1111",
            twenty_tomorrow_date.year(),
            twenty_tomorrow_date.month(),
            twenty_tomorrow_date.day()
        );

        let twenty_yesterday_date = (now - Duration::days(twenty_years_ago + 1)).date_naive();
        let twenty_yesterday = format!(
            "{}{:02}{:02}-1111",
            twenty_yesterday_date.year(),
            twenty_yesterday_date.month(),
            twenty_yesterday_date.day()
        );

        let hundred_years_ago_date = (now - Duration::days(hundred_years_ago)).date_naive();
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

        cases.insert("800161-3291", true);
        cases.insert("800101-3294", false);
        cases.insert("640327-3813", false);
        cases.insert("250561-0275", true);

        for (pnr, is_coordination) in cases {
            let p = Personnummer::new(pnr).unwrap();
            assert!(p.valid());
            assert_eq!(p.is_coordination_number(), is_coordination);
        }
    }

    #[test]
    fn test_format() {
        let mut cases: HashMap<&str, FormattedPersonnummer> = HashMap::new();

        cases.insert(
            "20000101-2392",
            FormattedPersonnummer {
                long: "200001012392".into(),
                short: "000101-2392".into(),
            },
        );
        cases.insert(
            "000101-2392",
            FormattedPersonnummer {
                long: "200001012392".into(),
                short: "000101-2392".into(),
            },
        );
        cases.insert(
            "000101+2392",
            FormattedPersonnummer {
                long: "190001012392".into(),
                short: "000101+2392".into(),
            },
        );
        // malformed but should be able to be handled
        cases.insert(
            "18680404-0043",
            FormattedPersonnummer {
                long: "186804040043".into(),
                short: "680404+0043".into(),
            },
        );
        // coordination number
        cases.insert(
            "950161-2395",
            FormattedPersonnummer {
                long: "199501612395".into(),
                short: "950161-2395".into(),
            },
        );

        for (pnr, formatted) in cases {
            let p = Personnummer::new(pnr).unwrap();

            assert!(p.valid());
            assert_eq!(p.format().long, formatted.long);
            assert_eq!(p.format().short, formatted.short);
        }
    }
}
