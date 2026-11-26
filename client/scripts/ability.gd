## What a hero's ability is, for display: its name, text, picture, price and
## when it is used. Every screen that shows an ability builds it from this,
## so a button, a held note and a hero sheet always agree.
##
## It holds no rules. The price comes from the hero's data file (`params`);
## the name and text still come from the temporary `HeroTexts`, and `WHEN`
## below is display knowledge that belongs in the hero data files too
## (docs/ui-research.md, view addition V7).
class_name Ability

enum When {
	## Pressed in the shop phase.
	SHOP,
	## Pressed during a battle; nothing is paid there.
	BATTLE,
	## Works by itself.
	PASSIVE,
}

## Hero kind -> when the ability is used. Kinds not listed are used in the shop.
const WHEN := {
	"Newton": When.BATTLE,
	"Herophilus": When.PASSIVE,
	"Pheidippides": When.PASSIVE,
	"Cleopatra": When.PASSIVE,
	"MarieCurie": When.PASSIVE,
}
## Hero kinds whose ability can be used once in each shop phase.
const ONCE_A_TURN := ["MarcoPolo", "Goethe"]
## Hero kind -> the param in its data file that holds its uses per match.
## A limited ability is used at most once in each round (the rule is in core).
const LIMIT := {"Darwin": "uses", "Benz": "uses", "Herodotus": "uses", "Shakespeare": "skips", "Beethoven": "payments", "Cervantes": "uses", "Homer": "payments"}
## Hero kind -> the param that says how many players alive the limit counts above.
## With fewer left the ability is not limited (Shakespeare's "to be").
const LIMIT_ABOVE := {"Shakespeare": "skip_above"}
## Hero kinds whose ability takes all the gold the player has.
const ALL_GOLD := ["Shakespeare"]

var name := ""
var text := ""
## The round painted icon, or null until it is drawn.
var icon: Texture2D
## Gold paid in the shop to use it. 0: free.
var cost := 0
## It takes all the gold the player has, whatever that is.
var costs_all_gold := false
var when := When.SHOP
## Usable once in each shop phase.
var once_a_turn := false
## Uses per match, at most one a round. 0: not limited.
var limit := 0
## The limit counts only while more than this many players are alive. 0: always.
var limit_above := 0
## Uses one purchase gives, for abilities bought in the shop and used in battle. 0: not counted.
var uses_per_purchase := 0


## The ability of a hero from `Store.heroes`, or null when the hero has none.
static func of(hero: Dictionary) -> Ability:
	var kind := String(hero.get("kind", "NoAbility"))
	if hero.is_empty() or kind == "NoAbility":
		return null
	var params: Dictionary = hero.get("params", {})
	var ability := Ability.new()
	ability.name = HeroTexts.ability_name(hero)
	ability.text = HeroTexts.ability_text(hero)
	ability.icon = UI.art("abilities/%s.png" % kind.to_lower())
	ability.cost = int(params.get("cost", 0))
	ability.costs_all_gold = ALL_GOLD.has(kind)
	ability.when = WHEN.get(kind, When.SHOP)
	ability.once_a_turn = ONCE_A_TURN.has(kind)
	if LIMIT.has(kind):
		ability.limit = int(params.get(LIMIT[kind], 0))
		ability.limit_above = int(params.get(LIMIT_ABOVE.get(kind, ""), 0))
	if ability.when == When.BATTLE:
		ability.uses_per_purchase = int(params.get("questions", 0))
	return ability


## What stands on the coin beside the button: the price, "All", or nothing when it is free.
func coin_text() -> String:
	if costs_all_gold:
		return "All"
	return "" if cost <= 0 else str(cost)


## The limit counts with this many players alive.
func counted(alive: int) -> bool:
	return limit > 0 and (limit_above <= 0 or alive > limit_above)


## Uses left this match, from the player's `hero_state`.
func left(hero_state: Dictionary) -> int:
	return maxi(0, limit - int(hero_state.get("uses", 0)))


## Uses left out of the limit, as "1/2".
func counter(hero_state: Dictionary) -> String:
	return "%d/%d" % [left(hero_state), limit]


## Used in this round: it waits for the next one. Only for abilities used
## once a turn or limited per match.
func used_this_round(hero_state: Dictionary, turn: int) -> bool:
	return (once_a_turn or limit > 0) and int(hero_state.get("used_turn", 0)) == turn


## A short tag for abilities that are not pressed in the shop. Empty otherwise.
func tag() -> String:
	match when:
		When.BATTLE:
			return "" if cost > 0 else "In battle"
		When.PASSIVE:
			return "Passive"
	return ""


## One line on what it costs and when it is used, for the held note.
func terms() -> String:
	match when:
		When.BATTLE:
			var price := "Free" if cost <= 0 else "Costs %d gold when the shop offers it" % cost
			var uses := "" if uses_per_purchase <= 0 else ", %d times in all" % uses_per_purchase
			return "%s. Used during a battle%s." % [price, uses]
		When.PASSIVE:
			return "Free. Works by itself."
	if costs_all_gold:
		var all := "Costs all the gold you have. Used in the shop, once a round."
		if limit > 0 and limit_above > 0:
			all += " Skips at most %d battles per match, while more than %d players are left." % [limit, limit_above]
		return all
	var often := ", once a turn" if once_a_turn else ""
	if limit == 1:
		often = ", once per match"
	elif limit > 1:
		often = ", %d times per match, at most once a round" % limit
	return "Free. Used in the shop%s." % often if cost <= 0 else "Costs %d gold. Used in the shop%s." % [cost, often]
