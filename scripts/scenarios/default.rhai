// The default scenario: what a new game starts with when nothing else is chosen.
//
// Templates are defined here, at the top level, through the `scenario` handle the top level is run
// with. A template's `type` defaults to its `id`; a building level's `sprite` to `{type}_{level}`,
// the naming the client's art follows.

// The divisions, named after the client's counters. A division fights with its companies, 1 to 8;
// combat suppresses some and destroys others. At the end of every turn its suppressed companies
// recover: `recovery` is "all" of them, the default, or at most so many.
// TODO: placeholder strengths, to tune with combat.
scenario.define_unit_template(#{
    id: "infantry_division", movement: 2, sight: 2, companies: 4, recovery: "all"
});
scenario.define_unit_template(#{
    id: "motorized_division", movement: 4, sight: 3, companies: 3, recovery: 2
});
scenario.define_unit_template(#{
    id: "light_tank_division", movement: 3, sight: 2, companies: 3, recovery: 1
});

// The buildings, named after the client's sprites, by level: a level's cost and turns are what
// reaching it takes - the construction for the first, an upgrade above.
// TODO: levels 2 and 3 extend level 1's balance, as placeholders to tune.

// It extracts metals: it stands on a metals deposit, and its metals come out of it.
scenario.define_building_template(#{
    id: "mine",
    deposit: "metals",
    levels: [
        #{ cost: #{ credits: 200, metals: 100 }, turns: 3,
           maintenance: #{ credits: 2 }, production: #{ metals: 5 } },
        #{ cost: #{ credits: 300, metals: 150 }, turns: 4,
           maintenance: #{ credits: 3 }, production: #{ metals: 8 } },
        #{ cost: #{ credits: 450, metals: 250 }, turns: 5,
           maintenance: #{ credits: 5 }, production: #{ metals: 12 } }
    ]
});
// It pumps water: a water deposit springs back, yielding up to its recharge - 4 to 12 a turn -
// whatever the level asks, and never running dry.
scenario.define_building_template(#{
    id: "water_pump",
    deposit: "water",
    levels: [
        #{ cost: #{ credits: 150, metals: 60 }, turns: 2,
           maintenance: #{ credits: 1 }, production: #{ water: 4 } },
        #{ cost: #{ credits: 225, metals: 90 }, turns: 3,
           maintenance: #{ credits: 2 }, production: #{ water: 7 } },
        #{ cost: #{ credits: 350, metals: 150 }, turns: 4,
           maintenance: #{ credits: 3 }, production: #{ water: 10 } }
    ]
});
scenario.define_building_template(#{
    id: "solar_plant",
    levels: [
        #{ cost: #{ credits: 500, metals: 250 }, turns: 4,
           maintenance: #{ credits: 5 }, production: #{ electricity: 3 } },
        #{ cost: #{ credits: 700, metals: 350 }, turns: 5,
           maintenance: #{ credits: 7 }, production: #{ electricity: 5 } },
        #{ cost: #{ credits: 1000, metals: 500 }, turns: 6,
           maintenance: #{ credits: 10 }, production: #{ electricity: 8 } }
    ]
});

// What a settlement's next district costs and takes. A district is a building slot it lays out on
// its land next to it, and any building but an extractor stands on one. `settlement` is
// #{ districts, stage, capital, population }: the districts it has, those being laid out included;
// `stage`, what its people amount to, "outpost" to "city"; `capital`, whether its nation rules from
// it. Returns #{ cost, turns } - or `throw` why it cannot have one, the reason its order is refused
// with.
// TODO: placeholder figures, to tune with the economy; a cap by population comes later.
fn district(settlement) {
    let n = settlement.districts;
    #{ cost: #{ credits: 100 + 50 * n, metals: 40 + 20 * n }, turns: 2 + n / 2 }
}

// Who lives in a slot, counting from 0: the local player's is the first.
fn nation(slot) {
    #{
        definition: `nation_${slot}`,
        name: `Nation ${slot + 1}`,
        capital: `Nation ${slot + 1} capital`,
        stockpile: #{ credits: 250, water: 120, food: 180, electricity: 60, metals: 90 },
        income: #{ credits: 12, water: 4, food: 6, electricity: 3, metals: 2 }
    }
}

// What a slot starts with: placed on the capital, then on its land neighbours, nearest first.
fn starting_units(slot) {
    ["infantry_division", "motorized_division", "light_tank_division"]
}
