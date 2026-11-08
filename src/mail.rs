//! The model behind the webmail mockup: an inbox grouped by recency, and one
//! open thread whose older messages fold into single lines.

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

/// A folder in the sidebar. `count` is what is waiting in it, if anything.
#[derive(Debug, Clone, Copy)]
pub struct Folder {
    pub name: &'static str,
    pub count: u16,
}

/// One row of the message list: the newest message of a thread.
#[derive(Debug, Clone, Copy)]
pub struct Mail {
    pub from: &'static str,
    pub subject: &'static str,
    pub snippet: &'static str,
    pub stamp: Stamp,
    pub unread: bool,
    pub pinned: bool,
    /// Messages in the thread; 1 for a single message.
    pub thread: u8,
    pub attachments: u8,
    pub category: Option<Category>,
    /// A reply the user owes, and by when: an outlined tag, like a todo.
    pub reply_by: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub struct Attachment {
    pub name: &'static str,
    pub size: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct Message {
    pub from: &'static str,
    pub to: &'static str,
    pub stamp: Stamp,
    pub body: &'static [&'static str],
    pub attachments: &'static [Attachment],
}

impl Message {
    /// The first paragraph, for a folded message.
    #[must_use]
    pub fn gist(&self) -> &'static str {
        self.body.first().copied().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Thread {
    pub subject: &'static str,
    pub category: Category,
    pub messages: &'static [Message],
    pub draft: &'static str,
}

/// How many of the newest messages a thread shows in full.
const EXPANDED: usize = 2;

impl Thread {
    /// Older messages, folded into one line each.
    #[must_use]
    pub fn folded(&self) -> &'static [Message] {
        let cut = self.messages.len().saturating_sub(EXPANDED);
        &self.messages[..cut]
    }

    /// The newest messages, shown in full.
    #[must_use]
    pub fn expanded(&self) -> &'static [Message] {
        let cut = self.messages.len().saturating_sub(EXPANDED);
        &self.messages[cut..]
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
        let days = usize::from(stamp.days_ago);
        match days {
            0 => clock(stamp.at),
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
        match stamp.days_ago {
            0 => format!("Today, {}", clock(stamp.at)),
            _ => format!("{}, {}", self.when(stamp), clock(stamp.at)),
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

    /// Threads per category, for the sidebar's labels.
    #[must_use]
    pub fn labels(&self) -> Vec<(Category, usize)> {
        [Category::Work, Category::Personal, Category::Travel]
            .into_iter()
            .map(|c| {
                let n = self.mails.iter().filter(|m| m.category == Some(c)).count();
                (c, n)
            })
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
            from: "Ava Lindqvist",
            subject: "Off-site in Berlin: travel and agenda",
            snippet: "Tickets are booked. The train leaves Bonn at 7:34 on Thursday, \
                      and we start at Factory at one.",
            stamp: at(0, 13, 52),
            unread: false,
            pinned: true,
            thread: 4,
            attachments: 2,
            category: Some(Category::Travel),
            reply_by: None,
        },
        Mail {
            from: "Lena",
            subject: "Dinner on Saturday",
            snippet: "Table for eight at seven. Can you bring the projector? \
                      I want to show the old Super 8 reels.",
            stamp: at(1, 21, 4),
            unread: false,
            pinned: true,
            thread: 3,
            attachments: 0,
            category: Some(Category::Personal),
            reply_by: Some("Reply by Thu"),
        },
        Mail {
            from: "Jonas Weber",
            subject: "Notes from Monday's 1:1",
            snippet: "Three things we agreed on, and one I still owe you: \
                      the hiring plan for Q1.",
            stamp: at(0, 12, 40),
            unread: true,
            pinned: false,
            thread: 2,
            attachments: 0,
            category: Some(Category::Work),
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
            category: Some(Category::Personal),
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
            category: Some(Category::Travel),
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
            category: None,
            reply_by: None,
        },
        Mail {
            from: "Ava Lindqvist",
            subject: "Release 0.3 is out",
            snippet: "Thanks, everyone. The changelog is in the wiki, \
                      and the retro is on Thursday's agenda.",
            stamp: at(1, 17, 45),
            unread: false,
            pinned: false,
            thread: 3,
            attachments: 0,
            category: Some(Category::Work),
            reply_by: None,
        },
        Mail {
            from: "Jonas Weber",
            subject: "Sprint planning: draft board",
            snippet: "I moved the Leptos islands work up. \
                      Could you size the review tickets?",
            stamp: at(2, 9, 30),
            unread: false,
            pinned: false,
            thread: 2,
            attachments: 1,
            category: Some(Category::Work),
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
            category: Some(Category::Personal),
            reply_by: None,
        },
        Mail {
            from: "Mara Okafor",
            subject: "Photos from the climb",
            snippet: "The light on the last route was unreal. \
                      I put the best ones in a shared album.",
            stamp: at(5, 19, 20),
            unread: false,
            pinned: false,
            thread: 2,
            attachments: 0,
            category: Some(Category::Personal),
            reply_by: None,
        },
    ],
    open: 0,
    thread: Thread {
        subject: "Off-site in Berlin: travel and agenda",
        category: Category::Travel,
        messages: &[
            Message {
                from: "Ava Lindqvist",
                to: "Team",
                stamp: at(2, 9, 12),
                body: &[
                    "The off-site is on: Thursday and Friday at Factory Berlin. \
                         I'll send the agenda once the rooms are confirmed.",
                ],
                attachments: &[],
            },
            Message {
                from: "Jonas Weber",
                to: "Ava, Team",
                stamp: at(2, 10, 3),
                body: &["Could we start after one on Thursday? \
                         The early trains from Bonn are always full."],
                attachments: &[],
            },
            Message {
                from: "You",
                to: "Ava, Jonas",
                stamp: at(1, 16, 20),
                body: &[
                    "One works for me too. I'll take the 7:34 and work on the train.",
                    "Should I bring the Colorways prototype for Friday's review?",
                ],
                attachments: &[],
            },
            Message {
                from: "Ava Lindqvist",
                to: "You, Jonas, Team",
                stamp: at(0, 13, 52),
                body: &[
                    "Tickets are booked. The train leaves Bonn at 7:34 on Thursday, \
                     and we start at Factory at one, so there's time for lunch.",
                    "Yes, please bring Colorways. Friday morning is for reviews, and \
                     the afternoon is free for anyone who wants to stay. The train \
                     home leaves at three.",
                    "The itinerary and the agenda are attached. See you on Thursday!",
                ],
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
        ],
        draft: "Perfect, thanks Ava. I'll bring Colorways on a laptop and a tablet, \
                so people can stir it by hand.",
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_is_a_clock_today_and_a_day_this_week() {
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
        assert_eq!(rows.len(), MAILBOX.mails.len());
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
        let row = MAILBOX.mail(MAILBOX.open);
        assert_eq!(row.subject, MAILBOX.thread.subject);
        assert_eq!(usize::from(row.thread), MAILBOX.thread.messages.len());
        assert_eq!(row.category, Some(MAILBOX.thread.category));
        let newest = MAILBOX.thread.messages.last().map(|m| m.stamp);
        assert_eq!(newest, Some(row.stamp));
    }

    #[test]
    fn a_thread_folds_all_but_its_newest_two() {
        let t = MAILBOX.thread;
        assert_eq!(t.folded().len(), 2);
        assert_eq!(t.expanded().len(), 2);
        let single = Thread {
            messages: &t.messages[..1],
            ..t
        };
        assert!(single.folded().is_empty());
        assert_eq!(single.expanded().len(), 1);
    }

    #[test]
    fn labels_count_threads_per_category() {
        let total: usize = MAILBOX.labels().iter().map(|(_, n)| n).sum();
        let tagged = MAILBOX
            .mails
            .iter()
            .filter(|m| m.category.is_some())
            .count();
        assert_eq!(total, tagged);
    }

    #[test]
    fn today_reads_like_a_date() {
        assert_eq!(MAILBOX.today(), "Wed, Nov 11");
    }
}
