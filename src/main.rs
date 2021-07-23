use reqwest::blocking as reqwest;
use scraper::{Html, Selector};
use std::env;
use imdb_id::SearchResult;
use std::convert::TryFrom;

const URL_START: &str = "https://www.imdb.com/find?s=tt&q=";

fn main() -> anyhow::Result<()> {
    let search_term = env::args().skip(1).collect::<String>();
    eprintln!("Args: {:?}", search_term);

    let html = reqwest::get(format!("{}{}", URL_START, search_term))?.text()?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("td.result_text").unwrap();
    let link_selector = document.select(&selector);

    let links = link_selector
        .take(10)
        .map(|er| er.inner_html())
        .map(|html| SearchResult::try_from(html.as_str()))
        .filter_map(|res| match res {
            Ok(sr) => Some(sr),
            Err(why) => { eprintln!("{}", why); None }
        })
        .collect::<Vec<_>>();
    println!("{:#?}", links);

    Ok(())
}
