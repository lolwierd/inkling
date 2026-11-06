#!/bin/bash
# contact.sh [film.mp4] [out.jpg]: 36 moments of the film, 6 x 6, each with its time.
# Frames come from the encoded film itself, so the sheet shows exactly what plays; the sheet
# is set in a page (labels in Figtree) and photographed with headless Chrome. The moments are
# the cut's (7 Nov 2026): the guide, the correction and its fix, each ask, the threads, the
# minds, the montages, the mark. Its work files stay in a fresh folder under $TMPDIR.
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
film=${1:-$here/page/film.mp4}; out=${2:-$here/contact-sheet.jpg}
FF=/opt/homebrew/bin/ffmpeg
CH="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/inkling-contact.XXXXXX")
times=(2.60 7.20 11.75 23.15 33.95 45.50 52.20 54.90 57.22 65.50 74.25 82.25 84.05 92.15 101.25 105.20 108.35 114.35 116.45 128.00 130.90 136.82 140.25 142.45 145.45 151.05 155.50 161.35 165.55 184.62 187.72 189.82 207.20 209.70 216.15 219.30)
cells=""
i=0
for t in "${times[@]}"; do
  f=$(printf %02d $i).png
  $FF -hide_banner -loglevel error -ss "$t" -i "$film" -frames:v 1 -vf scale=480:270:flags=lanczos "$tmp/$f"
  label=$(python3 -c "t=$t; print(f'{int(t//60)}:{int(t%60):02d}')")
  cells="$cells<figure><img src=\"$f\"><figcaption>$label</figcaption></figure>"
  i=$((i+1))
done
cat > "$tmp/sheet.html" <<HTML
<html><head><style>
@font-face { font-family: Figtree; src: url("file://$here/../../assets/fonts/Figtree-SemiBold.ttf"); }
body { margin: 0; background: #18161A; }
main { display: grid; grid-template-columns: repeat(6, 480px); gap: 6px; padding: 6px; width: max-content; }
figure { position: relative; margin: 0; width: 480px; height: 270px; }
img { display: block; width: 480px; height: 270px; }
figcaption { position: absolute; left: 8px; top: 8px; padding: 3px 8px; border-radius: 6px; background: rgba(38,35,43,.78); color: #FBF8F2; font: 600 15px/18px Figtree; }
</style></head><body><main>$cells</main></body></html>
HTML
# (a virtual time budget makes this Chrome hang; the screenshot is written, then Chrome ends itself)
timeout 90 "$CH" --headless --disable-gpu --hide-scrollbars --user-data-dir="$tmp/chrome" --allow-file-access-from-files --force-device-scale-factor=1 \
  --window-size=2922,1662 --screenshot="$tmp/sheet.png" "file://$tmp/sheet.html" >/dev/null 2>&1 || test -s "$tmp/sheet.png"
$FF -hide_banner -loglevel error -y -i "$tmp/sheet.png" -q:v 3 "$out"
echo "$out (work files in $tmp)"
