#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod board;
mod client;
mod game;
mod message;
mod renderer;
mod server;
mod world;

use macroquad::audio::{PlaySoundParams, play_sound, stop_sound};
use macroquad::color::BLACK;
use macroquad::input::{KeyCode, get_char_pressed, is_key_down, is_key_pressed};
use macroquad::main as macroquad_main;
use macroquad::window::{Conf, clear_background, next_frame};

use assets::Assets;
use game::Game;
use game::Input;

fn window_conf() -> Conf {
    Conf {
        window_title: "ograCk!".to_owned(),
        window_width: game::WINDOW_WIDTH as i32,
        window_height: game::WINDOW_HEIGHT as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad_main(window_conf)]
async fn main() {
    let assets = Assets::load().await;
    let mut game = Game::new();
    play_sound(
        &assets.intro_music,
        PlaySoundParams {
            looped: true,
            volume: 1.0,
        },
    );
    let mut intro_music_playing = true;

    loop {
        clear_background(BLACK);

        let input = lire_entrees();
        if input.escape {
            break;
        }

        game.update(&input);
        if intro_music_playing && game.get_state() == game::GameState::Playing {
            stop_sound(&assets.intro_music);
            intro_music_playing = false;
        }
        renderer::draw(&game, &assets);

        next_frame().await;
    }
}

/// Associe les entrées réelles à un nouvel état des entrées
fn lire_entrees() -> Input {
    Input {
        start: is_key_pressed(KeyCode::Space),
        enter: is_key_pressed(KeyCode::Enter),

        left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
        right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
        up: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
        down: is_key_down(KeyCode::Down) || is_key_down(KeyCode::S),
        ready: is_key_pressed(KeyCode::Enter),

        backspace: is_key_pressed(KeyCode::Backspace),
        tab: is_key_pressed(KeyCode::Tab),
        character: get_char_pressed(),

        escape: is_key_pressed(KeyCode::Escape),
    }
}
