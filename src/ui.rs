use std::error::Error;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;

use cursive::Cursive;
use cursive::views::{Button, Dialog, DummyView, LinearLayout, TextView};

use crate::{CURRENT_VERSION, HOSTS_BEBASIN, HOSTS_HEADER, REPOSITORY_URL, updater};
use crate::error::{GenericError, ThreadError};
use crate::helpers::AppendableMap;
use crate::os::{HOSTS_BACKUP_PATH, HOSTS_PATH};
use crate::parser::{parse_hosts_from_file, parse_hosts_from_str};
use crate::updater::{backup, is_backed, is_installed};
use crate::writer::write_hosts_to_file;

fn clear_layer(cursive: &mut Cursive) {
    while cursive.pop_layer().is_some() {}
}

fn error(cursive: &mut Cursive, err: GenericError) {
    cursive.pop_layer();

    cursive.add_layer(
        Dialog::text(err.to_string())
            .button("Ok", |cursive| {
                cursive.pop_layer();
            })
            .title("Error"),
    );
}

fn install(cursive: &mut Cursive) {
    let box_layout = Dialog::text("Parsing the file...").title("Loading...");

    cursive.add_layer(box_layout);

    if !is_backed() {
        if let Err(err) = backup() {
            error(cursive, err.into());
            return;
        }
    }

    match parse_hosts_from_str(HOSTS_BEBASIN) {
        Ok(mut hosts_bebasin) => {
            match parse_hosts_from_file(HOSTS_BACKUP_PATH) {
                Ok(hosts_backup) => {
                    hosts_bebasin.append(hosts_backup);
                    cursive.pop_layer();

                    let box_layout = Dialog::text(
                        "Are you sure you want to\n\
                    merge your hosts file with\n\
                    Bebasin hosts?",
                    )
                        .title("Confirmation")
                        .button("Confirm", move |cursive| {
                            match write_hosts_to_file(HOSTS_PATH, &hosts_bebasin, HOSTS_HEADER) {
                                Err(err) => {
                                    cursive.add_layer(
                                        Dialog::text(err.to_string()).title("Error").button(
                                            "Ok",
                                            |cursive| {
                                                cursive.pop_layer();
                                                cursive.pop_layer();
                                            },
                                        ),
                                    );
                                }
                                _ => {
                                    cursive.add_layer(
                                        Dialog::text(
                                            "The hosts file has been updated,\n\
                        Please restart your machine",
                                        )
                                            .title("Done")
                                            .button("Ok", |cursive| {
                                                // Re-create the main menu
                                                clear_layer(cursive);
                                                main(cursive);
                                            }),
                                    );
                                }
                            };
                        })
                        .button("Cancel", |cursive| {
                            cursive.pop_layer();
                        });

                    cursive.add_layer(box_layout);
                }
                Err(err) => {
                    error(cursive, err);
                }
            };
        }
        Err(err) => {
            error(cursive, GenericError::from(err));
        }
    };
}

fn uninstall_finish(cursive: &mut Cursive) {
    let layer = Dialog::text(
        "The hosts file has been updated,\n\
        Please restart your network/machine",
    )
        .title("Done")
        .button("Ok", |cursive| {
            cursive.pop_layer();

            // Re-create the main menu
            clear_layer(cursive);
            main(cursive);
        });

    cursive.add_layer(layer);
}

fn uninstall(cursive: &mut Cursive) {
    let box_layout = Dialog::text(
        "Are you sure you want to\n\
        uninstall Bebasin hosts?",
    )
        .title("Confirmation")
        .button("Confirm", move |cursive| {
            // 1. Copy the backup to the real hosts
            // 2. Delete the backup
            // 3, Remove all temporary file
            if let Err(err) = crate::core::uninstall() {
                error(cursive, GenericError::from(err));
            } else {
                uninstall_finish(cursive);
            }
        })
        .button("Cancel", |cursive| {
            cursive.pop_layer();
        });

    cursive.add_layer(box_layout);
}

pub(crate) fn open_browser(cursive: &mut Cursive, url: &str) {
    if let Err(err) = webbrowser::open(url) {
        ;
        cursive.add_layer(Dialog::text(
            format!("Can't open any browser\nReason: {}", err.to_string())
        )
            .title("Error")
            .button("Ok", |cursive| {
                cursive.pop_layer();
            }));
    }
}

fn update(cursive: &mut Cursive) {
    let mut updater_instance = updater::Updater::new();

    let loading_layer =
        Dialog::text("Retrieving latest application information").title("Loading...");
    cursive.add_layer(loading_layer);

    let latest = match updater_instance.get_latest_info() {
        Ok(latest) => latest,
        Err(err) => {
            return {
                error(cursive, err.into());
            };
        }
    };

    cursive.pop_layer();

    if !updater_instance.is_updatable() {
        let warning_layer = Dialog::text("You have been using the latest update application")
            .button("Ok", |cursive| {
                cursive.pop_layer();
            })
            .title("Warning");
        cursive.add_layer(warning_layer);
        return;
    }

    let confirmation_layer = Dialog::text(format!(
        "Are you sure you want to update to version {}?",
        latest.version
    ))
        .title("Confirmation")
        .button("No", |cursive| {
            cursive.pop_layer();
        })
        .button("Yes", move |cursive| {
            let update_err = Arc::new(Mutex::<Option<Result<(), GenericError>>>::new(None));

            let spawned_thread = thread::Builder::new()
                .name("update".into())
                .spawn(|| {
                    crate::updater::Updater::get_release_data();
                });

            if let Ok(handle) = spawned_thread {
                cursive.add_layer(
                    Dialog::text("Updating the application")
                        .title("Loading...")
                );

                if let Err(_) = handle.join() {
                    error(cursive, GenericError::ThreadError(ThreadError("Error joining the thread".into())));
                    return;
                } else {
                    cursive.pop_layer();
                    cursive.add_layer(
                        Dialog::text("The application has been updated, please re-run the application")
                            .title("Success")
                            .button("Quit", |cursive| {
                                cursive.quit();
                            })
                    );
                }
            } else {
                // Do sequentially
                // updater_instance::update();
            }
        });
    cursive.add_layer(confirmation_layer);
}

pub fn main(cursive: &mut Cursive) {
    let text_header = TextView::new(format!("Bebasin version {}", CURRENT_VERSION));
    let mut menu_buttons = LinearLayout::vertical();

    if is_installed() {
        menu_buttons = menu_buttons.child(Button::new("Uninstall", uninstall));
    } else {
        menu_buttons = menu_buttons.child(Button::new("Install", install));
    }

    menu_buttons = menu_buttons
        .child(Button::new("Update", update))
        .child(DummyView)
        .child(Button::new("Repository", |cursive| {
            crate::ui::open_browser(cursive, REPOSITORY_URL);
        }))
        .child(Button::new("Report a problem", |cursive| {
            let repository_create_issue_url = &format!("{}/issues/new", REPOSITORY_URL);

            open_browser(cursive, repository_create_issue_url);
        }))
        .child(DummyView)
        .child(Button::new("Quit", Cursive::quit));
    let layout = Dialog::around(
        LinearLayout::vertical()
            .child(text_header)
            .child(DummyView)
            .child(menu_buttons),
    )
        .title("Menu");

    cursive.add_layer(layout);
}
