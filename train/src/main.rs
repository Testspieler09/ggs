use ggs_core::engine::simulate_one_game;
use ggs_core::variant::Variant;
use ggs_strategy::rl::RlStrategy;
use ggs_strategy::random::RandomStrategyMut;
use ggs_core::observation::observation_dim;

fn main() {
    println!("GGS RL training stub");
    println!("Observation dim: {}", observation_dim());
    println!("Running 10 random episodes to verify engine...");

    let mut wins = 0u32;
    for seed in 0..10u64 {
        let mut strats: Vec<RandomStrategyMut> = (0..4)
            .map(|i| RandomStrategyMut::new(seed ^ (i * 0x1234567890abcdef)))
            .collect();
        let result = simulate_one_game(seed, Variant::BASE, 4, &mut strats);
        if result.won { wins += 1; }
    }
    println!("Wins: {}/10", wins);
    println!("To implement: run_epoch() with PPO self-play.");
}

#[allow(dead_code)]
fn run_epoch(
    _strategy: &mut RlStrategy,
    _episodes: u32,
    _seed_offset: u64,
    _variant: Variant,
    _figure_count: u8,
) {
    // TODO: simulate episodes, collect (observation, action, reward) tuples,
    // compute policy gradient, update weights.
}
