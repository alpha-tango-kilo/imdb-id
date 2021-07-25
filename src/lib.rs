mod clap_wrap;
mod error;

pub use clap_wrap::*;
pub use error::*;

use lazy_regex::*;
use scraper::Selector;
use std::convert::TryFrom;
use std::fmt;
use requestty::Question;
use requestty::question::Choice;

/*
The DIRT_MARGIN_* constants refer to the amount of unwanted characters captured by the regex.
For example, to capture the movie name we have to also find the > character to know we're at the start of the name, and the </a> tag to know we're at the end of the movie name.
This gives movie names a 'dirt margin' of (1, 4): 1 character at the start, 4 characters at the end
 */

pub const URL_START: &str = "https://www.imdb.com/find?s=tt&q=";
pub static RESULT_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("td.result_text").unwrap());
// Matches something like "tt6856242"
static ID_REGEX: Lazy<Regex> = lazy_regex!("tt[0-9]+");
// Matches something like ">Kingsman: The Secret Service</a>"
// +? means 1 or more, not greedily
static NAME_REGEX: Lazy<Regex> = lazy_regex!(">.+?</a>");
const DIRT_MARGIN_NAME: (usize, usize) = (1, 4);
// Matches something like "(TV Series)"
static GENRE_REGEX: Lazy<Regex> = lazy_regex!("\\([A-z]+(\\s[A-z]+)?\\)");
const DIRT_MARGIN_GENRE: (usize, usize) = (1, 1);

// TODO: year
#[derive(Debug)]
pub struct SearchResult {
    pub name: String,
    pub id: String,
    pub genre: String,
}

impl SearchResult {
    fn find_name_in_fragment(fragment: &str) -> Result<&str> {
        let m = NAME_REGEX
            .find(fragment)
            .ok_or(RunError::NameNotFound(fragment.into()))?;
        let dirty_name = m.as_str();
        let clean_name = &dirty_name[DIRT_MARGIN_NAME.0..dirty_name.len() - DIRT_MARGIN_NAME.1];
        Ok(clean_name)
    }
}

impl TryFrom<&str> for SearchResult {
    type Error = RunError;

    fn try_from(fragment: &str) -> Result<Self> {
        let id = ID_REGEX
            .find(fragment)
            .ok_or(RunError::ImdbIdNotFound(fragment.into()))?
            .as_str()
            .into();
        let name = SearchResult::find_name_in_fragment(fragment)?.to_string();

        if cfg!(debug_assertions) && name.len() > 40 {
            println!("DEBUG: Strangely long fragment: {:?}", fragment);
        }

        let genre = match GENRE_REGEX.find(fragment) {
            Some(m) => {
                let s = m.as_str();
                String::from(&s[DIRT_MARGIN_GENRE.0..s.len() - DIRT_MARGIN_GENRE.1])
            }
            None => String::from("Movie"),
        };
        Ok(SearchResult { name, id, genre })
    }
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.genre)
    }
}

pub fn choose_from_results(results: &Vec<SearchResult>) -> Result<&SearchResult> {
    let choices = {
        let mut choices = results.iter()
            .map(|sr| sr.to_string().into())
            .collect::<Vec<Choice<String>>>();
        choices.push(requestty::DefaultSeparator);
        choices.push(Choice::Choice("I can't see what I'm looking for".into()));
        choices
    };

    let question = Question::select("result")
        .message("Pick the correct search result")
        .choices(choices)
        .build();

    let answer = requestty::prompt_one(question)?;
    let item = answer.as_list_item().unwrap();
    if item.index < results.len() {
        Ok(results.get(item.index).unwrap())
    } else {
        // User selected "I can't see what I'm looking for"
        Err(RunError::NoDesiredSearchResults)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    // Data taken from search term "kingsmen"
    const INPUTS: [&str; 10] = [
        " <a href=\"/title/tt6856242/?ref_=fn_tt_tt_1\">The King's Man</a> (2021) ",
        " <a href=\"/title/tt0405676/?ref_=fn_tt_tt_2\">All the King's Men</a> (2006) ",
        " <a href=\"/title/tt2119547/?ref_=fn_tt_tt_3\">The Kingsmen</a> (2011) (Short) ",
        " <a href=\"/title/tt0041113/?ref_=fn_tt_tt_4\">All the King's Men</a> (1949) ",
        " <a href=\"/title/tt4649466/?ref_=fn_tt_tt_5\">Kingsman: The Golden Circle</a> (2017) ",
        " <a href=\"/title/tt2802144/?ref_=fn_tt_tt_6\">Kingsman: The Secret Service</a> (2014) ",
        " <a href=\"/title/tt0222577/?ref_=fn_tt_tt_7\">King's Men</a> (1975) (TV Series) ",
        " <a href=\"/title/tt14642606/?ref_=fn_tt_tt_8\">The Kingsmen</a> (2017) (TV Episode) <br> <small>- Season 3 <sp
    an class=\"ghost\">|</span> Episode 22 </small> <br><small>- <a href=\"/title/tt3319722/?ref_=fn_tt_tt_8a\">Gosp
    el Music Showcase</a> (2011) (TV Series) </small> ",
        " <a href=\"/title/tt0220969/?ref_=fn_tt_tt_9\">All the King's Men</a> (1999) (TV Movie) ",
        " <a href=\"/title/tt0084793/?ref_=fn_tt_tt_10\">Tian xia di yi</a> (1983) ",
    ];
    static SEARCH_RESULTS: Lazy<Vec<SearchResult>> = Lazy::new(|| {
        INPUTS
            .iter()
            .map(|s| match SearchResult::try_from(*s) {
                Ok(sr) => sr,
                Err(why) => panic!("Failed to process test data: {}", why),
            })
            .collect()
    });

    #[test]
    fn name_searching() {
        let names = [
            "The King's Man",
            "All the King's Men",
            "The Kingsmen",
            "All the King's Men",
            "Kingsman: The Golden Circle",
            "Kingsman: The Secret Service",
            "King's Men",
            "The Kingsmen",
            "All the King's Men",
            "Tian xia di yi",
        ];
        names
            .iter()
            .zip(SEARCH_RESULTS.iter())
            .for_each(|(name, sr)| {
                assert_eq!(sr.name, *name);
            });
    }

    #[test]
    fn id_searching() {
        let ids = [
            "tt6856242",
            "tt0405676",
            "tt2119547",
            "tt0041113",
            "tt4649466",
            "tt2802144",
            "tt0222577",
            "tt14642606",
            "tt0220969",
            "tt0084793",
        ];
        ids.iter().zip(SEARCH_RESULTS.iter()).for_each(|(id, sr)| {
            assert_eq!(sr.id, *id);
        });
    }

    #[test]
    fn genre_searching() {
        let genres = [
            "Movie",
            "Movie",
            "Short",
            "Movie",
            "Movie",
            "Movie",
            "TV Series",
            "TV Episode",
            "TV Movie",
            "Movie",
        ];

        genres
            .iter()
            .zip(SEARCH_RESULTS.iter())
            .for_each(|(genre, sr)| {
                assert_eq!(&sr.genre, *genre);
            });
    }

    #[test]
    fn name_not_found() {
        let fragments = [
            "tt1234 (Movie)",
            "The King's Man tt123124 (TV Episode)",
            "<a href=\"/title/tt6856242/?ref_=fn_tt_tt_1\">The King's Man<a> (2021)",
        ];
        for fragment in fragments.iter() {
            match SearchResult::try_from(*fragment).unwrap_err() {
                RunError::NameNotFound(_) => {}
                e => panic!("Incorrect error type raised: {:?}", e),
            }
        }
    }

    #[test]
    fn id_not_found() {
        let fragments = [
            "The King's Man (Movie)",
            "The King's Man 123124 (TV Episode)",
            "<a href=\"/title/tta6856242/?ref_=fn_tt_tt_1\">The King's Man</a> (2021)",
        ];
        for fragment in fragments.iter() {
            match SearchResult::try_from(*fragment).unwrap_err() {
                RunError::ImdbIdNotFound(_) => {}
                e => panic!("Incorrect error type raised: {:?}", e),
            }
        }
    }
}
