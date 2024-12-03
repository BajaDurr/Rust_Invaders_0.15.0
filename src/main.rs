/*
Project by https://github.com/jeremychone-channel/rust-invaders
Modified by David Krotzer
Last Modified: 12/03/2024

Files Modified: main.rs, player.rs, components.rs
Added Assets: player_a_01_dimmed.png, *.ogg 

*** PERSONAL PROJECT ( NOT FOR REDISTRIBUTION ) ***
Programming Language: Rust
Game Engine: Bevy

Bevy version: 0.15.0

--Notes for Submission--

Not all FSM's fully implemented 

Finished: PlayerState, AudioState

Partial: GameState, MovementState

*/



#![allow(unused)] // silence unused warnings while exploring (to comment out)

//packages
use bevy::audio::Volume;
use bevy::color::palettes::css::WHITE;
use bevy::ecs::world;
use bevy::math::bounding::IntersectsVolume;
use bevy::math::{bounding::Aabb2d, Vec3Swizzles};
use bevy::{prelude::*, state};
use bevy::window::PrimaryWindow;
//crates
use components::{
	Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, FromEnemy, FromPlayer, Laser, Movable,
	Player, SpriteSize, Velocity,
};
use enemy::EnemyPlugin;
use player::PlayerPlugin;
use std::collections::HashSet;

//mods (inheritance)
mod components;
mod enemy;
mod player;


// region:    --- Asset Constants

const PLAYER_SPRITE: &str = "player_a_01.png"; // Normal sprite
const PLAYER_DIMMED_SPRITE: &str = "player_a_01_dimmed.png"; // Dimmed sprite (for invincibility state)
const PLAYER_SIZE: (f32, f32) = (144., 75.);
const PLAYER_LASER_SPRITE: &str = "laser_a_01.png";
const PLAYER_LASER_SIZE: (f32, f32) = (9., 54.);

const ENEMY_SPRITE: &str = "enemy_a_01.png";
const ENEMY_SIZE: (f32, f32) = (144., 75.);
const ENEMY_LASER_SPRITE: &str = "laser_b_01.png";
const ENEMY_LASER_SIZE: (f32, f32) = (17., 55.);

const EXPLOSION_SHEET: &str = "explo_a_sheet.png";
const EXPLOSION_LEN: usize = 16;

const SPRITE_SCALE: f32 = 0.5;

// endregion: --- Asset Constants

// region:    --- Game Constants

const BASE_SPEED: f32 = 500.;

const PLAYER_RESPAWN_DELAY: f64 = 2.;
const ENEMY_MAX: u32 = 3;
const FORMATION_MEMBERS_MAX: u32 = 2;

const SCREEN_WIDTH: f32 = 598.0;
const SCREEN_HEIGHT: f32 = 676.0;

// endregion: --- Game Constants

// region:    --- Resources

#[derive(Component)]
struct Invincible {
    timer: Timer,
}

#[derive(Resource)]
pub struct WinSize {
	pub w: f32,
	pub h: f32,
}

#[derive(Resource)]
struct GameTextures {
	player: Handle<Image>,
	player_dimmed: Handle<Image>,
	player_laser: Handle<Image>,
	enemy: Handle<Image>,
	enemy_laser: Handle<Image>,
	explosion_layout: Handle<TextureAtlasLayout>,
	explosion_texture: Handle<Image>,
}

#[derive(Resource)]
struct EnemyCount(u32);

// TODO
#[derive(Debug, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    Playing,
    Paused,
    GameOver,
}

#[derive(Component)]
struct PauseMenu;

#[derive(Resource)]
struct AudioState {
    bgm: Handle<AudioSource>,      // Background music
    shoot_sfx: Handle<AudioSource>, // Shoot sound effect
    boom_sfx: Handle<AudioSource>,  // Explosion sound effect
    gameover_sfx: Handle<AudioSource>, // Game over sound effect

}
// (TODO) Include invincibility state
#[derive(Resource)]
struct PlayerState {
	on: bool,       // alive
	last_shot: f64, // -1 if not shot
}
impl Default for PlayerState {
	fn default() -> Self {
		Self {
			on: false,
			last_shot: -1.,
		}
	}
}

impl PlayerState {
	pub fn shot(&mut self, time: f64) {
		self.on = false;
		self.last_shot = time;
	}
	pub fn spawned(&mut self) {
		self.on = true;
		self.last_shot = -1.;
	}
}
// endregion: --- Resources

fn main() {
	App::new()	//main app builder
		.insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.04)))
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Rust Invaders!".into(),
				resolution: (SCREEN_HEIGHT, SCREEN_WIDTH).into(), //598/676
				resizable: false,	// lock window 
				..Default::default()
			}),
			..Default::default()
		}))
		.add_plugins(PlayerPlugin)
		.add_plugins(EnemyPlugin)
		.add_systems(OnEnter(GameState::Playing), setup_playing)
        .add_systems(Update, game_logic.run_if(in_state(GameState::Playing))) //logic
		/*Unfinished States */
	//	.add_systems(OnEnter(GameState::Paused), enter_pause_state) // Enter Paused state    
	//  .add_systems(Update, handle_pause_input.run_if(in_state(GameState::Paused))) // Run input handling in Pause
	//	.add_systems(OnExit(GameState::Paused), exit_pause_state) // Exit Paused state
    //  .add_systems(OnEnter(GameState::GameOver), game_over_screen)
		.add_systems(Startup, setup_system)
		.add_systems(Update, movable_system)
		.add_systems(Update, player_laser_hit_enemy_system)
		.add_systems(Update, enemy_laser_hit_player_system)
		.add_systems(Update, explosion_to_spawn_system)
		.add_systems(Update, explosion_animation_system)
		.add_systems(Update, invincibility_timer_system)
		.add_systems(Update, invincibility_sprite_switch_system) 
		.run();
}


//main setup for assets and textures
fn setup_system(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
	query: Query<&Window, With<PrimaryWindow>>,
) {
	// camera
	commands.spawn(Camera2d);

	// capture window size
	let Ok(primary) = query.get_single() else {
		return;
	};
	let (win_w, win_h) = (primary.width(), primary.height());

	// add WinSize resource
	let win_size = WinSize { w: win_w, h: win_h };
	commands.insert_resource(win_size);

	// create explosion texture atlas
	let texture_handle = asset_server.load(EXPLOSION_SHEET);
	let texture_atlas = TextureAtlasLayout::from_grid(UVec2::new(64, 64), 4, 4, None, None);
	let explosion_layout = texture_atlases.add(texture_atlas);

	// add GameTextures resource
	let game_textures = GameTextures {
		player: asset_server.load(PLAYER_SPRITE),
		player_dimmed: asset_server.load(PLAYER_DIMMED_SPRITE),
		player_laser: asset_server.load(PLAYER_LASER_SPRITE),
		enemy: asset_server.load(ENEMY_SPRITE),
		enemy_laser: asset_server.load(ENEMY_LASER_SPRITE),
		explosion_layout,
		explosion_texture: texture_handle,

	};
	commands.insert_resource(game_textures);
	commands.insert_resource(EnemyCount(0));

	// audio
	let bgm = asset_server.load("lasagna.ogg");            // Background Music
    let shoot_sfx = asset_server.load("pew.ogg"); // Shoot Sound Effect
    let boom_sfx = asset_server.load("deltarune_boom.ogg");   // Boom Sound Effect
    let gameover_sfx = asset_server.load("gameover.ogg"); // Game Over Sound Effect"");   // TODO

	commands.insert_resource(AudioState {
        bgm,
        shoot_sfx,
        boom_sfx,
        gameover_sfx, // TODO
    });

	// Spawn background music player

    commands.spawn(( (
		AudioPlayer::<AudioSource>( asset_server.load("lasagna.ogg"))), 
		PlaybackSettings::LOOP.with_volume(Volume::new(0.8)),	// loops bgm and lowers volume to 80% of source
	));
	
}

// playermovement
fn movable_system(
	mut commands: Commands,
	time: Res<Time>,
	win_size: Res<WinSize>,
	mut query: Query<(Entity, &Velocity, &mut Transform, &Movable)>,
) {
	let delta = time.delta_secs();

	for (entity, velocity, mut transform, movable) in &mut query {
		let translation = &mut transform.translation;
		translation.x += velocity.x * delta * BASE_SPEED;
		translation.y += velocity.y * delta * BASE_SPEED;

		if movable.auto_despawn {
			// despawn when out of screen
			const MARGIN: f32 = 200.;
			if translation.y > win_size.h / 2. + MARGIN
				|| translation.y < -win_size.h / 2. - MARGIN
				|| translation.x > win_size.w / 2. + MARGIN
				|| translation.x < -win_size.w / 2. - MARGIN
			{
				commands.entity(entity).despawn();
			}
		}
	}
}

#[allow(clippy::type_complexity)] // for the Query types.
fn player_laser_hit_enemy_system(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut enemy_count: ResMut<EnemyCount>,
	laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromPlayer>)>,
	enemy_query: Query<(Entity, &Transform, &SpriteSize), With<Enemy>>,
	invincible_query: Query<&Invincible, With<Player>>,
) {
	let mut despawned_entities: HashSet<Entity> = HashSet::new();

	// iterate through the lasers
	for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
		if despawned_entities.contains(&laser_entity) {
			continue;
		}

		let laser_scale = laser_tf.scale.xy();

		// iterate through the enemies
		for (enemy_entity, enemy_tf, enemy_size) in enemy_query.iter() {
			if despawned_entities.contains(&enemy_entity)
				|| despawned_entities.contains(&laser_entity)
			{
				continue;
			}

			let enemy_scale = enemy_tf.scale.xy();

			// determine if collision
			let collision = Aabb2d::new(
				laser_tf.translation.truncate(),
				(laser_size.0 * laser_scale) / 2.,
			)
			.intersects(&Aabb2d::new(
				enemy_tf.translation.truncate(),
				(enemy_size.0 * enemy_scale) / 2.,
			));

			// perform collision
			if collision {
				// remove the enemy
				commands.entity(enemy_entity).despawn();
				despawned_entities.insert(enemy_entity);
				enemy_count.0 -= 1;

				// remove the laser
				commands.entity(laser_entity).despawn();
				despawned_entities.insert(laser_entity);

				// spawn the explosion sfx
				commands.spawn(( (
				AudioPlayer::<AudioSource>( asset_server.load("deltarune_boom.ogg")),
				PlaybackSettings::ONCE.with_volume(Volume::new(0.3)))));
				// spawn the explosionToSpawn
				commands.spawn(ExplosionToSpawn(enemy_tf.translation));
			}
		}
	}
}

#[allow(clippy::type_complexity)] // for the Query types.
fn enemy_laser_hit_player_system(
	mut commands: Commands,
	mut player_state: ResMut<PlayerState>,
	time: Res<Time>,
	asset_server: Res<AssetServer>,
	laser_query: Query<(Entity, &Transform, &SpriteSize), (With<Laser>, With<FromEnemy>)>,
	mut player_query: Query<(Entity, &Transform, &SpriteSize, Option<&mut Invincible>), With<Player>>,
) {
	if let Ok((player_entity, player_tf, player_size, invincible)) = player_query.get_single_mut() {
	   // Check if player is invincible
	   if let Some(mut invincible) = invincible {
		invincible.timer.tick(time.delta());
		if !invincible.timer.finished() {
			return; // Skip collision if invincibility is active
		} else {
			commands.entity(player_entity).remove::<Invincible>(); // Remove invincibility once timer expires
		}
	}

	let player_scale = player_tf.scale.xy();

	for (laser_entity, laser_tf, laser_size) in laser_query.iter() {
		let laser_scale = laser_tf.scale.xy();

		// determine if collision
		let collision = Aabb2d::new(
			laser_tf.translation.truncate(),
			(laser_size.0 * laser_scale) / 2.,
		)
		.intersects(&Aabb2d::new(
			player_tf.translation.truncate(),
			(player_size.0 * player_scale) / 2.,
		));

			// perform the collision
			if collision {
				// remove the player
				commands.entity(player_entity).despawn();
				player_state.shot(time.elapsed_secs_f64());

				// remove the laser
				commands.entity(laser_entity).despawn();

				// spawn the explosionToSpawn
				commands.spawn(ExplosionToSpawn(player_tf.translation));
				// spawn the explosion sfx
				commands.spawn(( (
					AudioPlayer::<AudioSource>( asset_server.load("deltarune_boom.ogg")),
					PlaybackSettings::ONCE.with_volume(Volume::new(0.3)))));

				break;
			}
		}
	}
}

fn explosion_to_spawn_system(
	mut commands: Commands,
	game_textures: Res<GameTextures>,
	query: Query<(Entity, &ExplosionToSpawn)>,
) {
	for (explosion_spawn_entity, explosion_to_spawn) in query.iter() {
		// spawn the explosion sprite
		commands
			.spawn((
				Sprite {
					image: game_textures.explosion_texture.clone(),
					texture_atlas: Some(TextureAtlas {
						layout: game_textures.explosion_layout.clone(),
						index: 0,
					}),
					..Default::default()
				},
				Transform::from_translation(explosion_to_spawn.0),
			))
			.insert(Explosion)
			.insert(ExplosionTimer::default());

		// despawn the explosionToSpawn
		commands.entity(explosion_spawn_entity).despawn();
	}
}

fn explosion_animation_system(
	mut commands: Commands,
	time: Res<Time>,
	mut query: Query<(Entity, &mut ExplosionTimer, &mut Sprite), With<Explosion>>,
) {
	for (entity, mut timer, mut sprite) in &mut query {
		timer.0.tick(time.delta());
		if timer.0.finished() {
			if let Some(texture) = sprite.texture_atlas.as_mut() {
				texture.index += 1;
				if texture.index >= EXPLOSION_LEN {
					commands.entity(entity).despawn();
				}
			}
		}
	}
}

// state when first spawned (1s)
fn invincibility_timer_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Invincible)>,
) {
    for (entity, mut invincible) in query.iter_mut() {
        invincible.timer.tick(time.delta());
        if invincible.timer.finished() {
            commands.entity(entity).remove::<Invincible>();
        }
    }
}

// handles the switch from dimmed to normal player sprite
fn invincibility_sprite_switch_system(
    mut query: Query<(Entity, &mut Sprite, Option<&Invincible>), With<Player>>,
    game_textures: Res<GameTextures>,
) {
    for (entity, mut sprite, invincible) in query.iter_mut() {
        // Check if the player has the Invincible component
        if invincible.is_some() {
            // If invincible, switch to the dimmed sprite
            sprite.image = game_textures.player_dimmed.clone();
        } else {
            // If not invincible, revert to the normal sprite
            sprite.image = game_textures.player.clone();
        }
    }
}

// how that movement system state would work  (Event Listener (i.e. Playing --Key P--> Paused ))
fn handle_game_state_input(
    mut state: ResMut<NextState<GameState>>,
    kb: Res<ButtonInput<KeyCode>>,
) {
    if kb.just_pressed(KeyCode::KeyP) {
        // Toggle between Playing and Paused
        state.set(GameState::Paused);
    } else if kb.just_pressed(KeyCode::KeyR) {
        state.set(GameState::Playing);
    } else if kb.just_pressed(KeyCode::KeyG) {
        state.set(GameState::GameOver);
    }
}

fn setup_playing(mut commands: Commands) {
    println!("Game is now Playing");
    // Initialize or reset gameplay
}

fn game_over_screen() {
    println!("Game Over!");
    // (TODO) game over logic should go here
}

fn game_logic() {
    // Core gameplay logic
    println!("Playing the game...");
}

fn pause_game() {
    println!("Game is Paused.");
}

// (TODO) Sends Playing state to Paused state
fn enter_pause_state(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("Game is now Paused");

    // pause menu UI
	// TODO: Update to 0,15.0 (Not Working)
    commands.spawn((
        Text::new("Paused\nPress R to Resume\nPress Q to Quit"),
		//	TextLayout::new_with_justify(JustifyText::Center),  //???
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 50.0,
				font_smoothing: default(),
                
                
            },
			//TextColor(WHITE.into(), //may be depricated
		)
        
     //   PauseMenu, // Marker component (identify)
    );
}

fn exit_pause_state(mut commands: Commands, query: Query<Entity, With<PauseMenu>>) {
    println!("Exiting Pause State");

    // Despawn the pause menu UI
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// TODO: Unfunctional (Like other fn, has issues with moving state to state)
fn handle_pause_input(
    mut state: ResMut<NextState<GameState>>,
	kb: Res<ButtonInput<KeyCode>>,
) {
    if kb.just_pressed(KeyCode::KeyR) {
        // Resume the game
        state.set(GameState::Playing);
    } else if kb.just_pressed(KeyCode::KeyQ) {
        // Quit the game
        state.set(GameState::GameOver);
    }
}
