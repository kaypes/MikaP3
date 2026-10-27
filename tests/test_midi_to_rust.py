from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import pretty_midi

from scripts import midi_to_rust as converter


ROOT = Path(__file__).resolve().parents[1]


def note(pitch, start, end, velocity=80):
    return pretty_midi.Note(velocity, pitch, start, end)


def instrument(notes=(), controls=(), bends=(), *, drum=False):
    result = pretty_midi.Instrument(0, is_drum=drum)
    result.notes = list(notes)
    result.control_changes = [pretty_midi.ControlChange(*control) for control in controls]
    result.pitch_bends = [pretty_midi.PitchBend(*bend) for bend in bends]
    return result


def midi(*instruments):
    result = pretty_midi.PrettyMIDI()
    result.instruments = list(instruments)
    return result


def sounds(track):
    return [(segment['freq'], segment['vol'], segment['dur']) for segment in track]


class MidiConversionTests(unittest.TestCase):
    def test_low_melody_note_is_not_lost_to_a_global_median(self):
        source = midi(instrument([
            note(36, 0, 4), note(60, 0, 1), note(72, 1, 2),
            note(74, 2, 3), note(76, 3, 4),
        ]))
        high, low = converter.make_tracks(source)
        self.assertEqual([n['freq'] for n in high], [262, 523, 587, 659])
        self.assertEqual(sounds(low), [(65, 80, 4000)])

    def test_monophonic_music_does_not_move_between_buzzers(self):
        source = midi(instrument([note(48, 1, 2), note(84, 2, 3)]))
        high, low = converter.make_tracks(source)
        self.assertEqual(sounds(high), [(0, 0, 1000), (131, 80, 1000), (1047, 80, 1000)])
        self.assertEqual(sounds(low), [(0, 0, 3000)])

    def test_chords_keep_the_highest_and_lowest_simultaneous_pitches(self):
        source = midi(instrument([note(60, 0, 2), note(64, 0, 1)]),
                      instrument([note(72, 0, 1)]))
        high, low = converter.make_tracks(source)
        self.assertEqual(sounds(high), [(523, 80, 1000), (262, 80, 1000)])
        self.assertEqual(sounds(low), [(262, 80, 1000), (0, 0, 1000)])

    def test_unisons_use_the_louder_note_without_doubling(self):
        high, low = converter.make_tracks(midi(
            instrument([note(60, 0, 1, 30)]), instrument([note(60, 0, 1, 90)])))
        self.assertEqual(sounds(high), [(262, 90, 1000)])
        self.assertEqual(sounds(low), [(0, 0, 1000)])

    def test_original_octaves_are_preserved_unless_requested(self):
        self.assertEqual(converter.freq_from_pitch(21), 28)
        self.assertEqual(converter.freq_from_pitch(108), 4186)
        self.assertEqual(converter.freq_from_pitch(21, fold_range=(40, 100)), 110)
        self.assertEqual(converter.freq_from_pitch(108, fold_range=(40, 100)), 2093)
        self.assertEqual(converter.freq_from_pitch(60), 262)
        for limits in [(50, 40), (-1, 100), (0, 128), (40, 45)]:
            with self.subTest(limits=limits), self.assertRaises(ValueError):
                converter.freq_from_pitch(60, fold_range=limits)

    def test_sustain_extends_notes_until_pedal_release(self):
        source = midi(instrument([note(60, 0, 1)], controls=[
            (64, 127, 0.5), (64, 0, 2), (11, 127, 3),
        ]))
        high, low = converter.make_tracks(source)
        self.assertEqual(sounds(high), [(262, 80, 2000), (0, 0, 1000)])
        self.assertEqual(sum(n['dur'] for n in low), 3000)

    def test_pedal_is_independent_for_each_instrument(self):
        source = midi(
            instrument([note(48, 0, 1)], controls=[(64, 127, 0), (64, 0, 2)]),
            instrument([note(72, 0, 1)]),
        )
        high, low = converter.make_tracks(source)
        self.assertEqual(sounds(high), [(523, 80, 1000), (131, 80, 1000)])
        self.assertEqual(sounds(low), [(131, 80, 1000), (0, 0, 1000)])

    def test_volume_and_expression_split_a_sounding_note(self):
        source = midi(instrument([note(69, 0, 1, 100)], controls=[
            (7, 64, 0), (11, 64, 0.5), (11, 0, 0.75),
        ]))
        high, _ = converter.make_tracks(source)
        self.assertEqual(sounds(high), [(440, 50, 500), (440, 25, 250), (0, 0, 250)])

    def test_pitch_bends_split_a_sounding_note(self):
        source = midi(instrument([note(69, 0, 1)], bends=[(4096, 0.5)]))
        high, _ = converter.make_tracks(source)
        self.assertEqual(sounds(high), [(440, 80, 500), (466, 80, 500)])
        high, _ = converter.make_tracks(source, bend_range=12)
        self.assertEqual(high[-1]['freq'], converter.freq_from_pitch(75))

    def test_rpn_pitch_bend_sensitivity_includes_cents_and_null_selection(self):
        source = midi(instrument([note(69, 0, 2)], controls=[
            (101, 0, 0), (100, 0, 0), (6, 12, 0), (38, 50, 0),
            (101, 127, 1), (100, 127, 1), (6, 2, 1),
        ], bends=[(4096, 0.5)]))
        high, _ = converter.make_tracks(source)
        self.assertEqual(sounds(high), [
            (440, 80, 500), (converter.freq_from_pitch(75.25), 80, 1500),
        ])

    def test_same_timestamp_releases_attacks_and_submillisecond_notes(self):
        source = midi(instrument([
            note(60, 0, 1), note(72, 1, 2), note(100, 0.1001, 0.1002),
        ]))
        high, low = converter.make_tracks(source)
        self.assertEqual(sounds(high), [(262, 80, 1000), (523, 80, 1000)])
        self.assertEqual(sounds(low), [(0, 0, 2000)])

    def test_nrpn_data_entry_does_not_change_pitch_bend_sensitivity(self):
        source = midi(instrument([note(69, 0, 1)], controls=[
            (101, 0, 0), (100, 0, 0), (6, 12, 0),
            (99, 1, 0.25), (98, 2, 0.25), (6, 2, 0.25),
        ], bends=[(4096, 0.5)]))
        high, _ = converter.make_tracks(source)
        self.assertEqual(high[-1]['freq'], converter.freq_from_pitch(75))

    def test_a_rearticulated_sustained_note_does_not_return_after_pedal_release(self):
        source = midi(instrument([note(60, 0, 1, 100), note(60, 1.5, 2.5, 40)],
                                 controls=[(64, 127, 0), (64, 0, 2)]))
        high, _ = converter.make_tracks(source)
        self.assertEqual(sounds(high), [(262, 100, 1500), (262, 40, 1000)])

    def test_empty_drum_only_and_invalid_parameters_are_rejected(self):
        for source in [midi(), midi(instrument([note(60, 0, 1)], drum=True))]:
            with self.assertRaises(ValueError):
                converter.make_tracks(source)
        for bend_range in [-1, float('nan'), float('inf')]:
            with self.assertRaises(ValueError):
                converter.make_tracks(midi(instrument([note(60, 0, 1)])), bend_range=bend_range)

    def test_cli_errors_are_nonzero_and_preserve_previous_output(self):
        with tempfile.TemporaryDirectory() as directory:
            source, output = Path(directory) / 'bad.mid', Path(directory) / 'song.rs'
            source.write_bytes(b'not a MIDI')
            output.write_text('previous output')
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/midi_to_rust.py'),
                                     str(source), str(output)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn('[ERR]', result.stderr)
            self.assertEqual(output.read_text(), 'previous output')
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/midi_to_rust.py')],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)

    def test_empty_midi_does_not_replace_previous_output(self):
        with tempfile.TemporaryDirectory() as directory:
            source, output = Path(directory) / 'empty.mid', Path(directory) / 'song.rs'
            midi().write(str(source))
            output.write_text('previous output')
            with self.assertRaises(ValueError):
                converter.convert_midi_to_rust(source, output)
            self.assertEqual(output.read_text(), 'previous output')

    def test_failed_atomic_replace_keeps_old_file_and_removes_temporary_file(self):
        with tempfile.TemporaryDirectory() as directory:
            source, output = Path(directory) / 'song.mid', Path(directory) / 'song.rs'
            midi(instrument([note(69, 0, 1)])).write(str(source))
            output.write_text('previous output')
            with patch.object(converter.os, 'replace', side_effect=OSError('write failed')):
                with self.assertRaises(OSError):
                    converter.convert_midi_to_rust(source, output)
            self.assertEqual(output.read_text(), 'previous output')
            self.assertEqual({p.name for p in Path(directory).iterdir()}, {'song.mid', 'song.rs'})

    def test_input_and_output_must_differ(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'song.mid'
            midi(instrument([note(69, 0, 1)])).write(str(source))
            previous = source.read_bytes()
            with self.assertRaises(ValueError):
                converter.convert_midi_to_rust(source, source)
            self.assertEqual(source.read_bytes(), previous)

    def test_all_project_midis_generate_synchronized_valid_tracks(self):
        with tempfile.TemporaryDirectory() as directory:
            for source in sorted((ROOT / 'assets').glob('*.mid')):
                with self.subTest(source=source.name):
                    tracks = converter.convert_midi_to_rust(source, Path(directory) / f'{source.stem}.rs')
                    durations = [sum(n['dur'] for n in track) for track in tracks]
                    self.assertEqual(durations[0], durations[1])
                    self.assertGreater(durations[0], 0)
                    for track in tracks:
                        for n in track:
                            self.assertTrue(0 <= n['vol'] <= 127)
                            self.assertTrue(0 <= n['freq'] <= 0xffffffff)
                            self.assertGreater(n['dur'], 0)
                            self.assertEqual(n['freq'] == 0, n['vol'] == 0)


if __name__ == '__main__':
    unittest.main()
