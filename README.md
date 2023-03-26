# Typst-live
This is a simple utility to watch for changes in your [typst](https://github.com/typst/typst) file and automaticaly
recompile them for live feedback.

## Difference from `--watch` flag
`typst-live` hosts a webserver that automatically refreshes the page so you don't have to manually reload it with `typst --watch`

## Installation
If you have [rust](https://www.rust-lang.org) setup use the following command:
```
cargo install typst-live
```

## Usage
* Lauch `typst-live` from your terminal:
```
$ ./typst-live <file.typ>
Server is listening on http://127.0.0.1:5599/
```
* Go to `http://127.0.0.1:5599/` in your browser.
* Now edit your `file.typ` and watch changes appear in browser tab.
