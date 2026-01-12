// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use jiff::{civil::DateTime, tz::TimeZone};
use rstest::rstest;

mod common;
use common::{check_absolute, check_relative};

// The expected values are produced by GNU date version 8.32
// export LC_TIME=en_US.UTF-8
// export TZ=UTC
// date --rfc-3339=seconds --date="2022-11-14"
//
// Documentation for the date format can be found at:
// https://www.gnu.org/software/coreutils/manual/html_node/Calendar-date-items.html

#[rstest]
#[case::iso8601("2022-11-14", "2022-11-14 00:00:00+00:00")]
#[case::short_year_22("22-11-14", "2022-11-14 00:00:00+00:00")]
#[case::short_year_68("68-11-14", "2068-11-14 00:00:00+00:00")]
#[case::short_year_00("00-11-14", "2000-11-14 00:00:00+00:00")]
#[case::short_year_69("69-11-14", "1969-11-14 00:00:00+00:00")]
#[case::short_year_99("99-11-14", "1999-11-14 00:00:00+00:00")]
#[case::us_style("11/14/2022", "2022-11-14 00:00:00+00:00")]
#[case::us_style_short_year("11/14/22", "2022-11-14 00:00:00+00:00")]
#[case::year_zero("0000-01-01", "0000-01-01 00:00:00+00:00")]
#[case::year_001("001-11-14", "0001-11-14 00:00:00+00:00")]
#[case::year_100("100-11-14", "0100-11-14 00:00:00+00:00")]
#[case::year_999("999-11-14", "0999-11-14 00:00:00+00:00")]
#[case::year_9999("9999-11-14", "9999-11-14 00:00:00+00:00")]
/** TODO: https://github.com/uutils/parse_datetime/issues/160
#[case::year_10000("10000-12-31", "10000-12-31 00:00:00+00:00")]
#[case::year_100000("100000-12-31", "100000-12-31 00:00:00+00:00")]
#[case::year_1000000("1000000-12-31", "1000000-12-31 00:00:00+00:00")]
#[case::year_10000000("10000000-12-31", "10000000-12-31 00:00:00+00:00")]
#[case::max_date("2147485547-12-31", "2147485547-12-31 00:00:00+00:00")]
**/
#[case::long_month_in_the_middle("14 November 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_lowercase("14 november 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_uppercase("14 NOVEMBER 2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_in_the_middle("14 nov 2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_in_the_uppercase("14 NOV 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_hyphened("14-november-2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_nospace("14november2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_hyphened("14-nov-2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_nospace("14nov2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_at_start("November 14 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_at_start_with_comma("November 14, 2022", "2022-11-14 00:00:00+00:00")]
#[case::short_month_at_start("nov 14 2022", "2022-11-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_jan("14 January 2022", "2022-01-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_feb("14 February 2022", "2022-02-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_mar("14 March 2022", "2022-03-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_apr("14 April 2022", "2022-04-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_may("14 May 2022", "2022-05-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_jun("14 June 2022", "2022-06-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_jul("14 July 2022", "2022-07-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_aug("14 August 2022", "2022-08-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_sep("14 September 2022", "2022-09-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_oct("14 October 2022", "2022-10-14 00:00:00+00:00")]
#[case::long_month_in_the_middle_dec("14 December 2022", "2022-12-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_jan("14 jan 2022", "2022-01-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_feb("14 feb 2022", "2022-02-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_mar("14 mar 2022", "2022-03-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_apr("14 apr 2022", "2022-04-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_may("14 may 2022", "2022-05-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_jun("14 jun 2022", "2022-06-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_jul("14 jul 2022", "2022-07-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_aug("14 aug 2022", "2022-08-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_sep("14 sep 2022", "2022-09-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_sept("14 sept 2022", "2022-09-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_oct("14 oct 2022", "2022-10-14 00:00:00+00:00")]
#[case::short_month_in_the_middle_dec("14 dec 2022", "2022-12-14 00:00:00+00:00")]
fn test_absolute_date_numeric(#[case] input: &str, #[case] expected: &str) {
    check_absolute(input, expected);
}

#[rstest]
#[case::us_style("11/14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_full_month_in_front("november 14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_full_month_at_back("14 november", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_in_front("nov 14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_at_back("14 nov", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_full_month_in_front("november 14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_full_month_at_back("14 november", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_in_front("nov 14", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_at_back("14 nov", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_long_month_at_back_hyphen("14-november", 2022, "2022-11-14 00:00:00+00:00")]
#[case::alphabetical_short_month_at_back_hyphen("14-nov", 2022, "2022-11-14 00:00:00+00:00")]
fn test_date_omitting_year(#[case] input: &str, #[case] year: u32, #[case] expected: &str) {
    let now = format!("{year}-06-01 00:00:00")
        .parse::<DateTime>()
        .unwrap()
        .to_zoned(TimeZone::UTC)
        .unwrap();
    check_relative(now, input, expected);
}
