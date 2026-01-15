# Advent of Code 2019 Day 18 Part 2 MultiWorld Setup Guide

## Required Software

- Advent of Code 2019 Day 18 Part 2 (playable version)
  - [Github](https://github.com/M1n3c4rt/aoc-manyworlds-archi/releases)

## Installation and Game Start Procedures

1. Install the executable from the above Github repository's latest release.
2. Run the executable through a command line tool as shown below.

# Joining a MultiWorld Game

To join an Archipelago MultiWorld game, for example a multiworld hosted on `https://archipelago.gg:12345` where your slot name is `Minecart`:

`aoc-manyworlds-archi --url "https://archipelago.gg:12345" --slot "Minecart"`

If the game you are joining requires a password, for example `codekata`, you should also add the following to your flags:  
`aoc-manyworlds-archi --url "https://archipelago.gg:12345" --password "codekata" --slot "Minecart"`

# Playing offline

If the game is to be played offline in single-player mode, you should instead run as follows:  
`aoc-manyworlds-archi --singleplayer`

Note that this will not create/join a multiworld whatsoever, and instead just use the game's native randomization.

By default, the seed is picked at random. You can supply your own seed in this situation:
`aoc-manyworlds-archi --singleplayer --seed 123`

Keep in mind that this seed is separate from the multiworld seed. Using the same number for the multiworld seed will likely give you a different puzzle input.