use crate::{Result, RunError, Year};
use reqwest::blocking::{Client, RequestBuilder};
use serde::de::{DeserializeOwned, Error};
use serde::{Deserialize, Deserializer, Serialize};
use smallvec::SmallVec;
use std::fmt::{self, Debug};
use std::str::FromStr;

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
    pub media_type: String,
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}, {})", self.title, self.media_type, self.year)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "PascalCase"))]
pub struct Entry {
    pub title: String,
    pub year: Year,
    #[serde(rename(deserialize = "Rated"))]
    pub rating: String,
    pub runtime: String,
    #[serde(rename(deserialize = "Genre"), deserialize_with = "de_comma_list")]
    pub genres: SmallVec<[String; 3]>,
    #[serde(rename(deserialize = "Director"), deserialize_with = "de_comma_list")]
    pub directors: SmallVec<[String; 3]>,
    #[serde(rename(deserialize = "Writer"), deserialize_with = "de_comma_list")]
    pub writers: SmallVec<[String; 3]>,
    #[serde(deserialize_with = "de_comma_list")]
    pub actors: SmallVec<[String; 3]>,
    pub plot: String,
    pub language: String,
    pub country: String,
    #[serde(rename(deserialize = "imdbID"))]
    pub imdb_id: String,
    #[serde(
        rename(deserialize = "imdbRating"),
        deserialize_with = "de_stringified"
    )]
    pub imdb_rating: f32,
    #[serde(rename(deserialize = "Type"))]
    pub media_type: String,
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} ({})", self.title, self.year)?;
        writeln!(f, "Genre(s): {}", self.genres.join(", "))?;
        writeln!(f, "\n{}\n", self.plot)?;
        writeln!(
            f,
            "IMDb rating: {}\tIMDb ID: {}",
            self.imdb_rating, self.imdb_id
        )
    }
}

/*
OMDb returns all values as JSON strings, even those that aren't, like ratings
This helper can be given to serde to try and convert those elements to a more
useful type, like u16 for years
 */
fn de_stringified<'de, D, T>(d: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    let s = String::deserialize(d)?;
    T::from_str(&s)
        .map_err(|e| D::Error::custom(format!("Could not parse field as desired type ({:?})", e)))
}

/*
Lists in OMDb are given like "Pete Docter, Bob Peterson, Tom McCarthy"
This helper throws that into a SmallVec<[String; 3]>
 */
fn de_comma_list<'de, D>(d: D) -> std::result::Result<SmallVec<[String; 3]>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(s.split(", ").map(|s| s.into()).collect())
}

/*
About using reqwest::blocking;
From the tokio website, "When not to use Tokio"
  - Sending a single web request. The place where Tokio gives you an advantage
    is when you need to do many things at the same time. If you need to use a
    library intended for asynchronous Rust such as reqwest, but you don't need
    to do a lot of things at once, you should prefer the blocking version of
    that library, as it will make your project simpler. Using Tokio will still
    work, of course, but provides no real advantage over the blocking API
- https://tokio.rs/tokio/tutorial

A note about the json feature of reqwest:
While it does seem like it'd be useful, in reality it prevents access to the raw JSON response
if the deserialisation fails. It also means I can't as specifically classify the type of
error for packaging into RunError
 */

pub fn lookup_title(api_key: &str, client: &Client, title: &str) -> Result<Entry> {
    let body = build_query(client, api_key)
        .query(&[("t", title)])
        .send()?
        .text()?;

    if body == r#"{"Response":"False","Error":"Movie not found!"}"# {
        Err(RunError::OmdbNotFound(title.into()))
    } else {
        serde_json::from_str(&body).map_err(|err| RunError::OmdbUnrecognised(body, err))
    }
}

pub fn search_by_title(api_key: &str, client: &Client, title: &str) -> Result<SearchResults> {
    let request = build_query(client, api_key).query(&[("s", title)]);
    send_request_deserialise_response(request)
}

pub fn test_api_key(api_key: &str, client: &Client) -> std::result::Result<(), String> {
    if api_key.parse::<u32>().is_err() {
        return Err("Invalid API key format".into());
    }

    let status = client
        .get("https://www.omdbapi.com/")
        .query(&[("apikey", api_key)])
        .send()
        .map_err(|e| e.to_string())?
        .status();

    if status.eq(&200) {
        Ok(())
    } else if status.eq(&401) {
        Err(String::from("Unauthorised API key, please edit your input"))
    } else {
        Err(format!("Unexpected response: {}", status))
    }
}

fn build_query(client: &Client, api_key: &str) -> RequestBuilder {
    client
        .get("https://www.omdbapi.com/")
        // Lock to API version 1 and return type JSON in case this changes in future
        .query(&[("apikey", api_key), ("v", "1"), ("r", "json")])
}

fn send_request_deserialise_response<T>(request: RequestBuilder) -> Result<T>
where
    T: DeserializeOwned,
{
    let body = request.send()?.text()?;
    serde_json::from_str(&body).map_err(|err| RunError::OmdbUnrecognised(body, err))
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use once_cell::unsync::Lazy;

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

    const DESERIALISED: Lazy<Vec<Entry>> = Lazy::new(|| {
        INPUTS
            .iter()
            .map(|json_str| serde_json::from_str(*json_str).expect("Failed to deserialise"))
            .collect()
    });

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
            .for_each(|(actual, expected)| assert_eq!(actual.as_slice(), expected.as_slice()));

        let directors = [
            vec!["Pete Docter", "Bob Peterson"],
            vec!["Sam Mendes"],
            vec!["Matthew Vaughn"],
            vec!["N/A"],
        ];
        DESERIALISED
            .iter()
            .map(|entry| &entry.directors)
            .zip(directors.iter())
            .for_each(|(actual, expected)| assert_eq!(actual.as_slice(), expected.as_slice()));

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
            .for_each(|(actual, expected)| assert_eq!(actual.as_slice(), expected.as_slice()));

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
            .for_each(|(actual, expected)| assert_eq!(actual.as_slice(), expected.as_slice()));
    }

    #[test]
    fn api_key_u32_check() {
        let client = reqwest::blocking::Client::new();
        let err = test_api_key("foo", &client).unwrap_err();
        assert_eq!(
            &err, "Invalid API key format, please edit your input",
            "Different error returned than expected: '{}'",
            err
        );
    }
}
