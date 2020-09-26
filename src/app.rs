use cursive::{Cursive, CursiveExt};
use cursive::event::Key;
use cursive::menu::MenuTree;
use cursive::views::Dialog;

use crate::{REPOSITORY_URL, ui};

pub struct App {
    cursive: Cursive,
}

impl App {
    pub fn new() -> Self {
        Self {
            cursive: Cursive::crossterm().unwrap(),
        }
    }

    fn set_global_callback(&mut self) {
        self.cursive.add_global_callback('q', Cursive::quit);
        self.cursive.add_global_callback(Key::Esc, Cursive::quit);
    }

    pub fn run(&mut self) {
        self.set_global_callback();

        ui::main(&mut self.cursive);

        self.cursive.run();
    }
}
