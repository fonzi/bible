"""Render the opt-in startup sounds: the opening notes of public-domain hymn
tunes, synthesised here (no recordings), each 2 seconds or less."""
import numpy as np, subprocess, sys, os
SR = 44100
def hz(n):  # 'G4' -> frequency
    names = {'C':-9,'C#':-8,'D':-7,'D#':-6,'E':-5,'F':-4,'F#':-3,'G':-2,'G#':-1,'A':0,'A#':1,'B':2}
    return 440 * 2 ** ((names[n[:-1]] + 12 * (int(n[-1]) - 4)) / 12)

def organ(f, d):
    t = np.arange(int(SR * d)) / SR
    vib = 1 + 0.002 * np.sin(2 * np.pi * 5.5 * t)
    w = sum(a * np.sin(2 * np.pi * f * k * vib * t) for k, a in [(1, 1), (2, .5), (3, .25), (4, .18), (6, .08), (8, .05)])
    env = np.minimum(1, t / 0.03) * np.minimum(1, (d - t) / 0.05).clip(0, 1)
    return w * env

def bell(f, d):
    t = np.arange(int(SR * d)) / SR
    parts = [(0.5, 1, 1.2), (1, 1, 1.6), (1.19, .5, 2), (1.5, .4, 2.2), (2, .45, 2.6), (2.74, .25, 3.5), (3, .2, 4), (4.07, .12, 5)]
    return sum(a * np.sin(2 * np.pi * f * r * t) * np.exp(-t * k) for r, a, k in parts) * np.minimum(1, t / 0.004)

def harp(f, d):
    t = np.arange(int(SR * d)) / SR
    w = sum(a * np.sin(2 * np.pi * f * k * t) * np.exp(-t * (2.5 + k)) for k, a in [(1, 1), (2, .45), (3, .2), (4, .1), (5, .05)])
    return w * np.minimum(1, t / 0.003)

def render(notes, voice, total, ring=0.0):
    """notes: (start beat, length beats, [pitches]); beats spread over total."""
    beats = max(s + l for s, l, _ in notes)
    out = np.zeros(int(SR * total) + 1)
    sec = (total - ring) / beats
    for s, l, ps in notes:
        dur = l * sec + (ring if voice is not organ else 0)
        for i, p in enumerate(ps):
            w = voice(hz(p), min(dur, total - s * sec)) * (1 if i == 0 else 0.45)
            a = int(s * sec * SR)
            out[a:a + len(w)] += w[:len(out) - a]
    t = np.arange(len(out)) / SR
    out *= np.minimum(1, (total - t) / 0.25).clip(0, 1)  # fade out
    return out / np.abs(out).max() * 0.6

SOUNDS = {
    # Old Hundredth (Genevan Psalter, 1551): "Praise God from whom all blessings flow"
    "old-hundredth": (render([(0, 1, ['G4', 'G3', 'B3']), (1, 1, ['G4', 'B3', 'D3']), (2, 1, ['F#4', 'D3', 'A3']), (3, 1, ['E4', 'C3', 'G3']), (4, 2, ['D4', 'G2', 'B3'])], organ, 2.0)),
    # New Britain (1829), "Amazing Grace"
    "amazing-grace": render([(0, 1, ['D4']), (1, 2, ['G4', 'G3']), (3, .5, ['B4']), (3.5, .5, ['G4']), (4, 2, ['B4', 'D3']), (6, 1, ['A4']), (7, 2, ['G4', 'G2'])], harp, 2.0, ring=0.6),
    # Nicaea (Dykes, 1861), "Holy, holy, holy"
    "holy-holy-holy": render([(0, 1, ['D5']), (1, 1, ['D5']), (2, 1, ['F#5']), (3, 1, ['F#5']), (4, 2, ['A5', 'D5'])], bell, 2.0, ring=0.8),
    # a single bell chord
    "bell": render([(0, 1, ['G4', 'D5', 'B4'])], bell, 1.8, ring=1.2),
}
dest = sys.argv[1]
for name, wave in SOUNDS.items():
    pcm = (wave * 32767).astype('<i2').tobytes()
    subprocess.run(['ffmpeg', '-y', '-loglevel', 'error', '-f', 's16le', '-ar', str(SR), '-ac', '1', '-i', '-',
                    '-c:a', 'libvorbis', '-q:a', '4', os.path.join(dest, name + '.ogg')], input=pcm, check=True)
    print(name, len(wave) / SR, 's')
