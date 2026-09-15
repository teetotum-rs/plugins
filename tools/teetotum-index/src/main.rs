//! Keeps `index.json` true to the modules it points at.
//!
//! ```sh
//! teetotum-index entry <wasm> --url <url> --source <url> [--license <spdx>] [--tag <tag>]...
//! teetotum-index check [index.json]    # fetch every module and compare it with its entry
//! ```
//!
//! An entry repeats what the module says about itself -- manifest, key, id, size, hash -- so that
//! an installer can show it before downloading anything. `check` fetches each module, verifies
//! its signature as the firmware does and reports every field that no longer matches.
//!
//! Tags are the author's, except `bundled`: it follows from the URL and cannot be declared.
//!
//! Exits 0 when every entry holds, 1 when one does not, 2 on a usage error.

use std::{collections::HashSet, env, fs, process::ExitCode};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use teetotum_pack::{Error, Manifest, PluginId, Signed, manifest::Rights, verify};

const USAGE: &str =
    "usage: teetotum-index entry <wasm> --url <url> --source <url> [--license <spdx>] [--tag <tag>]...
       teetotum-index check [index.json]";

/// The index format this tool reads and writes. `tags` is optional, so older readers still hold.
const FORMAT: u32 = 1;

/// The largest module fetched: the whole plugins partition.
const FETCH_LIMIT: u64 = 1 << 20;

/// The tag of a module the firmware ships.
const BUNDLED: &str = "bundled";

/// Where the firmware's own modules are published; only the project can write there.
const FIRMWARE_RAW: &str = "https://raw.githubusercontent.com/teetotum-rs/firmware/";
const FIRMWARE_PLUGINS: &str = "/firmware/assets/plugins/";

/// Tags an author may declare per entry, not counting `bundled`.
const TAGS_MAX: usize = 5;
const TAG_LEN_MAX: usize = 16;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    format: u32,
    plugins: Vec<Entry>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    version: String,
    abi: u16,
    rights: Vec<String>,
    /// Hex of the eight bytes the device knows the face by.
    id: String,
    /// Hex of the author's Ed25519 public key.
    key: String,
    size: usize,
    sha256: String,
    url: String,
    source: String,
    license: String,
}

/// What the module cannot say about itself.
struct Origin {
    url: String,
    source: String,
    license: String,
    /// The author's tags, without `bundled`.
    tags: Vec<String>,
}

impl Entry {
    fn describe(wasm: &[u8], origin: Origin) -> Result<Self, Error> {
        let manifest = Manifest::read(wasm)?;
        verify(wasm)?;
        let key = Signed::read(wasm)?.key;
        let bundled = ships_with_firmware(&origin.url).then(|| BUNDLED.to_string());
        Ok(Self {
            name: manifest.name().into(),
            summary: manifest.summary().into(),
            tags: bundled.into_iter().chain(origin.tags).collect(),
            version: manifest.version().to_string(),
            abi: manifest.abi(),
            rights: names(manifest.rights()),
            id: hex(&PluginId::new(key, manifest.name()).bytes()),
            key: hex(key),
            size: wasm.len(),
            sha256: hex(&Sha256::digest(wasm)),
            url: origin.url,
            source: origin.source,
            license: origin.license,
        })
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [command, wasm, rest @ ..] if command == "entry" => match origin(rest) {
            Some(origin) => entry(wasm, origin),
            None => usage(),
        },
        [command] if command == "check" => check("index.json"),
        [command, path] if command == "check" => check(path),
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

fn origin(args: &[String]) -> Option<Origin> {
    let (mut url, mut source) = (None, None);
    let mut license = String::from("MIT OR Apache-2.0");
    let mut tags = Vec::new();
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        let value = args.next()?.clone();
        match flag.as_str() {
            "--url" => url = Some(value),
            "--source" => source = Some(value),
            "--license" => license = value,
            "--tag" => tags.push(value),
            _ => return None,
        }
    }
    Some(Origin {
        url: url?,
        source: source?,
        license,
        tags,
    })
}

fn entry(path: &str, origin: Origin) -> ExitCode {
    let mut problems = tag_problems(&origin.tags);
    if origin.tags.iter().any(|tag| tag == BUNDLED) {
        problems.push(format!(
            "{BUNDLED} follows from the url and cannot be declared"
        ));
    }
    if !problems.is_empty() {
        return fail(path, &problems.join("; "));
    }
    let described = fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|wasm| Entry::describe(&wasm, origin).map_err(|e| e.to_string()));
    match described {
        Ok(entry) => match serde_json::to_string_pretty(&entry) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => fail(path, &e.to_string()),
        },
        Err(e) => fail(path, &e),
    }
}

fn check(path: &str) -> ExitCode {
    let index: Index = match fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
    {
        Ok(index) => index,
        Err(e) => return fail(path, &e),
    };
    if index.format != FORMAT {
        return fail(
            path,
            &format!("format {}, this tool reads {FORMAT}", index.format),
        );
    }
    let mut ids = HashSet::new();
    let mut passed = true;
    for listed in &index.plugins {
        let mut problems = problems(listed);
        if !ids.insert(listed.id.as_str()) {
            problems.push(format!("id {} is listed twice", listed.id));
        }
        if problems.is_empty() {
            println!("{} {}: holds", listed.name, listed.version);
        } else {
            passed = false;
            for problem in problems {
                println!("{} {}: {problem}", listed.name, listed.version);
            }
        }
    }
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Every way `listed` differs from the module behind its URL.
fn problems(listed: &Entry) -> Vec<String> {
    if listed.name.is_empty() || listed.source.is_empty() || listed.license.is_empty() {
        return vec!["name, source and license must not be empty".into()];
    }
    let mut problems = tag_problems(&listed.tags);
    let declared = listed.tags.iter().any(|tag| tag == BUNDLED);
    match (declared, ships_with_firmware(&listed.url)) {
        (true, false) => problems.push(format!(
            "tagged {BUNDLED}, but its url is not among the firmware's modules"
        )),
        (false, true) => problems.push(format!(
            "not tagged {BUNDLED}, but its url is among the firmware's modules"
        )),
        _ => {}
    }
    let wasm = match fetch(&listed.url) {
        Ok(wasm) => wasm,
        Err(e) => {
            problems.push(e);
            return problems;
        }
    };
    let origin = Origin {
        url: listed.url.clone(),
        source: listed.source.clone(),
        license: listed.license.clone(),
        tags: Vec::new(),
    };
    let found = match Entry::describe(&wasm, origin) {
        Ok(found) => found,
        Err(e) => {
            problems.push(format!("module: {e}"));
            return problems;
        }
    };
    let (Ok(Value::Object(listed)), Ok(Value::Object(found))) =
        (serde_json::to_value(listed), serde_json::to_value(found))
    else {
        problems.push("entry does not serialise to an object".into());
        return problems;
    };
    problems.extend(
        found
            .iter()
            .filter(|(field, value)| *field != "tags" && listed.get(*field) != Some(*value))
            .map(|(field, value)| {
                let was = listed.get(field).unwrap_or(&Value::Null);
                format!("{field} is {was}, the module says {value}")
            }),
    );
    problems
}

/// Whether `url` points at a module the firmware ships.
fn ships_with_firmware(url: &str) -> bool {
    url.strip_prefix(FIRMWARE_RAW)
        .is_some_and(|path| path.contains(FIRMWARE_PLUGINS) && !path.contains(".."))
}

/// Every way `tags` break the rules: short lowercase words, few, each once.
fn tag_problems(tags: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    let declared = tags.iter().filter(|tag| *tag != BUNDLED).count();
    if declared > TAGS_MAX {
        problems.push(format!("{declared} tags, at most {TAGS_MAX}"));
    }
    let mut seen = HashSet::new();
    for tag in tags {
        let word = tag.starts_with(|c: char| c.is_ascii_lowercase())
            && tag
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !word || tag.len() > TAG_LEN_MAX {
            problems.push(format!(
                "tag {tag:?}: a lowercase word of at most {TAG_LEN_MAX} letters, digits and dashes"
            ));
        }
        if !seen.insert(tag) {
            problems.push(format!("tag {tag:?} is listed twice"));
        }
    }
    problems
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    if !url.starts_with("https://") {
        return Err(format!("{url}: not https"));
    }
    ureq::get(url)
        .call()
        .and_then(|mut response| {
            response
                .body_mut()
                .with_config()
                .limit(FETCH_LIMIT)
                .read_to_vec()
        })
        .map_err(|e| format!("{url}: {e}"))
}

fn names(rights: Rights) -> Vec<String> {
    [
        (Rights::HID, "HID"),
        (Rights::KNOB, "KNOB"),
        (Rights::RANDOM, "RANDOM"),
        (Rights::RADIO, "RADIO"),
        (Rights::HAPTIC, "HAPTIC"),
    ]
    .into_iter()
    .filter(|&(right, _)| rights.contains(right))
    .map(|(_, name)| name.into())
    .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn fail(path: &str, message: &str) -> ExitCode {
    eprintln!("teetotum-index: {path}: {message}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn firmware_modules_are_bundled() {
        assert!(ships_with_firmware(
            "https://raw.githubusercontent.com/teetotum-rs/firmware/v0.1.0/firmware/assets/plugins/nearby.wasm"
        ));
        assert!(!ships_with_firmware(
            "https://raw.githubusercontent.com/someone/firmware/v0.1.0/firmware/assets/plugins/nearby.wasm"
        ));
        assert!(!ships_with_firmware(
            "https://raw.githubusercontent.com/teetotum-rs/firmware/main/web/x.wasm"
        ));
        assert!(!ships_with_firmware(
            "https://example.com/teetotum-rs/firmware/firmware/assets/plugins/x.wasm"
        ));
    }

    #[test]
    fn good_tags_pass() {
        assert!(tag_problems(&tags(&["bundled", "media", "remote-2"])).is_empty());
        assert!(tag_problems(&[]).is_empty());
    }

    #[test]
    fn bad_tags_are_named() {
        for bad in ["Media", "2d", "", "a b", "media!", "abcdefghijklmnopq"] {
            assert_eq!(tag_problems(&tags(&[bad])).len(), 1, "{bad:?}");
        }
        assert_eq!(tag_problems(&tags(&["game", "game"])).len(), 1);
        assert_eq!(
            tag_problems(&tags(&["a", "b", "c", "d", "e", "f"])).len(),
            1
        );
        assert!(tag_problems(&tags(&["bundled", "a", "b", "c", "d", "e"])).is_empty());
    }
}
