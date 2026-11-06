# Design tools

Scripts that made the directions board. Run from this folder.

- `fetch-fonts.sh`: downloads the wordmark faces into `fonts/` (not committed).
- `outline.py`: outlines a word in a font as SVG paths and reports where each i's dot sits.
  `uv run outline.py fonts/literata-72-500.ttf inkling 100`
- `logos.py`, `brand.py`, `brand2.py`: the wordmarks and app icons
  (`uv run brand2.py` writes `../directions/assets/<dir>/`). Tapered strokes: `ribbon`,
  `cubic`, `running_stitch`, `caret`, `knot`, `tick`.
- `index.src.html` + `build_index.py`: the overview page; the build inlines the three
  wordmarks so they follow the page's theme toggle. Edit the source, then
  `python3 build_index.py`.
- `phone.html`: headless Chrome will not make a window narrower than 500 px, so phone
  checks load a page in a 390 px iframe:
  `phone.html?src=<url-encoded file url>&h=844`, run with
  `--allow-file-access-from-files --disable-web-security`. With `qa=1` in the page URL it
  copies the page's layout checks out for `--dump-dom`.
