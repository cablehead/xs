use ParseError;
use ParseResult;

#[derive(Debug, PartialEq)]
pub enum DayOfWeek {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl DayOfWeek {
    pub fn to_numeral(&self) -> u32 {
        match *self {
            DayOfWeek::Sunday => 0,
            DayOfWeek::Monday => 1,
            DayOfWeek::Tuesday => 2,
            DayOfWeek::Wednesday => 3,
            DayOfWeek::Thursday => 4,
            DayOfWeek::Friday => 5,
            DayOfWeek::Saturday => 6,
        }
    }

    pub fn from_numeral(num: u32) -> DayOfWeek {
        match num % 7 {
            0 => DayOfWeek::Sunday,
            1 => DayOfWeek::Monday,
            2 => DayOfWeek::Tuesday,
            3 => DayOfWeek::Wednesday,
            4 => DayOfWeek::Thursday,
            5 => DayOfWeek::Friday,
            6 => DayOfWeek::Saturday,
            _ => panic!("Unreachable."),
        }
    }

    /// Given the current day of the week, how many days until the next day?
    pub fn difference(&self, other: &DayOfWeek) -> u32 {
        // Have to use i32 because of wraparound issues
        let s_num = self.to_numeral() as i32;
        let o_num = other.to_numeral() as i32;

        if o_num - s_num >= 0 {
            (o_num - s_num) as u32
        } else {
            (7 + o_num - s_num) as u32
        }
    }
}

pub fn day_of_week(year: u32, month: u32, day: u32) -> ParseResult<DayOfWeek> {
    // From https://en.wikipedia.org/wiki/Determination_of_the_day_of_the_week#Schwerdtfeger's_method
    let (c, g) = match month {
        3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 => {
            let c = year / 100;
            (c, year - 100 * c)
        }
        1 | 2 => {
            let c = (year - 1) / 100;
            (c, year - 1 - 100 * c)
        }
        _ => return Err(ParseError::ImpossibleTimestamp("Invalid month")),
    };

    let e = match month {
        1 | 5 => 0,
        2 | 6 => 3,
        3 | 11 => 2,
        4 | 7 => 5,
        8 => 1,
        9 | 12 => 4,
        10 => 6,
        _ => panic!("Unreachable."),
    };

    // This implementation is Gregorian-only.
    let f = match c % 4 {
        0 => 0,
        1 => 5,
        2 => 3,
        3 => 1,
        _ => panic!("Unreachable."),
    };

    match (day + e + f + g + g / 4) % 7 {
        0 => Ok(DayOfWeek::Sunday),
        1 => Ok(DayOfWeek::Monday),
        2 => Ok(DayOfWeek::Tuesday),
        3 => Ok(DayOfWeek::Wednesday),
        4 => Ok(DayOfWeek::Thursday),
        5 => Ok(DayOfWeek::Friday),
        6 => Ok(DayOfWeek::Saturday),
        _ => panic!("Unreachable."),
    }
}

// Rust warns about unused imports here, but they're definitely used.
#[allow(unused_imports)]
mod test {

    use weekday::day_of_week;
    use weekday::DayOfWeek;

    #[test]
    fn day_of_week_examples() {
        assert_eq!(day_of_week(2018, 6, 24).unwrap(), DayOfWeek::Sunday);
        assert_eq!(day_of_week(2003, 9, 25).unwrap(), DayOfWeek::Thursday);
    }

    #[test]
    fn weekday_difference() {
        assert_eq!(DayOfWeek::Sunday.difference(&DayOfWeek::Sunday), 0);
        assert_eq!(DayOfWeek::Sunday.difference(&DayOfWeek::Monday), 1);
        assert_eq!(DayOfWeek::Sunday.difference(&DayOfWeek::Tuesday), 2);
        assert_eq!(DayOfWeek::Sunday.difference(&DayOfWeek::Wednesday), 3);
        assert_eq!(DayOfWeek::Sunday.difference(&DayOfWeek::Thursday), 4);
        assert_eq!(DayOfWeek::Sunday.difference(&DayOfWeek::Friday), 5);
        assert_eq!(DayOfWeek::Sunday.difference(&DayOfWeek::Saturday), 6);
        assert_eq!(DayOfWeek::Monday.difference(&DayOfWeek::Sunday), 6);
        assert_eq!(DayOfWeek::Tuesday.difference(&DayOfWeek::Sunday), 5);
        assert_eq!(DayOfWeek::Wednesday.difference(&DayOfWeek::Sunday), 4);
        assert_eq!(DayOfWeek::Thursday.difference(&DayOfWeek::Sunday), 3);
        assert_eq!(DayOfWeek::Friday.difference(&DayOfWeek::Sunday), 2);
        assert_eq!(DayOfWeek::Saturday.difference(&DayOfWeek::Sunday), 1);
    }
}
