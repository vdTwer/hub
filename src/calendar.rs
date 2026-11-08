//! A calendar week for the mockup, plus the layout math that places it:
//! side-by-side columns for overlapping events and stacked lanes for
//! all-day items that span several days.

use std::borrow::Borrow;

/// The visible part of each day, in minutes after midnight.
pub const DAY_START: u16 = 7 * 60;
pub const DAY_END: u16 = 22 * 60;

/// Event colors, borrowed from the Colorways cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Cobalt,
    Vermilion,
    Emerald,
    Saffron,
    Magenta,
    Lagoon,
}

impl Tone {
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Cobalt => "cobalt",
            Self::Vermilion => "vermilion",
            Self::Emerald => "emerald",
            Self::Saffron => "saffron",
            Self::Magenta => "magenta",
            Self::Lagoon => "lagoon",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Day {
    pub name: &'static str,
    pub date: u8,
}

/// A timed event: it takes up a stretch of the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub day: usize,
    pub start: u16,
    pub end: u16,
    pub title: &'static str,
    pub place: &'static str,
    pub tone: Tone,
}

/// A todo: a point in time to get something done by, not a block of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Todo {
    pub day: usize,
    pub due: u16,
    pub title: &'static str,
    pub done: bool,
}

impl Todo {
    #[must_use]
    pub fn top(&self) -> f32 {
        percent(self.due)
    }

    #[must_use]
    pub fn due_clock(&self) -> String {
        clock(self.due)
    }
}

impl Event {
    #[must_use]
    pub fn start_clock(&self) -> String {
        clock(self.start)
    }
}

/// An all-day item covering the days `first..=last`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllDay {
    pub first: usize,
    pub last: usize,
    pub title: &'static str,
    pub tone: Tone,
}

/// Something that should happen this week, on no particular day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Someday {
    pub title: &'static str,
    pub detail: &'static str,
    pub todo: bool,
    pub tone: Tone,
}

/// A run of days that are different from the rest, drawn as a tinted
/// fluid-on-film background behind those columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tint {
    pub first: usize,
    pub last: usize,
    pub label: &'static str,
    pub swatch: &'static str,
    pub palette: [&'static str; 5],
}

impl Tint {
    #[must_use]
    pub fn starts_on(&self, day: impl Borrow<usize>) -> bool {
        let day = *day.borrow();
        self.first == day
    }

    /// CSS grid columns; column 1 is the hour gutter.
    #[must_use]
    pub const fn grid_start(&self) -> usize {
        self.first + 2
    }

    #[must_use]
    pub const fn grid_end(&self) -> usize {
        self.last + 3
    }

    #[must_use]
    pub const fn covers(&self, day: usize) -> bool {
        self.first <= day && day <= self.last
    }

    #[must_use]
    pub fn palette_attr(&self) -> String {
        self.palette.join(",")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Week {
    pub title: &'static str,
    pub days: [Day; 7],
    /// Index into `days`, and minutes after midnight.
    pub today: usize,
    pub now: u16,
    pub events: &'static [Event],
    pub todos: &'static [Todo],
    pub all_day: &'static [AllDay],
    pub someday: &'static [Someday],
    pub tints: &'static [Tint],
}

/// A vertical position or height inside the visible day, in percent.
#[must_use]
pub fn percent(minutes: u16) -> f32 {
    let clamped = minutes.clamp(DAY_START, DAY_END) - DAY_START;
    100.0 * f32::from(clamped) / f32::from(DAY_END - DAY_START)
}

#[must_use]
pub fn clock(minutes: u16) -> String {
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

/// An event with its column inside a group of overlapping events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub event: Event,
    pub column: u16,
    pub columns: u16,
}

impl Placed {
    #[must_use]
    pub fn top(&self) -> f32 {
        percent(self.event.start)
    }

    #[must_use]
    pub fn height(&self) -> f32 {
        percent(self.event.end) - percent(self.event.start)
    }

    #[must_use]
    pub fn left(&self) -> f32 {
        100.0 * f32::from(self.column) / f32::from(self.columns)
    }

    #[must_use]
    pub fn width(&self) -> f32 {
        100.0 / f32::from(self.columns)
    }

    #[must_use]
    pub fn span(&self) -> String {
        format!("{}–{}", clock(self.event.start), clock(self.event.end))
    }
}

/// Places one day's events. Events that overlap, directly or through a chain
/// of others, form a cluster that shares its width equally; each event takes
/// the leftmost column that is free when it starts.
#[must_use]
pub fn place_day(events: &[Event]) -> Vec<Placed> {
    let mut sorted = events.to_vec();
    sorted.sort_by_key(|e| (e.start, std::cmp::Reverse(e.end)));

    let mut placed = Vec::with_capacity(sorted.len());
    let mut cluster_start = 0;
    let mut cluster_end = 0;
    let mut column_ends: Vec<u16> = Vec::new();

    let close = |placed: &mut Vec<Placed>, from: usize, width: usize| {
        let columns = u16::try_from(width).unwrap_or(u16::MAX);
        for p in &mut placed[from..] {
            p.columns = columns;
        }
    };

    for event in sorted {
        if event.start >= cluster_end && !column_ends.is_empty() {
            close(&mut placed, cluster_start, column_ends.len());
            cluster_start = placed.len();
            column_ends.clear();
        }
        let column = column_ends
            .iter()
            .position(|&end| end <= event.start)
            .unwrap_or(column_ends.len());
        if column == column_ends.len() {
            column_ends.push(event.end);
        } else {
            column_ends[column] = event.end;
        }
        cluster_end = cluster_end.max(event.end);
        placed.push(Placed {
            event,
            column: u16::try_from(column).unwrap_or(u16::MAX),
            columns: 1,
        });
    }
    close(&mut placed, cluster_start, column_ends.len());
    placed
}

/// An all-day item with the lane (row) it sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Laned {
    pub item: AllDay,
    pub lane: usize,
}

impl Laned {
    /// CSS grid columns; column 1 is the hour gutter.
    #[must_use]
    pub const fn grid_start(&self) -> usize {
        self.item.first + 2
    }

    #[must_use]
    pub const fn grid_end(&self) -> usize {
        self.item.last + 3
    }

    #[must_use]
    pub const fn grid_row(&self) -> usize {
        self.lane + 1
    }

    #[must_use]
    pub const fn multi_day(&self) -> bool {
        self.item.last > self.item.first
    }
}

/// Stacks all-day items into as few lanes as possible.
#[must_use]
pub fn lanes(items: &[AllDay]) -> Vec<Laned> {
    let mut sorted = items.to_vec();
    sorted.sort_by_key(|i| (i.first, std::cmp::Reverse(i.last)));
    let mut lane_ends: Vec<usize> = Vec::new();
    sorted
        .into_iter()
        .map(|item| {
            let lane = lane_ends
                .iter()
                .position(|&last| last < item.first)
                .unwrap_or(lane_ends.len());
            if lane == lane_ends.len() {
                lane_ends.push(item.last);
            } else {
                lane_ends[lane] = item.last;
            }
            Laned { item, lane }
        })
        .collect()
}

impl Week {
    #[must_use]
    pub fn placed(&self, day: impl Borrow<usize>) -> Vec<Placed> {
        let day = *day.borrow();
        let events: Vec<Event> = self
            .events
            .iter()
            .filter(|e| e.day == day)
            .copied()
            .collect();
        place_day(&events)
    }

    #[must_use]
    pub fn todos_on(&self, day: impl Borrow<usize>) -> Vec<Todo> {
        let day = *day.borrow();
        self.todos
            .iter()
            .filter(|t| t.day == day)
            .copied()
            .collect()
    }

    #[must_use]
    pub fn lanes(&self) -> Vec<Laned> {
        lanes(self.all_day)
    }

    #[must_use]
    pub fn lane_count(&self) -> usize {
        self.lanes().iter().map(|l| l.lane + 1).max().unwrap_or(1)
    }

    #[must_use]
    pub fn tint_on(&self, day: impl Borrow<usize>) -> Option<&Tint> {
        let day = *day.borrow();
        self.tints.iter().find(|t| t.covers(day))
    }

    /// The event happening right now, if any.
    #[must_use]
    pub fn current(&self) -> Option<&Event> {
        self.events
            .iter()
            .find(|e| e.day == self.today && e.start <= self.now && self.now < e.end)
    }

    #[must_use]
    pub fn is_current(&self, event: &Event) -> bool {
        self.current() == Some(event)
    }

    #[must_use]
    pub fn minutes_left(&self) -> u16 {
        self.current().map_or(0, |e| e.end - self.now)
    }

    /// The next event today that has not started yet.
    #[must_use]
    pub fn next(&self) -> Option<&Event> {
        self.events
            .iter()
            .filter(|e| e.day == self.today && e.start > self.now)
            .min_by_key(|e| e.start)
    }

    #[must_use]
    pub fn is_today(&self, day: impl Borrow<usize>) -> bool {
        let day = *day.borrow();
        self.today == day
    }

    #[must_use]
    pub fn now_top(&self) -> f32 {
        percent(self.now)
    }

    #[must_use]
    pub fn now_clock(&self) -> String {
        clock(self.now)
    }

    /// Hour labels for the gutter: `(label, top percent)`.
    #[must_use]
    pub fn hours(&self) -> Vec<(String, f32)> {
        (DAY_START / 60..DAY_END / 60)
            .map(|h| (format!("{h}:00"), percent(h * 60)))
            .collect()
    }

    #[must_use]
    pub const fn open_somedays(&self) -> usize {
        self.someday.len()
    }
}

const fn at(hour: u16, minute: u16) -> u16 {
    hour * 60 + minute
}

const fn event(
    day: usize,
    start: u16,
    end: u16,
    title: &'static str,
    place: &'static str,
    tone: Tone,
) -> Event {
    Event {
        day,
        start,
        end,
        title,
        place,
        tone,
    }
}

const fn todo(day: usize, due: u16, title: &'static str, done: bool) -> Todo {
    Todo {
        day,
        due,
        title,
        done,
    }
}

pub const WEEK: Week = Week {
    title: "November 9–15, 2026",
    days: [
        Day {
            name: "Mon",
            date: 9,
        },
        Day {
            name: "Tue",
            date: 10,
        },
        Day {
            name: "Wed",
            date: 11,
        },
        Day {
            name: "Thu",
            date: 12,
        },
        Day {
            name: "Fri",
            date: 13,
        },
        Day {
            name: "Sat",
            date: 14,
        },
        Day {
            name: "Sun",
            date: 15,
        },
    ],
    today: 2,
    now: at(14, 20),
    events: &[
        event(0, at(9, 0), at(9, 30), "Standup", "Studio", Tone::Cobalt),
        event(
            0,
            at(10, 0),
            at(11, 30),
            "Deep work: shaders",
            "",
            Tone::Emerald,
        ),
        event(
            0,
            at(12, 30),
            at(13, 30),
            "Lunch with Mara",
            "Café Pigalle",
            Tone::Saffron,
        ),
        event(
            0,
            at(15, 0),
            at(16, 0),
            "1:1 with Jonas",
            "Room 2",
            Tone::Magenta,
        ),
        event(1, at(9, 0), at(9, 30), "Standup", "Studio", Tone::Cobalt),
        event(
            1,
            at(10, 0),
            at(11, 0),
            "Design review",
            "Room 1",
            Tone::Cobalt,
        ),
        event(
            1,
            at(10, 30),
            at(12, 0),
            "Interview: frontend",
            "Video call",
            Tone::Vermilion,
        ),
        event(
            1,
            at(11, 0),
            at(11, 45),
            "Vendor call",
            "Phone",
            Tone::Lagoon,
        ),
        event(
            1,
            at(14, 0),
            at(15, 30),
            "Sprint planning",
            "Studio",
            Tone::Magenta,
        ),
        event(
            1,
            at(18, 30),
            at(20, 0),
            "Climbing",
            "Boulderhalle",
            Tone::Emerald,
        ),
        event(2, at(9, 0), at(9, 30), "Standup", "Studio", Tone::Cobalt),
        event(
            2,
            at(10, 30),
            at(12, 0),
            "Typeface workshop",
            "Room 1",
            Tone::Saffron,
        ),
        event(
            2,
            at(13, 30),
            at(15, 0),
            "Prototype: Colorways",
            "Studio",
            Tone::Vermilion,
        ),
        event(
            2,
            at(15, 15),
            at(15, 45),
            "Quick sync with Ava",
            "Desk",
            Tone::Lagoon,
        ),
        event(
            2,
            at(16, 0),
            at(16, 30),
            "Dentist",
            "Dr. Weiss",
            Tone::Magenta,
        ),
        event(
            3,
            at(7, 30),
            at(11, 30),
            "Train to Berlin",
            "ICE 1033",
            Tone::Lagoon,
        ),
        event(
            3,
            at(13, 0),
            at(17, 30),
            "Off-site workshop",
            "Factory Berlin",
            Tone::Cobalt,
        ),
        event(
            3,
            at(19, 0),
            at(21, 30),
            "Team dinner",
            "Lode & Stijn",
            Tone::Saffron,
        ),
        event(
            4,
            at(9, 0),
            at(12, 0),
            "Off-site, day two",
            "Factory Berlin",
            Tone::Cobalt,
        ),
        event(
            4,
            at(15, 0),
            at(19, 0),
            "Train home",
            "ICE 1036",
            Tone::Lagoon,
        ),
        event(
            5,
            at(10, 0),
            at(12, 0),
            "Farmers market",
            "Marktplatz",
            Tone::Emerald,
        ),
        event(
            5,
            at(19, 0),
            at(21, 30),
            "Lena's birthday dinner",
            "At Lena's",
            Tone::Vermilion,
        ),
        event(6, at(11, 0), at(13, 0), "Brunch", "Home", Tone::Saffron),
    ],
    todos: &[
        todo(0, at(17, 0), "Send November invoice", true),
        todo(1, at(16, 0), "Review the Leptos island PR", true),
        todo(2, at(10, 0), "Book train to Berlin", true),
        todo(2, at(18, 0), "Buy film for the Leica", false),
        todo(3, at(12, 0), "Print workshop handouts", false),
        todo(6, at(18, 0), "Plan next week", false),
    ],
    all_day: &[
        AllDay {
            first: 0,
            last: 0,
            title: "Q4 kickoff",
            tone: Tone::Cobalt,
        },
        AllDay {
            first: 1,
            last: 1,
            title: "Release 0.3",
            tone: Tone::Emerald,
        },
        AllDay {
            first: 3,
            last: 4,
            title: "Off-site, Berlin",
            tone: Tone::Lagoon,
        },
        AllDay {
            first: 2,
            last: 5,
            title: "Mom visiting",
            tone: Tone::Magenta,
        },
        AllDay {
            first: 5,
            last: 5,
            title: "Lena's birthday",
            tone: Tone::Vermilion,
        },
    ],
    someday: &[
        Someday {
            title: "Call grandma",
            detail: "about 30 min",
            todo: false,
            tone: Tone::Saffron,
        },
        Someday {
            title: "Renew passport",
            detail: "before the 20th",
            todo: true,
            tone: Tone::Cobalt,
        },
        Someday {
            title: "Write the showcase README",
            detail: "1 h of focus",
            todo: false,
            tone: Tone::Emerald,
        },
        Someday {
            title: "Get a haircut",
            detail: "",
            todo: true,
            tone: Tone::Magenta,
        },
    ],
    tints: &[
        Tint {
            first: 3,
            last: 4,
            label: "Traveling",
            swatch: "#0a8d9b",
            palette: ["#eef8f8", "#bfeceb", "#7fd6d6", "#b9cdfc", "#a6ecd9"],
        },
        Tint {
            first: 5,
            last: 6,
            label: "Offline weekend",
            swatch: "#f0a800",
            palette: ["#fbf6ea", "#fde9b8", "#ffd27a", "#ffc2a0", "#f6dca6"],
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(start: u16, end: u16) -> Event {
        event(0, start, end, "", "", Tone::Cobalt)
    }

    fn layout(events: &[Event]) -> Vec<(u16, u16, u16, u16)> {
        place_day(events)
            .into_iter()
            .map(|p| (p.event.start, p.event.end, p.column, p.columns))
            .collect()
    }

    #[test]
    fn separate_events_take_full_width() {
        assert_eq!(
            layout(&[ev(600, 660), ev(540, 600)]),
            vec![(540, 600, 0, 1), (600, 660, 0, 1)]
        );
    }

    #[test]
    fn overlapping_events_share_the_width() {
        assert_eq!(
            layout(&[ev(600, 660), ev(630, 720)]),
            vec![(600, 660, 0, 2), (630, 720, 1, 2)]
        );
    }

    #[test]
    fn a_chain_of_overlaps_reuses_free_columns() {
        // A overlaps B, B overlaps C, but A ends before C starts.
        assert_eq!(
            layout(&[ev(600, 660), ev(630, 720), ev(660, 705)]),
            vec![(600, 660, 0, 2), (630, 720, 1, 2), (660, 705, 0, 2)]
        );
    }

    #[test]
    fn clusters_are_sized_independently() {
        assert_eq!(
            layout(&[ev(600, 660), ev(600, 660), ev(600, 660), ev(800, 860)]),
            vec![
                (600, 660, 0, 3),
                (600, 660, 1, 3),
                (600, 660, 2, 3),
                (800, 860, 0, 1)
            ]
        );
    }

    #[test]
    fn longer_events_go_left_when_starts_tie() {
        let placed = layout(&[ev(600, 630), ev(600, 720)]);
        assert_eq!(placed[0], (600, 720, 0, 2));
    }

    fn all_day(first: usize, last: usize) -> AllDay {
        AllDay {
            first,
            last,
            title: "",
            tone: Tone::Cobalt,
        }
    }

    #[test]
    fn all_day_items_stack_only_when_they_collide() {
        let laned: Vec<(usize, usize, usize)> =
            lanes(&[all_day(0, 0), all_day(3, 4), all_day(2, 5), all_day(1, 1)])
                .into_iter()
                .map(|l| (l.item.first, l.item.last, l.lane))
                .collect();
        assert_eq!(laned, vec![(0, 0, 0), (1, 1, 0), (2, 5, 0), (3, 4, 1)]);
    }

    #[test]
    fn percent_clamps_to_the_visible_day() {
        assert!(percent(0).abs() < f32::EPSILON);
        assert!((percent(23 * 60) - 100.0).abs() < f32::EPSILON);
        assert!((percent(at(14, 30)) - 50.0).abs() < 1e-4);
    }

    #[test]
    fn the_sample_week_knows_where_you_are() {
        assert_eq!(
            WEEK.current().map(|e| e.title),
            Some("Prototype: Colorways")
        );
        assert_eq!(WEEK.minutes_left(), 40);
        assert_eq!(WEEK.next().map(|e| e.title), Some("Quick sync with Ava"));
    }

    #[test]
    fn the_sample_week_is_consistent() {
        for e in WEEK.events {
            assert!(e.start < e.end && e.day < 7, "{}", e.title);
        }
        for a in WEEK.all_day {
            assert!(a.first <= a.last && a.last < 7, "{}", a.title);
        }
        for t in WEEK.tints {
            assert!(t.first <= t.last && t.last < 7, "{}", t.label);
        }
    }
}
