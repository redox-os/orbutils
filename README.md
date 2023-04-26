# OrbUtils

The Orbital Utilities a setup of desktop applications. Compatible with Redox and SDL2 platforms.

[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![crates.io](http://meritbadge.herokuapp.com/orbutils)](https://crates.io/crates/orbutils)

## Cross Platform Support
Some of the applications in this crate can be developed and ran across multiple operating systems
(namely redox-os, linux and macos) thanks to the display being rendered via the `orbclient` crate which in turn uses
`sdl2`

There are three applications that are more fundamental to redox-os, that interact with orbital more directly and are
(currently) deemed to not make sense on other platforms that are running other windowing systems or display managers.
These are namely:
- launcher
- orblogin
- background

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

## How to run Slint ports

For the `Slint` ports `SDL2` is not necessary. On `Redox` the port will run with
[slint_orbclient](https://gitlab.redox-os.org/redox-os/slint_orbclient) backend and on other platforms Slint will 
choose a suitable e.g. `winit` or `qt`. 

To can use the `slint_orbclient` also on other platform you can run for example
`cargo run --bin calculator --no-default-features --features=orbclient`

## Current project status

After the sunset of [OrbTk](https://gitlab.redox-os.org/redox-os/orbtk) the `OrbUtils` will be 
ported [Slint](https://slint-ui.com). 
With this also a new CI pipeline for GitLab will be used.

### Slint ports done

* calculator

## License

The source code of the OrbUtils is available under the terms the MIT license (See [LICENSE-MIT](LICENSE-MIT) for details.)

However, because of the use of GPL dependencies, the OrbUtils, as a whole, is licensed
under the terms of the GPLv3 (See [LICENSE-GPL](LICENSE-GPL))

