mod clap_wrap;

pub use clap_wrap::*;

use anyhow::anyhow;
use lazy_regex::*;
use scraper::Selector;
use std::convert::TryFrom;
use std::fmt;
use std::fmt::Display;

/*
The DIRT_MARGIN_* constants refer to the amount of unwanted characters captured by the regex
For example, to capture the movie name we have to also find the > character to know we're at the start of the name, and the </a> tag to know we're at the end of the movie name.
This gives movie names a 'dirt margin' of (1, 4): 1 character at the start, 4 characters at the end
 */

pub const URL_START: &str = "https://www.imdb.com/find?s=tt&q=";
pub static RESULT_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("td.result_text").unwrap());
static ID_REGEX: Lazy<Regex> = lazy_regex!("tt[0-9]+");
// +? means 1 or more, not greedily
static NAME_REGEX: Lazy<Regex> = lazy_regex!(">.+?</a>");
const DIRT_MARGIN_NAME: (usize, usize) = (1, 4);
// TODO: make GENRE_REGEX more non-English friendly
// Would use \w escape but it matches numbers as well as letters
static GENRE_REGEX: Lazy<Regex> = lazy_regex!("\\([A-z]+(\\s[A-z]+)?\\)");
const DIRT_MARGIN_GENRE: (usize, usize) = (1, 1);

// TODO: year
#[derive(Debug)]
pub struct SearchResult {
    pub name: String,
    pub id: String,
    pub genre: Genre,
}

impl SearchResult {
    fn find_name_in_fragment(fragment: &str) -> anyhow::Result<&str> {
        let m = NAME_REGEX
            .find(fragment)
            .ok_or(anyhow!("Couldn't find a name in {:?}", fragment))?;
        let dirty_name = m.as_str();
        let clean_name = &dirty_name[DIRT_MARGIN_NAME.0..dirty_name.len() - DIRT_MARGIN_NAME.1];
        Ok(clean_name)
    }
}

impl TryFrom<&str> for SearchResult {
    type Error = anyhow::Error;

    fn try_from(fragment: &str) -> Result<Self, Self::Error> {
        let id = ID_REGEX
            .find(fragment)
            .ok_or(anyhow!("Regex couldn't find an IMDb ID in {:?}", fragment))?
            .as_str()
            .into();
        let name = SearchResult::find_name_in_fragment(fragment)?.to_string();
        if name.len() > 40 {
            println!("{}", fragment);
        }
        let genre_option = match GENRE_REGEX.find(fragment) {
            Some(m) => {
                let s = m.as_str();
                &s[DIRT_MARGIN_GENRE.0..s.len() - DIRT_MARGIN_GENRE.1]
            }
            None => "",
        };
        let genre = Genre::from(genre_option);
        Ok(SearchResult { name, id, genre })
    }
}

impl Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.genre)
    }
}

// TODO: consider supporting more
#[derive(Debug)]
pub enum Genre {
    Movie,
    TvSeries,
    TvEpisode,
    Short,
    Video,
    Other(String),
}

impl From<&str> for Genre {
    fn from(s: &str) -> Self {
        use Genre::*;
        match s {
            "Movie" | "" => Movie,
            "TV Series" => TvSeries,
            "TV Episode" => TvEpisode,
            "Short" => Short,
            "Video" => Video,
            _ => Other(s.into()),
        }
    }
}

impl Display for Genre {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Genre::*;
        write!(
            f,
            "{}",
            match self {
                Movie => "Movie",
                TvSeries => "TV series",
                TvEpisode => "TV episode",
                Short => "Short",
                Video => "Video",
                Other(s) => s,
            }
        )
    }
}
