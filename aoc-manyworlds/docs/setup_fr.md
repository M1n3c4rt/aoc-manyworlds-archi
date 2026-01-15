# Guide d'installation du MultiWorld d'Advent of Code 2019 Jour 18 Partie 2

## Logiciels requis

- Advent of Code 2019 Jour 18 Partie 2 (version jouable)
  - [Github](https://github.com/M1n3c4rt/aoc-manyworlds-archi/releases)

## Procédures d'installation et du démarrage du jeu

1. Installez l'exécutable depuis la dernière release du dépôt Github.
2. Lancez l'exécutable à travers un outil de ligne de commande comme montré ci-dessous.

# Rejoindre un multiworld

Pour rejoindre une partie d'Archipelago Multiworld, par exemple un multiworld hébergé sur `https://archipelago.gg:12345` où votre nom de slot est `#Guigui` :

`aoc-manyworlds-archi --url "https://archipelago.gg:12345" --slot "#Guigui"`

Si la partie que vous rejoignez nécessite un mot de passe, par exemple `codekata`, vous devez aussi ajouter le flag suivant :
`aoc-manyworlds-archi --url "https://archipelago.gg:12345" --password "codekata" --slot "#Guigui"`

# Playing offline

Pour jouer au jeu en mode hors-ligne, vous devez à la place le lancer comme ci-contre :
`aoc-manyworlds-archi --singleplayer`

Notez que cela ne crééra/rejoindra pas de multiworld du tout, et utilisera juste l'aléatoire native du jeu.

Par défaut, la seed est choisie aléatoirement. Vous pouvez donner votre propre seed dans ce contexte :
`aoc-manyworlds-archi --singleplayer --seed 123`

Gardez en tête que cette seed est différente de la seed du multiworld. Utiliser le même nombre pour la seed du multiworld donnera sûrement une entrée de puzzle différente.