// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024 RemasteredArch
//
// This file is part of ip_geo.
//
// ip_geo is free software: you can redistribute it and/or modify it under the terms of the GNU
// Affero General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// ip_geo is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License along with ip_geo. If
// not, see <https://www.gnu.org/licenses/>.

use std::{collections::HashMap, process::Command, str::FromStr};

mod country;
use country::{Country, CountryPair};
mod wikidata;

use chrono::{SecondsFormat, Utc};
use mediawiki::MediaWikiError;

/// Represents all possible error states of this module.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StrFromUtf8(#[from] core::str::Utf8Error),

    #[error("can't parse line '{0}' into Country")]
    InvalidCountryLine(Box<str>),

    #[error("expected two letter country code, received '{0}'")]
    InvalidCode(Box<str>),

    #[error("out of bounds array access")]
    OutOfBounds,

    #[error("can't split url")]
    UrlSplit,

    #[error(transparent)]
    Wiki(#[from] MediaWikiError),

    #[error("iterator operation failed")]
    Iter, // Could probably be more specific

    #[error("can't map value to object")]
    InvalidObject,

    #[error("can't map value to array")]
    InvalidArray,

    #[error("can't convert value to string")]
    InvalidString,

    #[error("can't convert string to coordinates")]
    InvalidPoint,

    #[error("missing results in response")]
    MissingResults,

    #[error("missing binding in value")]
    MissingBindings,
}

fn main() {
    // Tor's additions to the database from libloc
    let additional_countries = vec![CountryPair::new("??", "Unknown")];

    // Country codes unique to libloc
    let nonstandard_countries = HashMap::from([
        // European Union
        ("EU", "Q458"),
        // Serbia and Montenegro
        ("CS", "Q37024"),
        // Asia/Pacific
        ("AP", "Q48"),
    ]);

    let countries = get_country_list(additional_countries, nonstandard_countries).unwrap();

    // dbg!(&countries);
    // print_country_list_as_code_and_name(&countries);
    print_country_list_as_rust_hashmap(&countries);
}

/// Formats and prints a list of countries' codes and names separated by a space
///
/// For exmaple:
///
/// ```text
/// AP Asia/Pacific
/// BE Belgium
/// CS Serbia and Montenegro
/// ?? Unkown
/// ```
#[allow(dead_code)]
fn print_country_list_as_code_and_name(countries: &[Country]) {
    countries
        .iter()
        .for_each(|c| println!("{} {}", c.code, c.name));
}

/// Formats prints a list of countries as valid Rust code that returns a `HashMap`.
#[allow(dead_code)]
fn print_country_list_as_rust_hashmap(countries: &[Country]) {
    let location_version = get_location_version().unwrap();
    let date_time = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true); // Ex. 2024-07-21T04:11:07Z

    print!(
        r#"// This file was @generated by ip_geo/geo using {location_version} and Wikidata at {date_time}. Do not edit!

// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024 RemasteredArch
//
// This file is part of ip_geo.
//
// ip_geo is free software: you can redistribute it and/or modify it under the terms of the GNU
// Affero General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// ip_geo is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License along with ip_geo. If
// not, see <https://www.gnu.org/licenses/>.

use std::{{collections::HashMap, rc::Rc}};

struct Country {{
    name: Box<str>,          // Ex. Belgium
    code: Rc<str>,           // Ex. BE
    coordinates: (f64, f64), // Ex. (4.668055555, 50.641111111)
}}

/// A map of countries, with the ISO 3166-1 alpha-2 code as the key.
#[rustfmt::skip]
pub fn get_countries() -> HashMap<Rc<str>, Country> {{HashMap::from([
"#
    );

    countries
        .iter()
        .for_each(|c| println!("{},", c.as_rust_map_entry(4)));

    println!("])}}");
}

/// Returns a list of countries.
///
/// List sourced from [`location(8)`](https://man-pages.ipfire.org/libloc/location.html)
/// and `additional_countries`.
/// Location sources from Wikidata.
///
/// `nonstandard_countries` represent a libloc country code and a Wikidata ID, where the code
/// deviates from ISO 3166-1 alpha-2.
fn get_country_list(
    mut additional_countries: Vec<CountryPair>,
    nonstandard_countries: HashMap<&str, &str>,
) -> Result<Box<[Country]>, Error> {
    let input = call("location list-countries --show-name")?;
    let mut countries = Vec::with_capacity(input.len() + additional_countries.len());

    for line in input {
        if line.len() == 0 {
            continue;
        }

        // Alternatively, this could bubble up an error
        match CountryPair::from_str(&line) {
            Ok(country) => countries.push(country),
            Err(error) => eprintln!("Error parsing country list: {error}"),
        }
    }

    countries.append(&mut additional_countries);
    countries.dedup_by_key(|c| c.code.clone());

    // For a given `CountryPair`, create a `Country` from it using the appropriate method.
    let from_pair = move |pair: &CountryPair| match pair.code.as_ref() {
        // The pair has no associated country
        "??" => Country::new(&pair.code, &pair.name, (0.0, 0.0)),

        // The pair is a real country or other geographic area
        _ => match nonstandard_countries.get(pair.code.as_ref()) {
            // The pair cannot be identified on Wikidata from its code, and must use a hardcoded ID
            Some(id) => Country::from_pair_and_id(pair, id),

            // The pair can be identified on Wikidata from its code
            None => Country::from_pair(pair),
        },
    };

    let countries: Vec<Country> = countries.iter().map(from_pair).collect();

    Ok(countries.into_boxed_slice())
}

fn get_location_version() -> Result<Box<str>, Error> {
    let lines = call("location --version")?;

    lines.first().ok_or(Error::OutOfBounds).cloned()
}

/// Make a shell call.
///
/// On Windows:
/// ```cmd
/// cmd /C command
/// ```
///
/// On POSIX:
/// ```sh
/// sh -c command
/// ```
fn call(command: &str) -> Result<Vec<Box<str>>, Error> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", command]).output()?
    } else {
        Command::new("sh").args(["-c", command]).output()?
    };

    // Parse into a string
    let output = std::str::from_utf8(&output.stdout)?;

    // Split into lines of `Box<str>`
    let output = output.lines().map(Into::into);

    Ok(output.collect())
}
