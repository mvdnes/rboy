#[derive(PartialEq, Copy, Clone)]
pub enum GbMode {
    Classic,
    Color,
    ColorAsClassic,
}

#[derive(PartialEq, Copy, Clone)]
pub enum GbSpeed {
    Single = 1,
    Double = 2,
}
