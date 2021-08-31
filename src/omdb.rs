use crate::Result;
use reqwest::blocking::Client;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::fmt::Debug;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OmdbResponse {
    Found(OmdbEntry),
    NotFound(String), // hold undeserialised JSON until we know what forms it comes in
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OmdbEntry {
    pub title: String,
    #[serde(deserialize_with = "de_stringified")]
    pub year: u16,
    #[serde(rename = "Rated")]
    pub rating: String,
    #[serde(rename = "Released")]
    pub release: String,
    pub runtime: String,
    #[serde(rename = "Genre", deserialize_with = "de_comma_separated_list")]
    pub genres: Vec<String>,
    #[serde(rename = "Director", deserialize_with = "de_comma_separated_list")]
    pub directors: Vec<String>,
    #[serde(rename = "Writer", deserialize_with = "de_comma_separated_list")]
    pub writers: Vec<String>,
    #[serde(deserialize_with = "de_comma_separated_list")]
    pub actors: Vec<String>,
    pub plot: String,
    pub language: String,
    pub country: String,
    #[serde(rename = "imdbID")]
    pub imdb_id: String,
    #[serde(rename = "imdbRating", deserialize_with = "de_stringified")]
    pub imdb_rating: f32,
    #[serde(rename = "Type")]
    pub media_type: String,
}

/*
OMDb returns all values as JSON strings, even those that aren't, like years
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
This helper throws that into a Vec<String>
 */
fn de_comma_separated_list<'de, D>(d: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(s.split(", ").map(|s| s.into()).collect())
}

pub fn query_by_title(api_key: &str, title: &str) -> Result<OmdbResponse> {
    let client = Client::new();
    let omdb_entry = client
        .get("https://www.omdbapi.com/")
        .query(&[("apikey", api_key), ("t", title)])
        .send()?
        .json()?;
    Ok(omdb_entry)
}
