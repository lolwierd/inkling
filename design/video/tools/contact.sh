#!/bin/bash
# contact.sh [film.mp4] [out.jpg]: 36 moments of the film, 6 x 6, each with its time.
# Frames come from the encoded film itself, so the sheet shows exactly what plays; the sheet
# is set in a page (labels in Figtree) and photographed with headless Chrome.
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
film=${1:-$here/inkling-walkthrough.mp4}; out=${2:-$here/contact-sheet.jpg}
FF=/opt/homebrew/bin/ffmpeg
CH="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
tmp=$(mktemp -d)
times=(2.6 7.2 11.6 18.5 21.3 23.6 33.5 38.2 44.9 47.4 52.6 58.3 62.4 65.4 69.9 73.6 78.9 83.4 89.6 99.4 101.8 105.5 110 113.2 120.8 128.9 132.5 137.9 141.3 145.5 151.9 156.8 163.3 173.4 187.2 206.6)
cells=""
i=0
for t in "${times[@]}"; do
  f=$(printf %02d $i).png
  $FF -hide_banner -loglevel error -ss "$t" -i "$film" -frames:v 1 -vf scale=480:270 "$tmp/$f"
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
"$CH" --headless=new --disable-gpu --hide-scrollbars --user-data-dir="$tmp/chrome" --allow-file-access-from-files --force-device-scale-factor=1 \
  --window-size=2922,1662 --virtual-time-budget=4000 --screenshot="$tmp/sheet.png" "file://$tmp/sheet.html" >/dev/null 2>&1
$FF -hide_banner -loglevel error -y -i "$tmp/sheet.png" -q:v 3 "$out"
rm -rf "$tmp"
echo "$out"
