#!/bin/bash
# Mixes the score (audio/music.wav) and the foley (audio/fx.wav) into audio/mix.wav:
# the music ducks under the effects, the lamp-out fades both, and the whole is brought to
# -16 LUFS with a -3 dBTP ceiling (AAC overshoots sharp clicks by a dB or so) (two-pass loudnorm). Run after `render.ts audio`.
set -euo pipefail
cd "$(dirname "$0")/audio"
FF=/opt/homebrew/bin/ffmpeg
PRE='[0:a]volume=-7dB[m];[1:a]volume=+5dB,asplit=2[fx][key];[m][key]sidechaincompress=threshold=0.035:ratio=2.5:attack=6:release=300:makeup=1[md];[md][fx]amix=inputs=2:normalize=0,afade=t=out:st=96.9:d=1.1'
# pass 1: measure
M=$("$FF" -hide_banner -nostats -i music.wav -i fx.wav -filter_complex "$PRE,loudnorm=I=-16:TP=-3:LRA=11:print_format=json" -f null - 2>&1 | sed -n '/^{/,/^}/p')
get() { echo "$M" | python3 -c "import json,sys; print(json.load(sys.stdin)['$1'])"; }
# pass 2: apply, linearly, so the dynamics stay as mixed
"$FF" -hide_banner -loglevel error -y -i music.wav -i fx.wav -filter_complex \
  "$PRE,loudnorm=I=-16:TP=-3:LRA=11:measured_I=$(get input_i):measured_TP=$(get input_tp):measured_LRA=$(get input_lra):measured_thresh=$(get input_thresh):offset=$(get target_offset):linear=true,aresample=48000" \
  -c:a pcm_s24le mix.wav
"$FF" -hide_banner -nostats -i mix.wav -af ebur128=peak=true -f null - 2>&1 | grep -A 12 Summary | grep -E "I:|Peak:|LRA:"
