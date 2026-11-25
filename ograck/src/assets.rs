use macroquad::audio::{Sound, load_sound};
use macroquad::text::{Font, load_ttf_font};
use macroquad::texture::{Texture2D, load_texture};

/// Ressources (images et sons)
pub struct Assets {
    pub intro_music: Sound,
    pub frog_font: Font,
    pub comic_font: Font,
    pub intro_background: Texture2D,
    pub host_join_background: Texture2D,
    pub lobby_background: Texture2D,
    pub human: Texture2D,
    pub zombie: Texture2D,
    pub wall: Texture2D,
    pub ground: Texture2D,
}

impl Assets {
    /// Charge les ressources (images et sons) utilisées par le jeu
    pub async fn load() -> Self {
        let intro_music = load_sound("./assets/abydos_music-60s-garage-rock-loop-1-238513.ogg")
            .await
            .unwrap();

        let frog_font = load_ttf_font("./assets/SuperFrog-Yqy1q.ttf").await.unwrap();
        let comic_font = load_ttf_font("./assets/SuperComic-qZg62.ttf")
            .await
            .unwrap();

        let intro_background = load_texture("./assets/runner1.png").await.unwrap();
        let host_join_background = load_texture("./assets/runner2.png").await.unwrap();
        let lobby_background = load_texture("./assets/runner3.png").await.unwrap();
        let human = load_texture("./assets/human.png").await.unwrap();
        let zombie = load_texture("./assets/zombie.png").await.unwrap();
        let wall = load_texture("./assets/wall.png").await.unwrap();
        let ground = load_texture("./assets/ground.png").await.unwrap();

        Self {
            intro_music,
            frog_font,
            comic_font,
            intro_background,
            host_join_background,
            lobby_background,
            human,
            zombie,
            wall,
            ground,
        }
    }
}
