/** Map an engine Reject debug name (or a server tag) to player-facing copy. */
export const humanReason = (reason: string): string =>
  ({
    NotCastable: "You can't play that right now.",
    NotYourPriority: "It's not your turn to act.",
    CannotPayCost: "Not enough mana for that.",
    CannotDiscardCost: "You don't have cards to discard for that.",
    CannotExileCost: "You don't have cards to exile for that.",
    CannotProduceMana: "That can't make mana right now.",
    CannotActivate: "That ability isn't available.",
    IllegalDeclaration: "That attack or block isn't legal.",
    IllegalTarget: "Pick a legal target.",
    IllegalMode: "Choose a valid mode.",
    WrongTiming: "You can't do that at this time.",
    ChoicePending: "Resolve the current choice first.",
    IllegalChoice: "That choice isn't valid.",
    UnknownObject: "That card is no longer there.",
    UnknownAction: "That action expired — try again.",
    NotYourSeat: "That's not your seat.",
    GameNotStarted: "The game hasn't started yet.",
    UnknownTable: "That table no longer exists.",
    EngineError: "Something went wrong resolving that.",
  })[reason] ?? reason;
