//! Deciding what period an expense is *for*.
//!
//! Two sources, in that order: what the bank prints on the line, and failing that the shape of
//! the template the expense was generated from. Both the moneydb import and the automation
//! rules ask the same questions, so they ask them here.

use std::sync::LazyLock;

use chrono::{DateTime, Datelike, Months, NaiveDate, TimeZone, Utc};

/// The frame the stored dates were written in.
///
/// A date in this application is an instant, and the editor writes the one that is midnight
/// where the reader sits -- 23:00 UTC the day before in winter, 22:00 in summer. Calendar
/// arithmetic on such an instant is only right in the frame it was expressed in: read in UTC,
/// a period a reader entered as starting on the 1st begins on the 31st of the month before,
/// and a monthly rule then snaps its periods to the wrong day. Adding months across a DST
/// boundary moves the day as well.
///
/// So the *reading* side names the zone. The writing side does not need it: `day` builds every
/// grid point at midday UTC, which is the same date in any frame this will be read in.
const ZONE: chrono_tz::Tz = chrono_tz::Europe::Berlin;

/// The day an instant denotes, in the frame it was written in.
pub fn on(t: DateTime<Utc>) -> NaiveDate {
    t.with_timezone(&ZONE).date_naive()
}

/// Moves the day an instant denotes, reading it in the frame it was written in and writing
/// the result back at midday, the way [`day`] does.
///
/// The time of day is not carried across. These instants stand for days, and a day that keeps
/// its wall clock drifts an hour against UTC every time the clocks change -- which is exactly
/// the kind of hour that turns a whole month into a month and thirteen hours.
fn shift_local(
    t: DateTime<Utc>,
    f: impl FnOnce(NaiveDate) -> Option<NaiveDate>,
) -> Option<DateTime<Utc>> {
    let moved = f(on(t))?;
    day(moved.year(), moved.month(), moved.day())
}

/// The same day of the month, `months` later, in the frame the instant was written in.
///
/// Counting in UTC would count the wrong day: a date the editor wrote as local midnight is
/// the day before in UTC, so a period starting on the 1st steps from the 28th or the 31st,
/// clamps in a short month, and never returns to the day it was written with. Clamping is
/// still what a 31st does in February -- but only after the day has been read in the frame
/// that named it.
pub fn add_months(t: DateTime<Utc>, months: u32) -> Option<DateTime<Utc>> {
    shift_local(t, |d| d.checked_add_months(Months::new(months)))
}
use regex::Regex;

/// The period the bank itself names on the line, where it names one.
///
/// Insurers and public broadcasters print what they are charging for, in four spellings, and
/// that beats anything a heuristic could derive: `01.08.26-31.08.26`, `07.2026 - 09.2026`,
/// `GRUNDST 4VJ.24`, `Abschlag (Strom) Juli / 2026`. A payslip names its month as `01/2025`.
///
/// `moved` is when the money moved, which is what the result is sanity-checked against.
pub fn statement_period(
    statement: &str,
    moved: DateTime<Utc>,
) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    // the four-digit year has to come first: offered the short one first, the match ends
    // after `31.08.20` and the period turns out to run backwards into 2020
    static DAYS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(\d{2})\.(\d{2})\.(\d{4}|\d{2})\s*-\s*(\d{2})\.(\d{2})\.(\d{4}|\d{2})")
            .unwrap()
    });
    static MONTHS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b(\d{2})\.(\d{4})\s*-\s*(\d{2})\.(\d{4})\b").unwrap());
    static QUARTER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\b([1-4])\s*VJ\s*\.?\s*(\d{2})\b").unwrap());
    static NAMED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(Januar|Februar|M(?:ä|ae)rz|April|Mai|Juni|Juli|August|September|Oktober|November|Dezember)\s*/?\s*(\d{4})\b",
        )
        .unwrap()
    });
    // an invoice number is full of month-shaped fragments — `Rechnung Nr. 12.3/09/2030` is
    // not September, so nothing that looks like part of a longer run counts
    static NUMBERED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\d/.])(0[1-9]|1[0-2])/(\d{4}|\d{2})(?:$|[^\d/])").unwrap()
    });

    let (start, end) = if let Some(c) = DAYS.captures(statement) {
        let start = day(year(&c[3])?, c[2].parse().ok()?, c[1].parse().ok()?)?;
        let end = day(year(&c[6])?, c[5].parse().ok()?, c[4].parse().ok()?)?;

        // An insurer names the day cover ends rather than the last day it runs: Beispielversicherung prints
        // `01.06.23-01.07.23` for the month of June, and a yearly policy reads
        // `13.08.2018-13.08.2019`. Nobody sells a month plus one day, so an end landing exactly
        // a whole number of months after the start is that convention rather than a period.
        let end = match months_apart(on(start), on(end)) {
            Some(_) => end - chrono::Duration::days(1),
            None => end,
        };
        (start, end)
    } else if let Some(c) = MONTHS.captures(statement) {
        (
            month_start(c[2].parse().ok()?, c[1].parse().ok()?)?,
            month_end(c[4].parse().ok()?, c[3].parse().ok()?)?,
        )
    } else if let Some(c) = QUARTER.captures(statement) {
        let (y, q) = (year(&c[2])?, c[1].parse::<u32>().ok()?);
        (month_start(y, q * 3 - 2)?, month_end(y, q * 3)?)
    } else if let Some(c) = NAMED.captures(statement) {
        let (y, m) = (c[2].parse().ok()?, named_month(&c[1])?);
        (month_start(y, m)?, month_end(y, m)?)
    } else {
        let c = NUMBERED.captures(statement)?;
        let (y, m) = (year(&c[2])?, c[1].parse().ok()?);
        (month_start(y, m)?, month_end(y, m)?)
    };

    // Every one of these patterns can also be a contract number, a meter reading or a date
    // copied along from somewhere else. What makes it this expense's period is that the
    // money moved while it ran, or close enough on either side for a bill.
    let sane = start < end
        && (end - start) <= chrono::Duration::days(3 * 366)
        && (moved - start) >= chrono::Duration::days(-100)
        && (moved - end) <= chrono::Duration::days(200);
    sane.then_some((start, end))
}

/// The period an expense names in its own title, where the statement names none.
///
/// A title is written by hand, so only two shapes are worth trusting, and both are anchored on
/// the day the money moved rather than read as absolute dates.
///
/// A month by name is the nearest such month to that day — `Lohn Februar` paid on 15.03. is the
/// February before it, `Urlaub im September` settled on 20.10. the September before that. Paid
/// in advance or in arrears both come out right, since nothing here assumes a direction.
///
/// A bare year counts only at the very end of the title, only within a year either way, and only
/// where the rest of the title says the money settles something. Position and window keep out an
/// invoice number (`Rechnung 2024-0042`) and a product whose name happens to end in a number
/// (`Weltraumspiel 2077`, `Zahnbürste Modell 2000`), but neither keeps out a product labelled
/// with the year it is *for*: `Steuersoftware 2024` is tax software bought the November before, and
/// `Fotobuch 2019` is a book of photographs, not twelve months of anything. Only [`SETTLED`] can
/// tell those from `Steuerrückzahlung 2023`.
pub fn title_period(title: &str, moved: DateTime<Utc>) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    static MONTH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(Januar|Februar|M(?:ä|ae)rz|April|Mai|Juni|Juli|August|September|Oktober|November|Dezember)\b",
        )
        .unwrap()
    });
    static TRAILING_YEAR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:^|\s)((?:19|20)\d{2})\s*$").unwrap());

    let nearby = moved.year() - 1..=moved.year() + 1;

    if let Some(c) = MONTH.captures(title) {
        let month = named_month(&c[1])?;
        let year = nearby
            .filter_map(|y| month_start(y, month).map(|start| (start, y)))
            .min_by_key(|(start, _)| (*start - moved).num_days().abs())
            .map(|(_, y)| y)?;
        return Some((month_start(year, month)?, month_end(year, month)?));
    }

    let year: i32 = TRAILING_YEAR.captures(title)?[1].parse().ok()?;
    let lower = title.to_lowercase();
    let settles = SETTLED.iter().any(|word| lower.contains(word));
    (settles && nearby.contains(&year))
        .then(|| Some((month_start(year, 1)?, month_end(year, 12)?)))
        .flatten()
}

/// What a title has to call itself before a year at the end of it is a period.
///
/// A year names the thing a purchase is *about* as readily as the span it settles, and nothing
/// in the numbers separates the two: `Steuersoftware 2023` costs 12,90 € and `Beitragsnachforderung
/// 2023` costs 12,85 €. The words do. These are the German nouns for reckoning up a span that
/// has run — a bill, an arrears demand, a refund, a distribution, a provision released — plus
/// the two that name a thing sold *by* the year. A title that says none of them keeps its single
/// day, which is where every expense started.
const SETTLED: &[&str] = &[
    "abrechnung",
    "nachzahlung",
    "nachforderung",
    "rückzahlung",
    "rückstellung",
    "ausschüttung",
    "erklärung",
    "versicherung",
    "beitrag",
    "gebühr",
];

/// The number of whole months from `start` to an *inclusive* `end`, if it is a whole number of
/// them: `01.06.` to `30.06.` is one, `15.02.` to `14.05.` is three.
fn whole_months(start: NaiveDate, end: NaiveDate) -> Option<u32> {
    months_apart(start, end + chrono::Duration::days(1))
}

/// The number of whole months between two instants, if it is a whole number of them.
///
/// Month arithmetic clamps rather than overflowing, the way `PeriodicInterval::next` does, so a
/// period starting on the 31st still counts as monthly in February.
fn months_apart(start: NaiveDate, end: NaiveDate) -> Option<u32> {
    (1..=MAX_MONTHS).find(|&k| {
        start
            .checked_add_months(Months::new(k))
            .is_some_and(|d| d == end)
    })
}

/// Long enough for the two-year contracts that exist, short enough that a nonsense pair of
/// dates is not talked into a cadence.
const MAX_MONTHS: u32 = 24;

/// What one instance of a template covers, and where in that period the payment falls.
///
/// The template is the only thing that knows either. Its own accrual period against its own
/// booking date says that rent is debited on the first day of the month it pays for while a
/// Kita fee is debited on the last — the *phase* — and whether the period sits on a grid of
/// whole months or floats freely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// days from the start of the period to the payment; negative when paid in advance
    pub phase: chrono::TimeDelta,
    /// the length, and whether it snaps to a grid
    pub length: Length,
    /// the day of the month a gridded period starts on, taken from the template
    pub anchor_day: u32,
    /// which months of the grid the template itself started in, so a quarterly period keeps
    /// the biller's quarters rather than the calendar's
    pub anchor_month: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    /// a whole number of months, snapped to the grid the template sits on
    Months(u32),
    /// anything else, kept as a span of days that follows the payment around
    Days(i64),
}

impl Shape {
    /// Reads the shape off a template and the date it was itself paid on.
    pub fn of(start: DateTime<Utc>, end: DateTime<Utc>, paid: DateTime<Utc>) -> Self {
        let (from, to) = (on(start), on(end));

        Shape {
            phase: paid - start,
            length: match whole_months(from, to) {
                Some(k) => Length::Months(k),
                None => Length::Days((to - from).num_days() + 1),
            },
            anchor_day: from.day(),
            anchor_month: from.month(),
        }
    }

    /// The period a payment made on `paid` belongs to.
    ///
    /// Undoing the phase says where the period would have started had the payment been
    /// punctual; the grid is then joined at the *nearest* point rather than the one below.
    /// That is what absorbs a delay: a fee debited on the last day of the month it pays for
    /// still belongs to that month when a bank holiday pushes it into the next one, and the
    /// tolerance is half a period in either direction, which is the most any rule can give.
    pub fn period(&self, paid: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let nominal = paid - self.phase;

        let k = match self.length {
            Length::Days(days) => {
                return (nominal, nominal + chrono::Duration::days(days - 1));
            }
            Length::Months(k) => k,
        };

        let start = self.nearest_grid_point(nominal, k);
        let end = start
            .checked_add_months(Months::new(k))
            .map(|d| d - chrono::Duration::days(1))
            .unwrap_or(start);
        (start, end)
    }

    /// The grid point closest to `nominal`, where the grid runs every `k` months on
    /// `anchor_day`, in phase with the month the template started in.
    fn nearest_grid_point(&self, nominal: DateTime<Utc>, k: u32) -> DateTime<Utc> {
        // months since the anchor, floored to the grid, in a coordinate that stays monotonic
        // across year boundaries
        let at = on(nominal);
        let months = i64::from(at.year()) * 12 + i64::from(at.month0());
        let anchor = i64::from(self.anchor_month) - 1;
        let steps = (months - anchor).div_euclid(i64::from(k));

        // the grid point at or before `nominal`, and the one after it; `anchor_day` clamps to
        // the end of a short month, so both are asked for rather than computed by arithmetic
        let below = self.grid_point(anchor + steps * i64::from(k));
        let above = self.grid_point(anchor + (steps + 1) * i64::from(k));

        match (below, above) {
            (Some(b), Some(a)) if (nominal - b) > (a - nominal) => a,
            (Some(b), _) => b,
            (None, Some(a)) => a,
            (None, None) => nominal,
        }
    }

    /// The grid point `months` after year zero, clamped into the month it lands in.
    fn grid_point(&self, months: i64) -> Option<DateTime<Utc>> {
        let (y, m) = (months.div_euclid(12), months.rem_euclid(12) as u32 + 1);
        let last = month_end(i32::try_from(y).ok()?, m)?.day();
        day(i32::try_from(y).ok()?, m, self.anchor_day.min(last))
    }
}

/// Where a period ends that starts at `start` and is as long as `from`..`to`.
///
/// A template spanning whole months keeps whole months rather than a count of days: a monthly
/// rule written as `01.01.-31.01.` covers February to the 28th, not to the 2nd of March. Only
/// a length that is not whole months moves rigidly, which is what a template booked across a
/// handful of days wants.
pub fn shifted_end(
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    start: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    // built from `start` rather than from `to`, so both ends of the period carry the same time
    // of day and its length stays a whole number of days
    match Shape::of(from, to, from).length {
        Length::Months(k) => shift_local(start, |d| {
            d.checked_add_months(Months::new(k))
                .map(|e| e - chrono::Duration::days(1))
        }),
        Length::Days(n) => shift_local(start, |d| Some(d + chrono::Duration::days(n - 1))),
    }
}

/// Midday, so that nothing about a date depends on which side of midnight a timezone falls.
fn day(year: i32, month: u32, day: u32) -> Option<DateTime<Utc>> {
    Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).single()
}

/// A statement writes the year with two digits as often as with four.
fn year(text: &str) -> Option<i32> {
    let n: i32 = text.parse().ok()?;
    Some(if text.len() == 2 { 2000 + n } else { n })
}

fn month_start(year: i32, month: u32) -> Option<DateTime<Utc>> {
    day(year, month, 1)
}

fn month_end(year: i32, month: u32) -> Option<DateTime<Utc>> {
    Some(month_start(year, month)?.checked_add_months(Months::new(1))? - chrono::Duration::days(1))
}

fn named_month(name: &str) -> Option<u32> {
    const NAMES: [&str; 12] = [
        "januar",
        "februar",
        "märz",
        "april",
        "mai",
        "juni",
        "juli",
        "august",
        "september",
        "oktober",
        "november",
        "dezember",
    ];
    let name = name.to_lowercase().replace("maerz", "märz");
    NAMES.iter().position(|n| *n == name).map(|i| i as u32 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        day(y, m, d).unwrap()
    }

    /// The convention insurers write in: the end is the day cover stops, not the last day it
    /// runs. Both of these were in the books as a period a day too long.
    #[test]
    fn an_end_a_whole_month_out_is_exclusive() {
        let moved = at(2023, 6, 1);
        assert_eq!(
            statement_period("Beispielversicherung 000000000000 01.06.23-01.07.23 42,00 EUR", moved),
            Some((at(2023, 6, 1), at(2023, 6, 30)))
        );

        let moved = at(2018, 8, 13);
        assert_eq!(
            statement_period("ZEITRAUM 13.08.2018-13.08.2019", moved),
            Some((at(2018, 8, 13), at(2019, 8, 12)))
        );
    }

    /// ...and a period the biller already wrote inclusively stays as it is.
    #[test]
    fn an_end_anywhere_else_is_taken_as_written() {
        let moved = at(2026, 8, 5);
        assert_eq!(
            statement_period("Abschlag 01.08.26-31.08.26", moved),
            Some((at(2026, 8, 1), at(2026, 8, 31)))
        );
        assert_eq!(
            statement_period("Reise 12.08.2026-20.08.2026", moved),
            Some((at(2026, 8, 12), at(2026, 8, 20)))
        );
    }

    #[test]
    fn whole_months_counts_only_exact_spans() {
        assert_eq!(
            whole_months(on(at(2023, 6, 1)), on(at(2023, 6, 30))),
            Some(1)
        );
        assert_eq!(
            whole_months(on(at(2024, 2, 15)), on(at(2024, 5, 14))),
            Some(3)
        );
        assert_eq!(
            whole_months(on(at(2023, 4, 18)), on(at(2023, 10, 17))),
            Some(6)
        );
        assert_eq!(
            whole_months(on(at(2017, 9, 1)), on(at(2018, 8, 31))),
            Some(12)
        );
        // February, which is why this is not a day count
        assert_eq!(
            whole_months(on(at(2017, 2, 1)), on(at(2017, 2, 28))),
            Some(1)
        );
        // the two templates that were off by a day
        assert_eq!(whole_months(on(at(2023, 6, 1)), on(at(2023, 7, 1))), None);
        assert_eq!(
            whole_months(on(at(2023, 4, 18)), on(at(2023, 10, 16))),
            None
        );
    }

    /// Midnight where the reader sits, which is what the editor writes: 23:00 UTC the day
    /// before in winter, 22:00 in summer.
    fn entered(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        ZONE.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap().to_utc()
    }

    /// A monthly rule typed into the editor: read in UTC its period began on the 31st of the
    /// month before, so every period it produced was a day out and February straddled two
    /// months.
    #[test]
    fn a_period_entered_in_the_editor_keeps_the_days_it_was_entered_with() {
        let shape = Shape::of(
            entered(2023, 1, 1),
            entered(2023, 1, 31),
            entered(2023, 1, 30),
        );

        assert_eq!(shape.length, Length::Months(1));
        assert_eq!(shape.anchor_day, 1);

        assert_eq!(
            shape.period(at(2026, 8, 30)),
            (at(2026, 8, 1), at(2026, 8, 31))
        );
        // the short month, where a day-count would have run into March
        assert_eq!(
            shape.period(at(2027, 2, 26)),
            (at(2027, 2, 1), at(2027, 2, 28))
        );
        // a payment that slips into the next month still buys the month it was due in
        assert_eq!(
            shape.period(at(2026, 9, 1)),
            (at(2026, 8, 1), at(2026, 8, 31))
        );
    }

    /// Summer and winter are written down two hours apart, so a span between them is a whole
    /// number of months on the calendar and two hours off it as an instant.
    #[test]
    fn a_span_across_the_clock_change_is_still_whole_months() {
        assert_eq!(
            whole_months(on(entered(2023, 7, 1)), on(entered(2023, 12, 31))),
            Some(6)
        );

        let shape = Shape::of(
            entered(2023, 7, 1),
            entered(2023, 12, 31),
            entered(2023, 7, 1),
        );
        assert_eq!(shape.length, Length::Months(6));
        assert_eq!(shape.anchor_day, 1);
        assert_eq!(
            shape.period(entered(2026, 1, 1)),
            (at(2026, 1, 1), at(2026, 6, 30))
        );
    }

    /// Rent: debited on the first day of the month it pays for.
    #[test]
    fn a_punctual_payment_lands_on_its_own_month() {
        let shape = Shape::of(at(2018, 10, 1), at(2018, 10, 31), at(2018, 10, 1));
        assert_eq!(shape.length, Length::Months(1));
        assert_eq!(
            shape.period(at(2026, 11, 1)),
            (at(2026, 11, 1), at(2026, 11, 30))
        );
        // a couple of days late is still November
        assert_eq!(
            shape.period(at(2026, 11, 3)),
            (at(2026, 11, 1), at(2026, 11, 30))
        );
        // and paid two days early it is still November, not October
        assert_eq!(
            shape.period(at(2026, 10, 30)),
            (at(2026, 11, 1), at(2026, 11, 30))
        );
    }

    /// The case that would otherwise want a `comes_in_at_end_of_month` flag: the Kita fee is
    /// debited on the last day of the month it pays for, and a bank holiday pushes it over.
    #[test]
    fn a_delay_past_the_month_end_stays_in_the_month_it_paid_for() {
        let shape = Shape::of(at(2024, 8, 1), at(2024, 8, 31), at(2024, 8, 31));
        assert_eq!(
            shape.period(at(2026, 9, 30)),
            (at(2026, 9, 1), at(2026, 9, 30))
        );
        assert_eq!(
            shape.period(at(2026, 10, 2)),
            (at(2026, 9, 1), at(2026, 9, 30))
        );
        // a fortnight late is the furthest this can be pushed, and it still holds
        assert_eq!(
            shape.period(at(2026, 10, 14)),
            (at(2026, 9, 1), at(2026, 9, 30))
        );
    }

    /// Salary: paid on the 23rd for the month it is earned in, so a late payment has more room
    /// on either side than either of the above.
    #[test]
    fn a_payment_mid_period_tolerates_the_most() {
        let shape = Shape::of(at(2018, 3, 1), at(2018, 3, 31), at(2018, 3, 23));
        assert_eq!(
            shape.period(at(2026, 10, 25)),
            (at(2026, 10, 1), at(2026, 10, 31))
        );
        assert_eq!(
            shape.period(at(2026, 11, 2)),
            (at(2026, 10, 1), at(2026, 10, 31))
        );
    }

    /// A quarterly bill keeps the quarters the biller uses, on the day of the month it uses.
    #[test]
    fn a_grid_wider_than_a_month_keeps_the_billers_quarters() {
        let shape = Shape::of(at(2024, 2, 15), at(2024, 5, 14), at(2024, 2, 15));
        assert_eq!(shape.length, Length::Months(3));
        assert_eq!(
            shape.period(at(2026, 8, 15)),
            (at(2026, 8, 15), at(2026, 11, 14))
        );
        // the grid is Feb/May/Aug/Nov, so a payment in September belongs to the August quarter
        assert_eq!(
            shape.period(at(2026, 9, 20)),
            (at(2026, 8, 15), at(2026, 11, 14))
        );
    }

    /// A template anchored on a day that not every month has still fires every month.
    #[test]
    fn an_anchor_day_clamps_into_a_short_month() {
        let shape = Shape::of(at(2026, 1, 31), at(2026, 2, 27), at(2026, 1, 31));
        assert_eq!(shape.length, Length::Months(1));
        assert_eq!(
            shape.period(at(2026, 2, 28)),
            (at(2026, 2, 28), at(2026, 3, 27))
        );
    }

    /// A single purchase covers the day it happened, whenever that is.
    #[test]
    fn a_span_that_is_not_whole_months_follows_the_payment() {
        let shape = Shape::of(at(2023, 1, 1), at(2023, 1, 1), at(2023, 1, 1));
        assert_eq!(shape.length, Length::Days(1));
        assert_eq!(
            shape.period(at(2026, 7, 4)),
            (at(2026, 7, 4), at(2026, 7, 4))
        );

        // a nine-day holiday booked three days before it starts stays nine days long
        let shape = Shape::of(at(2021, 6, 15), at(2021, 6, 23), at(2021, 6, 12));
        assert_eq!(shape.length, Length::Days(9));
        assert_eq!(
            shape.period(at(2026, 5, 2)),
            (at(2026, 5, 5), at(2026, 5, 13))
        );
    }
}
