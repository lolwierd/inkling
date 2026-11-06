#!/bin/bash
# Levels the score (audio/music.wav) and its few soft moments (audio/fx.wav) into
# audio/mix.wav: integrated loudness -20 LUFS, true peak at or under -3 dBTP, so the AAC in
# the page's film stays under it too. The balance is set in audio.js; here the stems are
# summed as they are, brought to the target by one measured gain (in two passes, since the
# gain is linear and loudness is not quite), and checked. A quiet piano and pad have peaks
# far below the ceiling, so no limiter touches them; if a peak ever came within 0.5 dB of
# it, a 4x-oversampled limiter at -3.5 dBTP would catch only that peak.
# Run after `render.ts audio`.
set -euo pipefail
cd "$(dirname "$0")/audio"
FF=/opt/homebrew/bin/ffmpeg
TARGET=-20
SUM='[0:a][1:a]amix=inputs=2:normalize=0'
loud() { "$FF" -hide_banner -nostats -i "$1" -af ebur128=peak=true -f null - 2>&1 | sed -n '/Summary/,$p'; }
I0=$("$FF" -hide_banner -nostats -i music.wav -i fx.wav -filter_complex "$SUM,ebur128" -f null - 2>&1 | sed -n '/Summary/,$p' | awk '/I:/ {print $2; exit}')
G=$(python3 -c "print(round($TARGET - ($I0), 2))")
for pass in 1 2; do
  "$FF" -hide_banner -loglevel error -y -i music.wav -i fx.wav -filter_complex "$SUM,volume=${G}dB" -c:a pcm_s24le mix.wav
  I1=$(loud mix.wav | awk '/I:/ {print $2; exit}')
  G=$(python3 -c "print(round($G + ($TARGET - ($I1)), 2))")
done
TP=$(loud mix.wav | awk '/Peak:/ {print $2; exit}')
if python3 -c "import sys; sys.exit(0 if $TP > -3.5 else 1)"; then
  echo "true peak $TP dBTP is near the ceiling: limiting at -3.5 dBTP"
  "$FF" -hide_banner -loglevel error -y -i music.wav -i fx.wav -filter_complex \
    "$SUM,volume=${G}dB,aresample=192000,alimiter=limit=0.668:attack=5:release=80:level=false,aresample=48000" -c:a pcm_s24le mix.wav
fi
echo "summed: $I0 LUFS; gain ${G} dB"
loud mix.wav | grep -E "I:|LRA:|Peak:"
