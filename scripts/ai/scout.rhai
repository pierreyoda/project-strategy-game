// The first AI: it scouts.
//
// It plays whole AI nations today, but nothing in it assumes one: it is just as much what a player
// handing their scouts over to automation would run.
//
// Every unit walks to the reachable tile bordering the most ground its nation has never seen, ties
// drawn at random. No two units head for - or through - the same tile in a turn: the second order
// would be refused, since the first unit stands there by the time it is judged.

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

fn plan(view, orders, rng) {
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
