use freedesktop_entry_parser::parse_entry;
#[cfg(feature = "search")]
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const TERMINAL: &str = "alacritty -e";

#[derive(Serialize)]
struct Desktop {
    name: String,
    categories: String,
    exec: String,
    icon: String,
}

fn find_data_dirs(path: &str) -> Vec<String> {
    let xdg_data_dirs = env::var("XDG_DATA_DIRS").unwrap_or("/usr/share".to_string());
    let xdg_data_home = env::var("XDG_DATA_HOME")
        .unwrap_or_else(|_| format!("{}/.local/share", env::var("HOME").unwrap()));

    xdg_data_dirs
        .split(':')
        .chain(std::iter::once(xdg_data_home.as_str()))
        .map(|base| format!("{}/{}", base, path))
        .collect()
}

fn find_desktop_files(paths: Vec<String>) -> Vec<PathBuf> {
    paths
        .iter()
        .flat_map(|path| fs::read_dir(path).ok())
        .flat_map(|dir| dir.filter_map(|entry| entry.ok()))
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "desktop"))
        .collect()
}

fn parse_desktop_files(paths: Vec<PathBuf>) -> Vec<Desktop> {
    paths
        .into_iter()
        .filter_map(|path| parse_entry(&path).ok())
        .filter_map(extract_desktop_from_entry)
        .collect()
}

fn extract_desktop_from_entry(entry: freedesktop_entry_parser::Entry) -> Option<Desktop> {
    let section = entry.section("Desktop Entry");
    let name = section.attr("Name")?.to_string();
    let exec = section.attr("Exec")?.to_string();
    let categories = section.attr("Categories").unwrap_or("").to_string();
    let icon = section.attr("Icon").unwrap_or("").to_string();

    if section.attr("NoDisplay") == Some("true") {
        return None;
    }

    let exec = if section.attr("Terminal") == Some("true") {
        format!("{} {}", TERMINAL, exec)
    } else {
        exec
    };

    Some(Desktop {
        name,
        categories,
        exec,
        icon,
    })
}

fn cleanup_exec(exec: String) -> String {
    [
        "%f", "%F", "%g", "%G", "%h", "%H", "%j", "%J", "%m", "%M", "%o", "%O", "%q", "%Q", "%r",
        "%R", "%u", "%U", "%y", "%Y", "%z", "%Z",
    ]
    .iter()
    .fold(exec, |exec, &pair| exec.replace(pair, ""))
}

fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(feature = "search")]
fn filter(input: BTreeMap<String, Desktop>, term: &str) -> Vec<Desktop> {
    let matcher = SkimMatcherV2::default();
    let entries: Vec<(i64, Desktop)> = input
        .into_values()
        .map(|entry| {
            let score = matcher
                .fuzzy(&entry.name, term, false)
                .map(|(score, _)| score);
            (score, entry)
        })
        .filter_map(|(score, entry)| score.map(|s| (s, entry)))
        .collect();

    entries.into_iter().map(|(_, entry)| entry).collect()
}

#[cfg(not(feature = "search"))]
fn filter(input: BTreeMap<String, Desktop>, _term: &str) -> Vec<Desktop> {
    input.into_iter().map(|(_, entry)| entry).collect()
}

fn main() {
    reset_sigpipe();
    let mut args: Vec<String> = env::args().collect();
    let application_dirs = find_data_dirs("applications");

    // let icon_dirs = find_data_dirs("icons");
    // println!("{:?}", icon_dirs);

    let json_out = if let Some(pos) = args.iter().position(|a| a == "--json") {
        args.remove(pos);
        true
    } else {
        false
    };

    let desktop_files = find_desktop_files(application_dirs);
    let entries = parse_desktop_files(desktop_files);

    let dedup_map: BTreeMap<_, _> = entries
        .into_iter()
        .map(|entry| {
            (
                entry.name.clone(),
                Desktop {
                    name: entry.name,
                    exec: cleanup_exec(entry.exec),
                    icon: entry.icon,
                    categories: entry.categories,
                },
            )
        })
        .collect();

    let mut out: Vec<Desktop> = if args.len() == 2 {
        filter(dedup_map, &args[1])
    } else {
        dedup_map.into_values().collect()
    };

    out.sort_by_key(|n| n.name.to_lowercase());

    if json_out {
        println!("{}", serde_json::to_string(&out).unwrap());
        return;
    }

    for entry in out {
        println!("{}\t{}\t{}", entry.name, entry.categories, entry.exec);
    }
}
