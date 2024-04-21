# 0.8.0
- **changed:** Typst-live will attempt to use ephemeral ports when preferred port is not available.
- **changed:** Temp file `output.pdf` has been removed, now changes are written to random file in a temp directory.
- **changed:** Frontend will not try to reconnect when connection has been closed (when `typst-live` is closed)

# 0.7.0
- **added:** Added `-T` or `--no-browser-tab` to disable creating a browser tab when launched.
- **fixed:** Do not delete `output.pdf` when recompilation is disabled.
- **fixed:** Improved reconnectinon on frontend. ([#16])

[#16]: https://github.com/ItsEthra/typst-live/pull/16

# 0.6.1
- **fixed:** Do not exit when failing to open link in browser.
