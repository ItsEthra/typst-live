# Typst-live
This is a simple utility to watch for changes in your [typst](https://github.com/typst/typst) file and automaticaly
recompile them for live feedback.

# 🚨 Warning
Official `typst` binary has a `--watch` flag I didn't know about at the time of writing this 🥲

## Installation
If you have [rust](https://www.rust-lang.org) setup use the following command:
```
cargo install typst-live
```

## Usage
* Lauch `typst-live` from your terminal:
```
$ ./typst-live <file.typ>
Server is listening on http://0.0.0.0:5599/
```
* Go to `http://0.0.0.0:5599/` in your browser.
* Now edit your `file.typ` and watch changes appear in browser tab.
