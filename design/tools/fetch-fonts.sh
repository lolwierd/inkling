#!/bin/sh
# Static instances of the wordmark faces, for outline.py. Google Fonts serves TTF to non-browser agents.
set -e
cd "$(dirname "$0")" && mkdir -p fonts && cd fonts
fetch() { url=$(curl -s "https://fonts.googleapis.com/css2?family=$1" | grep -o 'https://fonts.gstatic.com[^)]*' | head -1); curl -s -o "$2" "$url"; echo "$2"; }
fetch "Literata:opsz,wght@72,500" literata-72-500.ttf
fetch "Literata:opsz,wght@72,400" literata-72-400.ttf
fetch "Atkinson+Hyperlegible+Next:wght@700" atkinson-next-700.ttf
fetch "B612:wght@700" b612-700.ttf
fetch "Fraunces:opsz,wght,SOFT,WONK@144,500,100,0" fraunces-144-500-soft.ttf
fetch "Fraunces:opsz,wght,SOFT,WONK@144,400,100,0" fraunces-144-400-soft.ttf
