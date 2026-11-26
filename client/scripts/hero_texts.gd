## TEMPORARY. Hero ability names and texts, copied from HEROES.md.
##
## The hero data files carry no ability text yet (docs/ui-research.md, view
## addition V7). When the content does, delete this file and read the text
## from `Store.heroes` instead. Keyed by the hero's `id` in data/heroes/.
class_name HeroTexts

const BY_ID := {
	"herodotus": ["Second Draft", "Unlocks at the top library level, once per game. Either rewrite your last battle into a win with the most damage you could have dealt (you take any reflected damage), or, if unused when you die before the endgame, revive with half HP and no armor."],
	"darwin": ["Natural Selection", "Twice per game, make a golden card from only 2 copies instead of 3."],
	"benz": ["Two-Stroke Fusion", "Combine two cards into one that takes a single slot and keeps both effects. Twice per game."],
	"lumiere": ["Roll the Film Again", "For gold, pick a film question from your last battle. It appears again in your next battle."],
	"newton": ["Flaming Laser Sword", "Your shop sometimes offers the sword. Bought, it removes two wrong answers on 2 questions of your choice, in any battles you like. It is not offered again until both are used. Works together with Second Opinion."],
	"nietzsche": ["Will to Power", "For gold, choose the categories of the 2 house questions, a different one for each, from options the house shows you."],
	"shakespeare": ["To Be or Not To Be", "Costs all your gold and must be your first action of the round. With more than 3 players left: skip the next battle. With 3 left: gain 3 skulls. While he is in the lobby, skulls appear in every shop; a placed skull disables a random card of your opponent for one battle."],
	"marco_polo": ["Silk Road", "Once a turn, swap a random card in your shop for a random one a tier above your library level. You still have to buy it. At the top level a lower card becomes a top-tier one."],
	"pheidippides": ["Marathon", "When you win a battle, each of your own correct answers adds 1 damage, up to 6."],
	"cleopatra": ["Royal Treasury", "Gold you do not spend carries over to the next turn. Your gold stops at one and a half times the usual cap."],
	"marie_curie": ["Half-Life", "Only half of your opponent's armor counts against your damage, rounded down. This also works in the endgame."],
	"herophilus": ["Open Heart", "25% of any damage he takes is reflected back at the opponent. Cards can raise the percentage up to 80%, where everyone else stops at 60%."],
	"beethoven": ["Frenzied Notes", "Pay 3 gold, once a round, 4 times in all. After the fourth payment, Notes can appear in your shop for 1 gold. Placing a Note moves you to your next phase, in turn: Syncope (deal 2 damage to your opponent in every battle, whatever the result), Passionate Composer (40% Music instead of 25%), Can't Hear the Critics (15% reflect). After the third, back to Syncope."],
	"goethe": ["Prometheus", "In your battles, your opponent's hero ability does not work. For 3 gold, once a round, your Poetry bonus is 40% instead of 25% in your next battle."],
	"cervantes": ["Don Quixote", "Free, 3 times per game, once a round: put a Knight and a Windmill in your hand. Placed, a Knight deals 2 damage to your next opponent whatever the result; a Windmill keeps all damage off you in your next battle. Each lasts one battle, and a Knight knocks a Windmill off your board."],
	"homer": ["Bard of the Centuries", "Pay 3 gold, once a round, 3 times in all. After the third payment, Bards can appear in every player's shop while you are alive, for 1 gold. A placed Bard gives +25% Music on its holder's weighted questions for one battle. Each Bard another player buys pays you 1 gold at your next shop. Lyrical Peak: in the endgame your bias is 30% Poetry and 20% Music instead of 25% Poetry."],
}


static func ability_name(hero: Dictionary) -> String:
	return String(BY_ID.get(String(hero.get("id", "")), ["Ability", ""])[0])


static func ability_text(hero: Dictionary) -> String:
	return String(BY_ID.get(String(hero.get("id", "")), ["", "No description yet."])[1])
