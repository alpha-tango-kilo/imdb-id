use imdb_id::*;
use std::process;

fn main() {
    if let Err(why) = app() {
        eprintln!("Error: {}", why);
        process::exit(why.error_code());
    }
}

fn app() -> Result<()> {
    let config = RuntimeConfig::new()?;

    let fragments = request_and_scrape(&config.search_term)?;
    let search_results = SearchResult::try_many_lossy(fragments, &config.filters);

    if search_results.len() == 0 {
        return Err(RunError::NoSearchResults);
    } else if search_results.len() == 1 {
        let search_result = search_results.get(0).unwrap();
        if config.interactive {
            eprintln!("Only one result; {}", search_result);
        }
        println!("{}", search_result.id);
    } else {
        // Guaranteed to be interactive
        let mut pager = Pager::new(&search_results, &config);
        let selected = pager.ask()?;
        println!("{}", selected.id);
    }

    Ok(())
}
