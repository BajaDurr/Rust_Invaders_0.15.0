use crate::components::{FromPlayer, Laser, Movable, Player, SpriteSize, Velocity};
use crate::{
	GameTextures, Invincible, PlayerState, WinSize, PLAYER_LASER_SIZE, PLAYER_RESPAWN_DELAY, PLAYER_SIZE, SCREEN_WIDTH, SPRITE_SCALE
};
use bevy::audio::Volume;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use std::time::Duration;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(PlayerState::default())
			.add_systems(
				Update,
				player_spawn_system.run_if(on_timer(Duration::from_secs_f32(0.5))),
			)
			.add_systems(Update, player_keyboard_event_system)
			.add_systems(Update, player_fire_system);
	}
}

fn player_spawn_system(
	mut commands: Commands,
	mut player_state: ResMut<PlayerState>,
	time: Res<Time>,
	game_textures: Res<GameTextures>,
	win_size: Res<WinSize>,
) {
	let now = time.elapsed_secs_f64();
	let last_shot = player_state.last_shot;

	if !player_state.on && (last_shot == -1. || now > last_shot + PLAYER_RESPAWN_DELAY) {
		// add player
		let bottom = -win_size.h / 2.;
		commands
			.spawn((
				Sprite::from_image(game_textures.player.clone()),
				Transform {
					translation: Vec3::new(
						0.,
						bottom + PLAYER_SIZE.1 / 2. * SPRITE_SCALE + 5.,
						10.,
					),
					scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
					..Default::default()
				},
			))
			.insert(Player)
			.insert(SpriteSize::from(PLAYER_SIZE))
			.insert(Movable { auto_despawn: false })
			.insert(Velocity { x: 0., y: 0. })
			.insert(Invincible {
				timer: Timer::from_seconds(1.0, TimerMode::Once)
			});

		player_state.spawned();
	}
}

fn player_fire_system(
	mut commands: Commands,
	kb: Res<ButtonInput<KeyCode>>,
	asset_server: Res<AssetServer>,
	game_textures: Res<GameTextures>,
	query: Query<&Transform, With<Player>>,
) {
	if let Ok(player_tf) = query.get_single() {
		if kb.just_pressed(KeyCode::Space) {

		

			let (x, y) = (player_tf.translation.x, player_tf.translation.y);
			let x_offset = PLAYER_SIZE.0 / 2. * SPRITE_SCALE - 5.;

			

			let mut spawn_laser = |x_offset: f32| {
				commands
					.spawn((
						(AudioPlayer::<AudioSource>( asset_server.load("pew.ogg"))), 
						PlaybackSettings::ONCE.with_volume(Volume::new(0.3)),
						Sprite::from_image(game_textures.player_laser.clone()),
						Transform {
							translation: Vec3::new(x + x_offset, y + 15., 0.),
							scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
							..Default::default()
						},
					))
					.insert(Laser)
					.insert(FromPlayer)
					.insert(SpriteSize::from(PLAYER_LASER_SIZE))
					.insert(Movable { auto_despawn: true })
					.insert(Velocity { x: 0., y: 1. });
			};

			spawn_laser(x_offset);
			spawn_laser(-x_offset);
		}
	}
}

fn player_keyboard_event_system(
    kb: Res<ButtonInput<KeyCode>>,
    player_query: Query<&Transform, With<Player>>,
    mut velocity_query: Query<&mut Velocity, With<Player>>,
) {
    if let (Ok(player_tf), Ok(mut velocity)) = (player_query.get_single(), velocity_query.get_single_mut()) {
        let player_x = player_tf.translation.x;
        let half_screen_width = SCREEN_WIDTH / 2.0;

        velocity.x = if kb.pressed(KeyCode::ArrowLeft) || kb.pressed(KeyCode::KeyA) {
            if player_x <= -half_screen_width {
                0.0 // Prevent moving left if at the left screen boundary
            } else {
                -1.0
            }
        } else if kb.pressed(KeyCode::ArrowRight) || kb.pressed(KeyCode::KeyD) {
            if player_x >= half_screen_width {
                0.0 // Prevent moving right if at the right screen boundary
            } else {
                1.0
            }
        } else {
            0.0
        };
    }
}