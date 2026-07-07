use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub user_text: Color,
    pub ai_text: Color,
    pub system_text: Color,
    pub status_text: Color,
    pub hint_text: Color,
    pub input_border: Color,
    pub input_text: Color,
}

/// ChatGPT dark — near-black bg, white output, subtle blue user accent
pub const DARK: Theme = Theme {
    name: "dark",
    bg: Color::Rgb(14, 14, 18),
    user_text: Color::Rgb(220, 225, 235),
    ai_text: Color::Rgb(236, 238, 245),
    system_text: Color::Rgb(196, 188, 168),
    status_text: Color::Rgb(99, 99, 107),
    hint_text: Color::Rgb(58, 58, 65),
    input_border: Color::Rgb(99, 99, 107),
    input_text: Color::Rgb(240, 240, 246),
};

/// Clean white — pure white bg, near-black output
pub const LIGHT: Theme = Theme {
    name: "light",
    bg: Color::Rgb(255, 255, 255),
    user_text: Color::Rgb(45, 45, 52),
    ai_text: Color::Rgb(25, 25, 32),
    system_text: Color::Rgb(140, 130, 115),
    status_text: Color::Rgb(168, 168, 176),
    hint_text: Color::Rgb(212, 212, 218),
    input_border: Color::Rgb(168, 168, 176),
    input_text: Color::Rgb(18, 18, 24),
};

/// Dracula dark — deep purple bg, crisp white output, pink/green accents
pub const DRACULA: Theme = Theme {
    name: "dracula",
    bg: Color::Rgb(40, 42, 54),
    user_text: Color::Rgb(220, 230, 248),
    ai_text: Color::Rgb(242, 244, 252),
    system_text: Color::Rgb(196, 192, 178),
    status_text: Color::Rgb(108, 112, 126),
    hint_text: Color::Rgb(62, 64, 76),
    input_border: Color::Rgb(108, 112, 126),
    input_text: Color::Rgb(245, 246, 252),
};

/// Nord dark — navy bg, frost-white output
pub const NORD: Theme = Theme {
    name: "nord",
    bg: Color::Rgb(30, 33, 40),
    user_text: Color::Rgb(216, 222, 233),
    ai_text: Color::Rgb(236, 239, 244),
    system_text: Color::Rgb(204, 198, 176),
    status_text: Color::Rgb(94, 100, 114),
    hint_text: Color::Rgb(54, 58, 68),
    input_border: Color::Rgb(94, 100, 114),
    input_text: Color::Rgb(242, 244, 250),
};

pub const ALL: &[&Theme] = &[&DARK, &LIGHT, &DRACULA, &NORD];

pub fn by_name(name: &str) -> Option<&'static Theme> {
    ALL.iter().find(|t| t.name == name).copied()
}
