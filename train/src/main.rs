use ggs_core::engine::simulate_one_game;
use ggs_core::variant::Variant;
use ggs_strategy::rl::{RlStrategy, WEIGHT_COUNT};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

// ---------------------------------------------------------------------------
// Hyperparameters
// ---------------------------------------------------------------------------

const EPOCHS: usize = 500;
const PERTURBATIONS: usize = 50; // mirrored pairs, so 100 evaluations per epoch
const EPISODES_PER_EVAL: usize = 20; // games per perturbed policy
const NOISE_STD: f32 = 0.05;
const LEARNING_RATE: f32 = 0.01;
const FIGURES: u8 = 4;
const VARIANT: Variant = Variant::BASE;

fn main() {
    println!("GGS RL training - evolution strategy");
    println!("Weights:       {WEIGHT_COUNT}");
    println!("Epochs:        {EPOCHS}");
    println!("Perturbations: {PERTURBATIONS} mirrored pairs");
    println!("Episodes/eval: {EPISODES_PER_EVAL}");
    println!();

    let mut rng = SmallRng::seed_from_u64(42);
    let mut strategy = RlStrategy::new(rng.random());

    for epoch in 0..EPOCHS {
        let epoch_seed: u64 = rng.random();

        // --- Evaluate baseline win rate ---
        let baseline = eval_win_rate(&strategy, EPISODES_PER_EVAL, epoch_seed);

        // --- Generate perturbation seeds ---
        let perturbation_seeds: Vec<u64> = (0..PERTURBATIONS).map(|_| rng.random()).collect();

        // --- Evaluate all mirrored perturbation pairs in parallel ---
        // Each item: (noise_seed, win_rate_plus, win_rate_minus)
        let evals: Vec<(u64, f32, f32)> = perturbation_seeds
            .par_iter()
            .map(|&noise_seed| {
                let mut plus = strategy.clone();
                let mut minus = strategy.clone();
                apply_noise(plus.weights_mut(), noise_seed, NOISE_STD, Sign::Plus);
                apply_noise(minus.weights_mut(), noise_seed, NOISE_STD, Sign::Minus);
                let wr_plus = eval_win_rate(&plus, EPISODES_PER_EVAL, epoch_seed);
                let wr_minus = eval_win_rate(&minus, EPISODES_PER_EVAL, epoch_seed);
                (noise_seed, wr_plus, wr_minus)
            })
            .collect();

        // --- Compute weight update ---
        // update = lr / (2 * P * sigma) * sum_i [ (wr_plus_i - wr_minus_i) * noise_i ]
        let scale = LEARNING_RATE / (2 * PERTURBATIONS) as f32 / NOISE_STD;
        let weights = strategy.weights_mut();
        for (noise_seed, wr_plus, wr_minus) in &evals {
            let advantage = wr_plus - wr_minus;
            if advantage == 0.0 {
                continue;
            }
            let mut noise_rng = SmallRng::seed_from_u64(*noise_seed);
            for w in weights.iter_mut() {
                let n: f32 = noise_rng.random_range(-1.0..=1.0);
                *w += scale * advantage * n;
            }
        }

        if epoch % 10 == 0 {
            let evals_mean: f32 =
                evals.iter().map(|(_, p, m)| (p + m) / 2.0).sum::<f32>() / PERTURBATIONS as f32;
            println!(
                "Epoch {epoch:>4}  baseline={:.1}%  perturbation_mean={:.1}%",
                baseline * 100.0,
                evals_mean * 100.0,
            );
        }
    }

    // --- Final evaluation ---
    let final_wr = eval_win_rate(&strategy, 200, 0xdeadbeef);
    println!("\nFinal win rate (200 games): {:.1}%", final_wr * 100.0);

    strategy
        .save(std::path::Path::new("weights.bin"))
        .expect("failed to save weights");
    println!("Weights saved to weights.bin");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn eval_win_rate(strategy: &RlStrategy, episodes: usize, base_seed: u64) -> f32 {
    let wins: usize = (0..episodes)
        .map(|i| {
            let mut strats = vec![strategy.clone(); FIGURES as usize];
            let result = simulate_one_game(
                base_seed.wrapping_add(i as u64),
                VARIANT,
                FIGURES,
                &mut strats,
            );
            result.won as usize
        })
        .sum();
    wins as f32 / episodes as f32
}

enum Sign {
    Plus,
    Minus,
}

fn apply_noise(weights: &mut [f32], seed: u64, std: f32, sign: Sign) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let s = match sign {
        Sign::Plus => std,
        Sign::Minus => -std,
    };
    for w in weights.iter_mut() {
        let n: f32 = rng.random_range(-1.0..=1.0);
        *w += s * n;
    }
}
