// A player is about to be placed at a join: a newcomer is placed at a random spot, not at the world spawn.
fn smp_player_spawn_location(
    game: &mut crate::game::Game<'_>,
    player: u128,
    dimension: &mut String,
    position: &mut [f64; 3],
) {
    crate::game::smp_glue::place_start(game, player, true, dimension, position);
}
