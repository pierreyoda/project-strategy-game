import { RESOURCES, type Resource, type ResourceStock } from "../resources";
import type { ProjectionMeta } from "../session/protocol";

/**
 * The viewer's stockpiles, every resource in: what core forecasts the next turn gains and loses of each - buildings'
 * production and maintenance included - is taken as is, so the toolbar's figure is the change the turn makes. A
 * resource the projection leaves out is held at 0 and does not change.
 */
export const stocksOf = (meta: ProjectionMeta | null, viewer: number): Record<Resource, ResourceStock> => {
  const stocks = Object.fromEntries(
    RESOURCES.map((resource) => [resource, { quantity: 0, income: 0, depletion: 0 }]),
  ) as Record<Resource, ResourceStock>;
  const nation = meta?.nations.find(({ id }) => id === viewer);
  for (const { resource, quantity, income, depletion } of nation?.resources ?? []) {
    stocks[resource] = { quantity, income, depletion };
  }
  return stocks;
};
