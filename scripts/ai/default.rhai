// The default AI: it keeps an economy going, and scouts with every unit it has.
//
// Each turn, in this order:
// - the economy, one works at a time - a district being laid out is works too. It raises a building
//   it has - extractors first, then the lowest level - or failing that builds an extractor on a
//   deposit of its own land, or failing that any other building on a district laid out and free, or
//   failing that lays out a district next to a settlement, off the deposits it can extract. Never
//   what would leave it without `reserve_turns()` turns of its upkeep in hand once paid for.
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
    for settlement in view.settlements {
        for district in settlement.districts {
            if district.works != () {
                return;
            }
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

    // build: an extractor on a deposit of its own land, or anything else on a district laid out and
    // free - or failing that, lay a district out
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
    // the deposits it can extract, kept for its extractors
    let kept = #{};
    for tile in view.territory {
        let terrain = view.terrain(tile);
        if taken.contains(key(tile)) || terrain == "deep_water" || terrain == "shallow_water" {
            continue;
        }
        let deposit = view.deposit(tile);
        if deposit != () && deposit.level > 0 && extracted.contains(deposit.resource) {
            sites.push(#{ tile: tile, deposit: deposit });
            kept[key(tile)] = true;
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

    let others = [];
    for template in view.templates {
        if template.deposit == () {
            others.push(template);
        }
    }
    if others.is_empty() {
        return;
    }
    let districted = #{};
    let free = [];
    for settlement in view.settlements {
        for district in settlement.districts {
            districted[key(district.tile)] = true;
            if !taken.contains(key(district.tile)) && !kept.contains(key(district.tile)) {
                free.push(district.tile);
            }
        }
    }
    if !free.is_empty() {
        for template in others {
            let first = template.levels[0];
            if affordable(stockpile, first.cost, adjusted(upkeep, first.maintenance, #{})) {
                orders.construct_building(template.id, free[rng.range(0, free.len())]);
                return;
            }
        }
        return;
    }

    // no district to build on: lay one out on its land next to a settlement - a district has no
    // upkeep of its own, so its cost is all it asks
    for settlement in view.settlements {
        let terms = settlement.district_terms;
        if terms == () || !affordable(stockpile, terms.cost, upkeep) {
            continue;
        }
        let spots = [];
        for tile in view.territory {
            let terrain = view.terrain(tile);
            let at = key(tile);
            if view.distance(settlement.tile, tile) == 1 && !taken.contains(at)
                && !districted.contains(at) && !kept.contains(at)
                && terrain != "deep_water" && terrain != "shallow_water" {
                spots.push(tile);
            }
        }
        if !spots.is_empty() {
            orders.lay_out_district(settlement.id, spots[rng.range(0, spots.len())]);
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
