use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use itertools::Itertools as _;
use pest::Parser;

use crate::error::GenericError;

pub(crate) type Hosts = HashMap<String, HashSet<String>>;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct HostsParser;

pub fn parse_hosts_from_file<P>(file_path: P) -> Result<Hosts, GenericError>
    where
        P: AsRef<Path> {
    let content = fs::read_to_string(file_path)?;
    let res = parse_hosts_from_str(&content)?;

    Ok(res)
}

pub fn parse_hosts_from_str(str: &str) -> Result<Hosts, pest::error::Error<Rule>> {
    let mut hosts: Hosts = HashMap::new();
    let parse_result = HostsParser::parse(Rule::main, str)?;

    for pair in parse_result {
        if let Rule::statement = pair.as_rule() {
            let mut ip = String::new();
            let mut hostnames: HashSet<String> = HashSet::new();

            for inner_pair in pair.into_inner() {
                match inner_pair.as_rule() {
                    Rule::ip => {
                        ip = inner_pair.as_str().to_owned();
                    }
                    Rule::hostnames => {
                        for hostname in inner_pair.into_inner() {
                            hostnames.insert(hostname.as_str().to_owned());
                        }
                    }
                    _ => {}
                }
            }

            match hosts.get_mut(&ip) {
                Some(old_val) => {
                    hostnames.into_iter().for_each(|x| {
                        old_val.insert(x);
                    });
                }
                None => {
                    hosts.insert(ip, hostnames);
                }
            };
        }
    }
    Ok(hosts)
}
