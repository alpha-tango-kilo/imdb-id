# IMDb ID

[![Crates.io](https://img.shields.io/crates/v/imdb-id.svg)](https://crates.io/crates/imdb-id)
[![GPL v3](https://img.shields.io/badge/license-GPL-blue)](./LICENSE)

An easy-to-use commandline tool to look up the IMDb ID of a movie or show, given its name

It's built for both easy interactive use and scripting capabilities

![Demonstration GIF](./fluff/demo.gif)

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
    imdb-id [FLAGS] [OPTIONS] [--] [search_term]...

ARGS:
    <search_term>...
            The title of the movie/show you're looking for

FLAGS:
    -h, --help
            Prints help information

    -n, --non-interactive
            Disables interactive features (always picks the first result)

    -V, --version
            Prints version information


OPTIONS:
    -g, --genre <filter_genre>...
            Filters results to a specific genre
            Can be given multiple arguments or passed multiple times, working as a chain of OR
            statements logically. Filters are all case insensitive
            It is STRONGLY recommended you quote genres, as most have spaces
            Examples: Movie, "TV episode", "TV series"

    -y, --year <filter_year>
            Filters results to a specific year, or range of years
            Media which has no year specified will always be included
            Ranges are fully inclusive
            Examples: 2021, 1990-2000, 2000- (2000 onwards), -2000 (before 2000)

    -f, --format <format>
            Change output format to desired standard
            Formats are only available if you opted-IN at installation
            All the formats imdb-id can support are: json, yaml

    -r, --results <number_of_results>
            The maximum number of results to show from IMDb
```

### Format support

By default, `imdb-id` only comes with `-f/--format` support for JSON.
To install with all supported formats, use the following command:

```shell
cargo install imdb-id --all-features
```

Or choose the additional formats you want from the below:
* YAML

And run:

```shell
cargo install imdb-id --features "<format_one> <format_two>"
```

If you're reading this now having already installed `imdb-id` with the different feature selection to what you wanted, you need to add the `--force` flag to the install command to let Cargo override your existing installed binary cleanly

If there's a format you'd like to see supported, open up an issue or a pull request and I can see if I can make it happen!

## Roadmap

Version 1:

1. Proof of concept - done as of v0.1.0!
2. CLI option parsing and non-interactive 'feeling lucky' mode with appropriate output - done as of v0.2.0!
3. Interactive mode - done as v1.0.0!
4. Result pagination - done as of v1.0.1!
5. Filtering by genre - done as of v1.0.2! (note: the v1.0.2 release has glaring bugs, please don't use it)
6. Filtering by year - done as of v1.0.3!
7. Different output format support: JSON, YAML, etc. - done as of v1.0.4!
8. ~~Use async/tokio properly~~ - dismissed as unnecessary when the program only makes a single web request. See note in lib.rs for more

---

Version 2:

9. OMDb API migration
10. Spinners! (using [indicatif](https://crates.io/crates/indicatif))
11. OMDb automated sign-up
12. [miette](https://crates.io/crates/miette)?
13. ???
14. Profit
