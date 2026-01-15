from ..AutoWorld import World, WebWorld
from Options import PerGameCommonOptions, FreeText
from BaseClasses import Item, Location, Region, ItemClassification, Tutorial
from dataclasses import dataclass
from Utils import user_path
from ..generic.Rules import add_rule
from .Create import get_slot_data

class AOCManyWorldsItem(Item):
    game: str = "Advent of Code 2019 Day 18 Part 2"

class AOCManyWorldsLocation(Location):
    game: str = "Advent of Code 2019 Day 18 Part 2"

@dataclass
class AOCManyWorldsOptions(PerGameCommonOptions):
    dummy: FreeText

class AOCManyWorldsWeb(WebWorld):
    tutorials = [
        Tutorial(
            "Multiworld Setup Guide",
            "A guide to setting up Advent of Code 2019 Day 18 Part 2 for Multiworld.",
            "English",
            "setup_en.md",
            "setup/en",
            ["M1n3c4rt"]
        ),
        Tutorial(
            "Multiworld Setup Guide",
            "A guide to setting up Advent of Code 2019 Day 18 Part 2 for Multiworld.",
            "French",
            "setup_fr.md",
            "setup/fr",
            ["#Guigui"]
        )
    ]

class AOCManyWorldsWorld(World):
    game: str = "Advent of Code 2019 Day 18 Part 2"
    topology_present = False
    web = WebWorld()

    item_id_to_name = {ord(c):c for c in "abcdefghijklmnopqrstuvwxyz"}
    location_id_to_name = {ord(c):c for c in "abcdefghijklmnopqrstuvwxyz"}
    item_name_to_id = {c:ord(c) for c in "abcdefghijklmnopqrstuvwxyz"}
    location_name_to_id = {c:ord(c) for c in "abcdefghijklmnopqrstuvwxyz"}

    options_dataclass = AOCManyWorldsOptions

    slot_data = None

    def create_regions(self):
        menu = Region("Menu",self.player,self.multiworld)
        menu.locations += [AOCManyWorldsLocation(self.player, c, self.location_name_to_id[c], menu) for c in "abcdefghijklmnopqrstuvwxyz"]
        self.multiworld.regions.append(menu)

    def set_rules(self):
        if self.slot_data is None:
            self.slot_data = get_slot_data(self)
        logic = self.slot_data["logic"]
        for k, ks in logic.items():
            add_rule(self.multiworld.get_location(k,self.player), lambda state: all(state.has(c,self.player) for c in ks))
        self.multiworld.completion_condition[self.player] = lambda state: all(state.has(c,self.player) for c in "abcdefghijklmnopqrstuvwxyz")

    def create_item(self, name):
        return AOCManyWorldsItem(name, ItemClassification.progression, self.item_name_to_id[name], self.player)
    
    def create_items(self):
        keys = [self.create_item(name) for name in "abcdefghijklmnopqrstuvwxyz"]
        print(len(keys))
        self.multiworld.itempool += keys

    def fill_slot_data(self):
        if self.slot_data is None:
            self.slot_data = get_slot_data(self)
        return self.slot_data