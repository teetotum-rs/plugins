# Contributing

Thank you for writing a face. There are two ways to share one.

## A face in this repository

1. Generate it into `faces/`: `cargo generate --path template --destination faces`.
2. Make sure it passes what CI runs for every face:

   ```sh
   cargo fmt --check
   cargo clippy --release -- -D warnings
   sh build.sh
   ```

3. Open a pull request. CI signs the module with a key made for that run only, to prove it
   passes the loader's checks; nothing it builds is published.

Faces kept here are released signed with the project key and then listed in the index.

## A face from your own repository

Build and sign it with your own key, publish the signed `.wasm` at a stable https URL -- a
GitHub release asset is the usual place -- and add an entry to `index.json`. Do not write the
entry by hand; the tool reads it from the module:

```sh
cargo run --manifest-path tools/teetotum-index/Cargo.toml -- \
    entry my_face.wasm --url https://.../my_face.wasm --source https://github.com/you/my-face
cargo run --manifest-path tools/teetotum-index/Cargo.toml -- check index.json
```

`--license` defaults to `MIT OR Apache-2.0`. Open a pull request with the new entry.

**An update** replaces the entry: a new version, URL, size and hash, but the same key. A module
signed with another key is another face to the device, with its own id and its own settings.

## What every face keeps to

- **Keep your key and back it up.** It cannot be replaced without losing the face's identity.
- **Name at most 20 bytes, summary at most 32.** The SDK refuses more at compile time.
- **Ask for the rights the face uses, no more.** They are shown on the glass before install.
- **A face that shows what the radio hears nearby carries a note on the law** in its
  `README.md`: receiving and showing radio signals is regulated differently from country to
  country.
- **Identifiers stay harmless.** Nothing in names, comments or strings that sounds like
  surveillance or attack tooling.

## License

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this repository by you, as defined in the Apache-2.0 license, shall be dual licensed as
`MIT OR Apache-2.0`, without any additional terms or conditions.
