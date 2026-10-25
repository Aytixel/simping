use std::{collections::HashMap, error::Error, net::IpAddr, path::Path, time::Duration};

use serde::Deserialize;
use serde_valid::{Validate, utils::duration_minimum};
use serde_with::{
    OneOrMany,
    formats::{PreferMany, PreferOne},
    serde_as,
};
use smol::fs::read;

fn default_netns() -> String {
    "default".to_string()
}

fn default_ttl() -> u16 {
    64
}

fn default_interval() -> Duration {
    Duration::from_secs(1)
}

fn default_timeout() -> Duration {
    Duration::from_millis(100)
}

fn default_down_retry() -> u8 {
    3
}

fn default_up_retry() -> u8 {
    5
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub tests: HashMap<String, Box<[Test]>>,
    #[serde(default)]
    pub nexthops: HashMap<String, Box<[Nexthop]>>,
    #[serde(default)]
    pub routes: Box<[Routes]>,
}

impl Config {
    pub async fn load(path: impl AsRef<Path>) -> Result<Config, Box<dyn Error>> {
        log::info!("Loading config [{}]", path.as_ref().display());
        Ok(toml::from_slice(&read(path).await?)?)
    }
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct Test {
    pub ip: IpAddr,

    #[serde(alias = "src")]
    pub source: Option<IpAddr>,
    #[serde(alias = "dev")]
    pub device: Option<String>,
    #[serde(default = "default_netns")]
    pub netns: String,

    #[validate(minimum = 1)]
    #[serde(default = "default_ttl")]
    pub ttl: u16,
    #[validate(custom = duration_minimum(Duration::from_millis(10)))]
    #[serde(with = "humantime_serde", default = "default_interval")]
    pub interval: Duration,
    #[validate(custom = duration_minimum(Duration::from_millis(1)))]
    #[serde(with = "humantime_serde", default = "default_timeout")]
    pub timeout: Duration,

    #[validate(minimum = 1)]
    #[serde(default = "default_down_retry")]
    pub down_retry: u8,
    #[validate(minimum = 1)]
    #[serde(default = "default_up_retry")]
    pub up_retry: u8,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct Nexthop {
    #[serde(alias = "gw", alias = "via")]
    pub gateway: Option<IpAddr>,
    #[serde(alias = "dev")]
    pub device: Option<String>,
    #[serde(default = "default_netns")]
    pub netns: String,

    #[serde_as(as = "OneOrMany<_, PreferOne>")]
    #[serde(alias = "test", default)]
    pub tests: Box<[String]>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct Routes {
    #[serde_as(as = "OneOrMany<_, PreferMany>")]
    #[serde(alias = "route")]
    pub routes: Box<[String]>,

    #[serde_as(as = "OneOrMany<_, PreferMany>")]
    #[serde(alias = "nexthop", default)]
    pub nexthops: Box<[String]>,

    #[serde_as(as = "OneOrMany<_, PreferMany>")]
    #[serde(alias = "test", default)]
    pub tests: Box<[String]>,
}
