import { describe, expect, it } from "vitest";
import { cardBugReportUrl } from "~/lib/cardBugReport";

describe("cardBugReportUrl", () => {
  it("opens the card-bug issue form with card name and table id", () => {
    const url = new URL(
      cardBugReportUrl({
        cardName: "Sol Ring",
        tableId: "ABC123",
      }),
    );

    expect(url.origin + url.pathname).toBe("https://github.com/reilley64/mtgfr/issues/new");
    expect(url.searchParams.get("template")).toBe("card-bug.yml");
    expect(url.searchParams.get("title")).toBe("card: Sol Ring");
    expect(url.searchParams.get("card_name")).toBe("Sol Ring");
    expect(url.searchParams.get("table_id")).toBe("ABC123");
    expect(url.searchParams.has("card_id")).toBe(false);
    expect(url.searchParams.has("object_id")).toBe(false);
    expect(url.searchParams.has("approximates")).toBe(false);
  });

  it("includes oracle and object ids when known", () => {
    const url = new URL(
      cardBugReportUrl({
        cardName: "Emry, Lurker of the Loch",
        tableId: "XYZ9",
        cardId: "oracle-emry",
        objectId: 42,
      }),
    );

    expect(url.searchParams.get("card_id")).toBe("oracle-emry");
    expect(url.searchParams.get("object_id")).toBe("42");
    expect(url.searchParams.get("title")).toBe("card: Emry, Lurker of the Loch");
  });

  it("prefills the approximation note when the inspect face has one", () => {
    const url = new URL(
      cardBugReportUrl({
        cardName: "Complicated Card",
        tableId: "T9",
        approximates: "Ignores intervening-if; always draws on ETB.",
      }),
    );

    expect(url.searchParams.get("approximates")).toBe("Ignores intervening-if; always draws on ETB.");
  });

  it("omits blank optional ids and empty approximates", () => {
    const url = new URL(
      cardBugReportUrl({
        cardName: "Forest",
        tableId: "T1",
        cardId: "  ",
        approximates: "   ",
      }),
    );

    expect(url.searchParams.has("card_id")).toBe(false);
    expect(url.searchParams.has("object_id")).toBe(false);
    expect(url.searchParams.has("approximates")).toBe(false);
  });
});
