# IMDb ID

[![Crates.io](https://img.shields.io/crates/v/imdb-id.svg)](https://crates.io/crates/imdb-id)
[![GPL v3](https://img.shields.io/badge/license-GPL-blue)](./LICENCE)

An easy-to-use commandline tool to easily look up the IMDb ID of a movie given its name

It's built for both easy interactive use and scripting capabilities

## Why does this exist?

Because *certain sites* support searching using these IMDb IDs and it's really useful as an unambiguous identifier

## How do I install it?

You need to have [Rust](https://www.rust-lang.org/) installed on your system, then run the following:

```shell
cargo install imdb-id
```

## Usage

```
USAGE:
    imdb-id [FLAGS] [OPTIONS] [search_term]...

ARGS:
    <search_term>...    The title of the movie/show you're looking for

FLAGS:
    -h, --help               Prints help information
    -n, --non-interactive    Disables interactive features (always picks the first result)
    -V, --version            Prints version information

OPTIONS:
    -r, --results <number_of_results>    The maximum number of results to show from IMDb
```

## Roadmap

1. Proof of concept - done as of v0.1.0!
2. CLI option parsing and non-interactive 'feeling lucky' mode with appropriate output - done as of v0.2.0!
3. Interactive mode - done as v1.0.0!
4. Showing more results if you can't see what you're after
5. Filtering by genre
6. Filtering by year
7. Customising fields shown in search results
8. Use async/tokio properly
9. ???
10. Profit
