//! Pheidippides. Marathon: when he wins a battle, each of his own correct
//! answers in it adds damage, up to a cap. It does not carry into the endgame.

use super::HeroAbility;
use crate::content::HeroDef;
use crate::state::Player;
use crate::stats::BattleStats;

pub struct Pheidippides;

impl HeroAbility for Pheidippides {
    fn modify_stats(&self, hero: &HeroDef, _player: &Player, stats: &mut BattleStats) {
        stats.damage_per_correct += hero.param("per_correct");
        stats.damage_per_correct_max += hero.param("max");
    }
}
