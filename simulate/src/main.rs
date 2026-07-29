use clap::Parser;
use rayon::prelude::*;

use ggs_core::engine::{simulate_one_game, GameResult};
use ggs_core::variant::Variant;
use ggs_strategy::greedy::GreedyStrategy;
use ggs_strategy::mcts::MctsStrategy;
use ggs_strategy::random::RandomStrategyMut;

#[derive(Parser)]
#[command(name = "simulate", about = "Run GGS simulation batch")]
struct Cli {
    /// Number of games to simulate.
    #[arg(short, long, default_value_t = 1000)]
    games: u64,

    /// Base RNG seed; game i uses seed+i.
    #[arg(short, long, default_value_t = 0)]
    seed: u64,

    /// Strategy: random | greedy | mcts
    #[arg(long, default_value = "greedy")]
    strategy: String,

    /// Variant: base | draw2 | draw3 | doors | numbered | full
    #[arg(long, default_value = "base")]
    variant: String,

    /// Number of figures (3 or 4).
    #[arg(long, default_value_t = 4)]
    figures: u8,
}

fn main() {
    let cli = Cli::parse();
    let variant = parse_variant(&cli.variant);
    let strategy = cli.strategy.clone();
    let figures = cli.figures;
    let base_seed = cli.seed;
    let n = cli.games;

    let results: Vec<GameResult> = (0..n)
        .into_par_iter()
        .map(|i| {
            let game_seed = base_seed + i;
            match strategy.as_str() {
                "random" => {
                    let mut strats: Vec<RandomStrategyMut> = (0..figures)
                        .map(|f| RandomStrategyMut::new(game_seed.wrapping_add(f as u64).wrapping_mul(0x9e3779b97f4a7c15)))
                        .collect();
                    simulate_one_game(game_seed, variant, figures, &mut strats)
                }
                "mcts" => {
                    let mut strats: Vec<MctsStrategy> = (0..figures)
                        .map(|_| MctsStrategy::new(100, std::f32::consts::SQRT_2))
                        .collect();
                    simulate_one_game(game_seed, variant, figures, &mut strats)
                }
                _ => {
                    // greedy (default)
                    let mut strats: Vec<GreedyStrategy> = (0..figures)
                        .map(|_| GreedyStrategy::new())
                        .collect();
                    simulate_one_game(game_seed, variant, figures, &mut strats)
                }
            }
        })
        .collect();

    let mut stats = BatchStats::default();
    for r in results {
        stats.record(r);
    }
    stats.print_summary();
}

fn parse_variant(s: &str) -> Variant {
    match s {
        "full"     => Variant::FULL,
        "draw2"    => Variant { draw_two_card: true,  ..Variant::BASE },
        "draw3"    => Variant { draw_three_card: true, ..Variant::BASE },
        "doors"    => Variant { door_cards: true,     ..Variant::BASE },
        "numbered" => Variant { numbered_jewels: true, ..Variant::BASE },
        _          => Variant::BASE,
    }
}

#[derive(Debug, Default)]
struct BatchStats {
    games: u64,
    wins: u64,
    total_turns: u64,
    total_spuk: u64,
    total_jewels: u64,
}

impl BatchStats {
    fn record(&mut self, r: GameResult) {
        self.games += 1;
        if r.won { self.wins += 1; }
        self.total_turns += r.turns_taken as u64;
        self.total_spuk += r.spuk_placed as u64;
        self.total_jewels += r.jewels_deposited as u64;
    }

    fn win_rate(&self) -> f64 {
        if self.games == 0 { return 0.0; }
        self.wins as f64 / self.games as f64
    }

    fn print_summary(&self) {
        println!("Games:        {}", self.games);
        println!("Wins:         {} ({:.1}%)", self.wins, self.win_rate() * 100.0);
        println!("Avg turns:    {:.1}", self.total_turns as f64 / self.games as f64);
        println!("Avg Spuk:     {:.2}", self.total_spuk as f64 / self.games as f64);
        println!("Avg jewels:   {:.2}", self.total_jewels as f64 / self.games as f64);
    }
}
