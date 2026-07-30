use criterion::{criterion_group, criterion_main, Criterion};
use ggs_core::engine::{simulate_one_game, Game};
use ggs_core::variant::Variant;
use ggs_strategy::greedy::GreedyStrategy;
use ggs_strategy::random::RandomStrategyMut;

fn bench_simulate_one_game(c: &mut Criterion) {
    c.bench_function("simulate_one_game_greedy", |b| {
        b.iter(|| {
            let mut strats = vec![GreedyStrategy::new(); 4];
            simulate_one_game(42, Variant::BASE, 4, &mut strats)
        });
    });
    c.bench_function("simulate_one_game_random", |b| {
        b.iter(|| {
            let mut strats: Vec<RandomStrategyMut> =
                (0..4).map(|i| RandomStrategyMut::new(i * 7)).collect();
            simulate_one_game(42, Variant::BASE, 4, &mut strats)
        });
    });
}

fn bench_legal_actions(c: &mut Criterion) {
    let game = Game::new(42, Variant::BASE, 4);
    c.bench_function("legal_actions", |b| {
        b.iter(|| game.legal_actions());
    });
}

fn bench_observation_vector(c: &mut Criterion) {
    use ggs_core::observation::{observation_vector, PlayerView};
    let game = Game::new(42, Variant::BASE, 4);
    let actions = game.legal_actions();
    let view = PlayerView::from_state(&game.state, 0, &actions);
    c.bench_function("observation_vector", |b| {
        b.iter(|| observation_vector(&view));
    });
}

criterion_group!(
    benches,
    bench_simulate_one_game,
    bench_legal_actions,
    bench_observation_vector
);
criterion_main!(benches);
