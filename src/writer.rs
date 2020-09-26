use core::result::Result::Ok;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use itertools::Itertools;

use crate::parser::Hosts;

pub fn write_hosts_to_file<P>(file_path: P, hosts: &Hosts, header: &str) -> Result<(), io::Error>
    where
        P: AsRef<Path> {
    let mut file = fs::File::create(file_path)?;

    let mut hosts_stringify = String::from(header);

    for host in hosts {
        let ip = &host.0;
        let hostnames = &host.1.into_iter().join(" ");

        hosts_stringify.push_str(&format!("{} {}\n", ip, hostnames));
    }

    file.write_all(hosts_stringify.as_bytes().as_ref())?;

    Ok(())
}

