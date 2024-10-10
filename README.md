# egui-selectable-table

A library for [egui](https://github.com/emilk/egui) to create tables with draggable cell and row selection.

[](https://github.com/user-attachments/assets/88c889fd-9686-4b96-801e-dadb87de3176)

## Features

- Individual cell or full-row selection while dragging
- Auto vertical table scrolling during drag with adjustable parameters
- Sort rows by clicking headers, both ascending and descending
- Customizable rows and header UI
- Built-in select all (Ctrl+A) and copy (Ctrl+C) functionality
- Handles 150k~ rows with no noticeable lag

## Usage

This is still a work in progress and is not available on crates yet.

## Run Demo

The demo is accessible online via [this link](https://therustypickle.github.io/egui-selectable-table/)

- Clone the repository `git clone https://github.com/TheRustyPickle/egui-selectable-table`
- Move into the demo folder `cd egui-selectable-table/demo`

  - To run natively `cargo run --release`

  or

  - To run in wasm locally install the required target with `rustup target add wasm32-unknown-unknown`
  - Install Trunk with `cargo install --locked trunk`
  - `trunk serve` to run and visit `http://127.0.0.1:8080/`

## Contributing

Contributions, issues, and feature requests are welcome! If you'd like to contribute, please open a pull request.

## License

This project is licensed under the MIT License.
