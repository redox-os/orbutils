# OrbUtils
The Orbital Utilities. Compatible with Redox and SDL2 platforms.

[![Build status](https://gitlab.redox-os.org/redox-os/orbutils/badges/master/build.svg)](https://gitlab.redox-os.org/redox-os/orbutils/pipelines)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![crates.io](https://img.shields.io/badge/crates.io-v0.1.10-orange.svg)](https://crates.io/crates/orbtk)

## List of utils

* background: drawing desktop background image
* browser: rudimentary web browser
* calculator: visual calculator application
* character_map: simple font viewer
* file_manager: simple file manager application
* launcher: Redox launcher
* orblogin: Login ui
* viewer: simple image viewer

## Quick setup

To run on Linux/OSX you will need SDL2

Install SDL2 on Debian-based systems
```
sudo apt-get install libsdl2-dev
```

Install on OSX using Homebrew
```
brew install sdl2
```

You will need Rust nightly installed
```
curl https://sh.rustup.rs -sSf | sh
rustup override set nightly
```

Clone and run
```
git clone https://gitlab.redox-os.org/redox-os/orbutils.git
cargo run --bin calculator
```

Run with light theme
```
cargo run --bin calculator --features light-theme
```
