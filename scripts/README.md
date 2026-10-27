# Conversão MIDI para os buzzers

Para operar o player e os jogos na placa, consulte os
[controles do MikaP3](../README.md#controles).

Na raiz do projeto, prepare o ambiente Python:

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -r scripts/requirements.txt
```

Converta um arquivo:

```bash
python scripts/midi_to_rust.py assets/tetris.mid /tmp/tetris.rs
```

O conversor preserva as oitavas originais e arredonda frequências para Hz
inteiros, conforme o formato `Note`. Para adaptar notas à faixa antiga:

```bash
python scripts/midi_to_rust.py assets/tetris.mid /tmp/tetris.rs --fold-range 40 100
```

Em cada instante, `TRACK_A` recebe a nota mais aguda e `TRACK_B` a mais grave.
Passagens com uma única altura usam apenas `TRACK_A`. Acordes com mais de duas
alturas perdem as vozes intermediárias. Percussão é ignorada.

São considerados sustain (CC64), volume (CC7), expressão (CC11), pitch bend e
sua sensibilidade por RPN 0. O alcance inicial do bend é de dois semitons;
`--bend-range 12` altera esse valor para arquivos que não enviam RPN 0.
Os controles são aplicados por instrumento. Outros efeitos MIDI e timbres não
são sintetizados. A normalização de volume continua a cargo do reprodutor Rust.

Os tempos são quantizados em milissegundos. Notas que desaparecem nessa
quantização são ignoradas. As duas faixas preservam o silêncio inicial e
terminam juntas, no fim dos eventos do MIDI.

Erros de argumentos retornam código 2; erros de conversão retornam código 1.
O destino só é substituído depois que a conversão e a escrita terminam.

Execute os testes, incluindo os MIDIs em `assets/`:

```bash
python -m unittest discover -s tests -p 'test_midi_to_rust.py' -v
```
