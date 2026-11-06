#!/bin/bash
# Balances the score (audio/music.wav) and the foley (audio/fx.wav) into audio/mix.wav at
# -16 LUFS integrated, with the true peak at or under -2 dBTP so the AAC in the film stays
# at or under -1.5. The balance is set in audio.js; here the stems are summed as they are,
# lightly tilted (a little less below 150 Hz, a little more above 4.5 kHz: the thumps and
# plucked bass are the heaviest part of the score), brought to the target by one measured gain, and a limiter set below the ceiling catches
# the few stacked clicks (it touches transients only, so the loudness holds). It runs at four
# times the sample rate, so it sees the peaks between samples that bright clicks make.
# Run after `render.ts audio`.
set -euo pipefail
cd "$(dirname "$0")/audio"
FF=/opt/homebrew/bin/ffmpeg
SUM='[0:a][1:a]amix=inputs=2:normalize=0,bass=g=-4:f=150:w=0.7,treble=g=3:f=4500:w=0.7'
loud() { "$FF" -hide_banner -nostats -i "$@" -af ebur128=peak=true -f null - 2>&1 | grep -A 20 Summary; }
I0=$("$FF" -hide_banner -nostats -i music.wav -i fx.wav -filter_complex "$SUM,ebur128" -f null - 2>&1 | grep -A 6 Summary | awk '/I:/ {print $2}')
G=$(python3 -c "print(round(-16 - ($I0), 2))")
for pass in 1 2 3; do
  "$FF" -hide_banner -loglevel error -y -i music.wav -i fx.wav -filter_complex \
    "$SUM,volume=${G}dB,aresample=192000,alimiter=limit=0.68:attack=3:release=60:level=false,aresample=48000" -c:a pcm_s24le mix.wav
  I1=$(loud mix.wav | awk '/I:/ {print $2; exit}')
  G=$(python3 -c "print(round($G + (-16 - ($I1)), 2))")
done
echo "summed: $I0 LUFS; gain ${G} dB"
loud mix.wav | grep -E "I:|LRA:|Peak:"
