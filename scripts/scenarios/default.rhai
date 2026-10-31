// The default scenario: what a new game starts with when nothing else is chosen.
//
// Templates are defined here, at the top level, through the `scenario` handle the top level is run
// with. A template's `type` defaults to its `id`.

// The divisions, named after the client's counters.
scenario.define_unit_template(#{ id: "infantry_division", movement: 2, sight: 2 });
scenario.define_unit_template(#{ id: "motorized_division", movement: 4, sight: 3 });
scenario.define_unit_template(#{ id: "light_tank_division", movement: 3, sight: 2 });

// The buildings, named after the client's sprites
scenario.define_building_template(#{
    id: "mine",
    cost: #{ credits: 200, metals: 100 },
    maintenance: #{ credits: 2 },
    production: #{ metals: 5 }
});
scenario.define_building_template(#{
    id: "solar_plant",
    cost: #{ credits: 500, metals: 250 },
    maintenance: #{ credits: 5 },
    production: #{ electricity: 3 }
});

// Who lives in a slot, counting from 0: the local player's is the first.
fn nation(slot) {
    #{
        definition: `nation_${slot}`,
        name: `Nation ${slot + 1}`,
        capital: `Nation ${slot + 1} capital`,
        // something for the economy phase to tick and the toolbar to show
        stockpile: #{ credits: 250, water: 120, food: 180, electricity: 60, metals: 90 },
        income: #{ credits: 12, water: 4, food: 6, electricity: 3, metals: 2 }
    }
}

// What a slot starts with: placed on the capital, then on its land neighbours, nearest first.
fn starting_units(slot) {
    ["infantry_division", "motorized_division", "light_tank_division"]
}
