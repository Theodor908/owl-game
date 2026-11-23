//! A hero with no ability: only the category bonus and starting HP every
//! hero has, both of which come from the data file. Used by the temporary
//! placeholder heroes that fill the list out for the hero offer.

use super::HeroAbility;

pub struct NoAbility;

impl HeroAbility for NoAbility {}
