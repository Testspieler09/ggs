use std::io::{self, Read, Write};
use std::path::Path;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::action::{Action, SymbolFace};
use crate::board::{
    room_node_id, EdgeId, NodeKind, RoomLabel, ADJACENCY, BLUE_EDGES, EDGE_COUNT, ENTRANCE,
    GREEN_EDGES, NODES, NODE_COUNT,
};
use crate::rules;
use crate::state::{
    EdgeState, GameState, GhostCard, GhostDeck, NodeState, TurnPhase, GHOST_DECK_CAPACITY,
    JEWEL_COUNT, MAX_FIGURES, MAX_SPUK,
};
use crate::strategy_trait::Strategy;
use crate::variant::Variant;

// ---------------------------------------------------------------------------
// Public result types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// Game continues; phase has advanced.
    Continue,
    /// All 8 jewels deposited and all figures at entrance.
    Win,
    /// 6th Spuk placed.
    Loss,
    /// The submitted action was not legal in the current phase.
    /// In debug mode this panics; in release it is returned gracefully.
    IllegalAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameResult {
    pub won: bool,
    pub turns_taken: u32,
    pub spuk_placed: u8,
    pub jewels_deposited: u8,
}

/// Records what a ghost card resolution did (useful for logging / replay).
#[derive(Debug, Clone, Copy)]
pub enum GhostCardEvent {
    GhostPlaced {
        room: RoomLabel,
    },
    SpukPlaced {
        room: RoomLabel,
    },
    Reshuffled,
    /// Extra draws were triggered (Zieh 2 / Zieh 3). `extra` is the number of
    /// additional draws queued.
    DrawMultiple {
        extra: u8,
    },
    BlueDoorsClosed,
    GreenDoorsClosed,
}

// ---------------------------------------------------------------------------
// Game driver
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Game {
    pub state: GameState,
    rng: StdRng,
}

/// Returns the next room label in alphabetical order, wrapping L → A.
fn next_room(room: RoomLabel) -> RoomLabel {
    match room {
        RoomLabel::A => RoomLabel::B,
        RoomLabel::B => RoomLabel::C,
        RoomLabel::C => RoomLabel::D,
        RoomLabel::D => RoomLabel::E,
        RoomLabel::E => RoomLabel::F,
        RoomLabel::F => RoomLabel::G,
        RoomLabel::G => RoomLabel::H,
        RoomLabel::H => RoomLabel::I,
        RoomLabel::I => RoomLabel::J,
        RoomLabel::J => RoomLabel::K,
        RoomLabel::K => RoomLabel::L,
        RoomLabel::L => RoomLabel::A,
    }
}

impl Game {
    pub fn new(seed: u64, variant: Variant, figure_count: u8) -> Self {
        assert!(
            figure_count == 3 || figure_count == 4,
            "figure_count must be 3 or 4"
        );
        let mut rng = StdRng::seed_from_u64(seed);
        let state = Self::setup(&mut rng, variant, figure_count);
        Self { state, rng }
    }

    /// Create a game from an existing state snapshot with a new RNG seed.
    /// Used for MCTS rollouts.
    pub fn from_state(state: GameState, seed: u64) -> Self {
        Self {
            state,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn step(&mut self, action: Action) -> StepResult {
        if self.state.game_over {
            return if self.state.players_won {
                StepResult::Win
            } else {
                StepResult::Loss
            };
        }

        let result = self.apply(action);

        // Check terminal conditions after every step.
        if rules::is_loss(&self.state) {
            self.state.game_over = true;
            self.state.players_won = false;
            return StepResult::Loss;
        }
        if rules::is_win(&self.state) {
            self.state.game_over = true;
            self.state.players_won = true;
            return StepResult::Win;
        }
        result
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        rules::legal_actions(&self.state)
    }

    pub fn legal_actions_into(&self, buf: &mut Vec<Action>) {
        rules::legal_actions_into(&self.state, buf);
    }

    pub fn is_finished(&self) -> bool {
        self.state.game_over
    }

    /// `Some(true)` = win, `Some(false)` = loss, `None` = in progress.
    pub fn winner(&self) -> Option<bool> {
        if self.state.game_over {
            Some(self.state.players_won)
        } else {
            None
        }
    }

    pub fn state_snapshot(&self) -> GameState {
        self.state.clone()
    }

    // -----------------------------------------------------------------------
    // Internal: action dispatch
    // -----------------------------------------------------------------------

    fn apply(&mut self, action: Action) -> StepResult {
        match (&self.state.phase, action) {
            (TurnPhase::RollDie, Action::RollDie) => {
                let roll = self.roll_number_die();
                self.state.die_roll = roll;
                self.state.moves_remaining = roll;
                if roll < 6 {
                    self.state.phase = TurnPhase::DrawGhostCard { cards_remaining: 1 };
                } else {
                    self.state.phase = TurnPhase::Move;
                }
                StepResult::Continue
            }
            (TurnPhase::DrawGhostCard { cards_remaining }, Action::DrawGhostCard) => {
                let remaining = *cards_remaining;
                self.resolve_next_ghost_card();
                if self.state.game_over {
                    return StepResult::Loss;
                }
                // cards_remaining may have been bumped by a DrawTwo/DrawThree card.
                // Re-read it from state after resolution.
                match self.state.phase {
                    TurnPhase::DrawGhostCard { cards_remaining: r } if r > 0 => {
                        // More cards to draw; phase stays the same.
                    }
                    _ => {
                        self.state.phase = TurnPhase::Move;
                    }
                }
                let _ = remaining;
                StepResult::Continue
            }
            (TurnPhase::Move, Action::MoveAlongEdge { edge }) => self.move_figure(edge),
            (TurnPhase::Move, Action::StopMoving) => {
                self.state.moves_remaining = 0;
                self.advance_from_move();
                StepResult::Continue
            }
            (TurnPhase::Move, Action::DepositJewel) => {
                self.deposit_jewel();
                // Stay in Move phase — figure may still have moves.
                StepResult::Continue
            }
            (TurnPhase::PickupJewel, Action::PickupJewel { jewel }) => {
                self.pickup_jewel(jewel);
                self.state.phase = TurnPhase::Combat;
                StepResult::Continue
            }
            (TurnPhase::PickupJewel, Action::SkipPickup) => {
                self.state.phase = TurnPhase::Combat;
                StepResult::Continue
            }
            (TurnPhase::PickupJewel, Action::DepositJewel) => {
                self.deposit_jewel();
                self.state.phase = TurnPhase::Combat;
                StepResult::Continue
            }
            (TurnPhase::Combat, Action::Fight) => {
                self.fight();
                self.state.phase = TurnPhase::EndTurn;
                StepResult::Continue
            }
            (TurnPhase::Combat, Action::SkipCombat) => {
                self.state.phase = TurnPhase::EndTurn;
                StepResult::Continue
            }
            (TurnPhase::EndTurn, Action::EndTurn) => {
                self.advance_turn();
                StepResult::Continue
            }
            _ => {
                debug_assert!(
                    false,
                    "illegal action {:?} in phase {:?}",
                    action, self.state.phase
                );
                StepResult::IllegalAction
            }
        }
    }

    // -----------------------------------------------------------------------
    // Movement
    // -----------------------------------------------------------------------

    fn move_figure(&mut self, edge: EdgeId) -> StepResult {
        let fig = self.state.active_figure as usize;
        let from = self.state.figure_pos[fig];
        let dest = ADJACENCY.other_end(edge, from);

        // Remove figure from current node.
        self.state.node_states[from as usize].remove_figure(self.state.active_figure);
        // Place at destination.
        self.state.node_states[dest as usize].add_figure(self.state.active_figure);
        self.state.figure_pos[fig] = dest;
        self.state.moves_remaining -= 1;

        // In the numbered-jewel variant, reveal the jewel number the first time a figure
        // enters a room that has one (jewel_number == 0 means not yet revealed).
        if self.state.variant.numbered_jewels {
            if let Some(jewel_id) = self.state.node_states[dest as usize].jewel {
                if self.state.jewel_number[jewel_id as usize] == 0 {
                    self.state.jewel_number[jewel_id as usize] =
                        self.state.node_states[dest as usize].jewel_number;
                }
            }
        }

        if self.state.moves_remaining == 0 {
            self.advance_from_move();
        }
        StepResult::Continue
    }

    /// Transition out of the Move phase once the figure has used all moves or chose to stop.
    fn advance_from_move(&mut self) {
        let fig = self.state.active_figure as usize;
        let pos = self.state.figure_pos[fig];
        let in_room = matches!(NODES[pos as usize].kind, NodeKind::Room(_));
        if in_room || (pos == ENTRANCE && self.state.figure_carries[fig].is_some()) {
            self.state.phase = TurnPhase::PickupJewel;
        } else {
            self.state.phase = TurnPhase::EndTurn;
        }
    }

    // -----------------------------------------------------------------------
    // Jewel mechanics
    // -----------------------------------------------------------------------

    fn pickup_jewel(&mut self, jewel_id: crate::state::JewelId) {
        let fig = self.state.active_figure as usize;
        let pos = self.state.figure_pos[fig];
        self.state.node_states[pos as usize].jewel = None;
        self.state.figure_carries[fig] = Some(jewel_id);
    }

    fn deposit_jewel(&mut self) {
        let fig = self.state.active_figure as usize;
        if let Some(jewel_id) = self.state.figure_carries[fig].take() {
            self.state.jewel_deposited[jewel_id as usize] = true;
            // In numbered-jewel mode, advance the required counter.
            if self.state.variant.numbered_jewels {
                self.state.next_required_jewel += 1;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Combat
    // -----------------------------------------------------------------------

    fn fight(&mut self) {
        let fig = self.state.active_figure;
        let pos = self.state.figure_pos[fig as usize];
        let ns = &self.state.node_states[pos as usize];
        let figures_here = ns.figure_count();

        let die_a = self.roll_symbol_die();

        if figures_here >= 2 {
            let die_b = self.roll_symbol_die();
            // With 2+ figures: Spuk face on either die removes Spuk.
            if (die_a == SymbolFace::Spuk || die_b == SymbolFace::Spuk)
                && self.state.node_states[pos as usize].has_spuk
            {
                self.state.node_states[pos as usize].has_spuk = false;
                self.state.spuk_count -= 1;
                return;
            }
            // Ghost face on either die removes one ghost.
            if (die_a == SymbolFace::Ghost || die_b == SymbolFace::Ghost)
                && self.state.node_states[pos as usize].ghosts > 0
            {
                self.state.node_states[pos as usize].ghosts -= 1;
            }
        } else {
            // Single figure: only ghost face removes one ghost (Spuk cannot be fought alone).
            if die_a == SymbolFace::Ghost && self.state.node_states[pos as usize].ghosts > 0 {
                self.state.node_states[pos as usize].ghosts -= 1;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Ghost card resolution
    // -----------------------------------------------------------------------

    fn resolve_next_ghost_card(&mut self) {
        // Reshuffle if deck is empty.
        if self.state.deck.is_empty() {
            self.shuffle_deck();
        }
        let Some(card) = self.state.deck.draw() else {
            return;
        };

        match card {
            GhostCard::Room(room) => {
                let game_over = self.place_ghost_in(room);
                if game_over {
                    self.state.game_over = true;
                    self.state.players_won = false;
                }
                self.decrement_ghost_cards_remaining();
            }
            GhostCard::Reshuffle => {
                self.shuffle_deck();
                self.decrement_ghost_cards_remaining();
            }
            GhostCard::DrawTwo => {
                // Queue 2 extra card draws by bumping cards_remaining.
                self.bump_cards_remaining(2);
            }
            GhostCard::DrawThree => {
                self.bump_cards_remaining(3);
            }
            GhostCard::BlueDoors => {
                for &eid in BLUE_EDGES {
                    self.state.edge_states[eid as usize].closed = true;
                }
                self.decrement_ghost_cards_remaining();
            }
            GhostCard::GreenDoors => {
                for &eid in GREEN_EDGES {
                    self.state.edge_states[eid as usize].closed = true;
                }
                self.decrement_ghost_cards_remaining();
            }
        }
    }

    fn decrement_ghost_cards_remaining(&mut self) {
        if let TurnPhase::DrawGhostCard {
            ref mut cards_remaining,
        } = self.state.phase
        {
            if *cards_remaining > 0 {
                *cards_remaining -= 1;
            }
        }
    }

    fn bump_cards_remaining(&mut self, extra: u8) {
        if let TurnPhase::DrawGhostCard {
            ref mut cards_remaining,
        } = self.state.phase
        {
            // Replace the current draw count with `extra` (the card itself was the trigger).
            *cards_remaining = (*cards_remaining - 1).saturating_add(extra);
        }
    }

    /// Place one ghost figure in `room`. Returns `true` if this caused the 6th Spuk.
    pub(crate) fn place_ghost_in(&mut self, room: RoomLabel) -> bool {
        // If the target room already has a Spuk, redirect to the next room alphabetically
        // (wrapping A→B→…→L→A) until a non-Spuk room is found.
        let mut target = room;
        loop {
            let node = room_node_id(target) as usize;
            if !self.state.node_states[node].has_spuk {
                break;
            }
            target = next_room(target);
            if target == room {
                // All 12 rooms have Spuk — game should already be lost, but guard anyway.
                return self.state.spuk_count >= MAX_SPUK;
            }
        }

        let node = room_node_id(target) as usize;
        let ns = &mut self.state.node_states[node];
        ns.ghosts += 1;
        if ns.ghosts >= crate::state::MAX_GHOSTS_BEFORE_SPUK {
            ns.ghosts = 0;
            ns.has_spuk = true;
            self.state.spuk_count += 1;
            return self.state.spuk_count >= MAX_SPUK;
        }
        false
    }

    pub(crate) fn shuffle_deck(&mut self) {
        let size = self.state.deck.size as usize;
        self.state.deck.cards[..size].shuffle(&mut self.rng);
        self.state.deck.reset_cursor();
    }

    // -----------------------------------------------------------------------
    // Turn advancement
    // -----------------------------------------------------------------------

    fn advance_turn(&mut self) {
        self.state.active_figure = (self.state.active_figure + 1) % self.state.figure_count;
        self.state.phase = TurnPhase::RollDie;
        self.state.die_roll = 0;
        self.state.moves_remaining = 0;
    }

    // -----------------------------------------------------------------------
    // Dice
    // -----------------------------------------------------------------------

    pub(crate) fn roll_number_die(&mut self) -> u8 {
        self.rng.random_range(1u8..=6u8)
    }

    pub(crate) fn roll_symbol_die(&mut self) -> SymbolFace {
        let idx = self.rng.random_range(0usize..6);
        SymbolFace::FACES[idx]
    }

    // -----------------------------------------------------------------------
    // Game setup
    // -----------------------------------------------------------------------

    fn setup(rng: &mut StdRng, variant: Variant, figure_count: u8) -> GameState {
        let mut node_states = [NodeState::default(); NODE_COUNT];
        let edge_states = [EdgeState::default(); EDGE_COUNT];

        // Place starting figures at entrance.
        // (We don't set node_states[ENTRANCE].figures here; the engine tracks figure_pos;
        //  NodeState.figures is kept consistent in move_figure. We initialize it below.)
        let figure_pos = [ENTRANCE; MAX_FIGURES];
        for i in 0..figure_count as usize {
            node_states[ENTRANCE as usize].add_figure(i as u8);
        }

        // Place starting ghosts.
        for &room in &RoomLabel::STARTS_WITH_GHOST {
            node_states[room_node_id(room) as usize].ghosts = 1;
        }

        // Place jewels in jewel rooms.
        let jewel_number = [0u8; JEWEL_COUNT];
        let mut next_required_jewel = 1u8;

        // Assign jewel IDs 0..7 to the 8 jewel rooms.
        let jewel_rooms = RoomLabel::STARTS_WITH_JEWEL;
        for (jewel_id, &room) in jewel_rooms.iter().enumerate() {
            node_states[room_node_id(room) as usize].jewel = Some(jewel_id as u8);
        }

        // In numbered-jewel variant, assign a shuffled permutation 1..=8 into each room's
        // NodeState. jewel_number in GameState stays all-zeros until figures enter rooms.
        if variant.numbered_jewels {
            let mut numbers: [u8; JEWEL_COUNT] = [1, 2, 3, 4, 5, 6, 7, 8];
            numbers.shuffle(rng);
            for (jewel_id, &room) in jewel_rooms.iter().enumerate() {
                node_states[room_node_id(room) as usize].jewel_number = numbers[jewel_id];
            }
            next_required_jewel = 1;
        }

        // Build ghost card deck.
        let deck = Self::build_deck(rng, variant);

        GameState {
            variant,
            figure_count,
            node_states,
            edge_states,
            figure_pos,
            figure_carries: [None; MAX_FIGURES],
            jewel_deposited: [false; JEWEL_COUNT],
            jewel_number,
            next_required_jewel,
            spuk_count: 0,
            deck,
            active_figure: 0,
            phase: TurnPhase::RollDie,
            die_roll: 0,
            moves_remaining: 0,
            game_over: false,
            players_won: false,
        }
    }

    fn build_deck(rng: &mut StdRng, variant: Variant) -> GhostDeck {
        // Base deck: 12 room cards (A–L, one each) + 1 Reshuffle = 13 cards.
        // Advanced deck adds: DrawTwo, DrawThree, 2× BlueDoors, 2× GreenDoors = 6 more → 19 total.
        let mut cards: [GhostCard; GHOST_DECK_CAPACITY] =
            [GhostCard::Room(RoomLabel::A); GHOST_DECK_CAPACITY];
        let rooms = [
            RoomLabel::A,
            RoomLabel::B,
            RoomLabel::C,
            RoomLabel::D,
            RoomLabel::E,
            RoomLabel::F,
            RoomLabel::G,
            RoomLabel::H,
            RoomLabel::I,
            RoomLabel::J,
            RoomLabel::K,
            RoomLabel::L,
        ];
        let mut size = 0usize;
        for &r in &rooms {
            cards[size] = GhostCard::Room(r);
            size += 1;
        }
        cards[size] = GhostCard::Reshuffle;
        size += 1;

        // Advanced cards.
        if variant.draw_two_card {
            cards[size] = GhostCard::DrawTwo;
            size += 1;
        }
        if variant.draw_three_card {
            cards[size] = GhostCard::DrawThree;
            size += 1;
        }
        if variant.door_cards {
            cards[size] = GhostCard::BlueDoors;
            cards[size + 1] = GhostCard::BlueDoors;
            cards[size + 2] = GhostCard::GreenDoors;
            cards[size + 3] = GhostCard::GreenDoors;
            size += 4;
        }

        cards[..size].shuffle(rng);
        GhostDeck {
            cards,
            top: 0,
            size: size as u8,
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level simulation function (rayon-friendly)
// ---------------------------------------------------------------------------

/// Play one complete game to termination with the provided strategies.
/// `strategies[i]` controls figure `i`. Must have `figure_count` entries.
pub fn simulate_one_game<S>(
    seed: u64,
    variant: Variant,
    figure_count: u8,
    strategies: &mut [S],
) -> GameResult
where
    S: Strategy,
{
    let mut game = Game::new(seed, variant, figure_count);
    let mut turns: u32 = 0;
    let mut action_buf = Vec::with_capacity(8);

    for s in strategies.iter_mut() {
        s.on_game_start(0, variant);
    }

    loop {
        game.legal_actions_into(&mut action_buf);
        if action_buf.is_empty() || game.is_finished() {
            break;
        }

        // RollDie and DrawGhostCard are auto-resolved (single legal action).
        let action = if action_buf.len() == 1
            && matches!(
                action_buf[0],
                Action::RollDie | Action::DrawGhostCard | Action::EndTurn
            ) {
            action_buf[0]
        } else {
            let fig = game.state.active_figure as usize;
            let view = crate::observation::PlayerView::from_state(
                &game.state,
                game.state.active_figure,
                &action_buf,
            );
            strategies[fig % strategies.len()].choose_action(&view)
        };

        if matches!(action, Action::EndTurn) {
            turns += 1;
        }
        let result = game.step(action);
        if matches!(result, StepResult::Win | StepResult::Loss) {
            break;
        }
    }

    let won = game.state.players_won;
    let spuk = game.state.spuk_count;
    let jewels = game.state.jewel_deposited.iter().filter(|&&d| d).count() as u8;

    for s in strategies.iter_mut() {
        s.on_game_end(won);
    }

    GameResult {
        won,
        turns_taken: turns,
        spuk_placed: spuk,
        jewels_deposited: jewels,
    }
}

// ---------------------------------------------------------------------------
// Game log (replay)
// ---------------------------------------------------------------------------

/// A complete record of one game: seed + every action taken in order.
/// Because the engine is fully deterministic, replaying with the same seed
/// and the same action sequence reproduces the identical game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLog {
    pub seed: u64,
    pub variant: Variant,
    pub figure_count: u8,
    pub actions: Vec<Action>,
}

impl GameLog {
    /// Save to a binary file (hand-rolled, no extra dependency).
    ///
    /// Format:
    /// ```text
    /// [8]  seed: u64 le
    /// [1]  figure_count: u8
    /// [1]  variant flags: bit0=draw_two, bit1=draw_three, bit2=door_cards, bit3=numbered
    /// [4]  action_count: u32 le
    /// per action:
    ///   [1] tag
    ///   [1] payload byte (only for MoveAlongEdge and PickupJewel)
    /// ```
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(&self.seed.to_le_bytes())?;
        file.write_all(&[self.figure_count])?;
        let v = &self.variant;
        let flags: u8 = (v.draw_two_card as u8)
            | ((v.draw_three_card as u8) << 1)
            | ((v.door_cards as u8) << 2)
            | ((v.numbered_jewels as u8) << 3);
        file.write_all(&[flags])?;
        file.write_all(&(self.actions.len() as u32).to_le_bytes())?;
        for &action in &self.actions {
            encode_action(&mut file, action)?;
        }
        Ok(())
    }

    /// Load from a binary file written by `save`.
    pub fn load(path: &Path) -> io::Result<GameLog> {
        let mut file = std::fs::File::open(path)?;
        let mut buf8 = [0u8; 8];
        file.read_exact(&mut buf8)?;
        let seed = u64::from_le_bytes(buf8);
        let mut buf1 = [0u8; 1];
        file.read_exact(&mut buf1)?;
        let figure_count = buf1[0];
        file.read_exact(&mut buf1)?;
        let flags = buf1[0];
        let variant = Variant {
            draw_two_card: flags & 0x01 != 0,
            draw_three_card: flags & 0x02 != 0,
            door_cards: flags & 0x04 != 0,
            numbered_jewels: flags & 0x08 != 0,
        };
        let mut buf4 = [0u8; 4];
        file.read_exact(&mut buf4)?;
        let count = u32::from_le_bytes(buf4) as usize;
        let mut actions = Vec::with_capacity(count);
        for _ in 0..count {
            actions.push(decode_action(&mut file)?);
        }
        Ok(GameLog {
            seed,
            variant,
            figure_count,
            actions,
        })
    }
}

fn encode_action(w: &mut impl Write, action: Action) -> io::Result<()> {
    match action {
        Action::RollDie => w.write_all(&[0x00]),
        Action::DrawGhostCard => w.write_all(&[0x01]),
        Action::MoveAlongEdge { edge } => w.write_all(&[0x02, edge]),
        Action::StopMoving => w.write_all(&[0x03]),
        Action::PickupJewel { jewel } => w.write_all(&[0x04, jewel]),
        Action::SkipPickup => w.write_all(&[0x05]),
        Action::DepositJewel => w.write_all(&[0x06]),
        Action::Fight => w.write_all(&[0x07]),
        Action::SkipCombat => w.write_all(&[0x08]),
        Action::EndTurn => w.write_all(&[0x09]),
    }
}

fn decode_action(r: &mut impl Read) -> io::Result<Action> {
    let mut tag = [0u8; 1];
    r.read_exact(&mut tag)?;
    Ok(match tag[0] {
        0x00 => Action::RollDie,
        0x01 => Action::DrawGhostCard,
        0x02 => {
            let mut b = [0u8; 1];
            r.read_exact(&mut b)?;
            Action::MoveAlongEdge { edge: b[0] }
        }
        0x03 => Action::StopMoving,
        0x04 => {
            let mut b = [0u8; 1];
            r.read_exact(&mut b)?;
            Action::PickupJewel { jewel: b[0] }
        }
        0x05 => Action::SkipPickup,
        0x06 => Action::DepositJewel,
        0x07 => Action::Fight,
        0x08 => Action::SkipCombat,
        0x09 => Action::EndTurn,
        tag => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown action tag 0x{tag:02x}"),
            ))
        }
    })
}

/// Play one game and record every action taken (including auto-resolved ones).
pub fn simulate_one_game_logged<S>(
    seed: u64,
    variant: Variant,
    figure_count: u8,
    strategies: &mut [S],
) -> (GameResult, GameLog)
where
    S: Strategy,
{
    let mut game = Game::new(seed, variant, figure_count);
    let mut turns: u32 = 0;
    let mut action_buf = Vec::with_capacity(8);
    let mut log_actions: Vec<Action> = Vec::new();

    for s in strategies.iter_mut() {
        s.on_game_start(0, variant);
    }

    loop {
        game.legal_actions_into(&mut action_buf);
        if action_buf.is_empty() || game.is_finished() {
            break;
        }

        let action = if action_buf.len() == 1
            && matches!(
                action_buf[0],
                Action::RollDie | Action::DrawGhostCard | Action::EndTurn
            ) {
            action_buf[0]
        } else {
            let fig = game.state.active_figure as usize;
            let view = crate::observation::PlayerView::from_state(
                &game.state,
                game.state.active_figure,
                &action_buf,
            );
            strategies[fig % strategies.len()].choose_action(&view)
        };

        log_actions.push(action);
        if matches!(action, Action::EndTurn) {
            turns += 1;
        }
        let result = game.step(action);
        if matches!(result, StepResult::Win | StepResult::Loss) {
            break;
        }
    }

    let won = game.state.players_won;
    let spuk = game.state.spuk_count;
    let jewels = game.state.jewel_deposited.iter().filter(|&&d| d).count() as u8;

    for s in strategies.iter_mut() {
        s.on_game_end(won);
    }

    let result = GameResult {
        won,
        turns_taken: turns,
        spuk_placed: spuk,
        jewels_deposited: jewels,
    };
    let log = GameLog {
        seed,
        variant,
        figure_count,
        actions: log_actions,
    };
    (result, log)
}

/// Replay a recorded game by replaying the exact action sequence from `log`.
/// Returns the `GameResult` produced by replay (should match the original).
///
/// Panics in debug mode if any recorded action is illegal (indicates log corruption).
pub fn replay(log: &GameLog) -> GameResult {
    let mut game = Game::new(log.seed, log.variant, log.figure_count);
    let mut turns: u32 = 0;
    let mut action_buf = Vec::with_capacity(8);

    for &action in &log.actions {
        game.legal_actions_into(&mut action_buf);
        debug_assert!(
            action_buf.contains(&action),
            "replay: action {action:?} not legal in phase {:?}",
            game.state.phase
        );
        if matches!(action, Action::EndTurn) {
            turns += 1;
        }
        let result = game.step(action);
        if matches!(result, StepResult::Win | StepResult::Loss) {
            break;
        }
    }

    let won = game.state.players_won;
    let spuk = game.state.spuk_count;
    let jewels = game.state.jewel_deposited.iter().filter(|&&d| d).count() as u8;
    GameResult {
        won,
        turns_taken: turns,
        spuk_placed: spuk,
        jewels_deposited: jewels,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{H1, ROOM_A, ROOM_B, ROOM_C};
    use crate::rules;
    use crate::variant::Variant;

    #[derive(Clone)]
    struct FirstActionStrategy;
    impl crate::strategy_trait::Strategy for FirstActionStrategy {
        fn choose_action(&self, view: &crate::observation::PlayerView) -> Action {
            view.legal_actions[0]
        }
    }

    fn make_game(seed: u64) -> Game {
        Game::new(seed, Variant::BASE, 4)
    }

    #[test]
    fn replay_matches_original() {
        for seed in 0u64..20 {
            let mut strats = vec![FirstActionStrategy; 4];
            let (original, log) = simulate_one_game_logged(seed, Variant::BASE, 4, &mut strats);
            let replayed = replay(&log);
            assert_eq!(
                original, replayed,
                "seed={seed}: replay result differs from original"
            );
        }
    }

    #[test]
    fn log_roundtrip() {
        let mut strats = vec![FirstActionStrategy; 4];
        let (_, log) = simulate_one_game_logged(42, Variant::BASE, 4, &mut strats);
        let dir = std::env::temp_dir();
        let path = dir.join("ggs_test_log.bin");
        log.save(&path).expect("save failed");
        let loaded = GameLog::load(&path).expect("load failed");
        std::fs::remove_file(&path).ok();
        assert_eq!(log.seed, loaded.seed);
        assert_eq!(log.figure_count, loaded.figure_count);
        assert_eq!(log.variant, loaded.variant);
        assert_eq!(log.actions, loaded.actions);
        let replayed = replay(&loaded);
        let mut strats2 = vec![FirstActionStrategy; 4];
        let (original, _) = simulate_one_game_logged(42, Variant::BASE, 4, &mut strats2);
        assert_eq!(original, replayed);
    }

    // --- Deck composition ---

    #[test]
    fn base_deck_has_13_cards() {
        let game = make_game(0);
        assert_eq!(game.state.deck.size, 13);
    }

    #[test]
    fn advanced_deck_has_19_cards() {
        let game = Game::new(0, Variant::ADVANCED, 4);
        assert_eq!(game.state.deck.size, 19);
    }

    #[test]
    fn base_deck_has_no_advanced_cards() {
        let game = make_game(0);
        let cards = &game.state.deck.cards[..game.state.deck.size as usize];
        for card in cards {
            assert!(
                !matches!(
                    card,
                    GhostCard::DrawTwo
                        | GhostCard::DrawThree
                        | GhostCard::BlueDoors
                        | GhostCard::GreenDoors
                ),
                "base deck should not contain advanced card: {card:?}"
            );
        }
    }

    // --- Setup correctness ---

    #[test]
    fn starting_ghosts_in_correct_rooms() {
        use crate::board::room_node_id;
        let game = make_game(0);
        for &room in &crate::board::RoomLabel::STARTS_WITH_GHOST {
            let ns = &game.state.node_states[room_node_id(room) as usize];
            assert_eq!(ns.ghosts, 1, "room {room:?} should start with 1 ghost");
        }
    }

    #[test]
    fn starting_jewels_in_correct_rooms() {
        use crate::board::room_node_id;
        let game = make_game(0);
        for &room in &crate::board::RoomLabel::STARTS_WITH_JEWEL {
            let ns = &game.state.node_states[room_node_id(room) as usize];
            assert!(
                ns.jewel.is_some(),
                "room {room:?} should start with a jewel"
            );
        }
    }

    // --- Ghost overflow: redirect to next alphabetical room ---

    #[test]
    fn ghost_skips_spuk_room_and_goes_to_next() {
        let mut game = make_game(0);
        // Force room A into Spuk state.
        game.state.node_states[ROOM_A as usize].ghosts = 0;
        game.state.node_states[ROOM_A as usize].has_spuk = true;
        game.state.spuk_count = 1;
        // Place a ghost targeting room A — should redirect to B.
        game.place_ghost_in(crate::board::RoomLabel::A);
        assert_eq!(
            game.state.node_states[ROOM_A as usize].ghosts, 0,
            "A should still have 0 ghosts"
        );
        assert_eq!(
            game.state.node_states[ROOM_B as usize].ghosts, 1,
            "ghost should land in B"
        );
    }

    #[test]
    fn ghost_wraps_from_l_to_a() {
        let mut game = make_game(0);
        // Spuk all rooms B–L, leave A free.
        for room in [
            crate::board::RoomLabel::B,
            crate::board::RoomLabel::C,
            crate::board::RoomLabel::D,
            crate::board::RoomLabel::E,
            crate::board::RoomLabel::F,
            crate::board::RoomLabel::G,
            crate::board::RoomLabel::H,
            crate::board::RoomLabel::I,
            crate::board::RoomLabel::J,
            crate::board::RoomLabel::K,
            crate::board::RoomLabel::L,
        ] {
            game.state.node_states[crate::board::room_node_id(room) as usize].has_spuk = true;
        }
        game.state.spuk_count = 11;
        game.state.node_states[ROOM_A as usize].has_spuk = false;
        game.state.node_states[ROOM_A as usize].ghosts = 0;
        // Place ghost targeting L — should wrap around and land in A.
        game.place_ghost_in(crate::board::RoomLabel::L);
        assert_eq!(
            game.state.node_states[ROOM_A as usize].ghosts, 1,
            "ghost should wrap to A"
        );
    }

    // --- Loss condition: 6 Spuk ---

    #[test]
    fn six_spuk_triggers_loss() {
        let mut game = make_game(0);
        game.state.spuk_count = 5;
        // Place 3 ghosts in room C (which starts with 1) to trigger the 6th Spuk.
        game.state.node_states[ROOM_C as usize].ghosts = 2;
        game.place_ghost_in(crate::board::RoomLabel::C);
        assert!(rules::is_loss(&game.state));
    }

    // --- Hallway pass-through ---

    #[test]
    fn can_move_into_occupied_hallway() {
        let mut game = make_game(0);
        // Put figure 0 at H1, figure 1 also at H1 (to occupy it), figure 0 starts at entrance.
        // Actually place figure 1 at H1 so figure 0 can move through it.
        game.state.node_states[ENTRANCE as usize].remove_figure(1);
        game.state.node_states[H1 as usize].add_figure(1);
        game.state.figure_pos[1] = H1;
        // Set up figure 0 to be in Move phase at entrance.
        game.state.active_figure = 0;
        game.state.phase = crate::state::TurnPhase::Move;
        game.state.moves_remaining = 3;
        let actions = game.legal_actions();
        // Edge 0 connects Entrance→H1; H1 is occupied but should still be reachable.
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::MoveAlongEdge { edge: 0 })),
            "should be able to move into occupied hallway cell H1"
        );
    }

    #[test]
    fn cannot_stop_on_occupied_hallway() {
        let mut game = make_game(0);
        // Move figure 0 to H1, occupy H1 with figure 1 too.
        game.state.node_states[ENTRANCE as usize].remove_figure(0);
        game.state.node_states[ENTRANCE as usize].remove_figure(1);
        game.state.node_states[H1 as usize].add_figure(0);
        game.state.node_states[H1 as usize].add_figure(1);
        game.state.figure_pos[0] = H1;
        game.state.figure_pos[1] = H1;
        game.state.active_figure = 0;
        game.state.phase = crate::state::TurnPhase::Move;
        game.state.moves_remaining = 3;
        let actions = game.legal_actions();
        assert!(
            !actions.contains(&Action::StopMoving),
            "StopMoving should be illegal when on an occupied hallway cell"
        );
    }

    // --- Spuk-trap ---

    #[test]
    fn figure_with_jewel_cannot_leave_spuk_room() {
        let mut game = make_game(0);
        // Put figure 0 in room A carrying a jewel, and add Spuk to room A.
        game.state.node_states[ENTRANCE as usize].remove_figure(0);
        game.state.node_states[ROOM_A as usize].add_figure(0);
        game.state.node_states[ROOM_A as usize].has_spuk = true;
        game.state.node_states[ROOM_A as usize].jewel = None;
        game.state.figure_pos[0] = ROOM_A;
        game.state.figure_carries[0] = Some(0);
        game.state.spuk_count = 1;
        game.state.active_figure = 0;
        game.state.phase = crate::state::TurnPhase::Move;
        game.state.moves_remaining = 3;
        let actions = game.legal_actions();
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, Action::MoveAlongEdge { .. })),
            "figure carrying a jewel should have no move edges out of a Spuk room"
        );
    }

    // --- Jewel number revelation (advanced variant) ---

    #[test]
    fn jewel_number_hidden_before_entering_room() {
        let game = Game::new(0, Variant::ADVANCED, 4);
        // At setup, all jewel numbers should be 0 (unrevealed) in GameState.jewel_number.
        assert!(
            game.state.jewel_number.iter().all(|&n| n == 0),
            "jewel numbers should all be hidden at start"
        );
    }

    #[test]
    fn jewel_number_revealed_on_room_entry() {
        use crate::board::room_node_id;
        let mut game = Game::new(0, Variant::ADVANCED, 4);
        // Find the first jewel room that has a jewel still in it.
        let target_room = crate::board::RoomLabel::STARTS_WITH_JEWEL[0];
        let target_node = room_node_id(target_room);
        let jewel_id = game.state.node_states[target_node as usize].jewel.unwrap();
        let true_number = game.state.node_states[target_node as usize].jewel_number;
        assert!(true_number > 0, "hidden number should be set at setup");
        // Teleport figure 0 into the room.
        game.state.node_states[ENTRANCE as usize].remove_figure(0);
        game.state.node_states[target_node as usize].add_figure(0);
        game.state.figure_pos[0] = target_node;
        // Simulate arrival by calling move_figure indirectly — just trigger the reveal
        // by entering the room via a direct state manipulation followed by the reveal logic.
        // We test the reveal by checking jewel_number after moving via the engine.
        // Reset position and do it properly: place at adjacent hallway and move in.
        // Find an edge that connects to target_node.
        let entry_edge = crate::board::ADJACENCY
            .edges_of(target_node)
            .iter()
            .copied()
            .find(|&e| {
                let other = crate::board::ADJACENCY.other_end(e, target_node);
                matches!(
                    crate::board::NODES[other as usize].kind,
                    crate::board::NodeKind::Hallway
                )
            })
            .expect("room must have a hallway neighbor");
        let hallway = crate::board::ADJACENCY.other_end(entry_edge, target_node);
        game.state.node_states[target_node as usize].remove_figure(0);
        game.state.node_states[hallway as usize].add_figure(0);
        game.state.figure_pos[0] = hallway;
        game.state.active_figure = 0;
        game.state.phase = crate::state::TurnPhase::Move;
        game.state.moves_remaining = 1;
        game.step(Action::MoveAlongEdge { edge: entry_edge });
        assert_eq!(
            game.state.jewel_number[jewel_id as usize], true_number,
            "jewel number should be revealed after entering the room"
        );
    }

    // --- PlayerView hides unrevealed jewel numbers ---

    #[test]
    fn player_view_hides_unrevealed_jewel_numbers() {
        use crate::observation::PlayerView;
        let game = Game::new(0, Variant::ADVANCED, 4);
        let legal = game.legal_actions();
        let view = PlayerView::from_state(&game.state, 0, &legal);
        for ns in &view.node_states {
            assert_eq!(
                ns.jewel_number, 0,
                "node_states in PlayerView must never expose hidden jewel numbers"
            );
        }
    }
}
