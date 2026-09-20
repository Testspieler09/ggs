# GGS - Geister Geister Schatzsuchmeister Simulator

Rust workspace for simulating and training AI strategies for the cooperative board game
"Geister Geister Schatzsuchmeister" (Ghost Fightin' Treasure Hunters, 2014).

## Workspace layout

```
ggs/
├── ggs-core/        # Game engine, rules, board graph, observation, strategy trait
├── ggs-strategy/    # Strategy implementations (random, greedy, MCTS, RL)
├── simulate/        # CLI batch simulator
└── train/           # RL training loop (evolution strategy)
```

## Building

```sh
cargo build --release
```

---

## simulate

Run batches of games and print win-rate statistics.

```sh
cargo run -p simulate --release -- [OPTIONS]
```

### Options

| Flag              | Default  | Description                                     |
| ----------------- | -------- | ----------------------------------------------- |
| `-g, --games <N>` | `1000`   | Number of games to simulate                     |
| `-s, --seed <N>`  | `0`      | Base RNG seed (game `i` uses `seed + i`)        |
| `--strategy <S>`  | `greedy` | Strategy: `random`, `greedy`, `mcts`, `rl`      |
| `--variant <V>`   | `base`   | Game variant (see below)                        |
| `--figures <N>`   | `4`      | Number of figures: `3` or `4`                   |
| `--log <FILE>`    | -        | Save replay log to file (only with `--games 1`) |

### Variants

| Name       | Description                                       |
| ---------- | ------------------------------------------------- |
| `base`     | Standard ruleset                                  |
| `draw2`    | Adds the "draw 2 cards" ghost card                |
| `draw3`    | Adds the "draw 3 cards" ghost card                |
| `doors`    | Adds blue/green door cards that block edges       |
| `numbered` | Jewels must be collected in revealed number order |
| `advanced` | All advanced cards enabled (hard mode)            |

### Examples

```sh
# 1000 games with greedy strategy
cargo run -p simulate --release

# 500 games with MCTS, 3 figures, advanced variant
cargo run -p simulate --release -- --games 500 --strategy mcts --figures 3 --variant full

# Single game with replay log
cargo run -p simulate --release -- --games 1 --strategy greedy --log replay.json

# Reproducible run with fixed seed
cargo run -p simulate --release -- --seed 1234 --games 100
```

### Output

```
Games:        1000
Wins:         123 (12.3%)
Avg turns:    48.2
Avg Spuk:     4.31
Avg jewels:   6.12
```

---

## train

Train an RL policy network using an evolution strategy (finite differences).
Produces `weights.bin` in the current directory.

```sh
cargo run -p train --release
```

Hyperparameters are constants at the top of `train/src/main.rs`:

| Constant            | Default | Description                                      |
| ------------------- | ------- | ------------------------------------------------ |
| `EPOCHS`            | `500`   | Number of training epochs                        |
| `PERTURBATIONS`     | `50`    | Mirrored noise pairs per epoch (100 evaluations) |
| `EPISODES_PER_EVAL` | `20`    | Games played per perturbed policy                |
| `NOISE_STD`         | `0.05`  | Standard deviation of weight perturbations       |
| `LEARNING_RATE`     | `0.01`  | Step size for weight updates                     |

Progress is printed every 10 epochs:

```
Epoch    0  baseline= 0.0%  perturbation_mean= 0.2%
Epoch   10  baseline= 3.5%  perturbation_mean= 4.1%
...
Final win rate (200 games): 21.3%
Weights saved to weights.bin
```

To simulate with trained weights, load them in code via `RlStrategy::load(Path::new("weights.bin"))`.

---

## Benchmarks

```sh
cargo bench -p ggs-strategy
```

Runs criterion benchmarks for:

- `simulate_one_game_greedy`
- `simulate_one_game_random`
- `simulate_one_game_mcts`
- `simulate_one_game_rl`
- `legal_actions`
- `observation_vector`

HTML reports are written to `target/criterion/`.

---

## Strategies

| Name     | Description                                                                  |
| -------- | ---------------------------------------------------------------------------- |
| `random` | Picks uniformly at random from legal actions                                 |
| `greedy` | Heuristic BFS: moves toward nearest jewel or entrance                        |
| `mcts`   | Monte Carlo Tree Search with UCT (100 simulations/move by default)           |
| `rl`     | MLP policy network (259 -> 256 -> 128 -> 61), trained via evolution strategy |

### RL network shape

- Input: 259 features from `observation_vector` (positions, carry state, board state)
- Hidden: Linear(259, 256) + ReLU, Linear(256, 128) + ReLU
- Output: 61 logits over the action vocabulary, illegal actions masked before argmax

### MCTS parameters (code)

```rust
MctsStrategy::new(simulations, exploration, seed)
MctsStrategy::new_unseeded(simulations, exploration)
```

`exploration` defaults to `√2` in all tooling. Increase `simulations` for stronger play at the cost of speed.
