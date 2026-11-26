# Base card pool (v1)

The starting pool for the prototype. The cards are accepted; their numbers are tunable and belong in `data/cards/`. Rules these cards rely on are in `CLAUDE.md`.

Conventions:
- Every card costs 3 gold unless a cost is given.
- A golden card (3 identical copies) has doubled numbers, unless the card states its own golden effect. Shields cannot be made golden.
- Placing a golden card on the board, for the first time, lets the player pick 1 of 3 cards from the next tavern tier up. A pick left open when the shop closes becomes a Golden Reward card in hand.
- Special cards (Second Opinion, The Oracle, the eight difficulty cards) have one copy in the pool and cannot be golden. The three endgame effect cards have no golden version either.
- Difficulty: a question is easy, medium or hard. A difficulty card moves the questions of its holder's battle one step ("one step easier", "one step harder") or two ("always easy", "always hard"), for both players, house questions included. Steps from both boards add up and cancel, then the result stops at easy or hard.
- "Top category" is the holder's category with the highest weight percentage, hero and cards together, when the battle starts. Categories tied for the highest all count. With no weights the card does nothing.
- A card can appear in a shop only if its tier is at or below the player's tavern level.
- "[Category]" cards exist once per category, so each line of that kind is 13 cards in the prototype.
- "On win" means the holder got more questions right than the opponent.
- Category bonus (3 or more cards of one category on the board): on win, +2 damage for each question of that category the holder answered correctly. A golden category card counts as 3 cards and makes the bonus +3. The bonus does not stack with more cards.

## Tier 1
| Card | Type | Effect |
|---|---|---|
| [Category] Apprentice | Category | +10% chance of [category] on your weighted questions |
| Wooden Shield | Armor | 3 armor. Breaks and frees its slot when depleted |
| Hourglass | Time | +1 s answer time (total bonus capped at +3 s) |
| Whetstone | Damage | +1 damage on win |

## Tier 2
| Card | Type | Effect |
|---|---|---|
| Mirror Shard | Reflect | 10% reflect |
| Scales of Judgment | Tie | Tie-breaker: on a tie where both got at least one right, deal half your tavern level, minimum 1. Cancels if both hold one |
| Coin Purse | Economy | If you got 4 or more right in the battle, +1 gold next shop |
| Consolation | Defense | When you lose a battle, add a 2-armor shield card to your hand. Three of these shields combine into one 6-armor shield. A shield must be placed on the board to count |
| Glossary | Difficulty | Every question in your battle is one step easier, for both players |
| Footnotes | Difficulty | Every question in your battle is one step harder, for both players |
| Pocket Guide | Difficulty | Every question in your battle is always easy, for both players. Counts as two steps against cards that make them harder |
| Unabridged Edition | Difficulty | Every question in your battle is always hard, for both players. Counts as two steps against cards that make them easier |

## Tier 3
| Card | Type | Effect |
|---|---|---|
| [Category] Scholar | Category | +15% chance of [category] on your weighted questions |
| Iron Shield | Armor | 6 armor. Breaks when depleted |
| Long Hourglass | Time | +2 s answer time |
| Last Laugh | Tie | On a 0-0 tie, deal 1 damage. Cancels if both hold one |
| Second Opinion | Help | Once per battle, on a question you choose, remove one wrong answer. Can be used on the same question as Newton's Flaming Laser Sword |
| Thorned Mirror | Reflect | 20% reflect |
| Primer | Difficulty | Questions in your top category are one step easier, for both players |
| Treatise | Difficulty | Questions in your top category are one step harder, for both players |
| Abridged Edition | Difficulty | Questions in your top category are always easy, for both players. Counts as two steps against cards that make them harder |
| Magnum Opus | Difficulty | Questions in your top category are always hard, for both players. Counts as two steps against cards that make them easier |

## Tier 4
| Card | Type | Effect |
|---|---|---|
| Double or Nothing | Bet | +50% damage on win. When you lose, take +2 damage |
| Piercing Quill | Counter | Your damage ignores half of the opponent's reflect |
| Siege Ram | Counter | Your damage ignores 3 armor |
| Streak Banner | Damage | +1 damage for each battle won in a row, up to +5 |
| Rival's Notes | Weights | Your weight total counts as 20 points higher for the 3-1 split |

## Tier 5
| Card | Type | Effect |
|---|---|---|
| [Category] Master | Category | +25% chance of [category] on your weighted questions |
| Steel Bulwark | Armor | 10 armor. Breaks when depleted |
| Perfect Mind | Damage | If you answer all 6 questions correctly and win, double your damage |
| Hall of Mirrors | Reflect | 30% reflect |
| Narrowed Choice (cost 8) | Endgame effect | In the endgame, you choose between 3 answers instead of 4 |
| Deep Breath (cost 8) | Endgame effect | In the endgame, +3 s answer time |

## Tier 6
| Card | Type | Effect |
|---|---|---|
| The Oracle (cost 8) | Legendary | Instantly answers one question per regular battle correctly, and one endgame question in total |
| Encyclopedia | Category | Counts as a card of every category for category bonuses |
| Aegis | Defense | You take at most 8 damage from a single battle. Golden: also gives 10 armor |
| Second Wind (cost 8) | Endgame effect | In the endgame, the first time you would take damage, take none |

## Healing and hunting (numbers are placeholders)
Healing never raises a player above their starting HP. A player can have one Healer and one Hunter on the board at most.

| Card | Tier | Type | Effect |
|---|---|---|---|
| Hunter | 6 | Counter | If the opponent has a Healer, disables its next heal. Stays on the board, so it is a wasted slot against an opponent with no Healer. Golden (made from copies in hand): also gives one Arrow |
| Healer | 6 | Healing | Heals you 1 HP every turn. Golden: needs a Healer on the board that was not disabled last round plus 2 in hand; when made, heals you a quarter of your starting HP and every other player an eighth of theirs; per-turn healing is not doubled. One golden Healer per match, across all players |
| Potion | any | Legendary | Has no tier: can appear in any shop, with a small chance each round that grows over the match, as an extra card beside the normal offers. One use: instantly heal 5 HP. Removed from the game when used; never returns to the pool |
| Arrow | none | One-time | Deals 3 damage to your next opponent whatever the battle result, then vanishes. Ordinary damage: armor, HP and reflect apply. Only comes from a golden Hunter |

## Golden Reward
| Card | Tier | Type | Effect |
|---|---|---|---|
| Golden Reward | none | Reward card | Stands for a golden reward that was not picked before the shop closed. Place it to pick one of three cards from the next tier up; it then disappears and takes no place on the board. Sells for nothing and is never in the pool |

## Hero cards
Cards that only a hero's ability brings into the game. They are not pool cards: they are never golden, sell for nothing and never enter the pool.

| Card | Hero | Effect |
|---|---|---|
| Skull (cost 2) | Shakespeare | Appears in every shop while he is in the lobby, as an extra card beside the normal offers. Placed, it disables a random card of the next opponent for that battle, then vanishes. Takes a board slot |
| Note (cost 1) | Beethoven | Appears only in his shop, as an extra card beside the normal offers, once his four Frenzied Notes payments are made (30% chance per shop roll). Placed, it moves him to his next phase and vanishes. Takes no board slot |
| Knight | Cervantes | From Don Quixote. Placed, it deals 2 damage to the next opponent whatever the result (ordinary damage). Takes a board slot and is removed after that battle. Knocks a Windmill off his board |
| Windmill | Cervantes | From Don Quixote. Placed, he takes no damage in his next battle: battle, reflected, Arrow and overtime tie damage. He still deals his own. Takes a board slot and is removed after that battle. A Knight on his board knocks it off, whichever was placed second |
| Bard (cost 1) | Homer | Appears in every shop while he is alive, as an extra card beside the normal offers, once his three Bard of the Centuries payments are made (20% chance per shop roll). Placed, it gives +25% Music on its holder's weighted questions for the next battle, then vanishes. Takes a board slot. Not a Music card for the category bonus. Each Bard another player buys pays Homer 1 gold at his next shop |

## Totals
26 unique cards in the tiers, 3 healing and hunting cards (Healer, Hunter, Potion), 8 difficulty cards and 3 category lines (39 cards across 13 categories): 76 cards in the pool. The Arrow, the Golden Reward and the hero cards are not in the pool.

## Archetypes this pool should support
- Specialist: Apprentice, Scholar and Master of one or two categories, category bonus, Rival's Notes for the 3-1 split.
- Tank: shields, Consolation, Aegis, reflect cards.
- Scorer: Whetstone, Streak Banner, Double or Nothing, Perfect Mind.
- Tie player: Scales of Judgment, Last Laugh, time cards.
- Endgame gambler: holds gold for an endgame effect and The Oracle.
- Difficulty cards cut across these: a player who knows a lot raises the difficulty, a player who knows little lowers it.

## Settled
- Category cards exist for every broad category and any hero can buy any of them. A hero's own category is a head start, not a restriction; mixing categories is a valid generalist build.
- Skulls (Shakespeare) can disable any type of card. Nothing is disabled in the endgame, since hero abilities do not carry.
- Second Opinion and Flaming Laser Sword stack on one question.
