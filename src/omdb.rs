use crate::{ApiKeyError, Filters, MediaTypeParseError, RequestError, Year};
use itertools::Itertools;
use lazy_static::lazy_static;
use minreq::Request;
use serde::de::{DeserializeOwned, Error};
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
enum OmdbResult<T> {
    Err(OmdbError),
    Ok(T),
}

impl<T> From<OmdbResult<T>> for Result<T, RequestError> {
    fn from(omdb_result: OmdbResult<T>) -> Self {
        match omdb_result {
            OmdbResult::Ok(t) => Ok(t),
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
        deserialize_with = "de_parseable"
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all(deserialize = "PascalCase"))]
pub struct Entry {
    pub title: String,
    pub year: Year,
    #[serde(
        rename(deserialize = "Rated"),
        deserialize_with = "de_option_parseable"
    )]
    pub rating: Option<String>, // can be N/A (on series?)
    #[serde(deserialize_with = "de_option_parseable")]
    pub runtime: Option<String>, // can be N/A (on series?)
    #[serde(rename(deserialize = "Genre"), deserialize_with = "de_comma_list")]
    pub genres: Vec<String>,
    #[serde(
        rename(deserialize = "Director"),
        deserialize_with = "de_option_comma_list"
    )]
    pub directors: Option<Vec<String>>, // can be N/A
    #[serde(
        rename(deserialize = "Writer"),
        deserialize_with = "de_option_comma_list"
    )]
    pub writers: Option<Vec<String>>, // can be N/A
    #[serde(deserialize_with = "de_comma_list")]
    pub actors: Vec<String>,
    #[serde(deserialize_with = "de_option_parseable")]
    pub plot: Option<String>, // can be N/A
    #[serde(deserialize_with = "de_comma_list")]
    pub language: Vec<String>,
    #[serde(deserialize_with = "de_comma_list")]
    pub country: Vec<String>,
    #[serde(rename(deserialize = "Type"))]
    pub media_type: MediaType,
    // #[serde(default)] as movies don't have this
    #[serde(
        rename(deserialize = "totalSeasons"),
        deserialize_with = "de_option_parseable",
        default
    )]
    pub seasons: Option<u16>,
}

/*
Lists in OMDb are given like "Pete Docter, Bob Peterson, Tom McCarthy"
This helper could throw that into a Vec<String>
 */
fn de_comma_list<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + PartialEq,
    <T as FromStr>::Err: fmt::Display,
{
    // TODO: something more graceful than panicking
    let ts = de_option_comma_list(d)?.expect(
        "Unexpected N/A value\n\
        Please raise an issue, \
        making sure you at least state what you searched, \
        or preferably the entry that caused the issue",
    );
    Ok(ts)
}

fn de_option_comma_list<'de, D, T>(d: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + PartialEq,
    <T as FromStr>::Err: fmt::Display,
{
    let s = String::deserialize(d)?;
    let option = if s != "N/A" {
        let mut ts = Vec::new();
        for s in s.split(", ") {
            let t = s.parse().map_err(D::Error::custom)?;
            // Deduplicate as some entries have duplicates from the API,
            // e.g. tt11031770 has duplicate genres
            // This will only ever be done on small vectors so I would imagine
            // using a HashSet to do duplicate detection would be less
            // efficient
            if !ts.contains(&t) {
                ts.push(t);
            }
        }
        Some(ts)
    } else {
        None
    };
    Ok(option)
}

/*
OMDb returns all values as JSON strings, even those that aren't, like ratings
This helper can be given to serde to try and convert those elements to a more
useful type, like u16 for years
 */
fn de_parseable<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: fmt::Display,
{
    // TODO: something more graceful than panicking
    let t = de_option_parseable(d)?.expect(
        "Unexpected N/A value\n\
        Please raise an issue, \
        making sure you at least state what you searched, \
        or preferably the entry that caused the issue",
    );
    Ok(t)
}

/*
OMDb sometimes (not always, not never) includes fields that it doesn't have
anything useful to provide for, giving the value of said fields as "N/A". This
function produces an Option<T>, where T can be parsed using FromStr. "N/A"
cases will return None
 */
fn de_option_parseable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: fmt::Display,
{
    let s = String::deserialize(d)?;
    let option = if s != "N/A" {
        let t = s.parse().map_err(D::Error::custom)?;
        Some(t)
    } else {
        None
    };
    Ok(option)
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
            .map_err(D::Error::custom)
    }
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
                let request = base_query(self.api_key)
                    .with_param("s", self.title.as_ref());
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

pub fn get_entry(api_key: &str, imdb_id: &str) -> Result<Entry, RequestError> {
    let request = base_query(api_key).with_param("i", imdb_id);
    send_request_deserialise(request)
}

fn base_query(api_key: &str) -> Request {
    minreq::get("https://www.omdbapi.com/")
        .with_param("apikey", api_key)
        // Lock to API version 1 and return type JSON in case this changes in
        // future
        .with_param("v", "1")
        .with_param("r", "json")
}

// function is just a prettier, more explanatory name for
// send_request_deserialise<SearchResults>
fn send_omdb_search(request: Request) -> Result<SearchResults, RequestError> {
    send_request_deserialise(request)
}

fn send_request_deserialise<T>(request: Request) -> Result<T, RequestError>
where
    T: DeserialisableWithinOmdbResult + DeserializeOwned + Debug,
{
    let response = request.send()?;
    let body = response.as_str()?;

    serde_json::from_str::<OmdbResult<T>>(body)
        .map_err(|_| {
            // We re-attempt parsing to get a more useful error out of serde
            // If there's something bad in the SearchResults/Entry (usual
            // cause), then getting the issue with that is more useful than
            // "did not match untagged enum" or whatever
            // Yes this is probably expensive, hopefully I won't be doing it
            // often. This is the error path after all
            let useful_err = serde_json::from_str::<T>(body).expect_err(
                "Deserializing succeeded only when not wrapped in OmdbResult",
            );
            RequestError::Deserialisation(useful_err, body.to_owned())
        })?
        .into()
}

// Type system protection to ensure send_request_deserialise is used safely
trait DeserialisableWithinOmdbResult {}
impl DeserialisableWithinOmdbResult for SearchResults {}
impl DeserialisableWithinOmdbResult for Entry {}

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

    // TODO: "N/A" tests

    const INPUTS: [&str; 4] = [
        // Up
        r#"{"Title":"Up","Year":"2009","Rated":"PG","Released":"29 May 2009","Runtime":"96 min","Genre":"Animation, Adventure, Comedy","Director":"Pete Docter, Bob Peterson","Writer":"Pete Docter, Bob Peterson, Tom McCarthy","Actors":"Edward Asner, Jordan Nagai, John Ratzenberger","Plot":"78-year-old Carl Fredricksen travels to Paradise Falls in his house equipped with balloons, inadvertently taking a young stowaway.","Language":"English","Country":"United States","Awards":"Won 2 Oscars. 79 wins & 87 nominations total","Poster":"https://m.media-amazon.com/images/M/MV5BMTk3NDE2NzI4NF5BMl5BanBnXkFtZTgwNzE1MzEyMTE@._V1_SX300.jpg","Ratings":[{"Source":"Internet Movie Database","Value":"8.2/10"},{"Source":"Rotten Tomatoes","Value":"98%"},{"Source":"Metacritic","Value":"88/100"}],"Metascore":"88","imdbRating":"8.2","imdbVotes":"966,025","imdbID":"tt1049413","Type":"movie","DVD":"21 Nov 2015","BoxOffice":"$293,004,164","Production":"Pixar Animation Studios","Website":"N/A","Response":"True"}"#,
        // 1917
        r#"{"Title":"1917","Year":"2019","Rated":"R","Released":"10 Jan 2020","Runtime":"119 min","Genre":"Drama, Thriller, War","Director":"Sam Mendes","Writer":"Sam Mendes, Krysty Wilson-Cairns","Actors":"Dean-Charles Chapman, George MacKay, Daniel Mays","Plot":"April 6th, 1917. As a regiment assembles to wage war deep in enemy territory, two soldiers are assigned to race against time and deliver a message that will stop 1,600 men from walking straight into a deadly trap.","Language":"English, French, German","Country":"United States, United Kingdom, India, Spain, Canada, China","Awards":"Won 3 Oscars. 134 wins & 199 nominations total","Poster":"https://m.media-amazon.com/images/M/MV5BOTdmNTFjNDEtNzg0My00ZjkxLTg1ZDAtZTdkMDc2ZmFiNWQ1XkEyXkFqcGdeQXVyNTAzNzgwNTg@._V1_SX300.jpg","Ratings":[{"Source":"Internet Movie Database","Value":"8.3/10"},{"Source":"Rotten Tomatoes","Value":"89%"},{"Source":"Metacritic","Value":"78/100"}],"Metascore":"78","imdbRating":"8.3","imdbVotes":"483,190","imdbID":"tt8579674","Type":"movie","DVD":"25 Dec 2019","BoxOffice":"$159,227,644","Production":"Neal Street Productions, Amblin Entertainment","Website":"N/A","Response":"True"}"#,
        // Kingsman: The Secret Service
        r#"{"Title":"Kingsman: The Secret Service","Year":"2014","Rated":"R","Released":"13 Feb 2015","Runtime":"129 min","Genre":"Action, Adventure, Comedy","Director":"Matthew Vaughn","Writer":"Jane Goldman, Matthew Vaughn, Mark Millar","Actors":"Colin Firth, Taron Egerton, Samuel L. Jackson","Plot":"A spy organisation recruits a promising street kid into the agency's training program, while a global threat emerges from a twisted tech genius.","Language":"English, Arabic, Swedish","Country":"United Kingdom, United States","Awards":"10 wins & 32 nominations","Poster":"https://m.media-amazon.com/images/M/MV5BYTM3ZTllNzItNTNmOS00NzJiLTg1MWMtMjMxNDc0NmJhODU5XkEyXkFqcGdeQXVyODE5NzE3OTE@._V1_SX300.jpg","Ratings":[{"Source":"Internet Movie Database","Value":"7.7/10"},{"Source":"Metacritic","Value":"60/100"}],"Metascore":"60","imdbRating":"7.7","imdbVotes":"612,737","imdbID":"tt2802144","Type":"movie","DVD":"09 Jun 2015","BoxOffice":"$128,261,724","Production":"Marv Films, Cloudy","Website":"N/A","Response":"True"}"#,
        // Breakout Kings
        r#"{"Title":"Breakout Kings","Year":"2011–2012","Rated":"TV-14","Released":"06 Mar 2011","Runtime":"43 min","Genre":"Crime, Drama, Thriller","Director":"N/A","Writer":"Matt Olmstead, Nick Santora","Actors":"Domenick Lombardozzi, Brooke Nevin, Malcolm Goodwin","Plot":"A squad of U.S. marshals team up with cons (former fugitives) to work together on tracking down prison escapees in exchange for getting time off their sentences.","Language":"English","Country":"United States","Awards":"N/A","Poster":"https://m.media-amazon.com/images/M/MV5BMTcyNzUwNjMwM15BMl5BanBnXkFtZTcwOTgxNjk0Nw@@._V1_SX300.jpg","Ratings":[{"Source":"Internet Movie Database","Value":"7.3/10"}],"Metascore":"N/A","imdbRating":"7.3","imdbVotes":"15,196","imdbID":"tt1590961","Type":"series","totalSeasons":"2","Response":"True"}"#,
    ];

    lazy_static! {
        static ref DESERIALISED: Vec<Entry> = INPUTS
            .iter()
            .map(|json_str| {
                serde_json::from_str(*json_str).expect("Failed to deserialise")
            })
            .collect();
    }

    #[test]
    fn converts_comma_lists() {
        let genres = [
            vec!["Animation", "Adventure", "Comedy"],
            vec!["Drama", "Thriller", "War"],
            vec!["Action", "Adventure", "Comedy"],
            vec!["Crime", "Drama", "Thriller"],
        ];
        DESERIALISED
            .iter()
            .map(|entry| &entry.genres)
            .zip(genres.iter())
            .for_each(|(actual, expected)| {
                assert_eq!(actual.as_slice(), expected.as_slice())
            });

        let directors = [
            vec!["Pete Docter", "Bob Peterson"],
            vec!["Sam Mendes"],
            vec!["Matthew Vaughn"],
        ];
        DESERIALISED
            .iter()
            .map(|entry| &entry.directors)
            .zip(directors.iter())
            .for_each(|(actual, expected)| {
                assert_eq!(
                    actual.as_ref().unwrap().as_slice(),
                    expected.as_slice()
                )
            });

        let writers = [
            vec!["Pete Docter", "Bob Peterson", "Tom McCarthy"],
            vec!["Sam Mendes", "Krysty Wilson-Cairns"],
            vec!["Jane Goldman", "Matthew Vaughn", "Mark Millar"],
            vec!["Matt Olmstead", "Nick Santora"],
        ];
        DESERIALISED
            .iter()
            .map(|entry| &entry.writers)
            .zip(writers.iter())
            .for_each(|(actual, expected)| {
                assert_eq!(
                    actual.as_ref().unwrap().as_slice(),
                    expected.as_slice()
                )
            });

        let actors = [
            vec!["Edward Asner", "Jordan Nagai", "John Ratzenberger"],
            vec!["Dean-Charles Chapman", "George MacKay", "Daniel Mays"],
            vec!["Colin Firth", "Taron Egerton", "Samuel L. Jackson"],
            vec!["Domenick Lombardozzi", "Brooke Nevin", "Malcolm Goodwin"],
        ];
        DESERIALISED
            .iter()
            .map(|entry| &entry.actors)
            .zip(actors.iter())
            .for_each(|(actual, expected)| {
                assert_eq!(actual.as_slice(), expected.as_slice())
            });
    }
}
