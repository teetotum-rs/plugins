# teetotum plugins

Faces -- plugins -- for [teetotum](https://github.com/teetotum-rs/firmware), the firmware for the
Waveshare ESP32-S3 Knob Touch LCD 1.8. A face is a signed WebAssembly module: it gets the taps,
wipes and, if it asks, the knob, and names what the firmware draws.

This repository holds three things:

```
template/               a new face, for cargo generate
faces/                  faces kept here, built and checked by CI
index.json              every face known to the project, with where to get it
tools/teetotum-index/   builds index entries and checks them against their modules
```

## Start a face

```sh
rustup target add wasm32v1-none
cargo install cargo-generate
cargo install teetotum-pack --features cli

cargo generate teetotum-rs/plugins template
cd my-face
sh build.sh
```

`build.sh` builds the module, signs it with your key and checks it as the firmware will. The
[plugin guide](https://github.com/teetotum-rs/firmware/blob/main/docs/plugin-development.md)
explains the SDK, [`teetotum-face`](https://crates.io/crates/teetotum-face), and how a face gets
onto the device.

## The index

`index.json` lists faces by what their modules say about themselves, so that an installer can
show a face before fetching it:

| Field | Meaning |
|---|---|
| `name`, `summary`, `version`, `abi`, `rights` | from the manifest |
| `tags` | optional; short words the author chose, plus `bundled` for a face the firmware ships |
| `id` | the eight bytes the device knows the face by: key and name together |
| `key` | the author's Ed25519 public key; an update must be signed with the same one |
| `size`, `sha256` | of the signed module |
| `url` | where the signed module is fetched from, over https |
| `source`, `license` | where its code is, and under what terms |

CI fetches every module, verifies its signature and fails on any field that no longer matches.
`bundled` follows from the URL, a module in the firmware repository, and no entry can declare it.
The first entries are the faces the firmware ships; their code lives in the firmware repository.

## Contributing

A face can live here or in a repository of its own; either way it can be listed in the index.
[CONTRIBUTING.md](CONTRIBUTING.md) describes both.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
