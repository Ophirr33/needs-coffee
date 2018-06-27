extern crate chrono;
extern crate toml;

use chrono::{DateTime, Utc};
use errors::OResult;
use std::collections::BTreeMap;
use std::fs::Metadata;
use std::path::Path;
use util::{read_file, write_file};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, PartialOrd)]
pub struct Timing {
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

impl Timing {
    pub fn from_metadata_and_prev(metadata: &Metadata, prev: Option<&Timing>)
        -> OResult<Self>
    {
        let modified: DateTime<Utc> = metadata.modified()?.into();
        let created: DateTime<Utc> = metadata.created()
            .ok()
            .map(|st| st.into())
            .or_else(|| prev.map(|timing| timing.created.clone()))
            .unwrap_or_else(|| modified.clone());
        Ok(Timing { created, modified })
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub timings: BTreeMap<String, Timing>,
}

impl Config {
    pub fn new(timings: BTreeMap<String, Timing>) -> Self {
        Config { timings }
    }

    pub fn from_file<P: AsRef<Path>>(config_file: P) -> OResult<Self> {
        Ok(toml::from_slice(read_file(config_file)?.as_bytes())?)
    }

    pub fn to_file<P: AsRef<Path>>(&self, config_file: P) -> OResult<()> {
        write_file(config_file, toml::ser::to_vec(self)?)
    }
}
