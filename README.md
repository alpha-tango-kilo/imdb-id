# IMDb ID

[![Codeberg CI](https://ci.codeberg.org/api/badges/alpha-tango-kilo/imdb-id/status.svg)](https://ci.codeberg.org/alpha-tango-kilo/imdb-id)
[![Crates.io](https://img.shields.io/crates/v/imdb-id.svg)](https://crates.io/crates/imdb-id)
[![Dependencies](https://deps.rs/repo/codeberg/alpha-tango-kilo/imdb-id/status.svg)](https://deps.rs/repo/codeberg/alpha-tango-kilo/imdb-id)

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

### How do I update it?

If a new version has come out since you installed, you can run

```shell
cargo install --force imdb-id
```

To update the program.
For a nicer solution to updating any Rust programs you have installed through Cargo, check out [cargo-update](https://github.com/nabijaczleweli/cargo-update)

## Usage

```
USAGE:
    imdb-id [OPTIONS] [search_term]...

ARGS:
    <search_term>...
            The title of the movie/show you're looking for

OPTIONS:
        --api-key <api_key>
            Your OMDb API key (overrides saved value if present)

    -f, --format <format>
            Change output format to desired standard
            Formats are only available if you opted-IN at installation
            All the formats imdb-id can support are: json, yaml

    -h, --help
            Print help information

    -n, --non-interactive
            Disables interactive features (always picks the first result)

    -u, --print-url
            Print the full IMDb URL instead of just the ID

    -r, --results <number_of_results>
            The maximum number of results to show from IMDb

    -t, --type <filter_type>
            Filters results to a specific media type (movie or series). Can be given multiple times

    -V, --version
            Print version information

    -y, --year <filter_year>
            Filters results to a specific year, or range of years
            Media which has no year specified will always be included
            Ranges are fully inclusive
            Examples: 2021, 1990-2000, 2000- (2000 onwards), -2000 (before 2000)

ENVIRONMENT VARIABLES:
    IMDB_ID_MAX_REQUESTS_PER_SEARCH
            Adjusts the limit on the number of requests per search. Default is 10
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

Version 2:

9. OMDb API migration - done as of v2.0.0!
10. Apply filtering at a request level (will result in more results being shown) - done as of v2.0.5!
11. OMDb automated sign-up - done as of v2.1.0!
12. Show extra information about media when selecting them - done as of v3.0.0!

---

Version 3:

13. Bring back pagination
14. ???
15. Profit!
