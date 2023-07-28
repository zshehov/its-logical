### Testing locally

Make sure you are using the latest version of stable rust by running `rustup update`.

`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`


### Environment

You need to set the `KNOWLEDGE_PATH` env var to an existing directory.
page.pl and a binary descriptor file will be produced to persist the state produced during the
run of the program. The following runs will load the page.pl file in this directory.
