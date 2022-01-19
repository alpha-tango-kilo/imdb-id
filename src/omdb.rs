use crate::{Genre, Result, RunError, Year};
use minreq::Request;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{self, Debug};
use std::str::FromStr;

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
    pub total_results: u32,
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

pub fn search_by_title(api_key: &str, title: &str) -> Result<SearchResults> {
    let title = urlencoding::encode(title);
    let request = build_query(api_key).with_param("s", title);
    let body = request.send()?;
    let body = body.as_str()?;

    let de = serde_json::from_str(body)
        .map_err(|err| RunError::OmdbUnrecognised(body.to_owned(), err))?;
    match de {
        OmdbResult::Ok(s) => Ok(s),
        OmdbResult::Err(e) => Err(RunError::OmdbError(e.error)),
    }
}

pub fn test_api_key(api_key: &str) -> Result<(), String> {
    if api_key.parse::<u32>().is_err() {
        return Err("Invalid API key format".into());
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

fn build_query(api_key: &str) -> Request {
    minreq::get("https://www.omdbapi.com/")
        .with_param("apikey", api_key)
        // Lock to API version 1 and return type JSON in case this changes in
        // future
        .with_param("v", "1")
        .with_param("r", "json")
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn api_key_u32_check() {
        test_api_key("foo").unwrap_err();
    }
}
