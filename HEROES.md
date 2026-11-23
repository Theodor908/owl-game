# Starting heroes (12)

Rules that apply to all heroes:
- Every +25% category bonus carries into the endgame (for now).
- No hero ability carries into the endgame (for now).
- An ability with a limited number of uses per match (Darwin, Benz, Herodotus, Shakespeare) can be used at most once per round; after a use it waits for the next turn.
- The +25% applies to the player's own weighted questions, not to the 2 house questions.
- Any hero can buy category cards of any category.
- When a lobby starts, each player is dealt a private offer of 2 distinct heroes, drawn at random from the full list. No hero appears in two players' offers, so each hero appears at most once per lobby.
- A player may pick only from their own offer, and cannot see another player's offer. A player who has not picked when the timer runs out gets a random one of their two. Bots pick one of their two.
- The list must hold at least 2 heroes per player in the lobby: 12 for 6 players.

Starting HP values and costs are placeholders. They live in `data/tunables.toml` and are tuned from `sim` results.

| Hero | Title | Category bonus | Starting HP | Ability |
|---|---|---|---|---|
| Herodotus | Father of History | +25% History | 35 | Second Draft: unlocks at max tavern level, used once per game, in one of two ways: (a) rewrite the result of the last battle into a win with max damage to the opponent, effects applied in the next round; if the opponent has reflect, they take max damage and he takes the reflected damage; or (b) if unused when he dies before the endgame, he revives with half HP and no armor |
| Charles Darwin | Father of Evolution | +25% Flora and Fauna | 32 | Natural Selection: twice per game, get a golden card from only 2 base cards |
| Karl Benz | Inventor of the Car | +25% Cars and Mechanics | 35 | Two-Stroke Fusion: combine two cards into one that occupies a single slot, whether they are golden or common. Limited to two combined cards per game for now |
| Lumière brothers | Pioneers of Cinema | +25% Film | 36 | Roll the Film Again: for a gold cost, the player picks a film question they liked and it appears again in the next battle |
| Isaac Newton | Father of Classical Mechanics | +25% Physics | 30 | Active, Flaming Laser Sword: his shop sometimes offers the sword (a set chance each time the shop is filled, the reroll included). Bought for gold, it removes two wrong answers on 2 questions of his choice, in whichever battles he likes. While he still holds an unused revelation the shop does not offer it again. Can be combined with the Second Opinion card on the same question |
| Friedrich Nietzsche | Übermensch | +25% Philosophy | 35 | Active, Will to Power: for a gold cost, choose the broad categories of the 2 house questions, a different one for each, picked from random options shown by the house |
| William Shakespeare | Dramatic Genius | +25% Theater | 34 | To Be or Not To Be: usable in any shop phase, with no limit per game; each use costs all gold currently available for the round. It must be his first action of the round. With more than 3 players left ("not to be"): skip the next battle; he can do nothing else that round; at most 3 skips per game. With 3 players left ("to be"): no skip; he receives 3 skulls in his hand instead. Passive: while he is in the lobby, skulls appear randomly in every player's shop; a skull placed on his board disables a random card of any type on his opponent's board for that round |
| Herophilus of Chalcedon | Father of Anatomy | +25% Anatomy | 30 | Open Heart: 25% of any damage he takes is reflected back at the opponent. Cards can increase the percentage |
| Marco Polo | Traveller of the Silk Road | +25% Geography | 35 | Active, Silk Road: once per turn, for a gold cost, a random card in his shop is swapped for a random card one tier above his tavern level; he still has to buy it. At the top tavern level a random card below the top tier is swapped for a top-tier one; if every card in the shop is already top tier, one of them is swapped for a different top-tier card |
| Pheidippides | Runner of Marathon | +25% Sport | 35 | Marathon: when he wins a battle, each of his own correct answers in that battle adds 1 damage, up to 6. Regular battles only; it does not carry into the endgame |
| Cleopatra | Last Pharaoh of Egypt | +25% History | 35 | Royal Treasury: gold she does not spend carries over to the next turn. Carried gold and the turn's income together stop at one and a half times the gold cap |
| Marie Curie | Pioneer of Radioactivity | +25% Physics | 35 | Half-Life: only half of her opponent's armor counts against her damage, rounded down. A passive that carries into the endgame |


## Tunables (placeholders)
| Name | Placeholder |
|---|---|
| Heroes offered to each player (`hero_offer_size`) | 2 |
| Roll the Film Again cost (Lumière) | 2 gold |
| Will to Power cost / options shown | 2 gold / 4 categories |
| Skull cost / chance to appear per shop roll | 2 gold / 20% |
| Shakespeare skips per game | 3 |
| Flaming Laser Sword cost / chance to appear per shop roll / revelations per purchase | 2 gold / 50% / 2 |
| Silk Road cost | 2 gold |
| Marathon damage per correct answer / cap | 1 / 6 |
| Royal Treasury gold cap | 150% of the gold cap (15) |
| Half-Life share of armor ignored | 50% |
| Benz combined card | keeps both cards' full effects |
| Herodotus rewrite | refunds the damage he took in that battle |
| Herodotus "max damage" | the most he could have dealt in that battle: a 6-0 win with his bonuses, up to the per-battle cap. He takes the matching reflected damage if the opponent has reflect. |
| Herodotus rewrite safety | the rewrite never kills. Any player who would die from it, Herodotus, the opponent or both, is left at 1 HP and given a shield for the next battle (3 armor, placeholder) |

## Settled details
- Skulls are placed on his own board, one slot each, with no limit other than the board. Each disables a different random opponent card for that battle, and placed skulls disappear after one round.
- Flaming Laser Sword plus Second Opinion on one question leaves only the correct answer. This is intended.
