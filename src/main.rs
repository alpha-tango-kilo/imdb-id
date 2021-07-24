use imdb_id::RuntimeConfig;
use imdb_id::{SearchResult, RESULT_SELECTOR, URL_START};
use reqwest::blocking as reqwest;
use scraper::Html;
use std::convert::TryFrom;

fn main() -> anyhow::Result<()> {
    let config = RuntimeConfig::new();

    let html = reqwest::get(format!("{}{}", URL_START, &config.search_term))?.text()?;
    let document = Html::parse_document(&html);
    let search_result_iter = document.select(&RESULT_SELECTOR);

    let links = search_result_iter
        .take(config.number_of_results)
        .map(|er| er.inner_html())
        .map(|html| SearchResult::try_from(html.as_str()))
        .filter_map(|res| match res {
            Ok(sr) => Some(sr),
            Err(why) => {
                eprintln!("{}", why);
                None
            }
        })
        .collect::<Vec<_>>();

    if links.len() == 1 {
        if config.interactive {
            eprintln!("Only one result; {}", links.get(0).unwrap());
        }
        println!("{}", links.get(0).unwrap().id);
    } else {
        // Guaranteed to be interactive
        println!("{:#?}", links);
    }

    Ok(())
}
