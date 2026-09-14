//! Keeps `index.json` true to the modules it points at.
//!
//! ```sh
//! teetotum-index entry <wasm> --url <url> --source <url> [--license <spdx>]  # print an entry
//! teetotum-index check [index.json]    # fetch every module and compare it with its entry
//! ```
//!
//! An entry repeats what the module says about itself -- manifest, key, id, size, hash -- so that
//! an installer can show it before downloading anything. `check` fetches each module, verifies
//! its signature as the firmware does and reports every field that no longer matches.
//!
//! Exits 0 when every entry holds, 1 when one does not, 2 on a usage error.

use std::{collections::HashSet, env, fs, process::ExitCode};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use teetotum_pack::{Error, Manifest, PluginId, Signed, manifest::Rights, verify};

const USAGE: &str =
    "usage: teetotum-index entry <wasm> --url <url> --source <url> [--license <spdx>]
       teetotum-index check [index.json]";

/// The index format this tool reads and writes.
const FORMAT: u32 = 1;

/// The largest module fetched: the whole plugins partition.
const FETCH_LIMIT: u64 = 1 << 20;

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
}

impl Entry {
    fn describe(wasm: &[u8], origin: Origin) -> Result<Self, Error> {
        let manifest = Manifest::read(wasm)?;
        verify(wasm)?;
        let key = Signed::read(wasm)?.key;
        Ok(Self {
            name: manifest.name().into(),
            summary: manifest.summary().into(),
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
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        let value = args.next()?.clone();
        match flag.as_str() {
            "--url" => url = Some(value),
            "--source" => source = Some(value),
            "--license" => license = value,
            _ => return None,
        }
    }
    Some(Origin {
        url: url?,
        source: source?,
        license,
    })
}

fn entry(path: &str, origin: Origin) -> ExitCode {
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
    let wasm = match fetch(&listed.url) {
        Ok(wasm) => wasm,
        Err(e) => return vec![e],
    };
    let origin = Origin {
        url: listed.url.clone(),
        source: listed.source.clone(),
        license: listed.license.clone(),
    };
    let found = match Entry::describe(&wasm, origin) {
        Ok(found) => found,
        Err(e) => return vec![format!("module: {e}")],
    };
    let (Ok(Value::Object(listed)), Ok(Value::Object(found))) =
        (serde_json::to_value(listed), serde_json::to_value(found))
    else {
        return vec!["entry does not serialise to an object".into()];
    };
    found
        .iter()
        .filter(|(field, value)| listed.get(*field) != Some(*value))
        .map(|(field, value)| {
            let was = listed.get(field).unwrap_or(&Value::Null);
            format!("{field} is {was}, the module says {value}")
        })
        .collect()
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
