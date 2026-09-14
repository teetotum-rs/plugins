//! Counter: the knob counts detents, a tap starts again from zero.
//!
//! The smallest face that asks for a right. [`Rights::KNOB`] turns the detents into events for
//! the face; without it the knob keeps doing what the firmware does with it.

#![no_std]

use teetotum_face::{Colour, Event, Face, Icon, Rights, Size, face};

/// Four tally marks.
const ICON: Icon = Icon::new(&[
    "........................",
    "........................",
    "........................",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "...##...##...##...##....",
    "........................",
    "........................",
    "........................",
]);

/// The middle of the 360 by 360 glass.
const CENTRE: i32 = 180;

/// Room for any `i32` in decimal, sign included.
const DIGITS: usize = 11;

struct Counter {
    count: i32,
}

impl Face for Counter {
    fn event(&mut self, event: Event) -> bool {
        let count = match event {
            Event::Clockwise => self.count.saturating_add(1),
            Event::Anticlockwise => self.count.saturating_sub(1),
            Event::Tap => 0,
            _ => return false,
        };
        let changed = count != self.count;
        self.count = count;
        changed
    }

    fn draw(&self) {
        let mut digits = [0; DIGITS];
        teetotum_face::icon(&ICON, CENTRE, CENTRE - 64, Colour::ICON);
        let count = decimal(self.count, &mut digits);
        teetotum_face::text(count, CENTRE, CENTRE, Size::Large, Colour::VALUE);
        teetotum_face::text(
            "turn to count",
            CENTRE,
            CENTRE + 34,
            Size::Body,
            Colour::NAME,
        );
        teetotum_face::text(
            "tap for zero",
            CENTRE,
            CENTRE + 60,
            Size::Small,
            Colour::QUIET,
        );
    }
}

/// `value` in decimal, written into the end of `digits`: no allocator, no `core::fmt`.
fn decimal(value: i32, digits: &mut [u8; DIGITS]) -> &str {
    let mut at = DIGITS;
    let mut rest = value.unsigned_abs();
    loop {
        at -= 1;
        digits[at] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    if value < 0 {
        at -= 1;
        digits[at] = b'-';
    }
    core::str::from_utf8(&digits[at..]).unwrap_or("?")
}

face!(
    name: "Counter",
    summary: "counts detents",
    icon: ICON,
    rights: Rights::KNOB,
    face: Counter = Counter { count: 0 },
);
