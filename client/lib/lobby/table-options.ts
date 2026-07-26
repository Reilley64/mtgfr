export type TableOptionsRequest = {
  tableId: string;
  commanderDamageEnabled: boolean;
};

export function parseTableOptionsBody(
  body: Record<string, unknown>,
): TableOptionsRequest | "BadJson" {
  if (typeof body.commander_damage_enabled !== "boolean") {
    return "BadJson";
  }
  return {
    tableId: String(body.table_id ?? ""),
    commanderDamageEnabled: body.commander_damage_enabled,
  };
}
