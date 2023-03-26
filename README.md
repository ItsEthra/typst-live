# Typst-live
This is a simple utility to watch for changes in your [typst](https://github.com/typst/typst) file and automaticaly
recompile them for live feedback.

## Installation
If you have [rust](https://www.rust-lang.org) setup use the following command:
```
cargo install typst-live
```

## Usage
* 1. Lauch `typst-live` from your terminal:
```
$ ./typst-live <file.typ>
Server is listening on http://0.0.0.0:5599/
```
* 2. Go to `http://0.0.0.0:5599/` in your browser.
* 3. Now edit your `file.typ` and watch changes appear in browser tab.
