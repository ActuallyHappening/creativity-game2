//! Thrust is processed in various ways.
//!
//! Braking:
//! - Tries to stop player in space across all dimensions
//! - Ignores all other player inputs while braked

use crate::utils::*;

use super::ControllablePlayer;

mod helpers;
use bevy::{
	ecs::{query::WorldQuery, system::SystemParam},
	utils::HashSet,
};
use helpers::*;

mod stages;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
pub use stages::*;

mod info_gathering;
pub use info_gathering::*;
mod info_processors;
pub use info_processors::*;
mod info_enactors;
pub use info_enactors::*;
mod thrust_reactions;
pub use thrust_reactions::*;

pub mod types;

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Thrust<S: ThrustStage> {
	/// Positive is forward obviously
	pub forward: <S as self::ThrustStage>::DimensionType,

	pub up: <S as self::ThrustStage>::DimensionType,

	pub right: <S as self::ThrustStage>::DimensionType,

	pub turn_right: <S as self::ThrustStage>::DimensionType,

	/// Upwards is positive
	pub tilt_up: <S as self::ThrustStage>::DimensionType,

	/// Right is positive
	pub roll_right: <S as self::ThrustStage>::DimensionType,

	_stage: PhantomData<S>,
}

pub trait ThrustStage {
	type DimensionType: std::fmt::Debug + Clone + Serialize + DeserializeOwned;
}

impl<D, T> Default for Thrust<D>
where
	D: ThrustStage<DimensionType = T> + std::default::Default,
	D::DimensionType: Default,
{
	fn default() -> Self {
		Thrust::<D> {
			forward: T::default(),
			up: T::default(),
			right: T::default(),

			turn_right: T::default(),
			tilt_up: T::default(),
			roll_right: T::default(),
			_stage: PhantomData,
		}
	}
}

/// Combines the normal and relative thrusts into the final thrust vectors,
/// and saves the necessary information to various places including in the [MainPlayer] component
#[allow(clippy::type_complexity)]
pub fn save_thrust_stages(
	relative_strength: Thrust<RelativeStrength>,
	normal_vectors: Thrust<BasePositionNormalVectors>,
	max: Thrust<ForceFactors>,
	thrust_responses: Thrust<ThrustReactionsStage>,

	mut player: &mut ControllablePlayer,
) -> Thrust<FinalVectors> {
	let final_vectors = normal_vectors * relative_strength.clone() * max;

	player.relative_strength = relative_strength;
	player.thrust_responses = thrust_responses;

	final_vectors
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct PlayerMovementQuery<'w> {
	player: &'w mut ControllablePlayer,
	external_force: &'w mut ExternalForce,
	transform: &'w Transform,
	velocity: &'w Velocity,
}

pub fn authoritative_player_movement(
	mut player_bundle: Query<PlayerMovementQuery>,
	mut player_inputs: EventReader<FromClient<PlayerInputs>>,
	time: Res<Time>,
) {
	let mut player_inputs = player_inputs.iter().collect::<Vec<_>>();
	player_inputs.reverse();
	// keep only the latest event per .client_id
	let mut seen_ids = HashSet::new();
	let mut to_delete = Vec::new();
	for (i, e) in player_inputs.iter().enumerate() {
		if seen_ids.contains(&e.client_id) {
			to_delete.push(i);
			continue;
		}
		seen_ids.insert(e.client_id);
	}
	// delete in reverse order so indices don't get messed up
	for i in to_delete.into_iter().rev() {
		player_inputs.remove(i);
	}

	for FromClient {
		client_id,
		event: move_request,
	} in player_inputs
	{
		let player = player_bundle
			.iter_mut()
			.find(|player| player.player.network_id == *client_id);

		if let Some(player) = player {
			let base_normals = get_base_normal_vectors(player.transform);

			let relative_velocity_magnitudes =
				calculate_relative_velocity_magnitudes(&base_normals, player.velocity);

			let (thrust_reactions, force_factors) = process_inputs(
				move_request,
				&player.player.artificial_friction_flags,
				relative_velocity_magnitudes,
			);

			let relative_strength = get_relative_strengths(
				thrust_reactions.clone().into_generic_flags() * base_normals.clone(),
				max_velocity_magnitudes(),
				player.velocity,
			);

			let final_thrust = save_thrust_stages(
				relative_strength,
				base_normals,
				force_factors,
				thrust_reactions,
				player.player.into_inner(),
			);

			apply_thrust(final_thrust, player.external_force.into_inner(), &time);
		} else {
			warn!("No player found in the world with id: {}", client_id);
		}
	}

	// let base_normal = get_base_normal_vectors(player_transform.single());

	// let raw_inputs = gather_input_flags(keyboard_input);

	// let (input_flags, force_factors) = process_inputs(raw_inputs, artificial_friction_flags, current_velocity);

	// let flagged_inputs = input_flags.clone().into_generic_flags() * base_normal.clone();
	// let relative_strengths = get_relative_strengths(
	// 	In((flagged_inputs, max_velocity_magnitudes())),
	// 	player_velocity,
	// );
	// let final_vectors = save_thrust_stages(
	// 	In((
	// 		relative_strengths,
	// 		base_normal,
	// 		force_factors,
	// 		input_flags,
	// 	)),
	// 	player_data,
	// );

	// apply_thrust(In(final_vectors), player_physics, time);
}

// #[bevycheck::system]
// pub fn manual_get_final_thrust(
// 	keyboard_input: Res<Input<KeyCode>>,
// 	player_pos: Query<&Transform, With<MainPlayer>>,
// 	player_current: Query<(&Velocity, &Transform), With<MainPlayer>>,
// 	player_save: Query<&mut MainPlayer, With<MainPlayer>>,
// ) -> Thrust<FinalVectors> {
// 	let normals = flag_normal_vectors(In(gather_player_movement(keyboard_input)), player_pos);
// 	let relative = get_relative_strengths(
// 		In((get_max_velocity_magnitudes(), normals.clone())),
// 		player_current,
// 	);

// 	save_thrust_stages(In((normals, relative, get_force_factors())), player_save)
// }
