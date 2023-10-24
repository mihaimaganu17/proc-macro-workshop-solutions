// There is one other common type of pattern that would be nice to support --
// the wildcard or underscore pattern. The #[sorted] macro should check that if
// a wildcard pattern is present then it is the last one.

use sorted::sorted;

#[sorted]
pub enum Conference {
    RustBeltRust,
    RustConf,
    RustFest,
    RustLatam,
    RustRush,
    _RustLatam,
}

impl Conference {
    #[sorted::check]
    pub fn region(&self) -> &str {
        use self::Conference::*;

        #[sorted]
        match self {
            RustFest => "Europe",
            RustLatam => "Latin America",
            _ => "elsewhere",
        };

        #[sorted]
        match self {
            RustFest => "Europe",
            RustLatam => "Latin America",
            _RustLatam => "g",
            _ => "elsewhere",
        }
    }
}

fn main() {}
