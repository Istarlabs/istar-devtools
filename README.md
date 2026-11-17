# Dev Tools

A developer toolbox for the desktop, written in Rust with Slint. It targets Linux with Wayland and works offline.

## Tools

- Encoding and text: Base64, URL, HTML, string escaping, text inspector and case conversion, text diff, Lorem Ipsum, Markdown preview
- Data formats: JSON/YAML/TOML/CSV/XML converter, formatter, JSONPath and XPath, JSON schema validator, SQL formatter
- Security: hashes and checksums, HMAC, password and passphrase generator, JWT, PEM/X.509 certificate decoder
- Identifiers: UUID, ULID, NanoID, Snowflake IDs, random values, QR codes
- Time and scheduling: timestamp converter, cron expressions, systemd and Podman Quadlet unit generator (container, volume, network, pod, timer, backup)
- Numbers and units: calculator, vector calculator, number bases, unit and byte converter, CSS units and contrast
- System and network: chmod notation, CIDR and subnets, system information, hex viewer, gzip/zlib/deflate compression
- Color and image conversion, regex tester

## Build and run

Requires a current stable Rust toolchain.

```
cargo run --release
```

The Slint UI is compiled by `build.rs`. Clipboard access uses the Wayland data-control protocol and file dialogs use the XDG portal, so a Wayland session with a portal is needed.

## Development

```
cargo clippy
cargo test
cargo fmt
```

The Scripts directory holds project helpers from [devscripts](https://github.com/Istarlabs/devscripts), pinned as a submodule: clone
with `git clone --recurse-submodules`, or run `git submodule update --init` in an existing clone. For example
`Scripts/rust.py verify` and `Scripts/rust.py test`.

Source layout:

- `ui/` Slint files; one global per tool in `globals.slint`, one page per tool in `pages/`
- `src/pages/` callbacks, clipboard and file handling for each page
- `src/units/` logic that does not depend on the UI, with unit tests

To add a tool:

```
python3 Scripts/new_tool.py <Name> <module> <slug> "<Menu text>" <LucideIcon> "<Menu group>" ["<search tags>"]
```

This wires the page into the menu (at the end of the given group) and creates compilable skeletons. See `CLAUDE.md` for the details of the UI and Rust bridge.

## License

GPL-3.0-or-later, see [LICENSE.md](LICENSE.md). `patches/parley` is a patched copy of
[parley](https://github.com/linebender/parley) and keeps its own licence (Apache-2.0 or MIT).
