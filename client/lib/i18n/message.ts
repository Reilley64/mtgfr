import type { MessageParam, MessageRef } from "../wire/types";
import { enCatalog } from "./catalog/en";

export type MessageValue = boolean | number | string;
export type MessageParams = Readonly<Record<string, MessageValue>>;
export type MessageFormatter = (params: MessageParams, children: readonly string[]) => string;

const amountTokens: Readonly<Record<string, string>> = {
  fixed: "",
  half_x: "half X",
  half_x_rounded_down: "half X rounded down",
  per_creature_on_battlefield: "for each creature on the battlefield",
  per_creature_you_control: "for each creature you control",
  per_permanent_matching: "for each matching permanent",
  source_power: "source's power",
  source_toughness: "source's toughness",
  target_power: "target's power",
  twice_x: "twice X",
  x: "X",
};

function messageParamValue(param: MessageParam): MessageValue {
  if (param.string_value != null) return param.string_value;
  if (param.int_value != null) return param.int_value;
  if (param.bool_value != null) return param.bool_value;
  if (param.amount_token != null) return amountTokens[param.amount_token] ?? humanizeToken(param.amount_token);
  return "";
}

function paramsOf(params: readonly MessageParam[]): MessageParams {
  const out: Record<string, MessageValue> = {};
  for (const param of params) out[param.name] = messageParamValue(param);
  return out;
}

export function humanizeToken(value: MessageValue): string {
  if (typeof value !== "string") return String(value);
  return value
    .replace(/_/g, " ")
    .replace(/\bmv\b/g, "mana value")
    .replace(/\blte\b/g, "at most")
    .replace(/\bgte\b/g, "at least")
    .replace(/\bpt\b/g, "power/toughness")
    .replace(/\bgy\b/g, "graveyard")
    .replace(/\s+/g, " ")
    .trim();
}

export function formatMessage(ref: MessageRef | null | undefined): string {
  if (ref == null) return "";
  if (typeof ref === "string") return ref;

  const formatter = enCatalog[ref.key];
  if (formatter == null) return ref.key;

  return formatter(
    paramsOf(ref.params),
    ref.children.map((child) => formatMessage(child)),
  );
}
