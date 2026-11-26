//! Herophilus of Chalcedon. Vivisection: part of any damage he takes is
//! reflected back at the opponent. Reflect cards add to it.

use super::HeroAbility;
use crate::content::HeroDef;
use crate::state::Player;
use crate::stats::BattleStats;

pub struct Herophilus;

impl HeroAbility for Herophilus {
    fn battle_passive(&self, hero: &HeroDef, _player: &Player, stats: &mut BattleStats) {
        stats.reflect += hero.param("reflect");
    }
}
