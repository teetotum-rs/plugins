//! {{face_name}}: a face for teetotum.
//!
//! A tap switches what the face says. [`Face::event`] gets the taps and wipes, and the knob with
//! [`Rights::KNOB`]; it returns whether the glass should be drawn again. [`Face::draw`] names what
//! the firmware draws. The plugin guide lists every event, drawing call and right.

#![no_std]

use teetotum_face::{Colour, Event, Face, Icon, Rights, Size, face};

/// 24 by 24 pixels, `#` lit. It stands in both rings.
const ICON: Icon = Icon::new(&[
    "........................",
    "........................",
    ".......##########.......",
    ".....##############.....",
    "....####........####....",
    "...###............###...",
    "..###..............###..",
    "..##................##..",
    ".###................###.",
    ".##......######......##.",
    ".##.....########.....##.",
    ".##.....########.....##.",
    ".##.....########.....##.",
    ".##.....########.....##.",
    ".###................###.",
    "..##................##..",
    "..###..............###..",
    "...###............###...",
    "....####........####....",
    ".....##############.....",
    ".......##########.......",
    "........................",
    "........................",
    "........................",
]);

/// The middle of the 360 by 360 glass.
const CENTRE: i32 = 180;

struct App {
    /// Whether an odd number of taps has landed since the face came up.
    lit: bool,
}

impl Face for App {
    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Tap => {
                self.lit = !self.lit;
                true
            }
            _ => false,
        }
    }

    fn draw(&self) {
        teetotum_face::icon(&ICON, CENTRE, CENTRE - 60, Colour::ICON);
        teetotum_face::text("{{face_name}}", CENTRE, CENTRE, Size::Large, Colour::NAME);
        let line = match self.lit {
            false => "tap the glass",
            true => "tapped",
        };
        teetotum_face::text(line, CENTRE, CENTRE + 34, Size::Body, Colour::VALUE);
    }
}

face!(
    name: "{{face_name}}",
    summary: "{{summary}}",
    icon: ICON,
    rights: Rights::NONE,
    face: App = App { lit: false },
);
