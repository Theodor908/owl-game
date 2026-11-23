# Question bank

Questions live in `data/questions/<category>.toml`, one file per broad category. The file name is the category's slug from `categories` in `data/tunables.toml`.

Every question comes from a source that states the answer: a published quiz or test with its answer key, checked against a second reference (an encyclopedia, an official body, a textbook). The wording is our own; quiz questions are not copied word for word.

No question is used in a match until a person has reviewed it (`reviewed = true`).

## Format

```toml
[[question]]
id = "history-0001"              # category slug and a number, never reused
subcategory = "Ancient Rome"
difficulty = "medium"           # easy | medium | hard
text = "Which Roman emperor ordered the building of a wall across northern Britain in AD 122?"
answer = "Hadrian"
wrong = ["Trajan", "Augustus", "Nero"]
source = "https://..."          # the quiz or test with its answer key
check = "https://..."           # a second reference that confirms the answer
reviewed = false
```

- `answer` is the correct option and `wrong` holds exactly three others. The game shuffles them.
- Options are short, plausible and of the same kind (four people, four years, four countries). Exactly one is right, and none is right under another reading.
- No question depends on the date it is asked ("the current champion", "the tallest building").
- `difficulty` records how hard the source rated it, or the writer's judgement when the source has no levels. A question that leaves it out counts as medium; any other word is refused when the bank loads.

## Difficulty in play

The rules decide the difficulty a question should have (a random base, moved by difficulty cards; see `CLAUDE.md`) and ask the bank for it. The bank draws a question of that category and difficulty that neither player has seen. If none is left, it takes the nearest difficulty: for medium, easy and then hard; for easy, medium and then hard; for hard, medium and then easy. With nothing left in the category at all, it asks a placeholder sum. The player is shown the difficulty of the question that was actually drawn.
