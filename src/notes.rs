//! The model behind the notes: plain `CommonMark` with one addition, `@`,
//! which links a note to a person, a thread, a plan, another note or you.
//!
//! An `@` resolves to the longest title it starts, ignoring ASCII case, so
//! `@Lena's birthday dinner` is the plan and `@Lena's gift` is Lena. An `@`
//! inside a word, as in an email address, or one that names nothing stays
//! text, and so does any `@` in code.

use std::borrow::Borrow;

use pulldown_cmark::{CowStr, Event, Options, Parser, TagEnd, TextMergeStream, html};

use crate::calendar::Category;
use crate::contacts::{BOOK, Book, CLIMBING, FAMILY};
use crate::mail::{FILM, OFFSITE, PERSONAL, Stamp, Tag, WORK};

/// What an `@` can link to. Indices point into the book's contacts, the
/// mailbox, the week's events and the notebook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    Me,
    Person(usize),
    Thread(usize),
    Plan(usize),
    Note(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct Note {
    pub title: &'static str,
    pub edited: Stamp,
    pub tags: &'static [Tag],
    pub pinned: bool,
    /// `CommonMark` with `@` links.
    pub body: &'static str,
}

impl Note {
    /// The category class of the note's first category tag, or plain.
    #[must_use]
    pub fn hue(&self) -> &'static str {
        hue_of(self.tags)
    }

    #[must_use]
    pub fn category(&self) -> Option<Category> {
        self.tags.iter().find_map(|t| t.hue)
    }
}

/// The category class of the first category among some tags, or plain.
#[must_use]
pub fn hue_of(tags: &[Tag]) -> &'static str {
    tags.iter()
        .find_map(|t| t.hue)
        .map_or("plain", Category::class)
}

#[derive(Debug, Clone, Copy)]
pub struct Notebook {
    pub notes: &'static [Note],
    /// Index into `notes` of the note open when the page loads.
    pub open: usize,
    pub book: &'static Book,
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Whether `name` sits in `text` at byte `from`, ignoring ASCII case, and
/// ends where a word ends.
fn names_at(text: &str, from: usize, name: &str) -> bool {
    let end = from + name.len();
    text.get(from..end)
        .is_some_and(|s| s.eq_ignore_ascii_case(name))
        && text
            .get(end..)
            .and_then(|rest| rest.chars().next())
            .is_none_or(|c| !c.is_alphanumeric())
}

impl Notebook {
    #[must_use]
    pub fn note(&self, i: impl Borrow<usize>) -> &'static Note {
        &self.notes[(*i.borrow()).min(self.notes.len().saturating_sub(1))]
    }

    #[must_use]
    pub fn is_open(&self, i: impl Borrow<usize>) -> bool {
        *i.borrow() == self.open
    }

    /// Every name an `@` can use, longest first, so the longest wins.
    fn index(&self) -> Vec<(&'static str, Target)> {
        let book = self.book;
        let mut names: Vec<(&'static str, Target)> = vec![("me", Target::Me)];
        for (i, c) in book.contacts.iter().enumerate() {
            names.push((c.name, Target::Person(i)));
            names.push((c.first_name(), Target::Person(i)));
        }
        names.extend(
            book.mail
                .mails
                .iter()
                .enumerate()
                .map(|(i, m)| (m.subject, Target::Thread(i))),
        );
        names.extend(
            book.week
                .events
                .iter()
                .enumerate()
                .map(|(i, e)| (e.title, Target::Plan(i))),
        );
        names.extend(
            self.notes
                .iter()
                .enumerate()
                .map(|(i, n)| (n.title, Target::Note(i))),
        );
        // stable: on a tie, the earlier kind wins
        names.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));
        names
    }

    /// Splits text into runs, each either plain or an `@` link with what it
    /// links to. A link's run includes its `@`.
    #[must_use]
    pub fn scan<'a>(&self, text: &'a str) -> Vec<(&'a str, Option<Target>)> {
        let index = self.index();
        let mut runs = Vec::new();
        let (mut plain_from, mut search_from) = (0, 0);
        while let Some(offset) = text.get(search_from..).and_then(|t| t.find('@')) {
            let at = search_from + offset;
            let after = at + 1;
            let starts_word = text[..at]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric());
            let hit = index
                .iter()
                .filter(|_| starts_word)
                .find(|(name, _)| names_at(text, after, name));
            search_from = after;
            if let Some(&(name, target)) = hit {
                if at > plain_from {
                    runs.push((&text[plain_from..at], None));
                }
                let end = after + name.len();
                runs.push((&text[at..end], Some(target)));
                (plain_from, search_from) = (end, end);
            }
        }
        if plain_from < text.len() {
            runs.push((&text[plain_from..], None));
        }
        runs
    }

    /// What a note links to, each once, in the order first named.
    #[must_use]
    pub fn mentions(&self, i: impl Borrow<usize>) -> Vec<Target> {
        let mut seen = Vec::new();
        for event in TextMergeStream::new(Parser::new(self.note(i).body)) {
            if let Event::Text(text) = event {
                for (_, target) in self.scan(&text) {
                    if let Some(t) = target.filter(|t| !seen.contains(t)) {
                        seen.push(t);
                    }
                }
            }
        }
        seen
    }

    /// The notes that link to note `i`.
    #[must_use]
    pub fn backlinks(&self, i: impl Borrow<usize>) -> Vec<usize> {
        let i = *i.borrow();
        (0..self.notes.len())
            .filter(|&j| j != i && self.mentions(j).contains(&Target::Note(i)))
            .collect()
    }

    /// The list's groups: pinned notes, then the rest.
    #[must_use]
    pub fn groups(&self) -> Vec<(&'static str, Vec<usize>)> {
        let (pinned, recent) = self.order();
        vec![("Pinned", pinned), ("Recent", recent)]
    }

    /// A note's links by kind, for its footer, leaving out empty kinds.
    #[must_use]
    pub fn link_groups(
        &self,
        i: impl Borrow<usize>,
    ) -> Vec<(&'static str, &'static str, Vec<Target>)> {
        let i = *i.borrow();
        [
            ("person", "People"),
            ("thread", "Threads"),
            ("plan", "Plans"),
            ("note", "Notes"),
        ]
        .into_iter()
        .map(|(kind, label)| (kind, label, self.mentions_of(i, kind)))
        .filter(|(_, _, ts)| !ts.is_empty())
        .collect()
    }

    /// The notes that link to note `i`, as links.
    #[must_use]
    pub fn linked_from(&self, i: impl Borrow<usize>) -> Vec<Target> {
        self.backlinks(i).into_iter().map(Target::Note).collect()
    }

    /// The links of note `i` of one kind, for the note's footer.
    #[must_use]
    pub fn mentions_of(&self, i: impl Borrow<usize>, kind: &str) -> Vec<Target> {
        self.mentions(i)
            .into_iter()
            .filter(|&t| self.kind(t) == kind)
            .collect()
    }

    #[must_use]
    pub const fn kind(&self, target: Target) -> &'static str {
        match target {
            Target::Me | Target::Person(_) => "person",
            Target::Thread(_) => "thread",
            Target::Plan(_) => "plan",
            Target::Note(_) => "note",
        }
    }

    #[must_use]
    pub fn title(&self, target: impl Borrow<Target>) -> &'static str {
        let book = self.book;
        match *target.borrow() {
            Target::Me => "You",
            Target::Person(i) => book.contact(i).name,
            Target::Thread(i) => book.mail.mail(i).subject,
            Target::Plan(i) => book.week.events.get(i).map_or("", |e| e.title),
            Target::Note(i) => self.note(i).title,
        }
    }

    /// The category class a link is drawn in; `me` for you.
    #[must_use]
    pub fn hue(&self, target: impl Borrow<Target>) -> &'static str {
        let book = self.book;
        match *target.borrow() {
            Target::Me => "me",
            Target::Person(i) => book.contact(i).category.class(),
            Target::Thread(i) => hue_of(book.mail.mail(i).tags),
            Target::Plan(i) => book
                .week
                .events
                .get(i)
                .map_or("plain", |e| e.category.class()),
            Target::Note(i) => self.note(i).hue(),
        }
    }

    /// Where a link leads: the app that owns it, or the note itself.
    #[must_use]
    pub fn href(&self, target: impl Borrow<Target>) -> String {
        match *target.borrow() {
            Target::Me => "/p/graph".to_owned(),
            Target::Person(_) => "/p/contacts".to_owned(),
            Target::Thread(_) => "/p/mail".to_owned(),
            Target::Plan(_) => "/p/calendar".to_owned(),
            Target::Note(i) => format!("/p/notes#n{i}"),
        }
    }

    /// The small mark in front of a link: a monogram, or a glyph.
    #[must_use]
    pub fn mark(&self, target: impl Borrow<Target>) -> String {
        let target = *target.borrow();
        match target {
            Target::Me => r#"<span class="mono">Me</span>"#.to_owned(),
            Target::Person(i) => format!(
                r#"<span class="mono mine">{}</span>"#,
                self.book.contact(i).initials()
            ),
            Target::Thread(_) => r##"<svg class="sym"><use href="#mail"/></svg>"##.to_owned(),
            Target::Plan(_) => r#"<i class="key plan"></i>"#.to_owned(),
            Target::Note(_) => r##"<svg class="sym"><use href="#doc"/></svg>"##.to_owned(),
        }
    }

    fn chip(&self, text: &str, target: Target) -> String {
        format!(
            r#"<a class="mention {} {}" href="{}" title="{}">{}{}</a>"#,
            self.kind(target),
            self.hue(target),
            self.href(target),
            escape(self.title(target)),
            self.mark(target),
            escape(text.trim_start_matches('@')),
        )
    }

    /// A note rendered to HTML, with its `@` links as chips.
    #[must_use]
    pub fn html(&self, i: impl Borrow<usize>) -> String {
        let events = TextMergeStream::new(Parser::new_ext(self.note(i).body, Options::empty()))
            .flat_map(|event| match event {
                Event::Text(text) => self
                    .scan(&text)
                    .into_iter()
                    .map(|(run, target)| {
                        target.map_or_else(
                            || Event::Text(CowStr::from(run.to_owned())),
                            |t| Event::InlineHtml(CowStr::from(self.chip(run, t))),
                        )
                    })
                    .collect::<Vec<_>>(),
                other => vec![other],
            });
        let mut out = String::new();
        html::push_html(&mut out, events);
        out
    }

    /// A note's source, escaped, with each `@` link marked, for the
    /// Markdown view.
    #[must_use]
    pub fn source(&self, i: impl Borrow<usize>) -> String {
        self.scan(self.note(i).body)
            .into_iter()
            .map(|(run, target)| {
                target.map_or_else(
                    || escape(run),
                    |t| {
                        format!(
                            r#"<span class="at {} {}">{}</span>"#,
                            self.kind(t),
                            self.hue(t),
                            escape(run)
                        )
                    },
                )
            })
            .collect()
    }

    /// A note as plain text, links as their names, for the list.
    #[must_use]
    pub fn snippet(&self, i: impl Borrow<usize>) -> String {
        let mut words = Vec::new();
        for event in TextMergeStream::new(Parser::new(self.note(i).body)) {
            match event {
                Event::Text(text) | Event::Code(text) => words.push(
                    self.scan(&text)
                        .into_iter()
                        .map(|(run, _)| run.trim_start_matches('@'))
                        .collect::<String>(),
                ),
                Event::End(
                    TagEnd::Paragraph
                    | TagEnd::Heading(_)
                    | TagEnd::Item
                    | TagEnd::BlockQuote(_)
                    | TagEnd::CodeBlock,
                )
                | Event::SoftBreak
                | Event::HardBreak => words.push(" ".to_owned()),
                _ => {}
            }
        }
        words
            .concat()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The palette of a note's print: its category's, or work's for a
    /// note without one.
    #[must_use]
    pub fn palette_attr(&self, i: impl Borrow<usize>) -> String {
        self.note(i)
            .category()
            .unwrap_or(Category::Work)
            .palette()
            .join(",")
    }

    #[must_use]
    pub fn pinned(&self) -> usize {
        self.notes.iter().filter(|n| n.pinned).count()
    }

    #[must_use]
    pub fn when(&self, i: impl Borrow<usize>) -> String {
        self.book.mail.when(self.note(i).edited)
    }

    /// Pinned notes first, then the rest, most recently edited first.
    #[must_use]
    pub fn order(&self) -> (Vec<usize>, Vec<usize>) {
        let mut all: Vec<usize> = (0..self.notes.len()).collect();
        all.sort_by_key(|&i| {
            let s = self.notes[i].edited;
            (s.days_ago, std::cmp::Reverse(s.at))
        });
        all.into_iter().partition(|&i| self.notes[i].pinned)
    }

    /// Every tag on a note with the number of notes that carry it.
    #[must_use]
    pub fn tags(&self) -> Vec<(Tag, usize)> {
        let mut tags: Vec<(Tag, usize)> = Vec::new();
        for t in self.notes.iter().flat_map(|n| n.tags) {
            match tags.iter_mut().find(|(x, _)| x == t) {
                Some((_, n)) => *n += 1,
                None => tags.push((*t, 1)),
            }
        }
        tags
    }
}

const fn edited(days_ago: u8, hour: u16, minute: u16) -> Stamp {
    Stamp {
        days_ago,
        at: hour * 60 + minute,
    }
}

/// The notebook on the same Wednesday as the other apps, open on the
/// off-site agenda.
pub const NOTEBOOK: Notebook = Notebook {
    notes: &[
        Note {
            title: "Off-site agenda",
            edited: edited(0, 12, 40),
            tags: &[WORK, OFFSITE],
            pinned: true,
            body: "Two days in Berlin with @Ava, @Jonas and @Priya. We take the \
@Train to Berlin on Thursday morning, and the @Off-site workshop starts after lunch.

## Day one

- Show Colorways on a tablet, so people can *stir* it by hand
- Go through the feedback on @Release 0.3 is out
- Ask @Priya about the type ramp for the darkroom theme

## Day two

1. Roadmap for the first quarter
2. Who owns the design system?
3. Wrap up before the @Train home

> Keep it short. Ava prefers mornings, and so does @me.

Logistics live in @Off-site in Berlin: travel and agenda.",
        },
        Note {
            title: "November",
            edited: edited(1, 21, 5),
            tags: &[PERSONAL],
            pinned: true,
            body: "What @me wants from this month.

- Climb twice a week with @Nils and @Mara
- Call @Ingrid on Sundays, *before* lunch
- Finish the reel for @Super 8 night
- Read one book that has nothing to do with work

The rest can wait until December.",
        },
        Note {
            title: "1:1 with Jonas",
            edited: edited(2, 10, 15),
            tags: &[WORK],
            pinned: false,
            body: "Notes for @1:1 with Jonas. His own are in @Notes from Monday's 1:1.

- **Shaders**: he wants to own the halation pass. Yes.
- Prepare the board for @Sprint planning
- He asked about the `u_print` uniform, so write it up

Next time: career goals, not tickets.",
        },
        Note {
            title: "Lena's birthday",
            edited: edited(1, 18, 30),
            tags: &[PERSONAL],
            pinned: false,
            body: "@Lena turns 34 on Saturday. Dinner is @Lena's birthday dinner, \
and @Ingrid bakes the cake, as every year.

Gift ideas:

- A box of Super 8 film, *Kodak Vision3*
- A print from @Mara's last roll
- Tickets for something from @This week on 35mm

Plan the evening in @Dinner on Saturday, and keep the film night for @Super 8 night.",
        },
        Note {
            title: "Super 8 night",
            edited: edited(3, 22, 10),
            tags: &[PERSONAL, FILM],
            pinned: false,
            body: "@Lena has the projector and the reels, @Theo can get us the small \
room at the Kino for an evening.

1. The climbing reel, with @Mara and @Nils
2. Berlin, if the @Off-site workshop leaves any time to shoot
3. Old family reels from @Ingrid

Scans come back from the lab first; see @Your scans are ready.",
        },
        Note {
            title: "Climbing log",
            edited: edited(1, 20, 0),
            tags: &[PERSONAL, CLIMBING],
            pinned: false,
            body: "Tuesday at the Boulderhalle with @Mara and @Nils, see @Climbing.

- Sent the blue overhang on the third try
- Nils found a new crimp line; try it next week
- Mara shot a roll, see @Photos from the climb

`grade: 6b+`, and fingers are fine.",
        },
    ],
    open: 0,
    book: &BOOK,
};

/// The family tag, so the sidebar lists it even before a note carries it.
pub const TAGS: &[Tag] = &[WORK, PERSONAL, OFFSITE, FILM, CLIMBING, FAMILY];

#[cfg(test)]
mod tests {
    use super::*;

    fn links(text: &str) -> Vec<(String, Option<Target>)> {
        NOTEBOOK
            .scan(text)
            .into_iter()
            .map(|(run, t)| (run.to_owned(), t))
            .collect()
    }

    #[test]
    fn an_at_links_a_person_by_first_or_full_name() {
        assert_eq!(
            links("with @Ava and @Jonas Weber."),
            [
                ("with ".to_owned(), None),
                ("@Ava".to_owned(), Some(Target::Person(0))),
                (" and ".to_owned(), None),
                ("@Jonas Weber".to_owned(), Some(Target::Person(1))),
                (".".to_owned(), None),
            ]
        );
    }

    #[test]
    fn the_longest_title_wins() {
        let runs = links("@Lena's birthday dinner and @Lena's gift");
        assert_eq!(
            runs[0].1,
            Some(Target::Plan(find_event("Lena's birthday dinner")))
        );
        assert_eq!(runs[2], ("@Lena".to_owned(), Some(Target::Person(4))));
    }

    fn find_event(title: &str) -> usize {
        BOOK.week
            .events
            .iter()
            .position(|e| e.title == title)
            .unwrap_or(usize::MAX)
    }

    #[test]
    fn case_is_ignored_but_words_are_not_cut() {
        assert_eq!(links("@ava")[0].1, Some(Target::Person(0)));
        assert_eq!(links("@Avalanche"), [("@Avalanche".to_owned(), None)]);
        assert_eq!(
            links("ava@studio.example"),
            [("ava@studio.example".to_owned(), None)]
        );
        assert_eq!(links("@nobody here"), [("@nobody here".to_owned(), None)]);
    }

    #[test]
    fn an_at_links_threads_plans_notes_and_you() {
        assert_eq!(links("@me")[0].1, Some(Target::Me));
        assert_eq!(links("@Super 8 night")[0].1, Some(Target::Note(4)));
        assert_eq!(links("@Release 0.3 is out")[0].1, Some(Target::Thread(6)));
        assert_eq!(
            links("@Sprint planning")[0].1,
            Some(Target::Plan(find_event("Sprint planning")))
        );
    }

    #[test]
    fn notes_render_commonmark_with_chips() {
        let html = NOTEBOOK.html(0);
        assert!(html.contains("<h2>Day one</h2>"));
        assert!(html.contains("<ol>"));
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("<em>stir</em>"));
        assert!(html.contains(r#"<a class="mention person work" href="/p/contacts" title="Ava Lindqvist"><span class="mono mine">AL</span>Ava</a>"#));
        assert!(html.contains(r#"class="mention plan travel""#));
        assert!(!html.contains('@'));
    }

    #[test]
    fn code_keeps_its_at_and_text_is_escaped() {
        const ONE: &[Note] = &[Note {
            title: "Test",
            edited: edited(0, 9, 0),
            tags: &[],
            pinned: false,
            body: "`@Ava` and <b>@Ava</b> & @Ava",
        }];
        let book = Notebook {
            notes: ONE,
            ..NOTEBOOK
        };
        let html = book.html(0);
        assert!(html.contains("<code>@Ava</code>"));
        assert_eq!(html.matches("class=\"mention").count(), 2);
        assert!(html.contains("&amp;"));
    }

    #[test]
    fn mentions_are_unique_and_in_order() {
        let m = NOTEBOOK.mentions(1);
        assert_eq!(m[0], Target::Me);
        assert_eq!(m[1], Target::Person(6));
        let mut sorted = m.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), m.len());
    }

    #[test]
    fn backlinks_find_the_notes_that_link_here() {
        assert_eq!(NOTEBOOK.backlinks(4), [1, 3]);
        assert!(NOTEBOOK.backlinks(0).is_empty());
    }

    #[test]
    fn every_at_in_the_notebook_resolves() {
        for (i, note) in NOTEBOOK.notes.iter().enumerate() {
            let source = NOTEBOOK.source(i);
            let bare = source.matches('@').count();
            let linked = source.matches("<span class=\"at ").count();
            assert_eq!(bare, linked, "{}", note.title);
        }
    }

    #[test]
    fn the_list_puts_pinned_first_then_newest() {
        let (pinned, rest) = NOTEBOOK.order();
        assert_eq!(pinned, [0, 1]);
        assert_eq!(rest, [5, 3, 2, 4]);
    }

    #[test]
    fn snippets_are_plain_text() {
        let s = NOTEBOOK.snippet(0);
        assert!(s.starts_with("Two days in Berlin with Ava, Jonas and Priya."));
        assert!(!s.contains('#') && !s.contains('*') && !s.contains('@'));
        assert!(NOTEBOOK.snippet(2).contains("Shaders: he"));
    }
}
