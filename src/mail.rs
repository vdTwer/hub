//! The model behind the webmail mockup: an inbox grouped by recency, sorted
//! by composable tags rather than folders, and one open thread that folds
//! what was read and opens what is new.

use std::borrow::Borrow;

use crate::calendar::{Category, clock};

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// When something arrived: whole days before today, and minutes after
/// midnight on that day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub days_ago: u8,
    pub at: u16,
}

/// A system mailbox in the sidebar. There are no user folders: people sort
/// mail with tags, which compose. `count` is what is waiting in it.
#[derive(Debug, Clone, Copy)]
pub struct Folder {
    pub name: &'static str,
    pub count: u16,
}

/// A tag. Any number of them can sit on a thread and be combined in a
/// search. A tag with a hue is one of the calendar's categories; the rest
/// are plain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    pub name: &'static str,
    pub hue: Option<Category>,
}

impl Tag {
    #[must_use]
    pub const fn class(&self) -> &'static str {
        match self.hue {
            Some(c) => c.class(),
            None => "plain",
        }
    }
}

const fn hue(category: Category) -> Tag {
    Tag {
        name: category.label(),
        hue: Some(category),
    }
}

const fn plain(name: &'static str) -> Tag {
    Tag { name, hue: None }
}

pub const WORK: Tag = hue(Category::Work);
pub const PERSONAL: Tag = hue(Category::Personal);
pub const TRAVEL: Tag = hue(Category::Travel);
pub const OFFSITE: Tag = plain("Off-site");
pub const RECEIPTS: Tag = plain("Receipts");
pub const FILM: Tag = plain("Film");
pub const NEWSLETTER: Tag = plain("Newsletters");

/// Every tag, in the sidebar's order: categories first.
pub const TAGS: &[Tag] = &[WORK, PERSONAL, TRAVEL, OFFSITE, FILM, RECEIPTS, NEWSLETTER];

/// One row of the message list: the newest message of a thread.
#[derive(Debug, Clone, Copy)]
pub struct Mail {
    /// The people in the thread, newest sender first.
    pub from: &'static str,
    pub subject: &'static str,
    pub snippet: &'static str,
    pub stamp: Stamp,
    pub unread: bool,
    pub pinned: bool,
    /// Messages in the thread; 1 for a single message.
    pub thread: u8,
    pub attachments: u8,
    pub tags: &'static [Tag],
    /// A reply the user owes, and by when: an outlined tag, like a todo.
    pub reply_by: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attachment {
    pub name: &'static str,
    pub size: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct Message {
    pub from: &'static str,
    pub to: &'static str,
    pub stamp: Stamp,
    /// Arrived since the thread was last read.
    pub unread: bool,
    /// Whose message this one answers, when it isn't simply the one above.
    pub reply_to: Option<&'static str>,
    pub body: &'static [&'static str],
    /// Text quoted from earlier in the thread, hidden until asked for.
    pub quote: Option<&'static str>,
    pub attachments: &'static [Attachment],
}

impl Message {
    #[must_use]
    pub fn initials(&self) -> String {
        initials(self.from)
    }

    /// The first paragraph, for a folded message.
    #[must_use]
    pub fn gist(&self) -> &'static str {
        self.body.first().copied().unwrap_or_default()
    }

    #[must_use]
    pub fn is_mine(&self) -> bool {
        self.from == ME
    }
}

/// How the user appears in a thread.
pub const ME: &str = "You";

/// Up to two initials, for a monogram.
#[must_use]
pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect()
}

/// Something in the thread that is also on the calendar.
#[derive(Debug, Clone, Copy)]
pub struct Linked {
    pub title: &'static str,
    pub when: &'static str,
    pub category: Category,
}

/// One piece of a thread as the reader shows it.
#[derive(Debug, Clone, Copy)]
pub enum Part {
    /// A read message as a single line.
    Folded(&'static Message),
    /// Read messages folded together behind a count.
    Stack(&'static [Message]),
    /// The line above the messages that arrived since the last visit.
    New(usize),
    /// A message in full.
    Full(&'static Message),
}

/// Someone in the thread, and how many messages they wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub name: &'static str,
    pub messages: usize,
}

impl Person {
    #[must_use]
    pub fn initials(&self) -> String {
        initials(self.name)
    }

    #[must_use]
    pub fn is_me(&self) -> bool {
        self.name == ME
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Thread {
    pub subject: &'static str,
    /// The hue of the thread's print.
    pub category: Category,
    pub messages: &'static [Message],
    pub linked: &'static [Linked],
    pub draft: &'static str,
}

impl Thread {
    /// The thread as the reader lays it out. Everything new is shown in
    /// full, or the newest message if nothing is new. Of what was read
    /// before, the first message and the one just before the new ones stay
    /// as single lines, for context, and the rest fold into a stack.
    #[must_use]
    pub fn parts(&self) -> Vec<Part> {
        let all = self.messages;
        let new = all.iter().filter(|m| m.unread).count();
        let first_new = all
            .iter()
            .position(|m| m.unread)
            .unwrap_or_else(|| all.len().saturating_sub(1));
        let (read, rest) = all.split_at(first_new);

        let mut parts = Vec::with_capacity(read.len().min(3) + rest.len() + 1);
        match read {
            [] => {}
            [one] => parts.push(Part::Folded(one)),
            [first, middle @ .., last] => {
                parts.push(Part::Folded(first));
                match middle {
                    [] => {}
                    [one] => parts.push(Part::Folded(one)),
                    many => parts.push(Part::Stack(many)),
                }
                parts.push(Part::Folded(last));
            }
        }
        if new > 0 {
            parts.push(Part::New(new));
        }
        parts.extend(rest.iter().map(Part::Full));
        parts
    }

    /// Everyone who wrote, in order of their first message.
    #[must_use]
    pub fn people(&self) -> Vec<Person> {
        self.messages.iter().fold(Vec::new(), |mut people, m| {
            match people.iter_mut().find(|p: &&mut Person| p.name == m.from) {
                Some(p) => p.messages += 1,
                None => people.push(Person {
                    name: m.from,
                    messages: 1,
                }),
            }
            people
        })
    }

    /// Every file in the thread, newest first, with who sent it.
    #[must_use]
    pub fn files(&self) -> Vec<(&'static Attachment, &'static str)> {
        self.messages
            .iter()
            .rev()
            .flat_map(|m| m.attachments.iter().map(|a| (a, m.from)))
            .collect()
    }

    /// The thread's palette, comma separated, for a still print.
    #[must_use]
    pub fn palette_attr(&self) -> String {
        self.category.palette().join(",")
    }
}

/// A run of rows in the message list under one heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub heading: &'static str,
    /// Indices into [`Mailbox::mails`].
    pub rows: Vec<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct Mailbox {
    /// Today's weekday, 0 for Monday, and its date in `month`.
    pub weekday: usize,
    pub date: u8,
    pub month: &'static str,
    pub folders: &'static [Folder],
    pub mails: &'static [Mail],
    /// Index into `mails` of the open thread.
    pub open: usize,
    pub thread: Thread,
}

impl Mailbox {
    /// When a stamp happened, as short as it can be said: a time today,
    /// "Yesterday", a weekday this week, or a date before that.
    #[must_use]
    pub fn when(&self, stamp: impl Borrow<Stamp>) -> String {
        let stamp = stamp.borrow();
        match stamp.days_ago {
            0 => clock(stamp.at),
            _ => self.day(stamp),
        }
    }

    /// The day of a stamp: "Today", "Yesterday", a weekday or a date.
    #[must_use]
    pub fn day(&self, stamp: impl Borrow<Stamp>) -> String {
        match usize::from(stamp.borrow().days_ago) {
            0 => "Today".to_owned(),
            1 => "Yesterday".to_owned(),
            d if d <= self.weekday => WEEKDAYS[self.weekday - d].to_owned(),
            d => format!(
                "{} {}",
                self.month,
                usize::from(self.date).saturating_sub(d)
            ),
        }
    }

    /// The day and time of a message in the open thread, in full.
    #[must_use]
    pub fn when_long(&self, stamp: impl Borrow<Stamp>) -> String {
        let stamp = stamp.borrow();
        format!("{}, {}", self.day(stamp), clock(stamp.at))
    }

    /// The days the open thread spans, such as "Mon to today".
    #[must_use]
    pub fn span(&self) -> String {
        let messages = self.thread.messages;
        let (Some(first), Some(last)) = (messages.first(), messages.last()) else {
            return String::new();
        };
        let (from, to) = (self.day(first.stamp), self.day(last.stamp));
        if from == to {
            from
        } else {
            format!("{from} to {}", to.to_lowercase())
        }
    }

    /// The list, newest first: pinned threads, then today, yesterday,
    /// earlier this week and earlier. Empty groups are left out.
    #[must_use]
    pub fn groups(&self) -> Vec<Group> {
        let bucket = |m: &Mail| match usize::from(m.stamp.days_ago) {
            _ if m.pinned => 0,
            0 => 1,
            1 => 2,
            d if d <= self.weekday => 3,
            _ => 4,
        };
        [
            "Pinned",
            "Today",
            "Yesterday",
            "Earlier this week",
            "Earlier",
        ]
        .into_iter()
        .enumerate()
        .map(|(b, heading)| Group {
            heading,
            rows: (0..self.mails.len())
                .filter(|&i| bucket(&self.mails[i]) == b)
                .collect(),
        })
        .filter(|g| !g.rows.is_empty())
        .collect()
    }

    /// Unread threads in the inbox.
    #[must_use]
    pub fn unread(&self) -> usize {
        self.mails.iter().filter(|m| m.unread).count()
    }

    #[must_use]
    pub fn is_open(&self, row: impl Borrow<usize>) -> bool {
        *row.borrow() == self.open
    }

    #[must_use]
    pub fn mail(&self, row: impl Borrow<usize>) -> &'static Mail {
        &self.mails[(*row.borrow()).min(self.mails.len().saturating_sub(1))]
    }

    /// The open thread's row.
    #[must_use]
    pub fn open_mail(&self) -> &'static Mail {
        self.mail(self.open)
    }

    /// Every tag with the number of threads that carry it.
    #[must_use]
    pub fn tags(&self) -> Vec<(Tag, usize)> {
        TAGS.iter()
            .map(|&t| (t, self.mails.iter().filter(|m| m.tags.contains(&t)).count()))
            .collect()
    }

    #[must_use]
    pub fn today(&self) -> String {
        format!(
            "{}, {} {}",
            WEEKDAYS[self.weekday % 7],
            self.month,
            self.date
        )
    }
}

const fn at(days_ago: u8, hour: u16, minute: u16) -> Stamp {
    Stamp {
        days_ago,
        at: hour * 60 + minute,
    }
}

const fn read(
    from: &'static str,
    to: &'static str,
    stamp: Stamp,
    body: &'static [&'static str],
) -> Message {
    Message {
        from,
        to,
        stamp,
        unread: false,
        reply_to: None,
        body,
        quote: None,
        attachments: &[],
    }
}

/// The inbox on Wednesday, November 11, 2026, at 14:20: the same week as
/// the calendar, with the Berlin off-site open.
pub const MAILBOX: Mailbox = Mailbox {
    weekday: 2,
    date: 11,
    month: "Nov",
    folders: &[
        Folder {
            name: "Inbox",
            count: 4,
        },
        Folder {
            name: "Snoozed",
            count: 2,
        },
        Folder {
            name: "Drafts",
            count: 1,
        },
        Folder {
            name: "Sent",
            count: 0,
        },
        Folder {
            name: "Archive",
            count: 0,
        },
    ],
    mails: &[
        Mail {
            from: "Jonas, Ava, Priya, me",
            subject: "Off-site in Berlin: travel and agenda",
            snippet: "Booked the same train, coach 7. I'm saving you the window seat.",
            stamp: at(0, 14, 8),
            unread: false,
            pinned: true,
            thread: 7,
            attachments: 3,
            tags: &[TRAVEL, WORK, OFFSITE],
            reply_by: None,
        },
        Mail {
            from: "Lena, me",
            subject: "Dinner on Saturday",
            snippet: "Table for eight at seven. Can you bring the projector? \
                      I want to show the old Super 8 reels.",
            stamp: at(1, 21, 4),
            unread: false,
            pinned: true,
            thread: 3,
            attachments: 0,
            tags: &[PERSONAL, FILM],
            reply_by: Some("Reply by Thu"),
        },
        Mail {
            from: "Jonas Weber, me",
            subject: "Notes from Monday's 1:1",
            snippet: "Three things we agreed on, and one I still owe you: \
                      the hiring plan for Q1.",
            stamp: at(0, 12, 40),
            unread: true,
            pinned: false,
            thread: 2,
            attachments: 0,
            tags: &[WORK],
            reply_by: None,
        },
        Mail {
            from: "Mara Okafor",
            subject: "Lunch next week?",
            snippet: "Café Pigalle was lovely. Same time next Tuesday, \
                      or are you still in Berlin?",
            stamp: at(0, 11, 15),
            unread: true,
            pinned: false,
            thread: 1,
            attachments: 0,
            tags: &[PERSONAL],
            reply_by: None,
        },
        Mail {
            from: "Rail tickets",
            subject: "Your booking: Bonn to Berlin, Thu 7:34",
            snippet: "Coach 7, seat 64, window. Your ticket is attached \
                      and ready to show on board.",
            stamp: at(0, 8, 10),
            unread: true,
            pinned: false,
            thread: 1,
            attachments: 1,
            tags: &[TRAVEL, OFFSITE, RECEIPTS],
            reply_by: None,
        },
        Mail {
            from: "Kino am Rhein",
            subject: "This week on 35mm",
            snippet: "Three nights of restored prints, a projectionist's talk \
                      on Friday, and the last outdoor screening.",
            stamp: at(0, 7, 2),
            unread: true,
            pinned: false,
            thread: 1,
            attachments: 0,
            tags: &[FILM, NEWSLETTER],
            reply_by: None,
        },
        Mail {
            from: "Ava Lindqvist, Jonas, me",
            subject: "Release 0.3 is out",
            snippet: "Thanks, everyone. The changelog is in the wiki, \
                      and the retro is on Thursday's agenda.",
            stamp: at(1, 17, 45),
            unread: false,
            pinned: false,
            thread: 3,
            attachments: 0,
            tags: &[WORK],
            reply_by: None,
        },
        Mail {
            from: "Jonas Weber, me",
            subject: "Sprint planning: draft board",
            snippet: "I moved the Leptos islands work up. \
                      Could you size the review tickets?",
            stamp: at(2, 9, 30),
            unread: false,
            pinned: false,
            thread: 2,
            attachments: 1,
            tags: &[WORK],
            reply_by: Some("Reply by Fri"),
        },
        Mail {
            from: "Film lab",
            subject: "Your scans are ready",
            snippet: "Two rolls of Portra 400, 72 frames, scanned at 16 bits. \
                      The contact sheet is attached.",
            stamp: at(2, 8, 5),
            unread: false,
            pinned: false,
            thread: 1,
            attachments: 3,
            tags: &[PERSONAL, FILM, RECEIPTS],
            reply_by: None,
        },
        Mail {
            from: "Mara Okafor, me",
            subject: "Photos from the climb",
            snippet: "The light on the last route was unreal. \
                      I put the best ones in a shared album.",
            stamp: at(5, 19, 20),
            unread: false,
            pinned: false,
            thread: 2,
            attachments: 0,
            tags: &[PERSONAL],
            reply_by: None,
        },
    ],
    open: 0,
    thread: Thread {
        subject: "Off-site in Berlin: travel and agenda",
        category: Category::Travel,
        messages: &[
            read(
                "Ava Lindqvist",
                "Team",
                at(2, 9, 12),
                &[
                    "The off-site is on: Thursday and Friday at Factory Berlin. \
                   I'll send the agenda once the rooms are confirmed.",
                ],
            ),
            Message {
                reply_to: Some("Ava"),
                ..read(
                    "Jonas Weber",
                    "Ava, Team",
                    at(2, 10, 3),
                    &["Could we start after one on Thursday? \
                       The early trains from Bonn are always full."],
                )
            },
            read(
                "Priya Nair",
                "Ava, Team",
                at(2, 11, 40),
                &["Later works for me as well. I'm coming from Hamburg, \
                   and I'd love a room with a projector for the demos."],
            ),
            Message {
                reply_to: Some("Jonas"),
                quote: Some("Could we start after one on Thursday?"),
                ..read(
                    ME,
                    "Ava, Jonas, Priya",
                    at(1, 16, 20),
                    &[
                        "One works for me too. I'll take the 7:34 and work on the train.",
                        "Should I bring the Colorways prototype for Friday's review?",
                    ],
                )
            },
            Message {
                attachments: &[Attachment {
                    name: "Floor plan.pdf",
                    size: "1.2 MB",
                }],
                ..read(
                    "Ava Lindqvist",
                    "Team",
                    at(1, 18, 5),
                    &["Both rooms are confirmed, and both have projectors. \
                       The floor plan is attached; we're on the third floor."],
                )
            },
            Message {
                from: "Ava Lindqvist",
                to: "You, Jonas, Priya",
                stamp: at(0, 13, 52),
                unread: true,
                reply_to: Some("you"),
                body: &[
                    "Tickets are booked. The train leaves Bonn at 7:34 on Thursday, \
                     and we start at one, so there's time for lunch.",
                    "Yes, please bring Colorways. Friday morning is for reviews, and \
                     the afternoon is free for anyone who wants to stay. The train \
                     home leaves at three.",
                ],
                quote: Some("Should I bring the Colorways prototype for Friday's review?"),
                attachments: &[
                    Attachment {
                        name: "Itinerary.pdf",
                        size: "182 KB",
                    },
                    Attachment {
                        name: "Agenda, Thu–Fri.pdf",
                        size: "96 KB",
                    },
                ],
            },
            Message {
                unread: true,
                reply_to: Some("Ava"),
                ..read(
                    "Jonas Weber",
                    "Ava, You, Priya",
                    at(0, 14, 8),
                    &["Booked the same train, coach 7. I'm saving you the window seat."],
                )
            },
        ],
        linked: &[
            Linked {
                title: "Train to Berlin",
                when: "Thu, 7:34",
                category: Category::Travel,
            },
            Linked {
                title: "Off-site workshop",
                when: "Thu, 13:00–17:30",
                category: Category::Work,
            },
            Linked {
                title: "Train home",
                when: "Fri, 15:00",
                category: Category::Travel,
            },
        ],
        draft: "Thank you both! I'll bring Colorways on a laptop and a tablet, \
                so people can stir it by hand.",
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(parts: &[Part]) -> Vec<String> {
        parts
            .iter()
            .map(|p| match p {
                Part::Folded(_) => "folded".to_owned(),
                Part::Stack(m) => format!("stack {}", m.len()),
                Part::New(n) => format!("new {n}"),
                Part::Full(_) => "full".to_owned(),
            })
            .collect()
    }

    #[test]
    fn when_is_a_clock_today_and_a_day_before() {
        let m = MAILBOX;
        assert_eq!(m.when(at(0, 9, 5)), "9:05");
        assert_eq!(m.when(at(1, 9, 5)), "Yesterday");
        assert_eq!(m.when(at(2, 9, 5)), "Mon");
        assert_eq!(m.when(at(5, 9, 5)), "Nov 6");
    }

    #[test]
    fn when_long_names_the_day_and_time() {
        assert_eq!(MAILBOX.when_long(at(0, 13, 52)), "Today, 13:52");
        assert_eq!(MAILBOX.when_long(at(2, 9, 12)), "Mon, 9:12");
    }

    #[test]
    fn span_runs_from_the_first_day_to_the_last() {
        assert_eq!(MAILBOX.span(), "Mon to today");
        let short = Mailbox {
            thread: Thread {
                messages: &MAILBOX.thread.messages[5..],
                ..MAILBOX.thread
            },
            ..MAILBOX
        };
        assert_eq!(short.span(), "Today");
    }

    #[test]
    fn groups_run_newest_first_and_pin_once() {
        let groups = MAILBOX.groups();
        let headings: Vec<_> = groups.iter().map(|g| g.heading).collect();
        assert_eq!(
            headings,
            [
                "Pinned",
                "Today",
                "Yesterday",
                "Earlier this week",
                "Earlier"
            ]
        );
        let mut rows: Vec<usize> = groups.iter().flat_map(|g| g.rows.clone()).collect();
        rows.sort_unstable();
        rows.dedup();
        assert_eq!(rows.len(), MAILBOX.mails.len());
        assert!(groups[0].rows.iter().all(|&i| MAILBOX.mails[i].pinned));
    }

    #[test]
    fn rows_in_a_group_are_newest_first() {
        for group in MAILBOX.groups() {
            let order: Vec<_> = group
                .rows
                .iter()
                .map(|&i| {
                    let s = MAILBOX.mails[i].stamp;
                    (std::cmp::Reverse(s.days_ago), s.at)
                })
                .collect();
            assert!(order.windows(2).all(|w| w[0] >= w[1]), "{}", group.heading);
        }
    }

    #[test]
    fn empty_groups_are_left_out() {
        let lonely = Mailbox {
            mails: &MAILBOX.mails[2..3],
            ..MAILBOX
        };
        let headings: Vec<_> = lonely.groups().iter().map(|g| g.heading).collect();
        assert_eq!(headings, ["Today"]);
    }

    #[test]
    fn inbox_count_is_the_unread_threads() {
        assert_eq!(usize::from(MAILBOX.folders[0].count), MAILBOX.unread());
    }

    #[test]
    fn the_open_row_is_the_open_thread() {
        let row = MAILBOX.open_mail();
        let thread = MAILBOX.thread;
        assert_eq!(row.subject, thread.subject);
        assert_eq!(usize::from(row.thread), thread.messages.len());
        assert_eq!(usize::from(row.attachments), thread.files().len());
        assert!(row.tags.iter().any(|t| t.hue == Some(thread.category)));
        assert_eq!(thread.messages.last().map(|m| m.stamp), Some(row.stamp));
        assert_eq!(thread.messages.last().map(Message::gist), Some(row.snippet));
    }

    #[test]
    fn a_thread_opens_what_is_new_and_folds_what_was_read() {
        let parts = MAILBOX.thread.parts();
        assert_eq!(
            kinds(&parts),
            ["folded", "stack 3", "folded", "new 2", "full", "full"]
        );
    }

    #[test]
    fn a_read_thread_opens_its_newest_message() {
        const READ: &[Message] = &[
            read("A", "B", at(1, 9, 0), &["one"]),
            read("B", "A", at(1, 10, 0), &["two"]),
            read("A", "B", at(0, 9, 0), &["three"]),
        ];
        let thread = Thread {
            messages: READ,
            ..MAILBOX.thread
        };
        assert_eq!(kinds(&thread.parts()), ["folded", "folded", "full"]);
        let single = Thread {
            messages: &READ[..1],
            ..thread
        };
        assert_eq!(kinds(&single.parts()), ["full"]);
    }

    #[test]
    fn a_lone_middle_message_stays_a_line() {
        const FOUR: &[Message] = &[
            read("A", "B", at(1, 9, 0), &["one"]),
            read("B", "A", at(1, 10, 0), &["two"]),
            read("A", "B", at(0, 9, 0), &["three"]),
            read("B", "A", at(0, 10, 0), &["four"]),
        ];
        let thread = Thread {
            messages: FOUR,
            ..MAILBOX.thread
        };
        assert_eq!(
            kinds(&thread.parts()),
            ["folded", "folded", "folded", "full"]
        );
    }

    #[test]
    fn every_message_appears_exactly_once() {
        let shown: usize = MAILBOX
            .thread
            .parts()
            .iter()
            .map(|p| match p {
                Part::Stack(m) => m.len(),
                Part::New(_) => 0,
                Part::Folded(_) | Part::Full(_) => 1,
            })
            .sum();
        assert_eq!(shown, MAILBOX.thread.messages.len());
    }

    #[test]
    fn people_are_counted_in_order_of_appearance() {
        let people = MAILBOX.thread.people();
        let names: Vec<_> = people.iter().map(|p| (p.name, p.messages)).collect();
        assert_eq!(
            names,
            [
                ("Ava Lindqvist", 3),
                ("Jonas Weber", 2),
                ("Priya Nair", 1),
                (ME, 1)
            ]
        );
        assert!(people[3].is_me());
    }

    #[test]
    fn files_are_newest_first_with_their_sender() {
        let files = MAILBOX.thread.files();
        let names: Vec<_> = files.iter().map(|(a, _)| a.name).collect();
        assert_eq!(
            names,
            ["Itinerary.pdf", "Agenda, Thu–Fri.pdf", "Floor plan.pdf"]
        );
        assert!(files.iter().all(|(_, from)| *from == "Ava Lindqvist"));
    }

    #[test]
    fn tags_compose_and_are_counted_per_thread() {
        let tags = MAILBOX.tags();
        assert_eq!(tags.len(), TAGS.len());
        let count = |t: Tag| tags.iter().find(|(x, _)| *x == t).map(|(_, n)| *n);
        assert_eq!(count(FILM), Some(3));
        assert_eq!(count(OFFSITE), Some(2));
        assert!(MAILBOX.mails.iter().any(|m| m.tags.len() >= 3));
        assert!(tags.iter().all(|(_, n)| *n > 0));
    }

    #[test]
    fn plain_tags_have_no_hue() {
        assert_eq!(FILM.class(), "plain");
        assert_eq!(TRAVEL.class(), "travel");
        assert_eq!(TRAVEL.name, "Travel");
    }

    #[test]
    fn initials_take_the_first_two_names() {
        assert_eq!(initials("Ava Lindqvist"), "AL");
        assert_eq!(initials(ME), "Y");
    }

    #[test]
    fn today_reads_like_a_date() {
        assert_eq!(MAILBOX.today(), "Wed, Nov 11");
    }
}
