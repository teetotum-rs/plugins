# {{face_name}}

A face for [teetotum](https://github.com/teetotum-rs/firmware), the firmware for the Waveshare
ESP32-S3 Knob Touch LCD 1.8.

## Build

```sh
rustup target add wasm32v1-none
cargo install teetotum-pack --features cli
sh build.sh
```

`build.sh` builds `{{project-name}}.wasm`, signs it with your key and checks it as the firmware
will. The key is `$TEETOTUM_KEY`, else `~/.config/teetotum/face-key.pem`, made on first use.
**Keep it and back it up:** key and name together are the face's identity, and a build signed
with another key is another face.

## Install

The [plugin guide](https://github.com/teetotum-rs/firmware/blob/main/docs/plugin-development.md)
describes how a face gets onto the device and how to find it there.

## List it

To have the face in the catalogue, add an entry with tags to `index.json` of
[teetotum-rs/plugins](https://github.com/teetotum-rs/plugins); its `CONTRIBUTING.md` shows how.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
