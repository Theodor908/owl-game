//! Cleopatra. Royal Treasury: the gold she does not spend carries over to
//! the next turn. Carried gold and the turn's income together stop at a set
//! share of the ordinary gold cap.

use super::HeroAbility;
use crate::content::{HeroDef, Tunables};

pub struct Cleopatra;

impl HeroAbility for Cleopatra {
    fn gold_kept(&self, hero: &HeroDef, leftover: u32, income: u32, t: &Tunables) -> u32 {
        let cap = t.gold_cap * hero.param("cap_percent") / 100;
        leftover.min(cap.saturating_sub(income))
    }
}
