//! The model behind the contacts mockup.
//!
//! An address book sorted by last name, with tags instead of groups, and one
//! open person whose threads and events are found in the mailbox and the
//! calendar rather than stored twice.

use std::borrow::Borrow;

use crate::calendar::{Category, Event, WEEK, Week, clock};
use crate::mail::{MAILBOX, Mail, Mailbox, PERSONAL, TRAVEL, Tag, WORK, initials, plain};

pub const TEAM: Tag = plain("Team");
pub const CLIMBING: Tag = plain("Climbing");
pub const FAMILY: Tag = plain("Family");
pub const FILM: Tag = plain("Film");

/// Every tag in the sidebar's order: categories first.
pub const TAGS: &[Tag] = &[WORK, PERSONAL, TRAVEL, TEAM, CLIMBING, FAMILY, FILM];

/// One labeled line of a contact card, such as an email address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detail {
    pub label: &'static str,
    pub value: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct Contact {
    pub name: &'static str,
    pub role: &'static str,
    /// The hue of the person's print.
    pub category: Category,
    pub tags: &'static [Tag],
    pub favorite: bool,
    pub details: &'static [Detail],
    pub note: &'static str,
    /// Words that find this person in event titles, besides the first name.
    pub also_in: &'static [&'static str],
}

impl Contact {
    #[must_use]
    pub fn initials(&self) -> String {
        initials(self.name)
    }

    #[must_use]
    pub fn first_name(&self) -> &'static str {
        self.name.split_whitespace().next().unwrap_or(self.name)
    }

    /// The name the book sorts by: the last word of the name.
    #[must_use]
    pub fn last_name(&self) -> &'static str {
        self.name.split_whitespace().last().unwrap_or(self.name)
    }

    /// The name without its last word, shown lighter than the last name.
    #[must_use]
    pub fn given_names(&self) -> &'static str {
        self.name.rsplit_once(' ').map_or("", |(given, _)| given)
    }

    #[must_use]
    pub fn palette_attr(&self) -> String {
        self.category.palette().join(",")
    }
}

/// A run of contacts under one letter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub letter: char,
    /// Indices into [`Book::contacts`].
    pub rows: Vec<usize>,
}

/// An event shared with the open contact, as the card lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Together {
    pub event: &'static Event,
    pub when: String,
    pub past: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Book {
    pub contacts: &'static [Contact],
    /// Index into `contacts` of the open card.
    pub open: usize,
    pub week: &'static Week,
    pub mail: &'static Mailbox,
}

impl Book {
    /// Contacts by the first letter of their last name, A to Z, and by
    /// last name within a letter.
    #[must_use]
    pub fn sections(&self) -> Vec<Section> {
        let mut order: Vec<usize> = (0..self.contacts.len()).collect();
        order.sort_by_key(|&i| {
            let c = &self.contacts[i];
            (c.last_name(), c.first_name())
        });
        order
            .into_iter()
            .fold(Vec::new(), |mut sections: Vec<Section>, i| {
                let letter = self.contacts[i]
                    .last_name()
                    .chars()
                    .next()
                    .map_or('#', |c| c.to_ascii_uppercase());
                match sections.last_mut() {
                    Some(s) if s.letter == letter => s.rows.push(i),
                    _ => sections.push(Section {
                        letter,
                        rows: vec![i],
                    }),
                }
                sections
            })
    }

    #[must_use]
    pub fn contact(&self, row: impl Borrow<usize>) -> &'static Contact {
        &self.contacts[(*row.borrow()).min(self.contacts.len().saturating_sub(1))]
    }

    #[must_use]
    pub fn is_open(&self, row: impl Borrow<usize>) -> bool {
        *row.borrow() == self.open
    }

    #[must_use]
    pub fn person(&self) -> &'static Contact {
        self.contact(self.open)
    }

    #[must_use]
    pub fn favorites(&self) -> usize {
        self.contacts.iter().filter(|c| c.favorite).count()
    }

    /// Every tag with the number of people who carry it.
    #[must_use]
    pub fn tags(&self) -> Vec<(Tag, usize)> {
        TAGS.iter()
            .map(|&t| {
                let n = self.contacts.iter().filter(|c| c.tags.contains(&t)).count();
                (t, n)
            })
            .collect()
    }

    /// Threads in the mailbox that the open contact is part of.
    #[must_use]
    pub fn threads(&self) -> Vec<&'static Mail> {
        let person = self.person();
        self.mail
            .mails
            .iter()
            .filter(|m| m.from.contains(person.first_name()))
            .collect()
    }

    /// Events this week with the open contact, in order.
    #[must_use]
    pub fn together(&self) -> Vec<Together> {
        let person = self.person();
        let week = self.week;
        week.events
            .iter()
            .filter(|e| {
                e.title.contains(person.first_name())
                    || person.also_in.iter().any(|w| e.title.contains(w))
            })
            .map(|e| Together {
                event: e,
                when: format!(
                    "{}, {}",
                    week.days.get(e.day).map_or("", |d| d.name),
                    clock(e.start)
                ),
                past: week.is_past(e),
            })
            .collect()
    }
}

const fn d(label: &'static str, value: &'static str) -> Detail {
    Detail { label, value }
}

/// The address book on the same Wednesday as the mail and the calendar,
/// open on Ava.
pub const BOOK: Book = Book {
    contacts: &[
        Contact {
            name: "Ava Lindqvist",
            role: "Product lead, Studio",
            category: Category::Work,
            tags: &[WORK, TEAM, TRAVEL],
            favorite: true,
            details: &[
                d("Email", "ava@studio.example"),
                d("Phone", "+49 228 555 0142"),
                d("Birthday", "March 3"),
                d("Office", "Studio, Room 1"),
            ],
            note: "Organizes the Berlin off-site. Prefers a call to a long thread, \
                   and mornings to afternoons.",
            also_in: &["Off-site", "Team dinner"],
        },
        Contact {
            name: "Jonas Weber",
            role: "Engineer, Studio",
            category: Category::Work,
            tags: &[WORK, TEAM],
            favorite: true,
            details: &[
                d("Email", "jonas@studio.example"),
                d("Phone", "+49 228 555 0178"),
            ],
            note: "Weekly 1:1 on Mondays.",
            also_in: &["Sprint planning", "Off-site"],
        },
        Contact {
            name: "Priya Nair",
            role: "Designer, Hamburg",
            category: Category::Work,
            tags: &[WORK, TEAM],
            favorite: false,
            details: &[d("Email", "priya@studio.example")],
            note: "",
            also_in: &["Off-site"],
        },
        Contact {
            name: "Mara Okafor",
            role: "Photographer",
            category: Category::Personal,
            tags: &[PERSONAL, CLIMBING, FILM],
            favorite: true,
            details: &[
                d("Email", "mara@okafor.example"),
                d("Phone", "+49 151 555 0193"),
            ],
            note: "Shoots medium format. Tuesday climbing, then lunch.",
            also_in: &["Climbing"],
        },
        Contact {
            name: "Lena Hoffmann",
            role: "Friend",
            category: Category::Personal,
            tags: &[PERSONAL, FILM],
            favorite: true,
            details: &[d("Phone", "+49 151 555 0127"), d("Birthday", "November 14")],
            note: "Has the Super 8 projector, and the reels.",
            also_in: &[],
        },
        Contact {
            name: "Ingrid Schulz",
            role: "Grandma",
            category: Category::Personal,
            tags: &[PERSONAL, FAMILY],
            favorite: true,
            details: &[d("Phone", "+49 228 555 0110")],
            note: "Call on Sundays.",
            also_in: &[],
        },
        Contact {
            name: "Nils Berg",
            role: "Climbing partner",
            category: Category::Personal,
            tags: &[PERSONAL, CLIMBING],
            favorite: false,
            details: &[d("Phone", "+49 160 555 0155")],
            note: "",
            also_in: &["Climbing"],
        },
        Contact {
            name: "Elif Demir",
            role: "Type designer",
            category: Category::Work,
            tags: &[WORK],
            favorite: false,
            details: &[d("Email", "elif@demir.example")],
            note: "Runs the typeface workshop.",
            also_in: &["Typeface workshop"],
        },
        Contact {
            name: "Theo Brandt",
            role: "Projectionist, Kino am Rhein",
            category: Category::Personal,
            tags: &[PERSONAL, FILM],
            favorite: false,
            details: &[d("Email", "theo@kino.example")],
            note: "",
            also_in: &[],
        },
        Contact {
            name: "Sam Adeyemi",
            role: "Travel desk, Studio",
            category: Category::Travel,
            tags: &[TRAVEL, WORK],
            favorite: false,
            details: &[
                d("Email", "travel@studio.example"),
                d("Phone", "+49 228 555 0100"),
            ],
            note: "Books trains and rooms for the team.",
            also_in: &["Train"],
        },
    ],
    open: 0,
    week: &WEEK,
    mail: &MAILBOX,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_split_into_given_and_last() {
        let ava = BOOK.contacts[0];
        assert_eq!(ava.first_name(), "Ava");
        assert_eq!(ava.last_name(), "Lindqvist");
        assert_eq!(ava.given_names(), "Ava");
        assert_eq!(ava.initials(), "AL");
    }

    #[test]
    fn sections_run_a_to_z_by_last_name() {
        let sections = BOOK.sections();
        let letters: String = sections.iter().map(|s| s.letter).collect();
        assert_eq!(letters, "ABDHLNOSW");
        let total: usize = sections.iter().map(|s| s.rows.len()).sum();
        assert_eq!(total, BOOK.contacts.len());
        for s in &sections {
            let names: Vec<_> = s
                .rows
                .iter()
                .map(|&i| BOOK.contacts[i].last_name())
                .collect();
            assert!(names.windows(2).all(|w| w[0] <= w[1]), "{}", s.letter);
            assert!(names.iter().all(|n| n.starts_with(s.letter)));
        }
    }

    #[test]
    fn letters_share_a_section() {
        let s = BOOK.sections();
        let s_rows = s.iter().find(|s| s.letter == 'S').map(|s| s.rows.len());
        assert_eq!(s_rows, Some(1));
        let n_rows = s.iter().find(|s| s.letter == 'N').map(|s| s.rows.len());
        assert_eq!(n_rows, Some(1));
    }

    #[test]
    fn threads_come_from_the_mailbox() {
        let subjects: Vec<_> = BOOK.threads().iter().map(|m| m.subject).collect();
        assert_eq!(
            subjects,
            [
                "Off-site in Berlin: travel and agenda",
                "Release 0.3 is out"
            ]
        );
    }

    #[test]
    fn events_come_from_the_calendar() {
        let together = BOOK.together();
        let titles: Vec<_> = together.iter().map(|t| t.event.title).collect();
        assert_eq!(
            titles,
            [
                "Quick sync with Ava",
                "Off-site workshop",
                "Team dinner",
                "Off-site, day two"
            ]
        );
        assert_eq!(together[0].when, "Wed, 15:15");
        assert!(together.iter().all(|t| !t.past));
    }

    #[test]
    fn other_people_find_their_own_events() {
        let mara = Book { open: 3, ..BOOK };
        let titles: Vec<_> = mara.together().iter().map(|t| t.event.title).collect();
        assert_eq!(titles, ["Lunch with Mara", "Climbing"]);
        assert!(mara.together().iter().all(|t| t.past));
    }

    #[test]
    fn tags_are_counted_per_person() {
        let tags = BOOK.tags();
        assert_eq!(tags.len(), TAGS.len());
        assert!(tags.iter().all(|(_, n)| *n > 0));
        let count = |t: Tag| tags.iter().find(|(x, _)| *x == t).map(|(_, n)| *n);
        assert_eq!(count(TEAM), Some(3));
        assert_eq!(count(FILM), Some(3));
    }

    #[test]
    fn favorites_are_counted() {
        assert_eq!(BOOK.favorites(), 5);
    }
}
