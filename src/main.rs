#[macro_use] extern crate rocket;
#[macro_use] extern crate lazy_static;
use rocket::response::Redirect;
use rocket::fs::NamedFile;
use urlencoding::encode;
use std::path::Path;
use std::collections::HashMap;
use std::process::Command;

/// A search engine that we can redirect to
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct SearchEngine<'a> {
    search_url: &'a str,
    suggest_url: &'a str,
}

// search engine declarations
const GOOGLE: SearchEngine = SearchEngine {
    search_url: "https://www.google.com/search?hl=en&q={searchTerms}",
    suggest_url: "https://www.google.com/complete/search?hl=en&client=firefox&q={searchTerms}"
};

const DEFAULT_SUGGEST: &'static str = "https://duckduckgo.com/ac/?q={searchTerms}&type=list";

const DUCKDUCKGO: SearchEngine = SearchEngine {
    search_url: "https://duckduckgo.com/?q={searchTerms}",
    suggest_url: "https://duckduckgo.com/ac/?q={searchTerms}&type=list"
};

const WIKIPEDIA: SearchEngine = SearchEngine {
    search_url: "https://en.wikipedia.org/w/index.php?title=Special:Search&search={searchTerms}",
    suggest_url: "https://en.wikipedia.org/w/api.php?action=opensearch&search={searchTerms}&namespace=0"
};

const NWS: SearchEngine = SearchEngine {
    search_url: "https://forecast.weather.gov/zipcity.php?inputstring={searchTerms}",
    suggest_url: DEFAULT_SUGGEST
};

const CPP: SearchEngine = SearchEngine {
    search_url: "https://en.cppreference.com/mwiki/index.php?search={searchTerms}",
    suggest_url: DEFAULT_SUGGEST
};

const RUST: SearchEngine = SearchEngine {
    search_url: "https://doc.rust-lang.org/std/?search={searchTerms}",
    suggest_url: DEFAULT_SUGGEST
};

const CRATES: SearchEngine = SearchEngine {
    search_url: "https://crates.io/search?q={searchTerms}",
    suggest_url: DEFAULT_SUGGEST
};

lazy_static!{
    static ref SEARCH_ENGINES: HashMap<&'static str, SearchEngine<'static>> = [
        ("g", GOOGLE),
        ("ddg", DUCKDUCKGO),
        ("w", WIKIPEDIA),
        ("nws", NWS),
        ("cpp", CPP),
        ("rust", RUST),
        ("crates", CRATES)
    ].iter().copied().collect();
}

/// get the ssid we're connected to
fn get_ssid() -> Option<String> {
    Some(String::from_utf8(Command::new("iwgetid").arg("-r").output().ok()?.stdout).ok()?)
}

/// get prefered search based on ssid
fn base_engine() -> SearchEngine<'static> {
    let ssid = get_ssid();
    match ssid {
        None => DUCKDUCKGO,
        Some(ssid) => {
            if ssid.contains("BVSD") {
                GOOGLE
            } else {
                DUCKDUCKGO
            }
        }
    }
}

/// return the prefered bang suggester (if available)
/// banged engines don't handle bangs in suggestions well, so use this instead
fn get_bang_suggester() -> Option<SearchEngine<'static>> {
    Some(DUCKDUCKGO)
}

/// select a search engine for use
fn get_engine(query: &str) -> (SearchEngine<'static>, &str) {
    // check for bang command
    if query.len() >= 1 && query.chars().nth(0) == Some('!') {
        // get name before space
        let mut index = 0;
        for c in query.chars() {
            index += 1;
            if c.is_whitespace() {
                break
            }
        }
        let bang = &query[1..index-1];
        // lookup bang
        match SEARCH_ENGINES.get(bang) {
            Some(engine) => {
                if bang.len() + 2 <= query.len() {
                    return (*engine, &query[bang.len() + 2..])
                }
            }
            None => {}
        }
    }

    (base_engine(), query)
}

fn format_url(q: &str, format: &str) -> String {
    format.replace("{searchTerms}", &encode(q).into_owned())
}

// search endpoint
#[get("/search?<q>")]
fn search(q: &str) -> Redirect {
    let (engine, new_query) = get_engine(q);
    Redirect::to(format_url(new_query, engine.search_url))
}

// search suggestion endpoint
#[get("/suggest?<q>")]
fn suggest(q: &str) -> Redirect {
    let (mut engine, new_query) = get_engine(q);
    // if this search is a bang, use the bang suggester if available
    if new_query != q {
        engine = match get_bang_suggester() {
            Some(e) => e,
            None => engine
        };
    }

    Redirect::to(format_url(q, engine.suggest_url))
}

// server index.html + opensearch.xml so that we can be added to browsers
#[get("/opensearch.xml")]
async fn opensearch() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/opensearch.xml")).await.ok()
}

#[get("/")]
async fn index() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/index.html")).await.ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![search, suggest, opensearch, index])
}