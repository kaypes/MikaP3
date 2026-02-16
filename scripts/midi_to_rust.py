import pretty_midi
import sys
import math

def freq_from_pitch(pitch):
    while pitch < 40: pitch += 12
    while pitch > 100: pitch -= 12
    return int(440.0 * math.pow(2.0, (pitch - 69.0) / 12.0))

def make_track(notes, is_bass=False):
    events = []
    for n in notes:
        events.append((round(n.start * 1000), 'start', n))
        events.append((round(n.end * 1000), 'end', n))

    # Ordena os eventos no tempo
    events.sort(key=lambda e: (e[0], 0 if e[1] == 'start' else 1))

    raw_slices = []
    active = []
    last_time = 0

    for time, ev_type, note in events:
        if time > last_time:
            dur = time - last_time
            if active:
                # Pega a nota mais grave pro baixo, e a mais aguda pra melodia
                if is_bass:
                    active.sort(key=lambda x: x.pitch)
                else:
                    active.sort(key=lambda x: x.pitch, reverse=True)

                selected = active[0]
                raw_slices.append({
                    'freq': freq_from_pitch(selected.pitch),
                    'vol': selected.velocity,
                    'dur': dur,
                    'ref': selected # Usado para saber se é a mesma nota contínua
                })
            else:
                raw_slices.append({'freq': 0, 'vol': 0, 'dur': dur, 'ref': None})
            last_time = time

        if ev_type == 'start':
            active.append(note)
        else:
            if note in active:
                active.remove(note)

    # O PULO DO GATO: Funde fatias da mesma exata nota (ou silêncios) numa coisa só!
    merged = []
    for s in raw_slices:
        if s['dur'] <= 0: continue
        if not merged:
            merged.append(s)
        else:
            prev = merged[-1]
            if prev['ref'] == s['ref'] or (prev['freq'] == 0 and s['freq'] == 0):
                prev['dur'] += s['dur']
            else:
                merged.append(s)

    return merged

def convert_midi_to_rust(midi_file, rust_file):
    try:
        midi_data = pretty_midi.PrettyMIDI(midi_file)
    except Exception as e:
        print(f"[ERR] falha ao abrir MIDI: {e}")
        return

    all_notes = []
    for inst in midi_data.instruments:
        if not inst.is_drum:
            all_notes.extend(inst.notes)

    if not all_notes: return

    # Encontra o meio da música para separar Melodia e Baixo
    all_notes.sort(key=lambda n: n.pitch)
    median_pitch = all_notes[len(all_notes)//2].pitch

    melody_notes = [n for n in all_notes if n.pitch >= median_pitch]
    bass_notes = [n for n in all_notes if n.pitch < median_pitch]

    if not bass_notes:
        melody_notes = all_notes[len(all_notes)//2:]
        bass_notes = all_notes[:len(all_notes)//2]

    track_a = make_track(melody_notes, is_bass=False)
    track_b = make_track(bass_notes, is_bass=True)

    rust_code = "// Gerado automaticamente\n"
    rust_code += "use crate::buzzer::Note;\n\n"

    # Escreve a Partitura A
    rust_code += "pub const TRACK_A: &[Note] = &[\n"
    for t in track_a:
        rust_code += f"    Note {{ freq: {t['freq']}, vol: {t['vol']}, duration_ms: {t['dur']} }},\n"
    rust_code += "];\n\n"

    # Escreve a Partitura B
    rust_code += "pub const TRACK_B: &[Note] = &[\n"
    for t in track_b:
        rust_code += f"    Note {{ freq: {t['freq']}, vol: {t['vol']}, duration_ms: {t['dur']} }},\n"
    rust_code += "];\n"

    with open(rust_file, "w", encoding="utf-8") as f:
        f.write(rust_code)
    print(f"[OK] Arquivo gerado para Async Rust!")

if __name__ == "__main__":
    args = sys.argv
    if len(args) < 3:
        print("Uso: python midi_to_rust.py <in.mid> <out.rs>")
    else:
        convert_midi_to_rust(args[1], args[2])