use crate::{ApiKeyError, Filters, MediaTypeParseError, RequestError, Year};
use itertools::Itertools;
use lazy_static::lazy_static;
use minreq::Request;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smallvec::{smallvec, SmallVec};
use std::borrow::Cow;
use std::fmt::{self, Debug};
use std::str::FromStr;
use std::{env, iter};

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

impl From<OmdbResult> for Result<SearchResults, RequestError> {
    fn from(omdb_result: OmdbResult) -> Self {
        match omdb_result {
            OmdbResult::Ok(sr) => Ok(sr),
            OmdbResult::Err(e) => Err(RequestError::Omdb(e.error)),
        }
    }
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
    pub media_type: MediaType,
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}, {})", self.title, self.media_type, self.year)
    }
}

// These are the OMDb API supported media typers to filter by (episode has been
// intentionally excluded as it always returns 0 results)
// Serialize and Deserialize and implemented by hand
#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum MediaType {
    Movie,
    Series,
}

impl AsRef<str> for MediaType {
    fn as_ref(&self) -> &str {
        use MediaType::*;
        match self {
            Movie => "movie",
            Series => "series",
        }
    }
}

impl FromStr for MediaType {
    type Err = MediaTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use MediaType::*;
        match s.to_ascii_lowercase().as_str() {
            "movie" => Ok(Movie),
            "series" => Ok(Series),
            _ => Err(MediaTypeParseError(s.to_owned())),
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

// Serialize with MediaType.as_str
impl Serialize for MediaType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

// Deserialize with FromStr
impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(|_| D::Error::custom("unrecognised media type"))
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

// Taking ownership of MediaType should always be cheap as it should never be
// the Other(String) variant
#[derive(Default, Debug)]
struct FilterParameters {
    media_type: Option<MediaType>,
    year: Option<u16>,
}

impl From<MediaType> for FilterParameters {
    fn from(media_type: MediaType) -> Self {
        FilterParameters {
            media_type: Some(media_type),
            year: None,
        }
    }
}

impl From<u16> for FilterParameters {
    fn from(year: u16) -> Self {
        FilterParameters {
            media_type: None,
            year: Some(year),
        }
    }
}

impl From<(u16, MediaType)> for FilterParameters {
    fn from((year, media_type): (u16, MediaType)) -> Self {
        FilterParameters {
            media_type: Some(media_type),
            year: Some(year),
        }
    }
}

impl fmt::Display for FilterParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.media_type, self.year) {
            (Some(media_type), Some(year)) => {
                write!(f, "{media_type}, year {year}")
            }
            (Some(media_type), None) => write!(f, "{media_type}"),
            (None, Some(year)) => write!(f, "year {year}"),
            (None, None) => write!(f, "no filters"),
        }
    }
}

#[derive(Debug)]
pub struct RequestBundle<'a> {
    api_key: &'a str,
    title: Cow<'a, str>,
    params: SmallVec<[FilterParameters; DEFAULT_MAX_REQUESTS_PER_SEARCH]>,
}

impl<'a> RequestBundle<'a> {
    pub fn new(api_key: &'a str, title: &'a str, filters: &'a Filters) -> Self {
        let combinations = filters.combinations();
        if combinations > *MAX_REQUESTS_PER_SEARCH {
            eprintln!(
                "WARNING: the combination of filters you've specified \
            can't be exhaustively covered in {} requests (it would take \
            {combinations} requests), so some results will be missed. You can \
            set the IMDB_ID_MAX_REQUESTS_PER_SEARCH environment variable to \
            change this number",
                *MAX_REQUESTS_PER_SEARCH
            );
        }

        let params = match (filters.media_type_option(), filters.years.as_ref())
        {
            (None, None) => {
                // No filters at all
                smallvec![FilterParameters::default()]
            }
            (None, Some(years)) => {
                // Just years specified
                years
                    .0
                    .clone()
                    .take(*MAX_REQUESTS_PER_SEARCH)
                    .map(FilterParameters::from)
                    .collect::<SmallVec<_>>()
            }
            (Some(mt), None) => {
                // Just media type specified
                smallvec![FilterParameters::from(mt)]
            }
            (Some(mt), Some(years)) => {
                // Both years and media type specified
                years
                    .0
                    .clone()
                    .zip(iter::repeat(mt))
                    .take(*MAX_REQUESTS_PER_SEARCH)
                    .map(FilterParameters::from)
                    .collect::<SmallVec<_>>()
            }
        };
        RequestBundle {
            api_key,
            title: urlencoding::encode(title),
            params,
        }
    }

    pub fn get_results(&self) -> Vec<SearchResult> {
        self.params
            .iter()
            .map(|params| {
                // Make request
                let request = base_query(self.api_key, &self.title);
                let request = match &params.media_type {
                    Some(mt) => {
                        request.with_param("type", mt.to_string())
                    }
                    None => request,
                };
                let request = match params.year {
                    Some(year) => request.with_param("y", year.to_string()),
                    None => request,
                };
                (params, request)
            })
            .filter_map(|(params, request)| match send_omdb_search(request) {
                // Enumerate results at this point to get their ranking from
                // their own search. See next comment for why this is done
                Ok(results) => Some(results.entries.into_iter().enumerate()),
                Err(why) => {
                    eprintln!("Problem with request ({params}): {why}");
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
                    .unwrap_or_else(|_| panic!("Invalid IMDb ID (not numerical after 2 characters) in {:#?}", sr))
            })
            .collect()
    }
}

pub fn test_api_key(api_key: &str) -> Result<(), ApiKeyError> {
    use ApiKeyError::*;

    // Check that API key is 8 hexademical characters
    if !api_key_format_acceptable(api_key) {
        return Err(InvalidFormat);
    }

    let status = minreq::get("https://www.omdbapi.com/")
        .with_param("apikey", api_key)
        .send()?
        .status_code;

    if status.eq(&200) {
        Ok(())
    } else if status.eq(&401) {
        Err(Unauthorised)
    } else {
        Err(UnexpectedStatus(status))
    }
}

fn api_key_format_acceptable(api_key: &str) -> bool {
    api_key.len() == 8 && api_key.chars().all(|c| c.is_ascii_hexdigit())
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

fn send_omdb_search(request: Request) -> Result<SearchResults, RequestError> {
    let body = request.send()?;
    let body = body.as_str()?;

    serde_json::from_str::<OmdbResult>(body)
        .map_err(|err| RequestError::Deserialisation(err, body.to_owned()))?
        .into()
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn api_key_format() {
        assert!(!api_key_format_acceptable("fizzbuzz"));
        assert!(!api_key_format_acceptable("3q;mgh3w"));
        assert!(!api_key_format_acceptable("foo"));
        assert!(!api_key_format_acceptable("foobarbaz"));

        assert!(!api_key_format_acceptable("123f3"));
        assert!(!api_key_format_acceptable("435adf312b"));

        assert!(api_key_format_acceptable("13495632"));
        assert!(api_key_format_acceptable("3a3d4e1f"));
    }
}
