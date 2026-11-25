const BOARD_WIDTH: usize = crate::game::WINDOW_WIDTH / crate::game::TILE_SIZE;
const BOARD_HEIGHT: usize = crate::game::WINDOW_HEIGHT / crate::game::TILE_SIZE;

/// Planche de jeu
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub obstacles: [bool; BOARD_WIDTH * BOARD_HEIGHT],
}

impl Board {
    /// Génère une nouvelle planche de jeu
    pub fn new() -> Self {
        let mut obstacles = [false; BOARD_WIDTH * BOARD_HEIGHT];

        let quart_x = BOARD_WIDTH / 4;
        let center_y = BOARD_HEIGHT / 2;
        
        for x in quart_x..(BOARD_WIDTH - quart_x) {
            obstacles[center_y * BOARD_WIDTH + x] = true;
        }

        let quart_y = BOARD_HEIGHT / 4;
        let center_x = BOARD_WIDTH / 2;

        for y in quart_y..center_y {
            obstacles[y * BOARD_WIDTH + center_x] = true;
        }

        Self {
            width: BOARD_WIDTH,
            height: BOARD_HEIGHT,
            obstacles,
        }
    }

    /// Crée une planche de jeu à partir d'une spécification
    /// 
    /// La spécification est une chaîne de caractères qui indique l'emplacement des obstacles
    pub fn from(obstacles: String) -> Self {
        let mut grid = Self::new();

        for (i, c) in obstacles.chars().enumerate() {
            if i < BOARD_WIDTH * BOARD_HEIGHT {
                grid.obstacles[i] = c == '1';
            } else {
                break; // ignore les caractères supplémentaires
            }
        }

        grid
    }

    /// Vérifie si un emplacement contient un obstacle
    pub fn is_obstacle(&self, x: usize, y: usize) -> Option<bool> {
        if x < self.width && y < self.height {
            Some(self.obstacles[y * self.width + x])
        } else {
            None
        }
    }

    /// Vérifie si un emplacement est libre
    pub fn is_empty_tile(&self, x: usize, y: usize) -> bool {
        self.is_obstacle(x, y) == Some(false)
    }

    /// Trouve un emplacement libre au hasard
    pub fn random_empty_tile(&self) -> Option<(usize, usize)> {
        let available_positions: Vec<usize> = (0..(self.width * self.height))
            .filter(|number| self.is_empty_tile(number % self.width, number / self.width))
            .collect();
        if available_positions.is_empty() {
            return None;
        }

        let position = available_positions[rand::random_range(0..available_positions.len())];

        Some((position % self.width, position / self.width))
    }

    /// Convertit une planche de jeu en chaîne de caractères
    pub fn serialize(&self) -> String {
        self.obstacles
            .iter()
            .map(|&b| if b { '1' } else { '0' })
            .collect()
    }
}
