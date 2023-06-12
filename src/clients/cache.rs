use std::{
    collections::{HashMap, HashSet},
    fs,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequest {
    pub reviews: HashSet<usize>,
    pub comments: HashSet<usize>,
}

pub type Data = HashMap<usize, PullRequest>;

pub struct CacheClient {
    filename: String,
}

impl CacheClient {
    pub fn new(filename: String) -> Self {
        Self { filename }
    }

    pub fn read(&self) -> Result<Data> {
        let contents = fs::read_to_string(&self.filename)?;
        let deserialized = serde_json::from_str::<Data>(&contents)?;
        Ok(deserialized)
    }

    pub fn write(&self, data: &Data) -> Result<()> {
        let serialized = serde_json::to_string_pretty(data)?;
        fs::write(&self.filename, serialized)?;
        Ok(())
    }
}
