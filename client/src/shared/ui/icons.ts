import Anvil from "@lucide/svelte/icons/anvil";
import Bell from "@lucide/svelte/icons/bell";
import Building from "@lucide/svelte/icons/building";
import Check from "@lucide/svelte/icons/check";
import ChevronDown from "@lucide/svelte/icons/chevron-down";
import ChevronRight from "@lucide/svelte/icons/chevron-right";
import Coins from "@lucide/svelte/icons/coins";
import Droplet from "@lucide/svelte/icons/droplet";
import FlaskConical from "@lucide/svelte/icons/flask-conical";
import Footprints from "@lucide/svelte/icons/footprints";
import Fuel from "@lucide/svelte/icons/fuel";
import Heart from "@lucide/svelte/icons/heart";
import Hourglass from "@lucide/svelte/icons/hourglass";
import Info from "@lucide/svelte/icons/info";
import Lock from "@lucide/svelte/icons/lock";
import MapIcon from "@lucide/svelte/icons/map";
import Minus from "@lucide/svelte/icons/minus";
import Plus from "@lucide/svelte/icons/plus";
import Radiation from "@lucide/svelte/icons/radiation";
import Settings from "@lucide/svelte/icons/settings";
import Shield from "@lucide/svelte/icons/shield";
import Swords from "@lucide/svelte/icons/swords";
import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
import Users from "@lucide/svelte/icons/users";
import Wheat from "@lucide/svelte/icons/wheat";
import X from "@lucide/svelte/icons/x";
import Zap from "@lucide/svelte/icons/zap";

/**
 * The game's icons, by meaning rather than by drawing: art can be swapped without touching their usages.
 *
 * Drawings come from [Lucide](https://lucide.dev/icons) for now.
 */
export const icons = {
  // resources, mirroring `core::simulation::resources::Resource`
  credits: Coins,
  water: Droplet,
  food: Wheat,
  electricity: Zap,
  metals: Anvil,
  oil: Fuel,
  uranium: Radiation,
  // game
  population: Users,
  settlement: Building,
  research: FlaskConical,
  attack: Swords,
  defense: Shield,
  movement: Footprints,
  health: Heart,
  turn: Hourglass,
  map: MapIcon,
  // interface
  close: X,
  "chevron-down": ChevronDown,
  "chevron-right": ChevronRight,
  plus: Plus,
  minus: Minus,
  check: Check,
  info: Info,
  warning: TriangleAlert,
  lock: Lock,
  notifications: Bell,
  settings: Settings,
};

export type IconName = keyof typeof icons;
