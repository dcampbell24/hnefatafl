use hnefatafl_copenhagen::{game::Game, mcts::monte_carlo_tree_search};

fn main() {
    let game = Game::default();

    let _ = monte_carlo_tree_search(&game);
}
