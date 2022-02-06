use crate::{Filters, Genre, Result, RunError, Year};
use itertools::Itertools;
use lazy_static::lazy_static;
use minreq::Request;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::env;
use std::fmt::{self, Debug};
use std::str::FromStr;

const DEFAULT_MAX_REQUESTS_PER_SEARCH: usize = 10;

lazy_static! {
    static ref MAX_REQUESTS_PER_SEARCH: usize = {
        match env::var("IMDB_ID_MAX_REQUESTS_PER_SEARCH") {
            Ok(str) => str.parse().unwrap_or(DEFAULT_MAX_REQUESTS_PER_SEARCH),
            Err(_) => DEFAULT_MAX_REQUESTS_PER_SEARCH,
        }
    };
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OmdbResult {
    Err(OmdbError),
    Ok(SearchResults),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct OmdbError {
    error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// When serialising, just give the list of results
#[serde(into = "Vec<SearchResult>")]
pub struct SearchResults {
    #[serde(rename(deserialize = "Search"))]
    pub entries: Vec<SearchResult>,
    #[serde(
        rename(deserialize = "totalResults"),
        deserialize_with = "de_stringified"
    )]
    pub total_results: u32, // not used or cared about currently
}

// For serialisation
impl From<SearchResults> for Vec<SearchResult> {
    fn from(search_results: SearchResults) -> Self {
        search_results.entries
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "PascalCase"))]
pub struct SearchResult {
    pub title: String,
    pub year: Year,
    #[serde(rename(deserialize = "imdbID"))]
    pub imdb_id: String,
    #[serde(rename(deserialize = "Type"))]
    pub media_type: Genre,
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}, {})", self.title, self.media_type, self.year)
    }
}

/*
OMDb returns all values as JSON strings, even those that aren't, like ratings
This helper can be given to serde to try and convert those elements to a more
useful type, like u16 for years
 */
fn de_stringified<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    let s = String::deserialize(d)?;
    T::from_str(&s).map_err(|e| {
        D::Error::custom(format!(
            "could not parse field as desired type ({:?})",
            e
        ))
    })
}

// TODO: nice debug printing - errors show which request they're from in a user
//       understandable fashion
// TODO: warn when unable to cover all of the filter combinations in the number
//       of requests allowed
pub struct RequestBundle(Vec<Request>);

impl RequestBundle {
    pub fn new(api_key: &str, title: &str, filters: &Filters) -> Self {
        let title = urlencoding::encode(title);
        let base_query = base_query(api_key, &title);
        let Filters { genres, years } = filters;
        let requests = match (genres.as_slice(), years) {
            (&[], None) => {
                // No filters at all
                vec![base_query]
            }
            (&[], Some(years)) => {
                // Just years specified
                years
                    .0
                    .clone()
                    .take(*MAX_REQUESTS_PER_SEARCH)
                    .map(|year| {
                        base_query.clone().with_param("y", year.to_string())
                    })
                    .collect::<Vec<_>>()
            }
            (genres, None) => {
                // Just genres specified
                genres
                    .iter()
                    .filter(|genre| !matches!(genre, Genre::Other(_)))
                    // The take should be redundant here as there are only
                    // three supported genres/types currently. Consider this
                    // future-proofing
                    .take(*MAX_REQUESTS_PER_SEARCH)
                    .map(|genre| {
                        base_query.clone().with_param("type", genre.to_string())
                    })
                    .collect::<Vec<_>>()
            }
            (genres, Some(years)) => {
                // Both years and genre specified
                genres
                    .iter()
                    .filter(|genre| !matches!(genre, Genre::Other(_)))
                    .cartesian_product(years.0.clone())
                    .take(*MAX_REQUESTS_PER_SEARCH)
                    .map(|(genre, year)| {
                        base_query
                            .clone()
                            .with_param("y", year.to_string())
                            .with_param("type", genre.to_string())
                    })
                    .collect::<Vec<_>>()
            }
        };
        RequestBundle(requests)
    }

    pub fn get_results(self) -> Vec<SearchResult> {
        self.0
            .into_iter()
            .map(send_omdb_search)
            .filter_map(|results| match results {
                // Enumerate results at this point to get their ranking from
                // their own search. See next comment for why this is done
                Ok(results) => Some(results.entries.into_iter().enumerate()),
                Err(why) => {
                    eprintln!("{why}");
                    None
                }
            })
            // Merge results for different searches based on their rankings
            // from their own search. The end result should be all the first
            // results, then all the second results, etc.
            .kmerge_by(|a, b| a.0 < b.0)
            .map(|(_, sr)| sr)
            // I've noticed some duplicates coming through even from the API
            // directly, so might as well use itertools now I have it
            // Have to parse the IMDb ID to a number because the value must be
            // Copy
            .unique_by(|sr| {
                sr.imdb_id[2..]
                    .parse::<u32>()
                    .expect("Invalid IMDb ID (not numerical after 2 digits)")
            })
            .collect()
    }
}

// TODO: proper error type
pub fn test_api_key(api_key: &str) -> Result<(), String> {
    if api_key.parse::<u32>().is_err() {
        return Err(String::from("Invalid API key format"));
    }

    let status = minreq::get("https://www.omdbapi.com/")
        .with_param("apikey", api_key)
        .send()
        .map_err(|e| e.to_string())?
        .status_code;

    if status.eq(&200) {
        Ok(())
    } else if status.eq(&401) {
        Err(String::from("Unauthorised API key, please edit your input"))
    } else {
        Err(format!("Unexpected response: {status}"))
    }
}

fn base_query(api_key: &str, title: &str) -> Request {
    minreq::get("https://www.omdbapi.com/")
        .with_param("apikey", api_key)
        .with_param("s", title)
        // Lock to API version 1 and return type JSON in case this changes in
        // future
        .with_param("v", "1")
        .with_param("r", "json")
}

fn send_omdb_search(request: Request) -> Result<SearchResults> {
    let body = request.send()?;
    let body = body.as_str()?;

    let de = serde_json::from_str(body)
        .map_err(|err| RunError::OmdbUnrecognised(body.to_owned(), err))?;
    match de {
        OmdbResult::Ok(s) => Ok(s),
        OmdbResult::Err(e) => Err(RunError::OmdbError(e.error)),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn api_key_u32_check() {
        test_api_key("foo").unwrap_err();
    }
}
