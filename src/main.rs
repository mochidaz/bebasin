extern crate crossterm;
extern crate cursive;
#[cfg(any(target_os = "linux", target_os = "macos"))]
extern crate nix;
extern crate pest;
extern crate webbrowser;
#[cfg(target_os = "windows")]
extern crate winapi;
#[macro_use]
extern crate pest_derive;
extern crate itertools;
extern crate zip;

mod app;
mod error;
mod helpers;
mod os;
mod parser;
mod ui;
mod updater;

#[allow(clippy::unreadable_literal)]
const CURRENT_VERSION: u64 = 202010112039;
const REPOSITORY_URL: &str = "https://github.com/bebasid/bebasin";
const LATEST_VERSION_URL: &str =
    "https://raw.githubusercontent.com/bebasid/bebasin/master/latest.json";
const UPDATE_URL: &str = "https://api.github.com/repos/bebasid/bebasin/releases/latest";
const HOSTS_HEADER: &str = include_str!("../misc/header-hosts");
const HOSTS_BEBASIN: &str = include_str!("../misc/hosts");

fn main() {
    updater::remove_temp_file();

    app::App::new().dispatch();
}
