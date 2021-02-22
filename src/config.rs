use std::fs;
use std::fs::File;
use std::io::prelude::*;

use serde_derive::Deserialize;

use crate::Error;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub tickers: Vec<Ticker>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ticker {
    pub name: String,
    pub from: String,
    pub to: String,
}

const DEFAULT: &'static str = include_str!("../data/config.example");

pub fn create_if_not_exists(file: &str) -> Result<(), Error> {
    let expanded = expand(file)?;

    if file_exists(&expanded) {
        Ok(())
    } else {
        println!(
            "Configuration file does not exist, a new one was created on: {}",
            &expanded
        );
        let mut file = File::create(expanded)?;
        file.write_all(DEFAULT.as_bytes())?;

        Ok(())
    }
}

pub fn parse(path: &str) -> Result<Config, Error> {
    let mut file = File::open(expand(&path)?)?;
    let mut buf = String::new();

    file.read_to_string(&mut buf)?;

    Ok(toml::from_str(&buf).map_err(|_| Error::ParseError)?)
}

fn expand(path: &str) -> Result<String, Error> {
    Ok(shellexpand::full(path)
        .map_err(|_| Error::PathExpansionError(path.to_string()))?
        .to_string())
}

fn file_exists(file: &str) -> bool {
    match fs::metadata(file) {
        Ok(file) => file.is_file(),
        _ => false,
    }
}
