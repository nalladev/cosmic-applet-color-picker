# Eyedropper

An eyedropper applet for the COSMIC™ desktop. Click the eyedropper icon in the panel, select a colour from anywhere on screen, and copy it as hex, RGB, or HSL.

## Screenshot

![Screenshot](https://github.com/nalladev/cosmic-ext-applet-eyedropper/raw/main/resources/screenshot.png)

## Installation

### From a release

Grab the `.deb`, `.rpm`, or tarball for your architecture from the [releases page](https://github.com/nalladev/cosmic-ext-applet-eyedropper/releases/latest).

```sh
# Debian/Ubuntu/Pop!_OS
sudo apt install --reinstall ./cosmic-ext-applet-eyedropper_*.deb

# Fedora
sudo dnf install ./cosmic-ext-applet-eyedropper-*.rpm

# Tarball (installs to ~/.local, no root required)
tar -xzf cosmic-ext-applet-eyedropper-*.tar.gz
cd cosmic-ext-applet-eyedropper
./install.sh
```

Then restart the panel with `pkill cosmic-panel` and add the applet.

### From source

```sh
git clone https://github.com/nalladev/cosmic-ext-applet-eyedropper
cd cosmic-ext-applet-eyedropper
cargo build --release
sudo just install
pkill cosmic-panel
```

Then right-click the panel → **Add Applet** → find **Eyedropper**.

### Dependencies

- Rust (edition 2024, MSRV 1.85+)
- [just](https://github.com/casey/just) (`sudo apt install just`)

## Development

```sh
# Build and run standalone (for testing capture/picker flow)
just run

# Build release
just

# Install locally
sudo just install

# Restart the panel to pick up changes
pkill cosmic-panel

# Check for warnings
just check
```

The applet can be run standalone with `just run` for quick testing. When run from the panel, install it first with `sudo just install` then restart the panel.

## License

MPL-2.0
