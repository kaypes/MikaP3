#!/usr/bin/env python3

# /// script
# requires-python = ">=3.11"
# dependencies = ["pretty_midi==0.2.11"]
# /// 

"""Reduce a MIDI to two synchronized buzzer voices without changing its octaves."""

import argparse
from dataclasses import dataclass
from itertools import count, groupby
import math
import os
from pathlib import Path
import sys
import tempfile


@dataclass
class InstrumentState:
    volume: int = 127
    expression: int = 127
    sustain: bool = False
    bend: int = 0
    bend_range: float = 2.0
    rpn_msb: int = 127
    rpn_lsb: int = 127
    bend_cents: int = 0


@dataclass
class ActiveNote:
    source: object
    instrument: int
    held: bool = True


def freq_from_pitch(pitch, *, fold_range=None):
    """Round to integer Hz; octave folding is optional and preserves pitch class."""
    if not math.isfinite(pitch):
        raise ValueError("altura de nota inválida")
    if fold_range is not None:
        low, high = fold_range
        if not (0 <= low < high <= 127 and high - low >= 12):
            raise ValueError("a faixa de oitavas deve estar entre 0 e 127 e cobrir uma oitava")
        while pitch < low:
            pitch += 12
        while pitch > high:
            pitch -= 12
    frequency = 440.0 * 2.0 ** ((pitch - 69.0) / 12.0)
    return max(1, round(frequency))


def _milliseconds(time):
    if not math.isfinite(time) or time < 0:
        raise ValueError("tempo MIDI inválido")
    return round(time * 1000)


def _append(track, sound, duration_ms):
    if duration_ms <= 0:
        return
    freq, vol = sound
    if track and (track[-1]["freq"], track[-1]["vol"]) == sound:
        track[-1]["dur"] += duration_ms
    else:
        track.append({"freq": freq, "vol": vol, "dur": duration_ms})


def _voices(active, states, fold_range):
    candidates = []
    for key, playing in active.items():
        state = states[playing.instrument]
        volume = round(playing.source.velocity * state.volume * state.expression / 127**2)
        if volume == 0:
            continue
        pitch = playing.source.pitch + state.bend * state.bend_range / 8192
        frequency = freq_from_pitch(pitch, fold_range=fold_range)
        candidates.append((pitch, volume, key, frequency))
    if not candidates:
        return (0, 0), (0, 0)
    # Prefer the louder note for simultaneous unisons. Do not double a single
    # pitch on both buzzers, including unisons produced by octave folding.
    high = max(candidates)
    remaining = [candidate for candidate in candidates if candidate[3] != high[3]]
    low = min(remaining, key=lambda candidate: (candidate[0], -candidate[1])) if remaining else None
    return (high[3], high[1]), (low[3], low[1]) if low else (0, 0)


def _control(state, control, active, instrument):
    number, value = control.number, control.value
    if number == 7:
        state.volume = value
    elif number == 11:
        state.expression = value
    elif number == 64:
        state.sustain = value >= 64
        if not state.sustain:
            for key, playing in list(active.items()):
                if playing.instrument == instrument and not playing.held:
                    del active[key]
    elif number == 101:
        state.rpn_msb = value
    elif number == 100:
        state.rpn_lsb = value
    elif number in (98, 99):
        # Selecting an NRPN cancels the registered-parameter selection.
        state.rpn_msb = state.rpn_lsb = 127
    elif (state.rpn_msb, state.rpn_lsb) == (0, 0):
        # Registered parameter 0: pitch-bend sensitivity, semitones + cents.
        if number == 6:
            state.bend_range = value + state.bend_cents / 100
        elif number == 38:
            state.bend_range += (value - state.bend_cents) / 100
            state.bend_cents = value


def make_tracks(midi_data, *, fold_range=None, bend_range=2.0):
    """Select simultaneous highest/lowest voices and preserve a common timeline.

    A monophonic passage uses only TRACK_A. With more than two pitches, inner
    voices are discarded. Sustain (CC64), volume (CC7), expression (CC11), pitch
    bend and its RPN 0 sensitivity are applied before choosing the two voices.
    """
    if not math.isfinite(bend_range) or not 0 <= bend_range <= 127:
        raise ValueError("o alcance do pitch bend deve estar entre 0 e 127 semitons")
    # Validate even when the file contains no sounding pitches.
    freq_from_pitch(69, fold_range=fold_range)
    instruments = [instrument for instrument in midi_data.instruments if not instrument.is_drum]
    states = [InstrumentState(bend_range=bend_range) for _ in instruments]
    events = []
    sequence = count()
    note_count = 0
    end_ms = _milliseconds(midi_data.get_end_time())
    for instrument_id, instrument in enumerate(instruments):
        for note in instrument.notes:
            if not (0 <= note.pitch <= 127 and 0 <= note.velocity <= 127):
                raise ValueError("nota MIDI fora dos limites de altura ou intensidade")
            start, end = _milliseconds(note.start), _milliseconds(note.end)
            if note.end <= note.start:
                raise ValueError("nota MIDI com duração inválida")
            if start == end:
                continue  # Shorter than the millisecond resolution of Note.
            note_count += 1
            events.append((start, 2, next(sequence), instrument_id, "start", note))
            events.append((end, 1, next(sequence), instrument_id, "end", note))
        for control in instrument.control_changes:
            events.append((_milliseconds(control.time), 0, next(sequence), instrument_id, "control", control))
        for bend in instrument.pitch_bends:
            events.append((_milliseconds(bend.time), 0, next(sequence), instrument_id, "bend", bend))
    if not note_count:
        raise ValueError("o MIDI não contém notas melódicas com duração de pelo menos 1 ms")
    end_ms = max(end_ms, max(event[0] for event in events))
    events.append((end_ms, 3, next(sequence), 0, "finish", None))
    events.sort(key=lambda event: event[:3])
    tracks = ([], [])
    active = {}
    previous_ms = 0
    for time_ms, group in groupby(events, key=lambda event: event[0]):
        if time_ms > previous_ms:
            for track, sound in zip(tracks, _voices(active, states, fold_range)):
                _append(track, sound, time_ms - previous_ms)
        # Apply every event at this timestamp before rendering the next interval.
        # Controllers precede releases, and releases precede new attacks.
        for _, _, _, instrument_id, kind, value in group:
            if kind == "finish":
                continue
            state = states[instrument_id]
            if kind == "control":
                _control(state, value, active, instrument_id)
            elif kind == "bend":
                state.bend = value.pitch
            elif kind == "start":
                for key, playing in list(active.items()):
                    if (playing.instrument == instrument_id and not playing.held
                            and playing.source.pitch == value.pitch):
                        del active[key]
                active[id(value)] = ActiveNote(value, instrument_id)
            elif id(value) in active:
                if state.sustain:
                    active[id(value)].held = False
                else:
                    del active[id(value)]
        previous_ms = time_ms
    return tracks


def _rust_source(tracks):
    sections = ["// generated by scripts/midi_to_rust.py", "", "use crate::audio::Note;", ""]
    for name, track in zip(("TRACK_A", "TRACK_B"), tracks):
        sections.append(f"pub const {name}: &[Note] = &[")
        for note in track:
            sections.extend([
                "    Note {",
                f"        freq: {note['freq']},",
                f"        vol: {note['vol']},",
                f"        duration_ms: {note['dur']},",
                "    },",
            ])
        sections.extend(["];", ""])
    return "\n".join(sections)


def convert_midi_to_rust(midi_file, rust_file, *, fold_range=None, bend_range=2.0):
    """Convert completely before atomically replacing the output; raise on failure."""
    try:
        import pretty_midi
    except ImportError as error:
        raise ValueError("instale as dependências: python -m pip install -r scripts/requirements.txt") from error
    source, output = Path(midi_file), Path(rust_file)
    if source.resolve() == output.resolve():
        raise ValueError("os arquivos MIDI e Rust de saída devem ser diferentes")
    midi_data = pretty_midi.PrettyMIDI(str(source))
    tracks = make_tracks(midi_data, fold_range=fold_range, bend_range=bend_range)
    rust_code = _rust_source(tracks)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", dir=output.parent,
            prefix=f".{output.name}.", suffix=".tmp", delete=False,
        ) as handle:
            temporary = Path(handle.name)
            handle.write(rust_code)
        # Preserve existing permissions; generated source files are readable.
        temporary.chmod(output.stat().st_mode & 0o777 if output.exists() else 0o644)
        os.replace(temporary, output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
    return tracks


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("midi_file", help="arquivo MIDI de entrada")
    parser.add_argument("rust_file", help="arquivo Rust de saída")
    parser.add_argument("--fold-range", nargs=2, type=int, metavar=("LOW", "HIGH"),
                        help="transpor por oitavas para esta faixa MIDI (ex.: 40 100)")
    parser.add_argument("--bend-range", type=float, default=2.0, metavar="SEMITONS",
                        help="alcance inicial do pitch bend, antes de RPN 0 (padrão: 2)")
    args = parser.parse_args(argv)
    try:
        tracks = convert_midi_to_rust(
            args.midi_file, args.rust_file,
            fold_range=args.fold_range, bend_range=args.bend_range,
        )
    except Exception as error:
        print(f"[ERR] {type(error).__name__}: {error}", file=sys.stderr)
        return 1
    print(f"[OK] {args.rust_file}: {len(tracks[0])} + {len(tracks[1])} notas/pausas")
    return 0


if __name__ == "__main__":
    sys.exit(main())
