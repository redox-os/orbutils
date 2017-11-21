# orbutils
The Orbital Utilities. Compatible with Redox and SDL2 platforms.

[![Travis Build Status](https://travis-ci.org/redox-os/orbutils.svg?branch=master)](https://travis-ci.org/redox-os/orbutils)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![crates.io](http://meritbadge.herokuapp.com/orbutils)](https://crates.io/crates/orbutils)

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
git clone https://github.com/redox-os/orbutils.git
cargo run --bin calculator
```
