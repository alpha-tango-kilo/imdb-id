use crate::{Result, RunError};
use reqwest::blocking::Client;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::fmt::{self, Debug};
use std::str::FromStr;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OmdbEntry {
    pub title: String,
    #[serde(deserialize_with = "de_stringified")]
    pub year: u16,
    #[serde(rename = "Rated")]
    pub rating: String,
    pub runtime: String,
    #[serde(rename = "Genre", deserialize_with = "de_comma_list")]
    pub genres: Vec<String>,
    #[serde(rename = "Director", deserialize_with = "de_comma_list")]
    pub directors: Vec<String>,
    #[serde(rename = "Writer", deserialize_with = "de_comma_list")]
    pub writers: Vec<String>,
    #[serde(deserialize_with = "de_comma_list")]
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

impl fmt::Display for OmdbEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} ({})", self.title, self.year)?;
        writeln!(f, "Genre(s): {}", self.genres.join(", "))?;
        writeln!(f, "\n{}\n", self.plot)?;
        writeln!(f, "IMDb rating: {}\tIMDb ID: {}", self.imdb_rating, self.imdb_id)
    }
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
fn de_comma_list<'de, D>(d: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(s.split(", ").map(|s| s.into()).collect())
}

pub fn query_by_title(api_key: &str, title: &str) -> Result<OmdbEntry> {
    let client = Client::new();
    let body = client
        .get("https://www.omdbapi.com/")
        .query(&[("apikey", api_key), ("t", title)])
        .send()?
        .text()?;

    if body == r#"{"Response":"False","Error":"Movie not found!"}"# {
        Err(RunError::OmdbNotFound(title.into()))
    } else {
        serde_json::from_str(&body).map_err(|_| RunError::OmdbUnrecognised(body))
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use lazy_regex::Lazy;

    const INPUTS: [&str; 3] = [
        // Up
        r#"{"Title":"Up","Year":"2009","Rated":"PG","Released":"29 May 2009","Runtime":"96 min","Genre":"Animation, Adventure, Comedy","Director":"Pete Docter, Bob Peterson","Writer":"Pete Docter, Bob Peterson, Tom McCarthy","Actors":"Edward Asner, Jordan Nagai, John Ratzenberger","Plot":"78-year-old Carl Fredricksen travels to Paradise Falls in his house equipped with balloons, inadvertently taking a young stowaway.","Language":"English","Country":"United States","Awards":"Won 2 Oscars. 79 wins & 87 nominations total","Poster":"https://m.media-amazon.com/images/M/MV5BMTk3NDE2NzI4NF5BMl5BanBnXkFtZTgwNzE1MzEyMTE@._V1_SX300.jpg","Ratings":[{"Source":"Internet Movie Database","Value":"8.2/10"},{"Source":"Rotten Tomatoes","Value":"98%"},{"Source":"Metacritic","Value":"88/100"}],"Metascore":"88","imdbRating":"8.2","imdbVotes":"966,025","imdbID":"tt1049413","Type":"movie","DVD":"21 Nov 2015","BoxOffice":"$293,004,164","Production":"Pixar Animation Studios","Website":"N/A","Response":"True"}"#,
        // 1917
        r#"{"Title":"1917","Year":"2019","Rated":"R","Released":"10 Jan 2020","Runtime":"119 min","Genre":"Drama, Thriller, War","Director":"Sam Mendes","Writer":"Sam Mendes, Krysty Wilson-Cairns","Actors":"Dean-Charles Chapman, George MacKay, Daniel Mays","Plot":"April 6th, 1917. As a regiment assembles to wage war deep in enemy territory, two soldiers are assigned to race against time and deliver a message that will stop 1,600 men from walking straight into a deadly trap.","Language":"English, French, German","Country":"United States, United Kingdom, India, Spain, Canada, China","Awards":"Won 3 Oscars. 134 wins & 199 nominations total","Poster":"https://m.media-amazon.com/images/M/MV5BOTdmNTFjNDEtNzg0My00ZjkxLTg1ZDAtZTdkMDc2ZmFiNWQ1XkEyXkFqcGdeQXVyNTAzNzgwNTg@._V1_SX300.jpg","Ratings":[{"Source":"Internet Movie Database","Value":"8.3/10"},{"Source":"Rotten Tomatoes","Value":"89%"},{"Source":"Metacritic","Value":"78/100"}],"Metascore":"78","imdbRating":"8.3","imdbVotes":"483,190","imdbID":"tt8579674","Type":"movie","DVD":"25 Dec 2019","BoxOffice":"$159,227,644","Production":"Neal Street Productions, Amblin Entertainment","Website":"N/A","Response":"True"}"#,
        // Kingsman: The Secret Service
        r#"{"Title":"Kingsman: The Secret Service","Year":"2014","Rated":"R","Released":"13 Feb 2015","Runtime":"129 min","Genre":"Action, Adventure, Comedy","Director":"Matthew Vaughn","Writer":"Jane Goldman, Matthew Vaughn, Mark Millar","Actors":"Colin Firth, Taron Egerton, Samuel L. Jackson","Plot":"A spy organisation recruits a promising street kid into the agency's training program, while a global threat emerges from a twisted tech genius.","Language":"English, Arabic, Swedish","Country":"United Kingdom, United States","Awards":"10 wins & 32 nominations","Poster":"https://m.media-amazon.com/images/M/MV5BYTM3ZTllNzItNTNmOS00NzJiLTg1MWMtMjMxNDc0NmJhODU5XkEyXkFqcGdeQXVyODE5NzE3OTE@._V1_SX300.jpg","Ratings":[{"Source":"Internet Movie Database","Value":"7.7/10"},{"Source":"Metacritic","Value":"60/100"}],"Metascore":"60","imdbRating":"7.7","imdbVotes":"612,737","imdbID":"tt2802144","Type":"movie","DVD":"09 Jun 2015","BoxOffice":"$128,261,724","Production":"Marv Films, Cloudy","Website":"N/A","Response":"True"}"#,
    ];

    const DESERIALISED: Lazy<Vec<OmdbEntry>> = Lazy::new(|| {
        use super::OmdbResponse::*;
        INPUTS
            .iter()
            .map(|json_str| serde_json::from_str(*json_str).expect("Failed to deserialise"))
            .map(|response| match response {
                Found(entry) => entry,
                NotFound(_) => panic!("All test data should produce OmdbEntry"),
            })
            .collect()
    });

    #[test]
    fn converts_years() {
        let answers: [u16; 3] = [2009, 2019, 2014];
        DESERIALISED
            .iter()
            .map(|entry| &entry.year)
            .zip(answers.iter())
            .for_each(|(actual, expected)| assert_eq!(actual, expected));
    }

    #[test]
    fn converts_comma_lists() {
        let genres = [
            vec!["Animation", "Adventure", "Comedy"],
            vec!["Drama", "Thriller", "War"],
            vec!["Action", "Adventure", "Comedy"],
        ];
        DESERIALISED
            .iter()
            .map(|entry| &entry.genres)
            .zip(genres.iter())
            .for_each(|(actual, expected)| assert_eq!(actual, expected));

        let directors = [
            vec!["Pete Docter", "Bob Peterson"],
            vec!["Sam Mendes"],
            vec!["Matthew Vaughn"],
        ];
        DESERIALISED
            .iter()
            .map(|entry| &entry.directors)
            .zip(directors.iter())
            .for_each(|(actual, expected)| assert_eq!(actual, expected));

        let writers = [
            vec!["Pete Docter", "Bob Peterson", "Tom McCarthy"],
            vec!["Sam Mendes", "Krysty Wilson-Cairns"],
            vec!["Jane Goldman", "Matthew Vaughn", "Mark Millar"],
        ];
        DESERIALISED
            .iter()
            .map(|entry| &entry.writers)
            .zip(writers.iter())
            .for_each(|(actual, expected)| assert_eq!(actual, expected));

        let actors = [
            vec!["Edward Asner", "Jordan Nagai", "John Ratzenberger"],
            vec!["Dean-Charles Chapman", "George MacKay", "Daniel Mays"],
            vec!["Colin Firth", "Taron Egerton", "Samuel L. Jackson"],
        ];
        DESERIALISED
            .iter()
            .map(|entry| &entry.actors)
            .zip(actors.iter())
            .for_each(|(actual, expected)| assert_eq!(actual, expected));
    }
}
