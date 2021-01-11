use crate::error::ErrorKind;
use crate::os::{HOSTS_BACKUP_PATH, HOSTS_PATH};
use crate::parser::{parse_from_file, parse_from_str, write_to_file};
use crate::{updater, CURRENT_VERSION, HOSTS_BEBASIN, HOSTS_HEADER, REPOSITORY_URL};

use cursive::traits::*;
use cursive::views::{Button, Dialog, DummyView, LinearLayout, TextView, EditView};
use cursive::Cursive;

use crate::helpers::AppendableMap;
use crate::updater::{backup, is_backed, is_installed, hosts_exists, create_default_hosts};
use std::fs;

fn clear_layer(cursive: &mut Cursive) {
    while cursive.pop_layer().is_some() {}
}

fn error(cursive: &mut Cursive, err: ErrorKind) {
    cursive.pop_layer();

    cursive.add_layer(
        Dialog::text(err.to_string())
            .button("Ok", |cursive| {
                cursive.pop_layer();
            })
            .title("Error"),
    );
}

fn create_default_hosts_ui(cursive: &mut Cursive) {
    let box_layout = Dialog::text("No hosts file found. Bebasin needs it in order to create
    backup. Allow bebasin create default hosts file? (If no, you can create it manually later)")
    .title("Notice")
    .button("Yes", move |cursive| {
        cursive.pop_layer();
        create_default_hosts();
        let notice = Dialog::text("Default hosts file has been successfully created. 
        Press install or install custom again to install bebasin/custom hosts file")
        .title("Success")
        .button("Ok", move |cursive| {
            cursive.pop_layer();
        });
        cursive.add_layer(notice);
    })
    .button("No", move |cursive| {
        cursive.pop_layer();
    });
    cursive.add_layer(box_layout);
}

fn install(cursive: &mut Cursive) {

    if !hosts_exists() {
        create_default_hosts_ui(cursive);
    }

    else {
        let box_layout = Dialog::text("Parsing the file...").title("Loading...");
        cursive.add_layer(box_layout);
    }

    if !is_backed() {
        let backup_result = backup();
        if backup_result.is_err() {
            if !hosts_exists() {}
            else {
                error(cursive, backup_result.err().unwrap());
                return;
            }
        }
    }

    match parse_from_str(HOSTS_BEBASIN) {
        Ok(mut hosts_bebasin) => {
            match parse_from_file(HOSTS_BACKUP_PATH) {
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
                        match write_to_file(HOSTS_PATH, &hosts_bebasin, HOSTS_HEADER) {
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
                    if !hosts_exists(){}
                    else {
                        error(cursive, err);
                    }
                }
            };
        }
        Err(err) => {
            if !hosts_exists(){}
            else {
                error(cursive, err);
            }
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
        match fs::copy(HOSTS_BACKUP_PATH, HOSTS_PATH) {
            Ok(_) => {
                updater::remove_temp_file();

                match fs::remove_file(HOSTS_BACKUP_PATH) {
                    Err(err) => return error(cursive, ErrorKind::IOError(err)),
                    _ => {}
                };

                uninstall_finish(cursive);
            }
            Err(err) => error(cursive, ErrorKind::IOError(err)),
        };
    })
    .button("Cancel", |cursive| {
        cursive.pop_layer();
    });

    cursive.add_layer(box_layout);
}

fn open_browser(cursive: &mut Cursive, url: &str) {
    if webbrowser::open(url).is_err() {
        let layout = Dialog::text("Can't open any browser")
            .title("Error")
            .button("Ok", |cursive| {
                cursive.pop_layer();
            });

        cursive.add_layer(layout);
    }
}

fn install_custom_ui(cursive: &mut Cursive) {

    if !hosts_exists() {
        create_default_hosts_ui(cursive);
    }

    else {

        let box_layout = Dialog::new()
        .title("Your custom hosts path")
        .content(
            EditView::new()
                .on_submit(install_custom)
                .with_name("custom_hosts")
                .fixed_width(20),
        )
        .button("Ok", |x|{
            let custom_hosts = x
                .call_on_name("custom_hosts", |view: &mut EditView| {
                    view.get_content()
                })
                .unwrap();
            install_custom(x, custom_hosts.as_str());
        });
        cursive.add_layer(box_layout);
    }
}

fn install_custom(cursive: &mut Cursive, path: &str) {
    use std::io::Read;
    let mut f = fs::File::open(path).expect("Unable to open file");
    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("Error");
    let hosts_custom = contents.as_str();

    if !is_backed() {
        let backup_result = backup();
        if backup_result.is_err() {
            error(cursive, backup_result.err().unwrap());
            return;
        }
    }
    match parse_from_str(hosts_custom) {
        Ok(mut hosts_custom) => {
            match parse_from_file(HOSTS_BACKUP_PATH) {
                Ok(hosts_backup) => {
                    hosts_custom.append(hosts_backup);
                    cursive.pop_layer();

                    let box_layout = Dialog::text(
                        "Are you sure you want to\n\
                    merge your hosts file with\n\
                    your custom hosts?",
                    )
                        .title("Confirmation")
                        .button("Confirm", move |cursive| {
                            match write_to_file(HOSTS_PATH, &hosts_custom, HOSTS_HEADER) {
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
            error(cursive, err);
        }
    };
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
                error(cursive, err);
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
    .button("Yes", move |cursive| match updater_instance.update() {
        Ok(_) => {
            let updated_layer =
                Dialog::text("The application has been updated, please re-run the application")
                    .button("Ok", |cursive| {
                        cursive.quit();
                    });
            cursive.add_layer(updated_layer);
        }
        Err(err) => error(cursive, err),
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
        menu_buttons = menu_buttons.child(Button::new("Install Custom", install_custom_ui));
    }

    menu_buttons = menu_buttons
        .child(Button::new("Update", update))
        .child(Button::new("Repository", |cursive| {
            open_browser(cursive, REPOSITORY_URL);
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
