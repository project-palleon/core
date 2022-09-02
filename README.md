# PALLEON CORE

> This is a repository that's part of the Palleon project, which in turn is a part of the SoC 2022.
>
> This project is still very much WIP, so everything is subject to radical change as I have to
> adjust everything to new requirements that come up on the way of developing.
>
> So I am sorry for everyone who has to look at this code. It will get better - I hope...

## How it works

1. starts all plugins defined in config.toml
2. redirect images from the input plugins to the data plugins
3. stores whatever they return
4. simultaneously provide everything to the gui
5. go to step 2

## Installation

1. install rust & cargo (Im using 1.62.0, and I have not tried any prior versions)
2. install all plugins (and their dependencies) and add them to the config.toml
3. build the program, in the best case using production optimizations: `cargo build --release`
4. run the core: `RUST_LOG=debug target/release/core`
5. connect to it using the gui
