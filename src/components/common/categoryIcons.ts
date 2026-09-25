import type { Category } from "../../lib/api/types";
import type { IconName } from "./icons";

export const CATEGORY_ICON: Record<Category, IconName> = {
  system: "windows",
  apps: "apps",
  games: "gamepad",
  media: "film",
  dev: "code",
  cache: "database",
  user_files: "user",
  virtualization: "box",
  messaging: "message",
  browsers: "globe",
  unknown: "help",
};
