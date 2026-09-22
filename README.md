# Wink

![Wink logo](assets/wink_icon.png)

A tiny Windows system-tray utility that keeps your computer awake and prevents
the display from turning off.

Wink has no application window or console. It uses a message-only Windows
window solely to receive tray events.

## Tray menu

- **Stay awake** — prevents system sleep and display timeout.
- **Allow sleep** — restores normal Windows power behavior.
- **Quit** — removes the tray icon and exits.

## Build and run

Install the [Rust toolchain](https://www.rust-lang.org/tools/install), then:

```powershell
cargo build --release
.\target\release\wink.exe
```

The release executable is at `target\release\wink.exe`.

## Development

```powershell
cargo fmt --check
cargo check
```

## License

Wink is licensed under the [MIT License](LICENSE).
