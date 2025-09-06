use hnefatafl_copenhagen::{board::BoardSize, game_tree::Tree};

fn main() {
    let mut tree = Tree::new(BoardSize::_11);
    let _ = tree.monte_carlo_tree_search(10);
}
