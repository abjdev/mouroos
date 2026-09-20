import math
import struct
import subprocess
import os

os.makedirs("samples/mp3", exist_ok=True)

SAMPLE_RATE = 44100

def generate_track(filename, notes, title, artist, album, genre, year="2026"):
    # notes: list of (freq, duration_sec, type)
    # Generate 16-bit stereo PCM
    pcm_bytes = bytearray()
    
    for (freq, dur, instrument) in notes:
        num_samples = int(dur * SAMPLE_RATE)
        for i in range(num_samples):
            t = i / SAMPLE_RATE
            env = 1.0
            # ADSR envelope
            attack = 0.02
            decay = dur - 0.04
            if t < attack:
                env = t / attack
            elif t > decay:
                env = max(0.0, (dur - t) / 0.04)
            
            if instrument == "synth":
                # Sawtooth + sub-oscillator
                val = 0.6 * math.sin(2 * math.pi * freq * t) + 0.3 * math.sin(2 * math.pi * (freq * 0.5) * t) + 0.1 * math.sin(2 * math.pi * (freq * 2) * t)
            elif instrument == "square":
                # 8-bit square wave
                phase = (t * freq) % 1.0
                val = 0.6 if phase < 0.5 else -0.6
            elif instrument == "bell":
                # Bell / chime harmonics with exponential decay
                decay_env = math.exp(-3.0 * t / dur)
                val = (0.6 * math.sin(2 * math.pi * freq * t) + 0.3 * math.sin(2 * math.pi * freq * 2.76 * t) + 0.1 * math.sin(2 * math.pi * freq * 5.4 * t)) * decay_env
            else:
                val = math.sin(2 * math.pi * freq * t)

            sample_val = int(val * env * 16000)
            sample_val = max(-32768, min(32767, sample_val))
            # Stereo left & right
            pcm_bytes.extend(struct.pack('<hh', sample_val, sample_val))

    target_mp3 = f"samples/mp3/{filename}"
    cmd = [
        "ffmpeg", "-y",
        "-f", "s16le",
        "-ar", str(SAMPLE_RATE),
        "-ac", "2",
        "-i", "-",
        "-metadata", f"title={title}",
        "-metadata", f"artist={artist}",
        "-metadata", f"album={album}",
        "-metadata", f"genre={genre}",
        "-metadata", f"year={year}",
        "-c:a", "libmp3lame",
        "-b:a", "96k",
        target_mp3
    ]
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    p.communicate(input=pcm_bytes)
    print(f"Generated {target_mp3} ({os.path.getsize(target_mp3)} bytes)")

# Track 1: Neon Horizons (Synthwave - A minor arpeggiated synth)
# A4=440, C5=523, E5=659, G5=784, A5=880, F4=349, G4=392, D4=293
notes_synth = [
    (440, 0.25, "synth"), (523, 0.25, "synth"), (659, 0.25, "synth"), (880, 0.35, "synth"),
    (784, 0.25, "synth"), (659, 0.25, "synth"), (523, 0.25, "synth"), (440, 0.35, "synth"),
    (349, 0.25, "synth"), (440, 0.25, "synth"), (523, 0.25, "synth"), (659, 0.35, "synth"),
    (392, 0.25, "synth"), (493, 0.25, "synth"), (587, 0.25, "synth"), (784, 0.35, "synth"),
    (440, 0.25, "synth"), (523, 0.25, "synth"), (659, 0.25, "synth"), (880, 0.50, "synth"),
]
generate_track("neon_horizons.mp3", notes_synth, "Neon Horizons", "Mouros Sound Lab", "Mouros Cyberpunk OST", "Synthwave")

# Track 2: Moonlight Echoes (Classical Arpeggios)
notes_classical = [
    (440, 0.30, "bell"), (523, 0.30, "bell"), (659, 0.30, "bell"), (880, 0.60, "bell"),
    (392, 0.30, "bell"), (493, 0.30, "bell"), (587, 0.30, "bell"), (784, 0.60, "bell"),
    (349, 0.30, "bell"), (440, 0.30, "bell"), (523, 0.30, "bell"), (698, 0.60, "bell"),
    (329, 0.30, "bell"), (440, 0.30, "bell"), (523, 0.30, "bell"), (659, 0.80, "bell"),
]
generate_track("moonlight.mp3", notes_classical, "Moonlight Echoes", "Ludwig van Beethoven", "Classical Piano Masterpieces", "Classical")

# Track 3: Pixel Quest (8-bit Chiptune Theme)
notes_chiptune = [
    (523, 0.18, "square"), (587, 0.18, "square"), (659, 0.18, "square"), (784, 0.25, "square"),
    (659, 0.18, "square"), (784, 0.18, "square"), (880, 0.35, "square"), (784, 0.18, "square"),
    (659, 0.18, "square"), (523, 0.18, "square"), (440, 0.25, "square"), (493, 0.25, "square"),
    (523, 0.40, "square"),
]
generate_track("pixel_quest.mp3", notes_chiptune, "Pixel Quest", "Atahan Bahadir", "Mouros Retro Arcade", "Chiptune")
