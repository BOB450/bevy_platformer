use std::collections::HashMap;

use bevy::prelude::*;

fn main() {
    App::new()
    .add_plugins(DefaultPlugins)
    .add_plugin(bevy_editor_pls::prelude::EditorPlugin)
    .add_startup_system(spawn_cam)
    .add_startup_system(spawn_player)
    .add_system(animate_sprite)
    .add_system(move_player)
    .add_system(change_player_animation)
    .init_resource::<PlayerAnimations>()
    .add_system(player_fall)
    .add_system(player_jump)
    .add_system(ground_detection)
    .run()
}

fn spawn_cam(
    mut commands: Commands,
) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
struct Player;

fn spawn_player(
    mut commands: Commands,
    animaitons: Res<PlayerAnimations>,
) {
    let Some((texture_atlas, animation)) = animaitons.get(Animation::Idle) else {error!("Failed to find animation: Idle"); return;};
    commands.spawn((SpriteSheetBundle {
        texture_atlas,
        sprite: TextureAtlasSprite {index: 0, ..Default::default()},
        ..Default::default()
    }, Player,
    animation,
    FrameTime(0.0),
    Grounded(true),
    ));
}

#[derive(Component, Clone, Copy)]
struct SpriteAnimation {
    len: usize,
    frame_time: f32,
}

#[derive(Component)]
struct FrameTime(f32);

fn animate_sprite(
    mut animations: Query<(&mut TextureAtlasSprite, &SpriteAnimation, &mut FrameTime)>,
    time: Res<Time>,
) {
    for (mut sprite, animation, mut frame_time) in animations.iter_mut() {
        frame_time.0 += time.delta_seconds();
        if frame_time.0 > animation.frame_time {
            let frames = (frame_time.0 / animation.frame_time) as usize;
            sprite.index += frames;
            if sprite.index >= animation.len {
                sprite.index %= animation.len;
            }
            frame_time.0 -= animation.frame_time;
        }
    }
}

const MOVE_SPEED: f32 = 100.;

fn move_player(
    mut commands: Commands,
    mut player: Query<(Entity, &mut Transform), With<Player>>,
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
) {
    let (entity, mut player) = player.single_mut();
    if input.any_just_pressed([KeyCode::W, KeyCode::Up, KeyCode::Space]) {
        commands.entity(entity).insert(Jump(100.));
    } else if input.any_pressed([KeyCode::A, KeyCode::Left]) {
        player.translation.x -= MOVE_SPEED * time.delta_seconds();
    } else if input.any_pressed([KeyCode::D, KeyCode::Right]) {
        player.translation.x += MOVE_SPEED * time.delta_seconds();
    }
}

fn change_player_animation(
    mut player: Query<(&mut Handle<TextureAtlas>, &mut SpriteAnimation, &mut TextureAtlasSprite), With<Player>>,
    player_jump: Query<(Option<&Jump>, &Grounded), With<Player>>,
    input: Res<Input<KeyCode>>,
    animaitons: Res<PlayerAnimations>,
) {
    let (mut atlas, mut animation, mut sprite) = player.single_mut();
    let (jump, grounded) = player_jump.single();
    if input.any_just_pressed([KeyCode::A, KeyCode::Left]) {
        sprite.flip_x = true;
    } else if input.any_just_pressed([KeyCode::D, KeyCode::Right])
    && !input.any_pressed([KeyCode::A, KeyCode::Left]) {
        sprite.flip_x = false;
    } else if input.any_just_released([KeyCode::A, KeyCode::Left])
    && !input.any_pressed([KeyCode::A, KeyCode::Left])
    && input.any_pressed([KeyCode::D, KeyCode::Right]) {
        sprite.flip_x = false;
    }
    
    let set = 
    //Jumping if jump
    if jump.is_some() {
        Animation::Jump
    //Falling if no on ground
    } else if !grounded.0 {
        Animation::Fall
    // if any move keys pressed set run sprite
    } else if input.any_pressed([KeyCode::A, KeyCode::Left, KeyCode::D, KeyCode::Right]) {
        Animation::Run
    } else {
        Animation::Idle
    };

    let Some((new_atlas, new_animaiton)) = animaitons.get(set) else {error!("No Animation Jump Loaded"); return;};
    *atlas = new_atlas;
    sprite.index %= new_animaiton.len;
    *animation = new_animaiton;
}

#[derive(Resource)]
struct PlayerAnimations {
    map: HashMap<Animation, (Handle<TextureAtlas>, SpriteAnimation)>,
}

impl FromWorld for PlayerAnimations {
    fn from_world(world: &mut World) -> Self {
        let mut map = PlayerAnimations {map: HashMap::new()};
        let asset_server = world.resource::<AssetServer>();
        let idel_atlas = TextureAtlas::from_grid(
            asset_server.load("Main Characters/Mask Dude/Idle (32x32).png"),
            Vec2::splat(32.),
            11, 1, None, None);
        let run_atlas = TextureAtlas::from_grid(
            asset_server.load("Main Characters/Mask Dude/Run (32x32).png"),
            Vec2::splat(32.),
            12, 1, None, None);
        let jump_atlas = TextureAtlas::from_grid(
            asset_server.load("Main Characters/Mask Dude/Jump (32x32).png"),
            Vec2::splat(32.),
            1, 1, None, None);
        let fall_atlas = TextureAtlas::from_grid(
            asset_server.load("Main Characters/Mask Dude/Fall (32x32).png"),
            Vec2::splat(32.),
            1, 1, None, None);
        
        let mut texture_atles = world.resource_mut::<Assets<TextureAtlas>>();
        
        map.add(Animation::Idle, texture_atles.add(idel_atlas), SpriteAnimation { len: 11, frame_time: 1./10. });
        map.add(Animation::Run, texture_atles.add(run_atlas), SpriteAnimation { len: 12, frame_time: 1./10. });
        map.add(Animation::Jump, texture_atles.add(jump_atlas), SpriteAnimation { len: 1, frame_time: 1. });
        map.add(Animation::Fall, texture_atles.add(fall_atlas), SpriteAnimation { len: 1, frame_time: 1. });

        map
    }
}

impl PlayerAnimations {
    fn add(&mut self, id: Animation, handle: Handle<TextureAtlas>, animation: SpriteAnimation) {
        self.map.insert(id, (handle, animation));
    }
    fn get(&self, id: Animation) -> Option<(Handle<TextureAtlas>, SpriteAnimation)> {
        self.map.get(&id).cloned()
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
enum Animation {
    Run,
    Idle,
    Jump,
    Fall,
}

#[derive(Component)]
struct Jump(f32);

const FALL_SPEED: f32 = 98.0;

fn player_jump(
    mut commands: Commands,
    mut player: Query<(Entity, &mut Transform, &mut Jump), With<Player>>,
    time: Res<Time>,
) {
    let Ok((player, mut transform,mut jump)) = player.get_single_mut() else {return;};
    let jump_power = (time.delta_seconds() * FALL_SPEED * 2.).min(jump.0);
    jump.0 -= jump_power;
    transform.translation.y += jump_power;
    if jump.0 == 0. {
        commands.entity(player).remove::<Jump>();
    }
}

fn player_fall(
    mut player: Query<&mut Transform, (With<Player>, Without<Jump>)>,
    time: Res<Time>,
) {
    let Ok(mut player) = player.get_single_mut() else {return;};
    if player.translation.y > 0.0 {
        player.translation.y -= time.delta_seconds() * FALL_SPEED;
        if player.translation.y < 0.0 {
            player.translation.y = 0.0
        }
    }
}

#[derive(Component)]
struct Grounded(bool);

fn ground_detection(
    mut player: Query<(&Transform, &mut Grounded), With<Player>>,
    mut last: Local<Transform>,
) {
    let (pos,mut on_ground) = player.single_mut();

    let current = if pos.translation.y == last.translation.y {
        true
    } else {
        false
    };

    if current != on_ground.0 {
        on_ground.0 = current;
    }

    *last = *pos;
}