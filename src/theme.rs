use ratatui::style::Color;

/// All colors used in the TUI are defined here — no terminal defaults.
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

pub const DARK: Theme = Theme {
    name: "dark",
    bg: Color::Rgb(24, 24, 32),
    user_text: Color::Rgb(86, 182, 194),
    ai_text: Color::Rgb(163, 218, 142),
    system_text: Color::Rgb(229, 181, 103),
    status_text: Color::Rgb(128, 128, 140),
    hint_text: Color::Rgb(80, 80, 90),
    input_border: Color::Rgb(86, 182, 194),
    input_text: Color::Rgb(220, 220, 230),
};

pub const LIGHT: Theme = Theme {
    name: "light",
    bg: Color::Rgb(250, 250, 252),
    user_text: Color::Rgb(28, 99, 167),
    ai_text: Color::Rgb(42, 130, 56),
    system_text: Color::Rgb(172, 93, 24),
    status_text: Color::Rgb(140, 140, 150),
    hint_text: Color::Rgb(190, 190, 200),
    input_border: Color::Rgb(28, 99, 167),
    input_text: Color::Rgb(30, 30, 40),
};

pub const DRACULA: Theme = Theme {
    name: "dracula",
    bg: Color::Rgb(40, 42, 54),
    user_text: Color::Rgb(139, 233, 253),
    ai_text: Color::Rgb(80, 250, 123),
    system_text: Color::Rgb(241, 250, 140),
    status_text: Color::Rgb(98, 114, 164),
    hint_text: Color::Rgb(68, 71, 90),
    input_border: Color::Rgb(139, 233, 253),
    input_text: Color::Rgb(248, 248, 242),
};

pub const NORD: Theme = Theme {
    name: "nord",
    bg: Color::Rgb(46, 52, 64),
    user_text: Color::Rgb(136, 192, 208),
    ai_text: Color::Rgb(163, 190, 140),
    system_text: Color::Rgb(235, 203, 139),
    status_text: Color::Rgb(76, 86, 106),
    hint_text: Color::Rgb(67, 76, 94),
    input_border: Color::Rgb(136, 192, 208),
    input_text: Color::Rgb(236, 239, 244),
};

pub const ALL: &[&Theme] = &[&DARK, &LIGHT, &DRACULA, &NORD];

pub fn by_name(name: &str) -> Option<&'static Theme> {
    ALL.iter().find(|t| t.name == name).copied()
}
