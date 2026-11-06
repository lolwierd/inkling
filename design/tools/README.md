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
- `logo_directions.py`: thirteen logo directions in the Loose Threads language, drawn from
  the app's own stitch geometry and palettes, with letters outlined from
  `assets/fonts/FrauncesDisplay-Medium.ttf`. `uv run logo_directions.py` writes
  `../logo/<nn-name>/{wordmark,mark,icon,icon-dark}.svg`, then the review page
  `../logo/index.html` through `logo_page.py` and its template `logo_page.src.html`.
- `logo_round2.py` + `logo_page_r2.py`: round 2 of the logo review (the darn, the dot on
  the i, Tied off, Sampler, One thread in the app). The same command writes
  `../logo/round-2/<name>/` and `../logo/round-2/dots/`, and puts round 2 at the top of the
  page with round 1 folded below.
- `phone.html`: headless Chrome will not make a window narrower than 500 px, so phone
  checks load a page in a 390 px iframe:
  `phone.html?src=<url-encoded file url>&h=844`, run with
  `--allow-file-access-from-files --disable-web-security`. With `qa=1` in the page URL it
  copies the page's layout checks out for `--dump-dom`.
