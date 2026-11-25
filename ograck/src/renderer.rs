use crate::assets::Assets;
use crate::game::{Game, GameState, HostJoinFocus};

use macroquad::color::{BLACK, Color, DARKGRAY, LIGHTGRAY, ORANGE, PINK, RED, SKYBLUE, WHITE};
use macroquad::math::vec2;
use macroquad::shapes::{draw_circle, draw_rectangle};
use macroquad::text::{TextParams, draw_text_ex, measure_text};
use macroquad::texture::{DrawTextureParams, draw_texture_ex};
use macroquad::time::get_time;
use macroquad::window::{screen_height, screen_width};

/// Dessine la scène courante 
/// 
/// L'affichage dépend de l'état du jeu et utilise les ressources fournies
/// comme argument.
pub fn draw(game: &Game, assets: &Assets) {
    match game.get_state() {
        GameState::Intro => {
            draw_intro(game, assets);
        }

        GameState::HostJoin => {
            draw_host_join(game, assets);
        }

        GameState::Lobby => {
            draw_lobby(game, assets);
        }

        GameState::Playing => {
            draw_playing(game, assets);
        }
    }
}

/// Dessine la scène d'introduction (écran titre)
fn draw_intro(game: &Game, assets: &Assets) {
    draw_cover_background(&assets.intro_background);

    let title_dims = measure_text("ograCk!", Some(&assets.frog_font), 90, 1.0);

    let title_x = (screen_width() - title_dims.width) / 2.0;
    let title_y = (screen_height() - title_dims.height) / 2.0 + title_dims.offset_y + 50.0;

    for (offset_x, offset_y) in [
        (-1.5, -1.5),
        (0.0, -1.5),
        (1.5, -1.5),
        (-1.5, 0.0),
        (1.5, 0.0),
        (-1.5, 1.5),
        (0.0, 1.5),
        (1.5, 1.5),
    ] {
        draw_text_ex(
            "ograCk!",
            title_x + offset_x,
            title_y + offset_y,
            TextParams {
                font: Some(&assets.frog_font),
                font_size: 90,
                color: BLACK,
                ..Default::default()
            },
        );
    }

    draw_text_ex(
        "ograCk!",
        title_x,
        title_y,
        TextParams {
            font: Some(&assets.frog_font),
            font_size: 90,
            color: WHITE,
            ..Default::default()
        },
    );

    let alpha = (get_time() * 4.0).sin() as f32 * 0.5 + 0.5;

    let fade_color = Color::new(1.0, 1.0, 1.0, alpha);

    let insert_dims = measure_text("Appuyer sur ENTER pour jouer", Some(&assets.comic_font), 20, 1.0);

    draw_text_with_outline(
        "Appuyer sur ENTER pour jouer",
        (screen_width() - insert_dims.width) / 2.0,
        screen_height() - 3.0 * insert_dims.height,
        &assets.comic_font,
        20,
        fade_color,
    );

    let fade_alpha = (1.0 - game.intro_elapsed() / 2.0).clamp(0.0, 1.0);
    if fade_alpha > 0.0 {
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, fade_alpha),
        );
    }
}

/// Dessine la scène héberger/rejoindre
fn draw_host_join(game: &Game, assets: &Assets) {
    draw_cover_background(&assets.host_join_background);

    // étiquette identifiant, texte identifiant, boîte identifiant, héberger, joindre
    let colors = match game.host_join_focus {
        HostJoinFocus::Identifier => (WHITE, WHITE, ORANGE, DARKGRAY, DARKGRAY),
        HostJoinFocus::Host => (DARKGRAY, LIGHTGRAY, DARKGRAY, ORANGE, DARKGRAY),
        HostJoinFocus::Join => (DARKGRAY, LIGHTGRAY, DARKGRAY, DARKGRAY, ORANGE),
    };

    let id_label = "Choisissez votre identifiant";
    let id_label_dims = measure_text(id_label, Some(&assets.comic_font), 15, 1.0);

    draw_text_with_outline(
        id_label,
        (screen_width() - id_label_dims.width) / 2.0,
        screen_height() / 2.0 - 40.0,
        &assets.comic_font,
        15,
        colors.0,
    );

    let identifier = if game.my_id.is_empty() {
        String::from("aucun")
    } else {
        game.my_id.clone()
    };

    let identifier_dims = measure_text(&identifier, Some(&assets.comic_font), 25, 1.0);

    draw_rounded_filled_rectangle(
        (screen_width() - identifier_dims.width) / 2.0 - 10.0,
        screen_height() / 2.0 - 32.0,
        identifier_dims.width + 20.0,
        42.0,
        8.0,
        colors.2,
    );

    draw_text_with_outline(
        &identifier,
        (screen_width() - identifier_dims.width) / 2.0,
        screen_height() / 2.0,
        &assets.comic_font,
        25,
        colors.1,
    );

    if let Some(error) = &game.connection_error {
        let error_dims = measure_text(error, Some(&assets.comic_font), 18, 1.0);
        
        draw_text_with_outline(
            error,
            (screen_width() - error_dims.width) / 2.0,
            screen_height() / 4.0 + 40.0,
            &assets.comic_font,
            18,
            RED,
        );
    }

    let host_dims = measure_text("Héberger", Some(&assets.comic_font), 25, 1.0);

    let host_x = screen_width() / 2.0 - 100.0 - host_dims.width;
    let host_y = screen_height() / 4.0 * 3.0;

    draw_text_with_outline("Héberger", host_x, host_y, &assets.comic_font, 25, colors.3);

    let join_x = screen_width() / 2.0 + 100.0;

    draw_text_with_outline(
        "Rejoindre",
        join_x,
        host_y,
        &assets.comic_font,
        25,
        colors.4,
    );

    let guide_dims = measure_text(
        "TAB : suivant   ENTER : sélectionner",
        Some(&assets.comic_font),
        16,
        1.0,
    );

    draw_text_with_outline(
        "TAB : suivant   ENTER : sélectionner",
        (screen_width() - guide_dims.width) / 2.0,
        screen_height() - 40.0,
        &assets.comic_font,
        16,
        WHITE,
    );
}

/// Dessine du texte de couleur avec un contour noir
fn draw_text_with_outline(
    text: &str,
    x: f32,
    y: f32,
    font: &macroquad::text::Font,
    font_size: u16,
    color: Color,
) {
    let outline_color = Color::new(BLACK.r, BLACK.g, BLACK.b, color.a);

    for (offset_x, offset_y) in [
        (-1.0, -1.0),
        (0.0, -1.0),
        (1.0, -1.0),
        (-1.0, 0.0),
        (1.0, 0.0),
        (-1.0, 1.0),
        (0.0, 1.0),
        (1.0, 1.0),
    ] {
        draw_text_ex(
            text,
            x + offset_x,
            y + offset_y,
            TextParams {
                font: Some(font),
                font_size,
                color: outline_color,
                ..Default::default()
            },
        );
    }

    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font: Some(font),
            font_size,
            color,
            ..Default::default()
        },
    );
}

/// Dessine une image d'arrière-plan
fn draw_cover_background(texture: &macroquad::texture::Texture2D) {
    let scale = (screen_width() / texture.width()).max(screen_height() / texture.height());
    let background_width = texture.width() * scale;
    let background_height = texture.height() * scale;

    draw_texture_ex(
        texture,
        (screen_width() - background_width) / 2.0,
        (screen_height() - background_height) / 2.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(background_width, background_height)),
            ..Default::default()
        },
    );
}

/// Dessine la salle d'attente
fn draw_lobby(game: &Game, assets: &Assets) {
    draw_cover_background(&assets.lobby_background);

    let address_label = if game.is_host {
        "Partie hébergée à"
    } else {
        "Rejoindre le serveur à"
    };

    let address_label_dims = measure_text(address_label, Some(&assets.comic_font), 15, 1.0);

    draw_text_with_outline(
        address_label,
        80.0,
        screen_height() / 6.0 - (address_label_dims.height + 12.0),
        &assets.comic_font,
        15,
        WHITE,
    );

    draw_text_with_outline(
        if game.is_host {
            &game.host_ip
        } else {
            &game.input_ip
        },
        80.0,
        screen_height() / 6.0,
        &assets.comic_font,
        25,
        WHITE,
    );

    draw_text_with_outline(
        "Joueurs",
        80.0,
        screen_height() / 3.0,
        &assets.comic_font,
        22,
        WHITE,
    );

    let mut players: Vec<_> = game.lobby_players.iter().collect();
    players.sort_by_key(|(id_a, _)| *id_a);

    for (index, (id, player)) in players.iter().enumerate() {
        let role = if player.is_host { " (Hôte)" } else { "" };

        let status = if player.is_ready {
            "Prêt"
        } else {
            "En attente"
        };

        draw_text_with_outline(
            &format!("{id}{role} - {status}"),
            80.0,
            screen_height() / 3.0 + 38.0 * (index as f32 + 1.0),
            &assets.comic_font,
            18,
            if player.is_ready { SKYBLUE } else { PINK },
        );
    }

    if game.client.is_none() {
        draw_text_with_outline(
            if game.is_host {
                "ENTER : ouvrir la salle d'attente"
            } else {
                "ENTER : rejoindre la salle d'attente"
            },
            80.0,
            screen_height() - 55.0,
            &assets.comic_font,
            18,
            WHITE,
        );

        return;
    }

    draw_text_with_outline(
        if game.is_ready {
            "ENTER : pas prêt"
        } else {
            "ENTER : prêt"
        },
        80.0,
        screen_height() - 90.0,
        &assets.comic_font,
        18,
        WHITE,
    );

    if game.is_host {
        let all_ready = !game.lobby_players.is_empty()
            && game.lobby_players.values().all(|player| player.is_ready);
        
        draw_text_with_outline(
            if all_ready {
                "SPACE : lancer la partie"
            } else {
                "En attente que tous les joueurs soient prêts..."
            },
            80.0,
            screen_height() - 55.0,
            &assets.comic_font,
            18,
            if all_ready { ORANGE } else { WHITE },
        );
    }
}

/// Dessine la scène d'action
fn draw_playing(game: &Game, assets: &Assets) {
    if let Some(board) = &game.board {
        for y in 0..board.height {
            for x in 0..board.width {
                let texture = match board.is_obstacle(x, y) {
                    Some(true) => &assets.wall,
                    Some(false) => &assets.ground,
                    None => continue,
                };

                draw_texture_ex(
                    texture,
                    x as f32 * crate::game::TILE_SIZE as f32,
                    y as f32 * crate::game::TILE_SIZE as f32,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(
                            crate::game::TILE_SIZE as f32,
                            crate::game::TILE_SIZE as f32,
                        )),
                        ..Default::default()
                    },
                );
            }
        }

        let player_tile = (
            game.position[0].round() as usize,
            game.position[1].round() as usize,
        );

        if board.is_empty_tile(player_tile.0, player_tile.1) {
            let texture = if game.is_zombie {
                &assets.zombie
            } else {
                &assets.human
            };

            draw_texture_ex(
                texture,
                (player_tile.0 * crate::game::TILE_SIZE) as f32,
                (player_tile.1 * crate::game::TILE_SIZE) as f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(
                        crate::game::TILE_SIZE as f32,
                        crate::game::TILE_SIZE as f32,
                    )),
                    ..Default::default()
                },
            );
        }

        for (id, (x, y)) in &game.other_players {
            if !board.is_empty_tile(*x, *y) {
                continue;
            }

            let texture = if game
                .other_player_zombie_states
                .get(id)
                .copied()
                .unwrap_or(false)
            {
                &assets.zombie
            } else {
                &assets.human
            };

            draw_texture_ex(
                texture,
                *x as f32 * crate::game::TILE_SIZE as f32,
                *y as f32 * crate::game::TILE_SIZE as f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(
                        crate::game::TILE_SIZE as f32,
                        crate::game::TILE_SIZE as f32,
                    )),
                    ..Default::default()
                },
            );
        }
    }
}

/// Dessine un rectangle aux coins arrondis
fn draw_rounded_filled_rectangle(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    color: Color,
) {
    let width = width.max(2.0 * radius);
    let height = height.max(2.0 * radius);

    let corners = [
        (x + radius, y + radius),
        (x + width - radius, y + radius),
        (x + width - radius, y + height - radius),
        (x + radius, y + height - radius),
    ];

    for (x, y) in corners {
        draw_circle(x, y, radius, color);
    }

    draw_rectangle(x, y + radius, width, height - 2.0 * radius, color);
    draw_rectangle(x + radius, y, width - 2.0 * radius, height, color);
}
