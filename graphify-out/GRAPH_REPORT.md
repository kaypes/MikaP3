# Graph Report - MikaP3  (2026-10-26)

## Corpus Check
- 41 files · ~53,765 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 15 file(s) not represented in the graph (top: .mid 11, (none) 2, .toml 1)

## Summary
- 169 nodes · 237 edges · 12 communities (7 shown, 5 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Inicialização e tarefas assíncronas
- Notas e faixas musicais
- Sprites da matriz LED
- Animações e controle LED
- Estado global e Flappy
- Interface do display OLED
- Lógica e renderização Snake
- Conversão MIDI para Rust
- Configuração do pacote Rust

## God Nodes (most connected - your core abstractions)
1. `Note` - 36 edges
2. `AppState` - 14 edges
3. `SnakeGame` - 11 edges
4. `FlappyGame` - 10 edges
5. `STATE` - 8 edges
6. `leds_task()` - 7 edges
7. `input_task()` - 5 edges
8. `ANIMATION` - 4 edges
9. `display_task()` - 4 edges
10. `make_track()` - 3 edges

## Surprising Connections (you probably didn't know these)
- `TRACK_A` --references--> `Note`  [EXTRACTED]
  src/songs/always_with_me.rs → src/buzzer.rs
- `TRACK_B` --references--> `Note`  [EXTRACTED]
  src/songs/always_with_me.rs → src/buzzer.rs
- `TRACK_A` --references--> `Note`  [EXTRACTED]
  src/songs/fallen_down.rs → src/buzzer.rs
- `TRACK_B` --references--> `Note`  [EXTRACTED]
  src/songs/fallen_down.rs → src/buzzer.rs
- `TRACK_A` --references--> `Note`  [EXTRACTED]
  src/songs/game_of_thrones.rs → src/buzzer.rs

## Import Cycles
- None detected.

## Communities (12 total, 5 thin omitted)

### Community 0 - "Inicialização e tarefas assíncronas"
Cohesion: 0.08
Nodes (4): play_track_a(), play_track_b(), input_task(), main()

### Community 1 - "Notas e faixas musicais"
Cohesion: 0.10
Nodes (23): Note, TRACK_A, TRACK_B, TRACK_A, TRACK_B, TRACK_A, TRACK_B, TRACK_A (+15 more)

### Community 2 - "Sprites da matriz LED"
Cohesion: 0.09
Nodes (11): COELHO, CORACAO, ESPADA, ESTRELA, FANSTASMA, FLOR, FOGUETE, FORMIGA (+3 more)

### Community 3 - "Animações e controle LED"
Cohesion: 0.13
Nodes (6): ANIMATION, LED_MAP, ARTS, leds_task(), parse_art(), parse_frame()

### Community 4 - "Estado global e Flappy"
Cohesion: 0.15
Nodes (10): FlappyGame, draw_frame(), AppState, EasterEgg, FlappyBird, GamesMenu, Menu, Playing (+2 more)

### Community 5 - "Interface do display OLED"
Cohesion: 0.13
Nodes (3): ART_NAMES, display_task(), SONG_NAMES

### Community 7 - "Conversão MIDI para Rust"
Cohesion: 0.53
Nodes (3): convert_midi_to_rust(), freq_from_pitch(), make_track()

## Knowledge Gaps
- **7 isolated node(s):** `MikaP3`, `Menu`, `Playing`, `GamesMenu`, `Snake` (+2 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 66 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **5 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Note` connect `Notas e faixas musicais` to `Inicialização e tarefas assíncronas`?**
  _High betweenness centrality (0.330) - this node is a cross-community bridge._
- **What connects `MikaP3`, `Menu`, `Playing` to the rest of the system?**
  _7 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Inicialização e tarefas assíncronas` be split into smaller, more focused modules?**
  _Cohesion score 0.07899159663865546 - nodes in this community are weakly interconnected._
- **Why does `AppState` connect `Estado global e Flappy` to `Inicialização e tarefas assíncronas`, `Animações e controle LED`, `Interface do display OLED`, `Lógica e renderização Snake`?**
  _High betweenness centrality (0.270) - this node is a cross-community bridge._
- **Should `Notas e faixas musicais` be split into smaller, more focused modules?**
  _Cohesion score 0.09803921568627451 - nodes in this community are weakly interconnected._
- **Why does `ARTS` connect `Animações e controle LED` to `Sprites da matriz LED`?**
  _High betweenness centrality (0.224) - this node is a cross-community bridge._
- **Should `Sprites da matriz LED` be split into smaller, more focused modules?**
  _Cohesion score 0.08695652173913043 - nodes in this community are weakly interconnected._
## Extraction integrity

The read-only diagnostic found 76 edges whose endpoints are absent from the raw node list, 6 exact duplicate edges, and 5 edges sharing endpoint pairs in the undirected graph. The builder can create reference nodes, so raw endpoint warnings do not imply every reference is absent from the exported graph. This graph is usable but structural coverage is limited.

<details><summary>Full diagnostic</summary>

```text
[graphify] MultiDiGraph edge-collapse diagnostic
input: <in-memory>
input_stage: provided JSON (normal graph.json is post-build)
effective_directed: <direct-call>
nodes: 127
unverified_code_nodes: 0
raw_edges: 264
valid_candidate_edges: 188
missing_endpoint_edges: 0
dangling_endpoint_edges: 76
external_reference_edges: 0
self_loop_edges: 0
exact_duplicate_edges: 6
directed_unique_endpoint_pairs: 183
directed_same_endpoint_collapsed_edges: 5
undirected_unique_endpoint_pairs: 183
undirected_same_endpoint_collapsed_edges: 5
same_endpoint_group_count: 3
relation_variant_groups: 0
source_file_variant_groups: 0
source_location_variant_groups: 0
context_variant_groups: 0
post_build_graph_type: Graph
post_build_edges: 237
producer_suppression_sites: 12
producer_suppression_examples:
  - L1815 seen_ids arity=unknown
  - L2379 seen_ids arity=unknown
  - L2381 seen_doc_refs arity=unknown
  - L3048 seen_ids arity=unknown
  - L3195 seen_ids arity=unknown
  - L4336 seen_keys arity=unknown
  - L4522 seen_keys arity=unknown
  - L7933 seen_ids arity=unknown
examples:
  - src_input_input_task -> input edges=3 relations=['references'] locations=['L11'] contexts=['parameter_type']
  - src_leds_leds_task -> pio0 edges=3 relations=['references'] locations=['L12'] contexts=['generic_arg']
  - src_input_input_task -> adcchannel edges=2 relations=['references'] locations=['L11'] contexts=['parameter_type']
note: normal graph.json is post-build; raw producer loss must be measured earlier.
```
</details>

`graphify-out/` was written into the Git working tree. Unless already ignored, it appears in `git status`; you can add it to `.gitignore` or commit only the outputs your team should share. See https://docs.graphify.com/guides/team-workflows.
