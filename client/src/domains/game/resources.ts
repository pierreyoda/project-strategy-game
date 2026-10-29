/** Resources, mirroring `Resource` in the `core` crate, in the same order. Their icons share their names. */
export const RESOURCES = ["credits", "water", "food", "electricity", "metals", "oil", "uranium"] as const;
export type Resource = (typeof RESOURCES)[number];

export const RESOURCE_NAMES: Record<Resource, string> = {
  credits: "Credits",
  water: "Water",
  food: "Food",
  electricity: "Electricity",
  metals: "Metals",
  oil: "Oil",
  uranium: "Uranium",
};

/** Stock of a resource, mirroring `ResourceData` in the `core` crate: a missing income or depletion is 0. */
export type ResourceStock = {
  quantity: number;
  /** Gained each turn. */
  income: number;
  /** Consumed each turn. */
  depletion: number;
};

/** How much the quantity changes each turn. */
export const netIncome = ({ income, depletion }: ResourceStock): number => income - depletion;
