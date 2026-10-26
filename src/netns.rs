use std::{
    fs::{File, exists},
    io,
};

use nix::sched::{CloneFlags, setns};

const SELF_NETNS_PATH: &str = "/proc/thread-self/ns/net";
const DEAULT_NETNS_PATH: &str = "/proc/1/ns/net";
const NETNS_PATH: &str = "/run/netns/";

fn get_netns_path(netns: impl AsRef<str>) -> String {
    if netns.as_ref() == "default" {
        DEAULT_NETNS_PATH.to_string()
    } else {
        NETNS_PATH.to_string() + netns.as_ref()
    }
}

pub fn run_in_netns<T>(netns: impl AsRef<str>, callback: impl FnOnce() -> T) -> io::Result<T> {
    let initial_netns = File::open(SELF_NETNS_PATH)?;
    let target_netns = File::open(get_netns_path(netns))?;

    setns(target_netns, CloneFlags::CLONE_NEWNET)?;

    let result = callback();

    setns(initial_netns, CloneFlags::CLONE_NEWNET)?;

    Ok(result)
}

pub fn netns_exists(netns: impl AsRef<str>) -> io::Result<bool> {
    exists(get_netns_path(netns))
}
