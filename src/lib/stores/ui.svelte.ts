import type { HeuristicKind, UiMode } from "../api/types";

export type View = "home" | "cleanup" | "history" | "growth" | "settings" | "about" | "heuristics";
export type ExpertTab = "tree" | "treemap" | "sunburst" | "largest" | "types" | "denied";

class UiState {
  view = $state<View>("home");
  mode = $state<UiMode>("simple");
  expertTab = $state<ExpertTab>("tree");
  heuristicKind = $state<HeuristicKind>("duplicates");
  showOnboarding = $state(false);

  go(view: View): void {
    this.view = view;
  }

  openHeuristics(kind: HeuristicKind): void {
    this.heuristicKind = kind;
    this.view = "heuristics";
  }
}

export const ui = new UiState();
