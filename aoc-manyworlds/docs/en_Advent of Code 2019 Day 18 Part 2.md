# Advent of Code 2019 Day 18 Part 2

## Where is the options page?

The player options page for this game doesnt exist. You shouldn't need it, because there aren't any game-specific options (except one dummy option) and so your yaml just needs to contain your slot name, the game name, and the dummy option like so:

```yaml
name: Minecart
game: Advent of Code 2019 Day 18 Part 2

Advent of Code 2019 Day 18 Part 2:
  dummy: ""
```

## What does randomization do to this game?
This game randomizes the 26 keys.
Additionally, the puzzle input is also randomly generated based on the multiworld seed.

## What is the goal of Advent of Code 2019 Day 18 Part 2 when randomized?
Collect all 26 keys.

## Which items can be in another player's world?
Any of the 26 keys.

## What does another world's item look like in Advent of Code 2019 Day 18 Part 2?
The key locations are displayed the same as in a regular input, but shows which item was sent in the lower text field while playing the game.

## When the player receives an item, what happens?
The key that was received is shown in the lower text field and is added to the "Keys collected:" list on the top-left of the screen.