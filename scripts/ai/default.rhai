// The default AI: it keeps an economy going, and scouts with every unit it has.
//
// Each turn, in this order:
// - the economy, one works at a time. It raises a building it has - extractors first, then the
//   lowest level - or failing that builds an extractor on a deposit of its own land, or failing that
//   any other building on free land. Never what would leave it without `reserve_turns()` turns of
//   its upkeep in hand once paid for.
// - scouting: every unit walks to the reachable tile bordering the most ground its nation has never
//   seen, ties drawn at random. No two units head for - or through - the same tile in a turn: the
//   second order would be refused, since the first unit stands there by the time it is judged.
//
// It founds no settlement: what land it builds on is the land it has.

fn plan(view, orders, rng) {
    economy(view, orders, rng);
    scout(view, orders, rng);
}

// ---- the economy ---------------------------------------------------------------------------

// The turns of upkeep a nation keeps in hand, whatever it orders.
fn reserve_turns() {
    10
}

// How much of a resource a set of figures holds: none when it does not name it.
fn amount(figures, resource) {
    let value = figures[resource];
    if value == () {
        0
    } else {
        value
    }
}

// `upkeep`, with `added` paid on top of it and `removed` no longer paid.
fn adjusted(upkeep, added, removed) {
    let total = upkeep;
    for resource in added.keys() {
        total[resource] = amount(total, resource) + added[resource];
    }
    for resource in removed.keys() {
        total[resource] = amount(total, resource) - removed[resource];
    }
    total
}

// What every building it has pays per turn, at the level it works at.
fn upkeep(view, templates) {
    let total = #{};
    for building in view.buildings {
        if building.level == 0 {
            continue;
        }
        let level = templates[building.template].levels[building.level - 1];
        total = adjusted(total, level.maintenance, #{});
    }
    total
}

// Whether it can pay `cost` and still hold `reserve_turns()` turns of `upkeep` - its upkeep with
// whatever the order adds to it.
fn affordable(stockpile, cost, upkeep) {
    for resource in cost.keys() + upkeep.keys() {
        let left = amount(stockpile, resource) - amount(cost, resource);
        if left < reserve_turns() * amount(upkeep, resource) {
            return false;
        }
    }
    true
}

fn economy(view, orders, rng) {
    let templates = #{};
    for template in view.templates {
        templates[template.id] = template;
    }
    // one works at a time: a nation that builds everything at once pays for all of it at once
    for building in view.buildings {
        if building.works != () {
            return;
        }
    }
    let stockpile = view.stockpile;
    let upkeep = upkeep(view, templates);

    // raise what it has: extractors first, then the lowest level, then in the order they were built
    let raisable = [];
    for building in view.buildings {
        let template = templates[building.template];
        if building.halted || building.level >= template.levels.len() {
            continue;
        }
        let rank = if template.deposit == () { 1000 } else { 0 };
        raisable.push(#{ rank: rank + building.level, building: building });
    }
    raisable.sort(|first, second| first.rank - second.rank);
    for entry in raisable {
        let building = entry.building;
        let levels = templates[building.template].levels;
        let current = levels[building.level - 1];
        let next = levels[building.level];
        if affordable(stockpile, next.cost, adjusted(upkeep, next.maintenance, current.maintenance)) {
            orders.upgrade_building(building.id);
            return;
        }
    }

    // build: an extractor on a deposit of its own land, or anything else on its free land
    let extracted = #{};
    for template in view.templates {
        if template.deposit != () {
            extracted[template.deposit] = true;
        }
    }
    let taken = #{};
    for building in view.buildings {
        taken[key(building.tile)] = true;
    }
    // a settlement's own tile is left to the settlement
    for settlement in view.settlements {
        taken[key(settlement.tile)] = true;
    }
    let sites = [];
    let free = [];
    for tile in view.territory {
        let terrain = view.terrain(tile);
        if taken.contains(key(tile)) || terrain == "deep_water" || terrain == "shallow_water" {
            continue;
        }
        let deposit = view.deposit(tile);
        if deposit != () && deposit.level > 0 && extracted.contains(deposit.resource) {
            sites.push(#{ tile: tile, deposit: deposit });
        } else {
            free.push(tile);
        }
    }
    // the richest deposits first
    sites.sort(|first, second| second.deposit.level - first.deposit.level);
    for site in sites {
        for template in view.templates {
            if template.deposit != site.deposit.resource {
                continue;
            }
            let first = template.levels[0];
            if affordable(stockpile, first.cost, adjusted(upkeep, first.maintenance, #{})) {
                orders.construct_building(template.id, site.tile);
                return;
            }
        }
    }
    if free.is_empty() {
        return;
    }
    for template in view.templates {
        if template.deposit != () {
            continue;
        }
        let first = template.levels[0];
        if affordable(stockpile, first.cost, adjusted(upkeep, first.maintenance, #{})) {
            orders.construct_building(template.id, free[rng.range(0, free.len())]);
            return;
        }
    }
}

// ---- scouting ------------------------------------------------------------------------------

// The key a tile is kept under in a set of tiles.
fn key(tile) {
    `${tile.q},${tile.r}`
}

// How much never-seen ground a tile borders.
fn unexplored_around(view, tile) {
    let count = 0;
    for next in view.neighbors(tile) {
        if view.visibility(next) == "unexplored" {
            count += 1;
        }
    }
    count
}

// Whether a path goes through a tile another unit already took this turn.
fn crosses(path, claimed) {
    for step in path {
        if claimed.contains(key(step)) {
            return true;
        }
    }
    false
}

fn scout(view, orders, rng) {
    let claimed = #{};
    for unit in view.units {
        let best = [];
        let best_score = 0;
        for spot in view.reachable(unit.id) {
            // the unit's own tile is no destination
            if spot.cost == 0 {
                continue;
            }
            let path = view.path_to(unit.id, spot.tile);
            if crosses(path, claimed) {
                continue;
            }
            let score = 1 + unexplored_around(view, spot.tile);
            if score > best_score {
                best = [path];
                best_score = score;
            } else if score == best_score {
                best.push(path);
            }
        }
        if best.is_empty() {
            continue;
        }

        let path = best[rng.range(0, best.len())];
        for step in path {
            claimed[key(step)] = true;
        }
        orders.move_unit(unit.id, path);
    }
}
