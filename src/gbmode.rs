#[deriving(PartialEq, Copy)]
pub enum GbMode {
	Classic,
	Color,
	ColorAsClassic,
}

#[deriving(PartialEq, Copy)]
pub enum GbSpeed {
	Single,
	Double,
}
