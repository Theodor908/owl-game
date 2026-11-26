//! Marie Curie. Half-Life: against her, only part of the opponent's armor
//! counts, rounded down. The passive carries into the endgame.

use super::HeroAbility;
use crate::content::HeroDef;
use crate::state::Player;
use crate::stats::BattleStats;

pub struct MarieCurie;

impl HeroAbility for MarieCurie {
    fn battle_passive(&self, hero: &HeroDef, _player: &Player, stats: &mut BattleStats) {
        stats.armor_pierce_percent += hero.param("armor_percent");
    }

    fn endgame_armor_pierce(&self, hero: &HeroDef) -> u32 {
        hero.param("armor_percent")
    }
}
