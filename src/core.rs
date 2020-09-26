use std::{fs, io};

use crate::os::{HOSTS_BACKUP_PATH, HOSTS_PATH};
use crate::updater;

pub fn uninstall() -> Result<(), io::Error> {
    fs::copy(HOSTS_BACKUP_PATH, HOSTS_PATH)?;
    updater::remove_temp_file()?;
    fs::remove_file(HOSTS_BACKUP_PATH)?;

    Ok(())
}