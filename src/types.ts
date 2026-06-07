export interface Snippet {
  id: string;
  abbreviation: string;
  expansion: string;
  enabled: boolean;
  group: string;
}

export interface Stats {
  total_expansions: number;
  chars_saved: number;
}

export interface Settings {
  require_word_boundary: boolean;
  blacklist: string[];
}
