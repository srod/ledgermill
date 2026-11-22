/**
 * JSON data shapes returned and accepted by `@ledgermill/wasm` (matching `crates/ledgermill/src/model.rs`).
 *
 * Import with:
 * `import type { Transaction, Holding, Finding } from "@ledgermill/wasm/types";`
 */

export interface SourceFile {
  name: string;
  content: string;
}

export type Status = "" | "!" | "*";

export interface Source {
  file: string;
  line: number;
}

export type PostingKind = "regular" | "virtual" | "balanced-virtual";

export interface Quantity {
  commodity: string;
  quantity: string;
}

export interface PriceDirective {
  date: string;
  commodity: string;
  rate: Quantity;
  file: string;
  line: number;
}

export interface HoldingsQuery {
  commodity: string;
  end?: string | null;
  today: string;
}

export interface Holding {
  account: string;
  commodity: string;
  units: string;
  cost: Quantity[];
  value: Quantity[];
  cost_value: Quantity[];
}

export type Cost =
  | { kind: "unit"; amount: Quantity }
  | { kind: "total"; amount: Quantity };

export interface Lot {
  cost: Quantity | null;
  date: string | null;
  label: string | null;
}

export interface Amount {
  commodity: string;
  quantity: string;
  cost?: Cost;
  lot?: Lot;
}

export interface Posting {
  account: string;
  kind: PostingKind;
  comment: string;
  amounts: Amount[];
  flag?: string;
  line?: number;
}

export interface Transaction {
  date: string;
  status: Status;
  flag?: string;
  payee?: string;
  description: string;
  comment: string;
  source: Source;
  postings: Posting[];
}

export interface FlatPosting {
  date: string;
  flag: string;
  payee?: string;
  narration: string;
  account: string;
  kind: PostingKind;
  number: string;
  currency: string;
  file: string;
  line: number;
}

export type Severity = "error" | "warning";

export interface Finding {
  code: string;
  severity: Severity;
  file: string;
  line: number;
  column?: number;
  details: Record<string, unknown>;
  message: string;
}
