// A dead player is about to respawn: one who starts at a random spot is placed there, not at the world spawn.
fn smp_player_respawn_position(
    game: &mut crate::game::Game<'_>,
    player: u128,
    dimension: &mut String,
    pos: &mut [f64; 3],
) {
    crate::game::smp_glue::place_start(game, player, false, dimension, pos);
}
