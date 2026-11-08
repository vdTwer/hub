//! The model behind the graph: people, threads and plans as nodes, and what
//! links them as edges.
//!
//! Nothing is stored twice. A link is found the way the contact card finds
//! it, by name or alias, and the layout is a deterministic force simulation,
//! so the page draws the same graph on every visit.

use std::borrow::Borrow;

use crate::calendar::{Category, Event, clock};
use crate::contacts::{Book, CLIMBING, Contact, FAMILY, FILM, TEAM};
use crate::mail::{Mail, Tag};

/// Tags that relate two people directly. A category is too broad for that.
const CIRCLES: &[Tag] = &[TEAM, CLIMBING, FAMILY, FILM];

/// The width of the simulated field, for a height of one.
const ASPECT: f64 = 1.5;
const STEPS: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Person,
    Thread,
    Plan,
}

impl Kind {
    pub const ALL: [Self; 3] = [Self::Person, Self::Thread, Self::Plan];

    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Thread => "thread",
            Self::Plan => "plan",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Person => "Person",
            Self::Thread => "Thread",
            Self::Plan => "Plan",
        }
    }

    #[must_use]
    pub const fn plural(self) -> &'static str {
        match self {
            Self::Person => "People",
            Self::Thread => "Threads",
            Self::Plan => "Plans",
        }
    }

    /// The app that owns this kind of node.
    #[must_use]
    pub const fn app(self) -> (&'static str, &'static str) {
        match self {
            Self::Person => ("/p/contacts", "Contacts"),
            Self::Thread => ("/p/mail", "Mail"),
            Self::Plan => ("/p/calendar", "Calendar"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// A stable id for the page, such as `p0` or `t3`.
    pub id: String,
    pub kind: Kind,
    pub title: &'static str,
    pub sub: String,
    /// The category class: work, personal, travel or plain.
    pub hue: &'static str,
    pub initials: String,
    /// The position in the field, from 0 to 1 on both axes.
    pub x: f64,
    pub y: f64,
}

impl Node {
    const fn new(
        id: String,
        kind: Kind,
        title: &'static str,
        sub: String,
        hue: &'static str,
    ) -> Self {
        Self {
            id,
            kind,
            title,
            sub,
            hue,
            initials: String::new(),
            x: 0.0,
            y: 0.0,
        }
    }

    #[must_use]
    pub fn left(&self) -> String {
        format!("{:.2}%", self.x * 100.0)
    }

    #[must_use]
    pub fn top(&self) -> String {
        format!("{:.2}%", self.y * 100.0)
    }

    /// The node's position in an SVG whose view box is 100 by 100.
    #[must_use]
    pub fn cx(&self) -> String {
        format!("{:.2}", self.x * 100.0)
    }

    #[must_use]
    pub fn cy(&self) -> String {
        format!("{:.2}", self.y * 100.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    /// The circle two people share, for a link between people.
    pub via: Option<&'static str>,
}

impl Edge {
    #[must_use]
    pub const fn touches(&self, i: usize) -> bool {
        self.a == i || self.b == i
    }

    #[must_use]
    pub const fn other(&self, i: usize) -> usize {
        if self.a == i { self.b } else { self.a }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Index into `nodes` of the node the inspector opens on.
    pub focus: usize,
}

fn hue_of(tags: &[Tag]) -> &'static str {
    tags.iter()
        .find_map(|t| t.hue)
        .map_or("plain", Category::class)
}

impl Graph {
    /// Every person in the book, and every thread and plan that links to
    /// someone or something, laid out.
    #[must_use]
    pub fn of(book: &Book) -> Self {
        let people = book.contacts;
        let mails = book.mail.mails;
        let events = book.week.events;

        let (threads, plans) = found(book);

        let mut nodes: Vec<Node> = people
            .iter()
            .enumerate()
            .map(|(i, c)| Node {
                initials: c.initials(),
                ..Node::new(
                    format!("p{i}"),
                    Kind::Person,
                    c.name,
                    c.role.to_owned(),
                    c.category.class(),
                )
            })
            .collect();
        let first_thread = nodes.len();
        nodes.extend(threads.iter().map(|&(j, _, _)| {
            let m = &mails[j];
            let sub = format!("{}, {}", m.from, book.mail.when(m.stamp));
            Node::new(
                format!("t{j}"),
                Kind::Thread,
                m.subject,
                sub,
                hue_of(m.tags),
            )
        }));
        let first_plan = nodes.len();
        nodes.extend(plans.iter().map(|&(e, _)| {
            let ev = &events[e];
            let day = book.week.days.get(ev.day).map_or("", |d| d.name);
            let sub = format!("{day}, {}, {}", clock(ev.start), ev.place);
            Node::new(
                format!("e{e}"),
                Kind::Plan,
                ev.title,
                sub,
                ev.category.class(),
            )
        }));
        let plan_node = |e: usize| {
            plans
                .iter()
                .position(|&(x, _)| x == e)
                .map(|k| first_plan + k)
        };

        let mut edges = circles(people);
        for (k, (_, ps, es)) in threads.iter().enumerate() {
            let t = first_thread + k;
            edges.extend(ps.iter().map(|&p| Edge {
                a: p,
                b: t,
                via: None,
            }));
            edges.extend(es.iter().filter_map(|&e| plan_node(e)).map(|n| Edge {
                a: t,
                b: n,
                via: None,
            }));
        }
        for (k, (_, ps)) in plans.iter().enumerate() {
            let n = first_plan + k;
            edges.extend(ps.iter().map(|&p| Edge {
                a: p,
                b: n,
                via: None,
            }));
        }

        let placed = settle(nodes.len(), &edges);
        for (node, (x, y)) in nodes.iter_mut().zip(placed) {
            node.x = x;
            node.y = y;
        }
        Self {
            nodes,
            edges,
            focus: book.open,
        }
    }

    #[must_use]
    pub fn node(&self, i: impl Borrow<usize>) -> &Node {
        &self.nodes[(*i.borrow()).min(self.nodes.len().saturating_sub(1))]
    }

    #[must_use]
    pub fn count(&self, kind: impl Borrow<Kind>) -> usize {
        let kind = *kind.borrow();
        self.nodes.iter().filter(|n| n.kind == kind).count()
    }

    #[must_use]
    pub fn degree(&self, i: impl Borrow<usize>) -> usize {
        let i = *i.borrow();
        self.edges.iter().filter(|e| e.touches(i)).count()
    }

    /// The diameter of a person's disc, which grows with their links.
    #[must_use]
    pub fn size(&self, i: impl Borrow<usize>) -> usize {
        26 + 2 * self.degree(i).min(8)
    }

    /// The nodes linked to `i` of one kind, by title.
    #[must_use]
    pub fn neighbors(&self, i: impl Borrow<usize>, kind: impl Borrow<Kind>) -> Vec<usize> {
        let (i, kind) = (*i.borrow(), *kind.borrow());
        let mut near: Vec<usize> = self
            .edges
            .iter()
            .filter(|e| e.touches(i))
            .map(|e| e.other(i))
            .filter(|&n| self.nodes[n].kind == kind)
            .collect();
        near.sort_by_key(|&n| self.nodes[n].title);
        near
    }

    /// The ids of every node linked to `i`, for the page's script.
    #[must_use]
    pub fn links_attr(&self, i: impl Borrow<usize>) -> String {
        let i = *i.borrow();
        self.edges
            .iter()
            .filter(|e| e.touches(i))
            .map(|e| self.nodes[e.other(i)].id.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The circle that links two people, if any.
    #[must_use]
    pub fn via(&self, a: impl Borrow<usize>, b: impl Borrow<usize>) -> Option<&'static str> {
        let (a, b) = (*a.borrow(), *b.borrow());
        self.edges
            .iter()
            .find(|e| e.touches(a) && e.touches(b))
            .and_then(|e| e.via)
    }

    /// A mask that shows a print only around the nodes of one category, so
    /// the grainy fluid pools under each cluster. Positions are fractions
    /// of the field, which the stage insets by `--fl`, `--fr`, `--ft` and
    /// `--fb`.
    #[must_use]
    pub fn pools(&self, hue: impl Borrow<Category>) -> String {
        let class = hue.borrow().class();
        self.nodes
            .iter()
            .filter(|n| n.hue == class)
            .map(|n| {
                format!(
                    "radial-gradient(circle var(--pool) at \
                     calc(var(--fl) + (100% - var(--fl) - var(--fr)) * {:.4}) \
                     calc(var(--ft) + (100% - var(--ft) - var(--fb)) * {:.4}), \
                     #000, rgb(0 0 0 / .4) 45%, transparent)",
                    n.x, n.y
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A thread by its index in the mailbox, with the people and plans it
/// links to.
type Found = (usize, Vec<usize>, Vec<usize>);

/// The threads and plans that link to anything, each with what it links to.
/// A plan is an index into the week's events, with its people.
fn found(book: &Book) -> (Vec<Found>, Vec<(usize, Vec<usize>)>) {
    let people = book.contacts;
    let events = book.week.events;
    let open_plans = book.mail.thread.linked;
    let event_index = |title: &str| events.iter().position(|e| e.title == title);
    let mail_people = |m: &Mail| -> Vec<usize> {
        (0..people.len())
            .filter(|&p| people[p].appears_in(m.from) || people[p].appears_in(m.subject))
            .collect()
    };
    let mail_plans = |(j, m): (usize, &Mail)| -> Vec<usize> {
        let mut plans: Vec<usize> = (0..events.len())
            .filter(|&e| m.subject.contains(events[e].title))
            .collect();
        if j == book.mail.open {
            plans.extend(open_plans.iter().filter_map(|l| event_index(l.title)));
        }
        plans.sort_unstable();
        plans.dedup();
        plans
    };
    let event_people = |e: &Event| -> Vec<usize> {
        (0..people.len())
            .filter(|&p| people[p].appears_in(e.title))
            .collect()
    };

    let threads: Vec<Found> = book
        .mail
        .mails
        .iter()
        .enumerate()
        .map(|(j, m)| (j, mail_people(m), mail_plans((j, m))))
        .filter(|(_, ps, es)| !ps.is_empty() || !es.is_empty())
        .collect();
    let plans: Vec<(usize, Vec<usize>)> = events
        .iter()
        .enumerate()
        .map(|(e, ev)| (e, event_people(ev)))
        .filter(|(e, ps)| !ps.is_empty() || threads.iter().any(|(_, _, es)| es.contains(e)))
        .collect();
    (threads, plans)
}

/// Links between people who share a circle, such as Team or Climbing.
fn circles(people: &[Contact]) -> Vec<Edge> {
    let mut edges = Vec::new();
    for (a, pa) in people.iter().enumerate() {
        for (b, pb) in people.iter().enumerate().skip(a + 1) {
            if let Some(t) = CIRCLES
                .iter()
                .find(|t| pa.tags.contains(t) && pb.tags.contains(t))
            {
                edges.push(Edge {
                    a,
                    b,
                    via: Some(t.name),
                });
            }
        }
    }
    edges
}

/// Fruchterman and Reingold's spring embedder, from a sunflower spiral
/// rather than random positions, so that it lands the same way every time.
/// Returns positions scaled into the unit square with a margin.
#[allow(
    clippy::cast_precision_loss,
    reason = "a graph of a few dozen nodes, far from 2^52"
)]
fn settle(n: usize, edges: &[Edge]) -> Vec<(f64, f64)> {
    if n == 0 {
        return Vec::new();
    }
    let center = (ASPECT / 2.0, 0.5);
    let golden = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
    let count = n as f64;
    let mut pos: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let i = i as f64;
            let r = 0.45 * ((i + 0.5) / count).sqrt();
            (
                center.0 + r * (i * golden).cos(),
                center.1 + r * (i * golden).sin(),
            )
        })
        .collect();
    let k = (ASPECT / count).sqrt() * 0.8;

    for step in 0..STEPS {
        let heat = 0.06f64.mul_add(1.0 - f64::from(step) / f64::from(STEPS), 0.001);
        let mut push = vec![(0.0, 0.0); n];
        for (i, &(xi, yi)) in pos.iter().enumerate() {
            for (j, &(xj, yj)) in pos.iter().enumerate().skip(i + 1) {
                let (dx, dy) = (xi - xj, yi - yj);
                let f = k * k / dx.mul_add(dx, dy * dy).max(1e-6);
                push[i].0 = dx.mul_add(f, push[i].0);
                push[i].1 = dy.mul_add(f, push[i].1);
                push[j].0 = (-dx).mul_add(f, push[j].0);
                push[j].1 = (-dy).mul_add(f, push[j].1);
            }
        }
        for e in edges {
            let (dx, dy) = (pos[e.a].0 - pos[e.b].0, pos[e.a].1 - pos[e.b].1);
            let f = dx.hypot(dy) / k;
            push[e.a].0 = (-dx).mul_add(f, push[e.a].0);
            push[e.a].1 = (-dy).mul_add(f, push[e.a].1);
            push[e.b].0 = dx.mul_add(f, push[e.b].0);
            push[e.b].1 = dy.mul_add(f, push[e.b].1);
        }
        for (p, d) in pos.iter_mut().zip(&push) {
            // gravity keeps loose nodes from drifting off the page
            let d = (
                (center.0 - p.0).mul_add(2.0, d.0),
                (center.1 - p.1).mul_add(2.0, d.1),
            );
            let len = d.0.hypot(d.1).max(1e-9);
            let step = len.min(heat) / len;
            p.0 = d.0.mul_add(step, p.0);
            p.1 = d.1.mul_add(step, p.1);
        }
    }

    let (lo_x, hi_x, lo_y, hi_y) = pos.iter().fold(
        (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
        |(a, b, c, d), &(x, y)| (a.min(x), b.max(x), c.min(y), d.max(y)),
    );
    let fit = |v: f64, lo: f64, hi: f64| {
        if hi - lo < 1e-9 {
            0.5
        } else {
            (v - lo) / (hi - lo)
        }
    };
    pos.into_iter()
        .map(|(x, y)| (fit(x, lo_x, hi_x), fit(y, lo_y, hi_y)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contacts::BOOK;

    fn graph() -> Graph {
        Graph::of(&BOOK)
    }

    fn find(g: &Graph, title: &str) -> usize {
        g.nodes
            .iter()
            .position(|n| n.title == title)
            .unwrap_or(usize::MAX)
    }

    fn linked(g: &Graph, a: &str, b: &str) -> bool {
        let (a, b) = (find(g, a), find(g, b));
        g.edges.iter().any(|e| e.touches(a) && e.touches(b))
    }

    #[test]
    fn everyone_is_a_node_and_nothing_else_is_alone() {
        let g = graph();
        assert_eq!(g.count(Kind::Person), BOOK.contacts.len());
        for (i, n) in g.nodes.iter().enumerate() {
            assert!(g.degree(i) > 0, "{} has no links", n.title);
        }
        assert_eq!(find(&g, "Standup"), usize::MAX);
        assert_eq!(find(&g, "Your scans are ready"), usize::MAX);
    }

    #[test]
    fn people_link_to_their_threads_plans_and_circles() {
        let g = graph();
        assert!(linked(
            &g,
            "Ava Lindqvist",
            "Off-site in Berlin: travel and agenda"
        ));
        assert!(linked(&g, "Ava Lindqvist", "Quick sync with Ava"));
        assert!(linked(&g, "Theo Brandt", "This week on 35mm"));
        assert!(linked(&g, "Ingrid Schulz", "Lena's birthday dinner"));
        assert!(linked(&g, "Mara Okafor", "Nils Berg"));
        assert!(!linked(&g, "Mara Okafor", "Ava Lindqvist"));
        let (mara, nils) = (find(&g, "Mara Okafor"), find(&g, "Nils Berg"));
        assert_eq!(g.via(mara, nils), Some("Climbing"));
    }

    #[test]
    fn threads_link_to_the_plans_they_made() {
        let g = graph();
        let offsite = "Off-site in Berlin: travel and agenda";
        assert!(linked(&g, offsite, "Train to Berlin"));
        assert!(linked(&g, offsite, "Off-site workshop"));
        assert!(linked(&g, offsite, "Train home"));
        assert!(linked(
            &g,
            "Sprint planning: draft board",
            "Sprint planning"
        ));
    }

    #[test]
    fn edges_are_unique_and_never_loops() {
        let g = graph();
        for (i, e) in g.edges.iter().enumerate() {
            assert_ne!(e.a, e.b);
            assert!(e.a < g.nodes.len() && e.b < g.nodes.len());
            let twin = g.edges[i + 1..]
                .iter()
                .any(|f| f.touches(e.a) && f.touches(e.b));
            assert!(!twin, "{} and {}", g.nodes[e.a].title, g.nodes[e.b].title);
        }
    }

    #[test]
    fn neighbors_are_symmetric_and_typed() {
        let g = graph();
        for i in 0..g.nodes.len() {
            for kind in Kind::ALL {
                for n in g.neighbors(i, kind) {
                    assert_eq!(g.nodes[n].kind, kind);
                    assert!(g.neighbors(n, g.nodes[i].kind).contains(&i));
                }
            }
        }
        let ava = g.neighbors(0, Kind::Person);
        let names: Vec<_> = ava.iter().map(|&n| g.nodes[n].title).collect();
        assert_eq!(names, ["Jonas Weber", "Priya Nair"]);
    }

    #[test]
    fn layout_fills_the_field_without_collisions() {
        let g = graph();
        for n in &g.nodes {
            assert!(
                (0.0..=1.0).contains(&n.x) && (0.0..=1.0).contains(&n.y),
                "{}",
                n.title
            );
        }
        for (i, a) in g.nodes.iter().enumerate() {
            for b in &g.nodes[i + 1..] {
                let d = ((a.x - b.x) * ASPECT).hypot(a.y - b.y);
                assert!(d > 0.05, "{} and {} at {d:.3}", a.title, b.title);
            }
        }
    }

    #[test]
    fn linked_nodes_sit_closer_than_strangers() {
        let g = graph();
        let dist = |a: &Node, b: &Node| ((a.x - b.x) * ASPECT).hypot(a.y - b.y);
        let mut near = (0.0, 0.0);
        let mut far = (0.0, 0.0);
        for i in 0..g.nodes.len() {
            for j in i + 1..g.nodes.len() {
                let d = dist(&g.nodes[i], &g.nodes[j]);
                let bucket = if g.edges.iter().any(|e| e.touches(i) && e.touches(j)) {
                    &mut near
                } else {
                    &mut far
                };
                bucket.0 += d;
                bucket.1 += 1.0;
            }
        }
        assert!(near.0 / near.1 < 0.6 * far.0 / far.1);
    }

    #[test]
    fn layout_is_the_same_every_time() {
        assert_eq!(graph(), graph());
    }

    #[test]
    fn pools_gather_one_category() {
        let g = graph();
        let work = g.pools(Category::Work);
        let n = g.nodes.iter().filter(|n| n.hue == "work").count();
        assert_eq!(work.matches("radial-gradient").count(), n);
        assert!(n > 0);
    }
}
