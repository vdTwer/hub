//! A calendar week for the mockup, plus the layout math that places it:
//! side-by-side columns for overlapping events and stacked lanes for
//! all-day items that span several days.

use std::borrow::Borrow;

/// The visible part of each day, in minutes after midnight.
pub const DAY_START: u16 = 7 * 60;
pub const DAY_END: u16 = 22 * 60;

/// Which calendar an item belongs to. Color encodes this and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Work,
    Personal,
    Travel,
}

impl Category {
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Personal => "personal",
            Self::Travel => "travel",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Work => "Work",
            Self::Personal => "Personal",
            Self::Travel => "Travel",
        }
    }

    /// Five colors for a still print of the fluid in this category's hue:
    /// base, soft, primary, accent and secondary.
    #[must_use]
    pub const fn palette(self) -> [&'static str; 5] {
        match self {
            Self::Work => ["#e3e6fb", "#a9b6ff", "#3d57e6", "#9b7bff", "#6cc2ff"],
            Self::Personal => ["#fcefe0", "#ffcf9e", "#ff8a4c", "#ff6f7d", "#ffc04d"],
            Self::Travel => ["#d9f1ee", "#8fdcd5", "#1fa7ad", "#6f8cff", "#3cc9a6"],
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
    pub category: Category,
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
    pub category: Category,
}

/// Something that should happen this week, on no particular day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Someday {
    pub title: &'static str,
    pub detail: &'static str,
    pub todo: bool,
    pub category: Category,
}

/// A run of days that are different from the rest, washed in a still print
/// of grainy fluid color drawn from `palette`. `swatch` is its label color.
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

    /// The palette as the stage reads it from `data-palette`.
    #[must_use]
    pub fn palette_attr(&self) -> String {
        self.palette.join(",")
    }
}

/// One still print behind the week. Tints that touch share a print, so the
/// fluid runs on across the boundary and the colors only cross-fade over a
/// narrow seam; a lone tint gets a print of its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Print {
    pub first: usize,
    pub last: usize,
    pub palette: [&'static str; 5],
    /// The palette after the seam, and where the seam sits as a fraction of
    /// the print's width.
    pub next: Option<([&'static str; 5], f32)>,
}

impl Print {
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
    pub fn palette_attr(&self) -> String {
        self.palette.join(",")
    }

    #[must_use]
    pub fn next_palette_attr(&self) -> String {
        self.next.map(|(p, _)| p.join(",")).unwrap_or_default()
    }

    /// The seam as a fraction of the print's width, or an empty string.
    #[must_use]
    pub fn seam_attr(&self) -> String {
        self.next
            .map(|(_, at)| format!("{at:.4}"))
            .unwrap_or_default()
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

/// `a / b` for small counts such as days, as a float.
fn ratio(a: usize, b: usize) -> f32 {
    let small = |n: usize| u16::try_from(n).map_or_else(|_| f32::from(u16::MAX), f32::from);
    small(a) / small(b)
}

#[must_use]
pub fn clock(minutes: u16) -> String {
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

/// An event and how deep it sits in a cascade of overlapping events.
/// Depth 0 is flush left; each level is indented a little and drawn on top,
/// so overlapping events overlap, as in Apple Calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub event: Event,
    pub depth: u16,
    /// Minutes from its start until a later event lands on top of it.
    pub visible: u16,
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
    pub fn span(&self) -> String {
        format!("{}–{}", clock(self.event.start), clock(self.event.end))
    }

    /// Too little of it shows for two lines of text, because it is short or
    /// a later event covers it early; title and time then share one line.
    #[must_use]
    pub const fn short(&self) -> bool {
        self.visible <= 45
    }

    /// The full text, for a tooltip when the slip truncates it.
    #[must_use]
    pub fn tooltip(&self) -> String {
        let place = if self.event.place.is_empty() {
            String::new()
        } else {
            format!(", {}", self.event.place)
        };
        format!("{}, {}{place}", self.event.title, self.span())
    }
}

/// Places one day's events in drawing order. An event that starts while
/// others are still running nests one level deeper than the deepest of them.
#[must_use]
pub fn place_day(events: &[Event]) -> Vec<Placed> {
    let mut sorted = events.to_vec();
    sorted.sort_by_key(|e| (e.start, std::cmp::Reverse(e.end)));
    let mut placed =
        sorted
            .iter()
            .enumerate()
            .fold(Vec::new(), |mut placed: Vec<Placed>, (i, &event)| {
                let depth = placed[..i]
                    .iter()
                    .filter(|p: &&Placed| p.event.end > event.start)
                    .map(|p| p.depth + 1)
                    .max()
                    .unwrap_or(0);
                placed.push(Placed {
                    event,
                    depth,
                    visible: event.end - event.start,
                });
                placed
            });
    // Later events start no earlier, so the first one to start before an
    // event ends is the one that covers it soonest.
    let starts: Vec<u16> = placed.iter().map(|p| p.event.start).collect();
    for (i, p) in placed.iter_mut().enumerate() {
        if let Some(&cover) = starts[i + 1..].iter().find(|&&s| s < p.event.end) {
            p.visible = cover - p.event.start;
        }
    }
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
    pub fn prints(&self) -> Vec<Print> {
        let mut tints = self.tints.to_vec();
        tints.sort_by_key(|t| t.first);
        tints
            .into_iter()
            .fold(Vec::new(), |mut prints: Vec<Print>, t| {
                match prints.last_mut() {
                    Some(p) if p.next.is_none() && p.last + 1 == t.first => {
                        let seam = ratio(t.first - p.first, t.last + 1 - p.first);
                        p.next = Some((t.palette, seam));
                        p.last = t.last;
                    }
                    _ => prints.push(Print {
                        first: t.first,
                        last: t.last,
                        palette: t.palette,
                        next: None,
                    }),
                }
                prints
            })
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

    /// Hour labels for the gutter: `(label, top percent)`. An hour within a
    /// quarter of an hour of now gives way to the time label for now.
    #[must_use]
    pub fn hours(&self) -> Vec<(String, f32)> {
        (DAY_START / 60..DAY_END / 60)
            .filter(|h| (h * 60).abs_diff(self.now) >= 15)
            .map(|h| (format!("{h}:00"), percent(h * 60)))
            .collect()
    }

    /// Whether an event is over, so it can recede.
    #[must_use]
    pub const fn is_past(&self, event: &Event) -> bool {
        event.day < self.today || (event.day == self.today && event.end <= self.now)
    }

    #[must_use]
    pub const fn all_day_past(&self, item: &AllDay) -> bool {
        item.last < self.today
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
    category: Category,
) -> Event {
    Event {
        day,
        start,
        end,
        title,
        place,
        category,
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
        event(0, at(9, 0), at(9, 30), "Standup", "Studio", Category::Work),
        event(
            0,
            at(10, 0),
            at(11, 30),
            "Deep work: shaders",
            "",
            Category::Work,
        ),
        event(
            0,
            at(12, 30),
            at(13, 30),
            "Lunch with Mara",
            "Café Pigalle",
            Category::Personal,
        ),
        event(
            0,
            at(15, 0),
            at(16, 0),
            "1:1 with Jonas",
            "Room 2",
            Category::Work,
        ),
        event(1, at(9, 0), at(9, 30), "Standup", "Studio", Category::Work),
        event(
            1,
            at(10, 0),
            at(11, 0),
            "Design review",
            "Room 1",
            Category::Work,
        ),
        event(
            1,
            at(10, 30),
            at(12, 0),
            "Interview: frontend",
            "Video call",
            Category::Work,
        ),
        event(
            1,
            at(11, 0),
            at(11, 45),
            "Vendor call",
            "Phone",
            Category::Work,
        ),
        event(
            1,
            at(14, 0),
            at(15, 30),
            "Sprint planning",
            "Studio",
            Category::Work,
        ),
        event(
            1,
            at(18, 30),
            at(20, 0),
            "Climbing",
            "Boulderhalle",
            Category::Personal,
        ),
        event(2, at(9, 0), at(9, 30), "Standup", "Studio", Category::Work),
        event(
            2,
            at(10, 30),
            at(12, 0),
            "Typeface workshop",
            "Room 1",
            Category::Work,
        ),
        event(
            2,
            at(11, 30),
            at(12, 30),
            "Call with the bank",
            "Phone",
            Category::Personal,
        ),
        event(
            2,
            at(13, 30),
            at(15, 0),
            "Prototype: Colorways",
            "Studio",
            Category::Work,
        ),
        event(
            2,
            at(15, 15),
            at(15, 45),
            "Quick sync with Ava",
            "Desk",
            Category::Work,
        ),
        event(
            2,
            at(16, 0),
            at(16, 30),
            "Dentist",
            "Dr. Weiss",
            Category::Personal,
        ),
        event(
            3,
            at(7, 30),
            at(11, 30),
            "Train to Berlin",
            "ICE 1033",
            Category::Travel,
        ),
        event(
            3,
            at(13, 0),
            at(17, 30),
            "Off-site workshop",
            "Factory Berlin",
            Category::Work,
        ),
        event(
            3,
            at(19, 0),
            at(21, 30),
            "Team dinner",
            "Lode & Stijn",
            Category::Work,
        ),
        event(
            4,
            at(9, 0),
            at(12, 0),
            "Off-site, day two",
            "Factory Berlin",
            Category::Work,
        ),
        event(
            4,
            at(15, 0),
            at(19, 0),
            "Train home",
            "ICE 1036",
            Category::Travel,
        ),
        event(
            5,
            at(10, 0),
            at(12, 0),
            "Farmers market",
            "Marktplatz",
            Category::Personal,
        ),
        event(
            5,
            at(19, 0),
            at(21, 30),
            "Lena's birthday dinner",
            "At Lena's",
            Category::Personal,
        ),
        event(
            6,
            at(11, 0),
            at(13, 0),
            "Brunch",
            "Home",
            Category::Personal,
        ),
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
            category: Category::Work,
        },
        AllDay {
            first: 1,
            last: 1,
            title: "Release 0.3",
            category: Category::Work,
        },
        AllDay {
            first: 3,
            last: 4,
            title: "Off-site, Berlin",
            category: Category::Travel,
        },
        AllDay {
            first: 2,
            last: 5,
            title: "Mom visiting",
            category: Category::Personal,
        },
        AllDay {
            first: 5,
            last: 5,
            title: "Lena's birthday",
            category: Category::Personal,
        },
    ],
    someday: &[
        Someday {
            title: "Call grandma",
            detail: "about 30 min",
            todo: false,
            category: Category::Personal,
        },
        Someday {
            title: "Renew passport",
            detail: "before the 20th",
            todo: true,
            category: Category::Personal,
        },
        Someday {
            title: "Write the showcase README",
            detail: "1 h of focus",
            todo: false,
            category: Category::Work,
        },
        Someday {
            title: "Get a haircut",
            detail: "",
            todo: true,
            category: Category::Personal,
        },
    ],
    tints: &[
        Tint {
            first: 3,
            last: 4,
            label: "traveling",
            swatch: "#0a8d9b",
            palette: Category::Travel.palette(),
        },
        Tint {
            first: 5,
            last: 6,
            label: "offline weekend",
            swatch: "#f0a800",
            palette: ["#fdf3dc", "#ffd98a", "#f5a524", "#ff7a45", "#ffc65c"],
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(start: u16, end: u16) -> Event {
        event(0, start, end, "", "", Category::Work)
    }

    fn layout(events: &[Event]) -> Vec<(u16, u16, u16)> {
        place_day(events)
            .into_iter()
            .map(|p| (p.event.start, p.event.end, p.depth))
            .collect()
    }

    #[test]
    fn separate_events_sit_flush_left_in_time_order() {
        assert_eq!(
            layout(&[ev(600, 660), ev(540, 600)]),
            vec![(540, 600, 0), (600, 660, 0)]
        );
    }

    #[test]
    fn a_later_overlapping_event_cascades_over_the_earlier_one() {
        assert_eq!(
            layout(&[ev(630, 720), ev(600, 660)]),
            vec![(600, 660, 0), (630, 720, 1)]
        );
    }

    #[test]
    fn depth_follows_the_events_still_running() {
        // A overlaps B and B overlaps C, but A has ended when C starts:
        // C still nests one level deeper than B, the event it lands on.
        assert_eq!(
            layout(&[ev(600, 660), ev(630, 720), ev(660, 705)]),
            vec![(600, 660, 0), (630, 720, 1), (660, 705, 2)]
        );
    }

    #[test]
    fn the_cascade_resets_once_the_day_clears() {
        assert_eq!(
            layout(&[ev(600, 660), ev(610, 640), ev(700, 760)]),
            vec![(600, 660, 0), (610, 640, 1), (700, 760, 0)]
        );
    }

    #[test]
    fn events_up_to_45_minutes_are_short() {
        let short = |start, end| place_day(&[ev(start, end)])[0].short();
        assert!(short(540, 585));
        assert!(!short(540, 600));
    }

    #[test]
    fn an_event_covered_early_gets_a_one_line_header() {
        let placed = place_day(&[ev(600, 660), ev(630, 720)]);
        assert_eq!((placed[0].visible, placed[0].short()), (30, true));
        assert_eq!((placed[1].visible, placed[1].short()), (90, false));
    }

    #[test]
    fn hour_labels_make_room_for_now() {
        let week = Week {
            now: at(14, 10),
            ..WEEK
        };
        let labels: Vec<String> = week.hours().into_iter().map(|(l, _)| l).collect();
        assert!(!labels.contains(&"14:00".to_string()));
        assert!(labels.contains(&"15:00".to_string()));
    }

    #[test]
    fn events_before_now_are_past() {
        let ev_at = |day, start, end| event(day, start, end, "", "", Category::Work);
        assert!(WEEK.is_past(&ev_at(1, at(9, 0), at(10, 0))));
        assert!(WEEK.is_past(&ev_at(2, at(9, 0), at(9, 30))));
        assert!(!WEEK.is_past(&ev_at(2, at(13, 30), at(15, 0))));
        assert!(!WEEK.is_past(&ev_at(3, at(9, 0), at(10, 0))));
    }

    #[test]
    fn longer_events_go_underneath_when_starts_tie() {
        assert_eq!(
            layout(&[ev(600, 630), ev(600, 720)]),
            vec![(600, 720, 0), (600, 630, 1)]
        );
    }

    fn all_day(first: usize, last: usize) -> AllDay {
        AllDay {
            first,
            last,
            title: "",
            category: Category::Work,
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
    fn touching_tints_share_one_print_with_a_seam() {
        let prints = WEEK.prints();
        assert_eq!(prints.len(), 1);
        let p = prints[0];
        assert_eq!((p.first, p.last), (3, 6));
        assert_eq!(p.palette, WEEK.tints[0].palette);
        assert_eq!(p.next, Some((WEEK.tints[1].palette, 0.5)));
        assert_eq!(p.seam_attr(), "0.5000");
    }

    #[test]
    fn apart_tints_keep_their_own_prints() {
        static APART: [Tint; 2] = [
            Tint {
                last: 1,
                ..WEEK.tints[0]
            },
            Tint {
                first: 3,
                last: 3,
                ..WEEK.tints[1]
            },
        ];
        let week = Week {
            tints: &APART,
            ..WEEK
        };
        let prints = week.prints();
        assert_eq!(prints.len(), 2);
        assert!(prints.iter().all(|p| p.next.is_none()));
        assert_eq!(prints[1].seam_attr(), "");
    }

    #[test]
    fn a_seam_sits_where_the_second_tint_starts() {
        static THREE: [Tint; 2] = [
            Tint {
                first: 0,
                last: 2,
                ..WEEK.tints[0]
            },
            Tint {
                first: 3,
                last: 3,
                ..WEEK.tints[1]
            },
        ];
        let week = Week {
            tints: &THREE,
            ..WEEK
        };
        let p = week.prints()[0];
        assert_eq!(p.next.map(|(_, at)| at), Some(0.75));
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
