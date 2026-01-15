# Advent of Code 2019 Day 18 Part 2

## Où est la page de paramètres ?

La page de paramètres de joueur n'existe pas. Vous ne devriez pas en avoir besoin, car il n'y a pas d'options spécifiques au jeu (mis à part une option muette) et donc votre yaml a juste besoin de contenir votre nom de slot, le nom du jeu, et l'option muette comme ci-dessous:

```yaml
name: '#Guigui'
game: Advent of Code 2019 Day 18 Part 2

Advent of Code 2019 Day 18 Part 2:
  dummy: ""
```

## Que fait la randomisation au jeu ?
Le jeu distribue aléatoirement les 26 clés.
De plus, l'entrée de puzzle est aussi générée aléatoirement, basée sur la seed du multiworld.

## Quel est l'objectif de Advent of Code 2019 Day 18 Part 2 une fois randomisé ?
Obtenir les 26 clés.

## Quels objets peuvent être dans le monde d'un autre joueur ?
N'importe lesquelles des 26 clés.

## À quoi ressemble un objet d'un autre monde dans Advent of Code 2019 Day 18 Part 2 ?
Les emplacements des clés sont affichés de la même manière que dans une entrée classique, mais affichent quel objet est envoyé dans le champ de texte inférieur en jeu.

## Que se passe-t-il quand le joueur reçoit un objet ?
La clé qui a été reçue est affichée dans le champ de texte inférieur et est ajoutée à la liste de "Keys collected:" (clés obtenues) en haut à gauche de l'écran