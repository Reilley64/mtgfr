//! Canonical engine events, redacted per-viewer for the wire.
//!
//! The engine emits canonical, full-information [`engine::Event`]s. Before an event reaches a
//! player it is passed through [`redact`], which strips information that player may not
//! legally see (a drawn card's identity is private to its owner). Redaction lives in
//! [`crate::projection`], never in the engine — the engine stays audience-unaware.

use serde::{Deserialize, Serialize};

use crate::ObjectId;
use crate::dto::VisibleState;
use crate::intent::WireTarget;

/// A batch of already-redacted events for one viewer plus the viewer's full render state
/// after they were applied. The `events` drive the game log / stack panel / combat highlights
/// (folded client-side); `state` is the board the client renders. Carrying both makes each
/// delta self-sufficient — the client never re-fetches a snapshot mid-stream (see ADR 0006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaEnvelope {
    pub seq: u64,
    pub events: Vec<VisibleEvent>,
    pub state: VisibleState,
    /// Human-readable labels of actions the server auto-submitted in this frame (a forced
    /// discard, an auto-passed priority with nothing to do) — for the client's "automatic"
    /// styling, so a forced play is never mistaken for the player's own move.
    #[serde(default)]
    pub auto_actions: Vec<String>,
}

/// An [`engine::Event`] after per-viewer redaction. Public facts pass through
/// unchanged; private facts (a drawn card's identity) become `None` for players
/// who may not see them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibleEvent {
    SpellCast {
        spell: ObjectId,
        from: ObjectId,
        controller: u8,
        target: Option<WireTarget>,
        /// Whether this was a flashback cast (CR 702.34) — public, like the rest of the stack.
        flashback: bool,
        /// Whether this was an escape cast (CR 702.19) — public, like the rest of the stack.
        escape: bool,
    },
    /// A multi-target spell's chosen targets were recorded (CR 601.2c). Public — the stack is
    /// visible, so every seat sees what a spell targets.
    SpellTargetsChosen {
        spell: ObjectId,
        targets: Vec<WireTarget>,
    },
    /// A permanent's "prepared" status changed (soc/sos prepare DFCs). Public battlefield status.
    PreparedChanged {
        object: ObjectId,
        prepared: bool,
    },
    /// A Class permanent gained a level (CR 717.2). Public battlefield status, like
    /// `PreparedChanged`.
    LeveledUp {
        object: ObjectId,
        level: u8,
    },
    /// A permanent flipped (CR 712 — a Kamigawa flip card): it now uses its back face's
    /// characteristics. Public battlefield status, like `PreparedChanged`; the client swaps to the
    /// back face from its own card data.
    Flipped {
        object: ObjectId,
    },
    /// A permanent phased out (CR 702.26 — Guardian of Faith). Public battlefield status, like
    /// `PreparedChanged`; anything attached to it phased out with it (the client mirrors the
    /// attachment cascade from its own board state).
    PhasedOut {
        object: ObjectId,
    },
    /// A permanent phased in (CR 702.26f — at its controller's untap). Public, the mirror of
    /// `PhasedOut`.
    PhasedIn {
        object: ObjectId,
    },
    /// An as-enters "choose a creature type" choice was answered (CR 614.12/700.9-style —
    /// Patchwork Banner). Public battlefield status, like `PreparedChanged`.
    CreatureTypeChosen {
        object: ObjectId,
        subtype: String,
    },
    /// An as-enters "choose a color" choice was answered (CR 614.12/700.9-style — Flickering
    /// Ward). Public battlefield status, like `CreatureTypeChosen`. `color` is the WUBRG index.
    ColorChosen {
        object: ObjectId,
        color: u8,
    },
    /// A "becomes the color of your choice until end of turn" choice was answered (CR 613.3c
    /// layer 5 — Wild Mongrel). Public battlefield status, like `ColorChosen`; `color` is the
    /// WUBRG index. Cleared alongside the other until-end-of-turn boosts (`TempBoostsEnded`).
    ColorSetUntilEndOfTurn {
        object: ObjectId,
        color: u8,
    },
    /// A copy of a prepared permanent's back-face spell went on the stack (soc/sos prepare DFCs).
    /// Public — the stack is visible.
    PreparedSpellCast {
        spell: ObjectId,
        source: ObjectId,
        controller: u8,
        target: Option<WireTarget>,
        /// The back face's chosen `{X}` value (0 for a non-`{X}` back face) — public, like the
        /// rest of the stack.
        x: u32,
    },
    /// The adventure half of an adventure card went on the stack (CR 715). Public — the stack is
    /// visible.
    AdventureSpellCast {
        spell: ObjectId,
        source: ObjectId,
        controller: u8,
        target: Option<WireTarget>,
        /// The adventure's chosen `{X}` value (0 for a non-`{X}` adventure) — public.
        x: u32,
    },
    StepBegan {
        /// Step discriminant; see `engine::Step`.
        step: u8,
        active_player: u8,
    },
    /// A triggered ability went on the stack (its effect detail is engine-internal).
    TriggeredAbilityOnStack {
        controller: u8,
        source: ObjectId,
        target: Option<WireTarget>,
    },
    AbilityResolved {
        source: ObjectId,
    },
    /// An activated ability on the stack was countered (CR 701.5c/112.7a — Azorius Guildmage). It
    /// leaves the stack and ceases to exist; nothing is hidden (which ability was countered is
    /// public), so this is a straight passthrough of the engine event's `source`.
    AbilityCountered {
        source: ObjectId,
    },
    LandPlayed {
        permanent: ObjectId,
        from: ObjectId,
        player: u8,
    },
    Tapped {
        object: ObjectId,
    },
    Untapped {
        object: ObjectId,
    },
    /// A permanent was removed from combat (CR 506.4 — `Effect::RemoveFromCombat`).
    RemovedFromCombat {
        object: ObjectId,
    },
    /// A regeneration shield was granted to a permanent (CR 701.15b).
    RegenerationShieldCreated {
        object: ObjectId,
    },
    /// A permanent was regenerated instead of destroyed (CR 701.15b) — tapped, removed from
    /// combat, damage healed, one shield consumed.
    Regenerated {
        object: ObjectId,
    },
    /// A permanent's regeneration shields expired at cleanup (CR 701.15b's "this turn").
    RegenerationShieldsExpired {
        object: ObjectId,
    },
    LostSummoningSickness {
        object: ObjectId,
    },
    CountersPlaced {
        object: ObjectId,
        count: i32,
    },
    /// A named non-P/T counter (CR 122.1 — charge, story, …) was placed on (`count` positive) or
    /// removed from (negative) a permanent — the [`Self::CountersPlaced`] sibling for a counter
    /// kind other than +1/+1. `counter_kind` mirrors `engine::CounterKind`'s discriminant
    /// (0 = charge, 1 = story), the same raw-index convention `Color`/`Step` cross the wire with
    /// (named `counter_kind`, not `kind`, because `kind` is this enum's own internal tag key).
    KindCountersPlaced {
        object: ObjectId,
        counter_kind: u8,
        count: i32,
    },
    /// A planeswalker's loyalty changed by `amount` (a loyalty ability's +N / 0 / −N cost).
    LoyaltyChanged {
        object: ObjectId,
        amount: i32,
    },
    /// A planeswalker's once-per-turn loyalty flag was set/cleared (CR 606.3).
    LoyaltyActivated {
        object: ObjectId,
        active: bool,
    },
    /// A `once_each_turn`-capped activated ability was activated (CR 602.2b).
    AbilityActivatedThisTurn {
        object: ObjectId,
        ability_index: usize,
    },
    /// A `once_each_turn`-capped *triggered* ability (Morbid Opportunist, Tocasia's Welcome) was
    /// placed on the stack.
    TriggeredAbilityThisTurn {
        source: ObjectId,
    },
    /// An Aura/Equipment's attachment changed (`host` absent = became unattached).
    AttachedTo {
        object: ObjectId,
        host: Option<ObjectId>,
    },
    TempBoost {
        object: ObjectId,
        power: i32,
        toughness: i32,
    },
    TempBoostsEnded {
        object: ObjectId,
    },
    /// A permanent's base power/toughness was set until end of turn (Biomass Mutation, Quandrix
    /// Charm). Public battlefield status, like `TempBoost`.
    BasePtSetUntilEndOfTurn {
        object: ObjectId,
        power: i32,
        toughness: i32,
    },
    /// A permanent gained card types/subtypes until end of turn (Restless Spire's self-animation).
    /// Public battlefield status, like `BasePtSetUntilEndOfTurn`.
    TypesAddedUntilEndOfTurn {
        object: ObjectId,
    },
    /// A just-reanimated permanent took on an indefinite characteristics set (Excava, the Risen
    /// Past → a 1/1 Spirit with flying). Public battlefield status, like `BasePtSetUntilEndOfTurn`.
    ReanimatedCreatureBecame {
        object: ObjectId,
    },
    /// A permanent gained an indefinite set of creature subtypes (Hofri Ghostforge's minted copy →
    /// a Spirit). Public battlefield status, like `ReanimatedCreatureBecame`.
    AddedSubtypes {
        object: ObjectId,
    },
    /// A permanent became a copy of another creature as it entered (Altered Ego, Cursed Mirror).
    /// Its projected name/types change accordingly — a copy is public. The copied `def` isn't
    /// threaded onto the wire event (the client's per-object state comes from a fresh snapshot each
    /// delta), like `AddedSubtypes`.
    BecameCopy {
        object: ObjectId,
    },
    /// A permanent lost keywords until end of turn and can't regain them (arcane_lighthouse's
    /// strip). Public battlefield status, like `TempBoost`.
    KeywordsStripped {
        object: ObjectId,
    },
    /// A one-shot control change took effect (CR 720): `object` is now controlled by
    /// `controller` until end of turn (Besmirch).
    ControlGainedUntilEndOfTurn {
        object: ObjectId,
        controller: u8,
    },
    /// An until-end-of-turn control override ended (cleanup, CR 514.2).
    ControlEndedUntilEndOfTurn {
        object: ObjectId,
    },
    /// `target` gained `source`'s other abilities until end of turn (CR 702.166 Backup — Guardian
    /// Scalelord). Public battlefield state.
    AbilitiesGranted {
        target: ObjectId,
        source: ObjectId,
    },
    /// Every until-end-of-turn ability grant ended (cleanup, CR 514.2 / 702.166).
    GrantedAbilitiesEnded,
    /// A permanent control change with no stated duration (CR 720 — Entrancing Melody):
    /// `object` is now controlled by `controller`, with no cleanup reversion.
    ControlGained {
        object: ObjectId,
        controller: u8,
    },
    /// A condition-scoped control change took effect (CR 611.2b — Rubinia Soulsinger): `object` is
    /// now controlled by `controller` for as long as the condition holds.
    ConditionedControlGained {
        object: ObjectId,
        controller: u8,
    },
    /// A condition-scoped control override ended because its condition stopped holding (the source
    /// untapped, left, or changed controller — CR 611.2b).
    ConditionedControlEnded {
        object: ObjectId,
    },
    AttackerDeclared {
        object: ObjectId,
        defender: u8,
    },
    /// A token was put onto the battlefield already tapped and attacking `defender` (Combat
    /// Calligrapher), not via the declare-attackers step (CR 508.4).
    TokenEnteredAttacking {
        token: ObjectId,
        defender: u8,
    },
    /// A creature was goaded by player `by` (CR 701.38).
    Goaded {
        object: ObjectId,
        by: u8,
    },
    /// Every goad done by player `by` ended (the start of `by`'s turn).
    GoadCleared {
        by: u8,
    },
    /// `object` was marked to skip its controller's next untap step (Pollen Lullaby). Public — a
    /// board fact, like goad.
    NextUntapSkipMarked {
        object: ObjectId,
    },
    /// `object`'s skip-next-untap mark was consumed as its controller's untap step arrived.
    NextUntapSkipConsumed {
        object: ObjectId,
    },
    /// A vow counter was placed on `object`, marking `protected` as the player it can't attack
    /// (Promise of Loyalty). Public — a counter placement happens openly, like the other counters.
    VowCountersPlaced {
        object: ObjectId,
        protected: u8,
    },
    /// `count` time counters were placed on the suspended card `card` in exile (Rousing Refrain,
    /// suspend). Public — a suspended card sits face-up in exile with its counters visible.
    TimeCountersPlaced {
        card: ObjectId,
        count: u32,
    },
    /// One time counter was removed from the suspended card `card` (the upkeep tick). Public,
    /// like [`Self::TimeCountersPlaced`].
    TimeCountersRemoved {
        card: ObjectId,
    },
    /// `object` must attack `defender` this turn if able (Furygale Flocking's minted tokens).
    /// Public combat state, like [`Self::Goaded`].
    MustAttackDeclared {
        object: ObjectId,
        defender: u8,
    },
    /// A CR 603.7 delayed trigger was scheduled: `controller` will act the next time its step
    /// begins (the effect/step detail is engine-internal, like `TriggeredAbilityOnStack`'s).
    DelayedTriggerScheduled {
        controller: u8,
        source: ObjectId,
    },
    /// Every scheduled delayed trigger for one step fired.
    DelayedTriggersFired,
    /// A CR 603.7 delayed one-shot was armed: `controller` will act the next time they cast a
    /// spell matching its filter this turn (the filter/effect detail is engine-internal, like
    /// `DelayedTriggerScheduled`'s).
    NextCastTriggerArmed {
        controller: u8,
        source: ObjectId,
    },
    /// A `NextCastTriggerArmed` watch fired and was removed.
    NextCastTriggerConsumed {
        controller: u8,
        source: ObjectId,
    },
    /// A CR 603.7 delayed watch was armed: `source` will become prepared the first time
    /// `watched` deals combat damage to a player, any time later this combat.
    CombatDamageWatchArmed {
        controller: u8,
        source: ObjectId,
        watched: ObjectId,
    },
    /// A `CombatDamageWatchArmed` watch fired and was removed.
    CombatDamageWatchConsumed {
        controller: u8,
        source: ObjectId,
    },
    /// A this-turn, controller-scoped, repeatable CR 603.7 delayed watch was armed: every
    /// creature `controller` controls that deals combat damage to a player for the rest of this
    /// turn mints a free copy of `card` (an already-exiled instant/sorcery). Unlike
    /// `CombatDamageWatchArmed`, never consumed — it keeps firing until the turn ends.
    CombatDamageCopyArmed {
        controller: u8,
        source: ObjectId,
        card: ObjectId,
    },
    /// A card was impulse-exiled face-up and `player` may play it until end of turn, or until the
    /// end of `player`'s next turn if `until_next_turn` (CR 118.6; Atsushi, the Blazing Sky).
    /// Public — the card is exiled face-up, like a card milled into a graveyard.
    ExiledFromLibraryMayPlay {
        player: u8,
        card: ObjectId,
        from: ObjectId,
        until_next_turn: bool,
    },
    /// A card was exiled face-up from the top of the library with no play permission attached
    /// (Herald of Amity's dig) — a follow-up choice decides which one, if any, gets free-cast
    /// permission. Public — the card is exiled face-up.
    ExiledFromLibraryToChooseCastFree {
        player: u8,
        card: ObjectId,
        from: ObjectId,
    },
    /// An extended impulse-draw permission's shield expired (its controller's own next turn
    /// began) — it now expires at this turn's cleanup like a normal permission.
    PlayFromExilePermissionArmed {
        card: ObjectId,
    },
    /// All impulse-draw play-until-end-of-turn permissions that aren't still shielded expired
    /// (cleanup).
    PlayFromExileEnded,
    BlockerDeclared {
        blocker: ObjectId,
        attacker: ObjectId,
    },
    /// A multi-blocked attacker's combat damage was divided among its blockers (public —
    /// combat damage assignment is announced to the table, CR 510.1c).
    CombatDamageDivided {
        attacker: ObjectId,
        assignment: Vec<(ObjectId, i32)>,
    },
    /// A divided-damage spell's total was split among its chosen targets (public — a divided
    /// burn spell's split is announced to the table, CR 601.2d, like combat's). `players` carries
    /// the shares dealt to player targets ("any number of targets" includes players); `assignment`
    /// the object shares.
    SpellDamageDivided {
        spell: ObjectId,
        assignment: Vec<(ObjectId, i32)>,
        players: Vec<(u8, i32)>,
    },
    /// A divided-counters spell's total was split among its chosen targets (public — CR 601.2d,
    /// the counter twin of `SpellDamageDivided`; Grove's Bounty).
    SpellCountersDivided {
        spell: ObjectId,
        assignment: Vec<(ObjectId, i32)>,
    },
    DeathtouchMarked {
        object: ObjectId,
    },
    CombatCleared,
    CommanderCastFromCommandZone {
        player: u8,
    },
    /// `player` may cast spells this turn as though they had flash (CR 601.3a — Alchemist's
    /// Refuge). Public — no hidden information.
    FlashPermissionGranted {
        player: u8,
    },
    /// `player` may, at mana-ability timing, pay 1 life to add {C} for the rest of the turn (CR
    /// 605 — Yavimaya Bloomsage's Channel). Public — no hidden information.
    ChannelColorlessManaGranted {
        player: u8,
    },
    CommanderDamageDealt {
        source: ObjectId,
        player: u8,
        amount: i32,
    },
    /// A creature dealt combat damage to a player (public — combat damage is announced to the
    /// table, CR 510.1c/510.4).
    CombatDamageDealtToPlayer {
        source: ObjectId,
        player: u8,
        amount: i32,
    },
    /// A creature dealt combat damage to another creature (public — combat damage is announced to
    /// the table, CR 510.1c/510.4) — the creature-target twin of `CombatDamageDealtToPlayer`.
    CombatDamageDealtToCreature {
        source: ObjectId,
        target: ObjectId,
        amount: i32,
    },
    /// `source` dealt noncombat damage to `player` (public — damage to a player is announced,
    /// CR 120.1) — the noncombat twin of `CombatDamageDealtToPlayer`.
    DamageDealtToPlayer {
        source: ObjectId,
        player: u8,
        amount: i32,
    },
    /// `amount` combat damage that would have been dealt to `player` was prevented by a shield
    /// (Inkshield, CR 615) — public, like the combat damage it replaces. The Inkling mints it
    /// drives arrive as accompanying `TokenCreated` events.
    CombatDamagePrevented {
        player: u8,
        amount: i32,
    },
    MovedToCommandZone {
        card: ObjectId,
        from: ObjectId,
    },
    ManaEmptied {
        player: u8,
    },
    DamageCleared {
        object: ObjectId,
    },
    ManaAdded {
        player: u8,
        /// Mana kind: 0-4 = WUBRG (`engine::Color::index`), 5 = colorless `{C}`,
        /// 6 = any color, 7 = either of two colors (a dual land's credit), 8 = a restricted
        /// color-set credit (Fellwar Stone / Exotic Orchard).
        mana: u8,
        amount: u8,
    },
    ManaSpent {
        player: u8,
        /// Per-color amounts removed, indexed WUBRG.
        /// ponytail: colorless/any spend isn't surfaced on the wire (the client renders a
        /// WUBRG pool); add it here when the UI shows `{C}`.
        mana: Vec<u8>,
    },
    PriorityPassed {
        player: u8,
    },
    PermanentEntered {
        permanent: ObjectId,
        from: ObjectId,
    },
    /// A graveyard card was reanimated onto the battlefield under `controller`'s control.
    ReanimatedToBattlefield {
        permanent: ObjectId,
        from: ObjectId,
        controller: u8,
        /// Whether it entered with a finality counter (CR 614.12 — Excava, the Risen Past).
        finality: bool,
        /// Whether it entered tapped (Teacher's Pest's "... to the battlefield tapped").
        tapped: bool,
    },
    TokenCreated {
        token: ObjectId,
        controller: u8,
        /// Resolving stack object or ability source (ADR 0033). Absent on older peers.
        creator: Option<ObjectId>,
    },
    TokenCeasedToExist {
        token: ObjectId,
    },
    /// A spell on the stack was copied (Twincast): a new spell object `copy` was put on the stack
    /// above `original`, controlled by `controller`.
    SpellCopied {
        copy: ObjectId,
        original: ObjectId,
        controller: u8,
    },
    /// A spell copy resolved and ceased to exist (it left the stack without becoming a card).
    SpellCeasedToExist {
        spell: ObjectId,
    },
    DamageMarked {
        object: ObjectId,
        amount: i32,
        /// What dealt the damage (for the log); `None` for engine-internal adjustments.
        source: Option<ObjectId>,
    },
    MovedToGraveyard {
        card: ObjectId,
        from: ObjectId,
    },
    MovedToExile {
        card: ObjectId,
        from: ObjectId,
    },
    /// An adventure spell resolved and was exiled "on an adventure" (CR 715.3d) as the card
    /// `card`. Public, like [`Self::MovedToExile`] (the exile zone is visible).
    ExiledOnAdventure {
        card: ObjectId,
        from: ObjectId,
        owner: u8,
    },
    /// The O-Ring pattern (CR 603.6e): `object` was exiled until `source` leaves the battlefield.
    ExiledUntilSourceLeaves {
        source: ObjectId,
        object: ObjectId,
    },
    /// Skyclave Apparition's linked exile: `object` was exiled linked to `source`, staying
    /// exiled forever — `source` leaving the battlefield mints its owner an Illusion instead of
    /// returning it (see `LeavesIllusionMinted`).
    ExiledUntilSourceLeavesMintingIllusion {
        source: ObjectId,
        object: ObjectId,
    },
    /// `source`'s linked exile (see `ExiledUntilSourceLeavesMintingIllusion`) finished minting
    /// its Illusion token — the exiled `object` stays exiled; the mint itself is a separate,
    /// public `TokenCreated`.
    LeavesIllusionMinted {
        source: ObjectId,
        object: ObjectId,
    },
    /// Hofri Ghostforge's minted Spirit token `token` gained its granted "When this token
    /// leaves the battlefield, return the exiled card to its owner's graveyard" rider, linked to
    /// the specific card `exiled` it must return.
    TokenGrantedReturnExiledOnLeave {
        token: ObjectId,
        exiled: ObjectId,
    },
    /// The granted rider's payoff: the exile-zone card `from` arrived in its owner's graveyard
    /// as `card` (public — the graveyard is a public zone). Not a death (CR 700.4 requires
    /// "from the battlefield") — see the engine event's doc.
    ReturnedExiledCardToGraveyard {
        card: ObjectId,
        from: ObjectId,
    },
    /// The "exiled with" pattern (CR 400.10a): `object` was exiled linked to `source`, staying
    /// there until a different ability of `source` cashes it out (Currency Converter).
    ExiledWithSource {
        source: ObjectId,
        object: ObjectId,
    },
    /// `source`'s cash-out ability pulled `object` back out of its exiled-with pile.
    CardExiledWithSourceLeftExile {
        source: ObjectId,
        object: ObjectId,
    },
    /// Quintorius, Loremaster's activated ability granted `player` permission to cast `card`
    /// (still sitting in its exiled-with pile) this turn without paying its mana cost. Public —
    /// the card is exile-zone, like the rest of the "exiled with" pattern.
    CastFromExileFreePermissionGranted {
        card: ObjectId,
        player: u8,
    },
    /// Quintorius, Loremaster's activated ability also granted `card` a one-shot "put into a
    /// graveyard" replacement (CR 614.6) — bottom of its owner's library instead. Public, like
    /// the rest of the "exiled with" pattern.
    CastFromExileFreeBottomsLibraryOnLeave {
        card: ObjectId,
    },
    /// Every active free-cast-from-exile permission expired (cleanup).
    CastFromExileFreeEnded,
    /// The linked exile ended: the card `from` returned to the battlefield as `permanent`, under
    /// `controller`'s control.
    ReturnedFromLinkedExile {
        permanent: ObjectId,
        from: ObjectId,
        controller: u8,
        source: ObjectId,
    },
    /// A flicker's return (immediate or the delayed end-step twin): the exiled card `from`
    /// returned to the battlefield as `permanent`, under `controller`'s control.
    FlickeredToBattlefield {
        permanent: ObjectId,
        from: ObjectId,
        controller: u8,
    },
    ReturnedToHand {
        card: ObjectId,
        from: ObjectId,
    },
    TuckedToLibrary {
        card: ObjectId,
        from: ObjectId,
        to_top: bool,
        second_from_top: bool,
    },
    /// `player`'s library was shuffled (Perpetual Timepiece's mandatory shuffle). No order
    /// information — the library is a hidden zone.
    LibraryShuffled {
        player: u8,
    },
    /// The top card of `player`'s library was revealed — public to every player (CR 701.30),
    /// unlike a private look. The card stays on top; a later event moves it if the effect does.
    RevealedTopOfLibrary {
        player: u8,
        card: ObjectId,
        def: String,
    },
    /// A previously-revealed card went to the bottom of `player`'s own library (Open the Way's
    /// non-matching reveals) — public, since [`Self::RevealedTopOfLibrary`] already showed it;
    /// not a zone change, so `card` keeps its id.
    PutOnBottomOfLibrary {
        player: u8,
        card: ObjectId,
    },
    /// A tutored library card entered `player`'s hand. Like a draw, the identity is private —
    /// `card` (and the library object it came `from`, which would otherwise leak that identity
    /// by its decklist-order id) are `None` for anyone but the searcher.
    SearchedToHand {
        player: u8,
        object: ObjectId,
        from: Option<ObjectId>,
        card: Option<String>,
    },
    /// A searched library card was put onto the battlefield under `controller`'s control (ramp /
    /// fetchland). The card is public once it's on the battlefield.
    SearchedToBattlefield {
        permanent: ObjectId,
        from: ObjectId,
        controller: u8,
        tapped: bool,
    },
    /// A library card was manifested (CR 701.34) onto the battlefield face down as a 2/2 under
    /// `controller`'s control. The moved card's identity is private (it came off the private
    /// library and stays hidden while face down), so `from` is dropped — the anonymous permanent
    /// is projected via [`ObjectView::face_down`].
    Manifested {
        permanent: ObjectId,
        controller: u8,
    },
    /// A face-down permanent was turned face up (CR 701.34e) — its real card is now revealed
    /// (public game information).
    TurnedFaceUp {
        permanent: ObjectId,
    },
    /// A hand land card was put onto the battlefield under `controller`'s control (CR 305.9,
    /// not a land play — Eureka Moment, Zimone). The card is public once it's on the battlefield.
    PutOntoBattlefieldFromHand {
        permanent: ObjectId,
        from: ObjectId,
        controller: u8,
        tapped: bool,
    },
    /// A card was milled into `player`'s graveyard (public — the graveyard is a public zone).
    Milled {
        player: u8,
        card: ObjectId,
        from: ObjectId,
    },
    LifeChanged {
        player: u8,
        amount: i32,
        /// What caused the change (for the log); `None` for setup adjustments.
        source: Option<ObjectId>,
    },
    DrewFromEmptyLibrary {
        player: u8,
    },
    PlayerLost {
        player: u8,
    },
    /// `player` got the city's blessing (CR 702.131 Ascend) — fully public.
    CitysBlessingGained {
        player: u8,
    },
    /// Opponents see that a draw happened, but `card` (and the library object it came `from`,
    /// which would otherwise leak that identity by its decklist-order id) are `None` for anyone
    /// but the drawer.
    CardDrawn {
        player: u8,
        object: ObjectId,
        from: Option<ObjectId>,
        card: Option<String>,
    },
    /// A permanent was sacrificed by `by` — public (a sacrifice happens openly on the
    /// battlefield), like [`Self::MovedToGraveyard`]/[`Self::TokenCeasedToExist`], which fire
    /// alongside this for the actual zone change.
    Sacrificed {
        object: ObjectId,
        by: u8,
    },
    /// A card was discarded by `player` — public (a discard happens openly to the graveyard),
    /// like [`Self::Sacrificed`], which fires alongside [`Self::MovedToGraveyard`] for the
    /// actual zone change.
    Discarded {
        card: ObjectId,
        from: ObjectId,
        player: u8,
    },
    /// `player` put a card from hand onto the top of their library (Brainstorm resolving) — a
    /// hidden-zone-to-hidden-zone move, like [`Self::CardDrawn`]: `from` and `card` are `None`
    /// for anyone but `player`.
    PutFromHandOnTop {
        player: u8,
        card: ObjectId,
        from: Option<ObjectId>,
        def: Option<String>,
    },
    /// A graveyard card was impulse-exiled face-up and `player` may play it until end of turn
    /// (Containment Construct's discard payoff, CR 601 impulse play). Public, like
    /// [`Self::ExiledFromLibraryMayPlay`].
    ExiledFromGraveyardMayPlay {
        player: u8,
        card: ObjectId,
        from: ObjectId,
    },
}

/// Project a canonical engine event into what a seated `viewer` is allowed to see.
pub fn redact(event: &engine::Event, viewer: engine::PlayerId) -> VisibleEvent {
    crate::projection::project_event(event, Some(viewer))
}

/// Project a canonical engine event into the public spectator view — a watcher with no seat, so
/// no player's private information (hidden draws/tutors stay hidden); only public facts pass.
pub fn spectator_redact(event: &engine::Event) -> VisibleEvent {
    crate::projection::project_event(event, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{StreamFrame, ViewExtras, complete_visible};
    use engine::{Event, Game, PlayerId};

    fn snapshot(game: &Game, viewer: PlayerId) -> crate::dto::VisibleState {
        complete_visible(game, Some(viewer), &ViewExtras::default())
    }

    #[test]
    fn a_spectator_redaction_hides_a_hidden_draw() {
        // A drawn card's identity is private to the drawer; a spectator sees only that a draw
        // happened, never which card (the same guarantee an opponent gets).
        let ev = Event::CardDrawn {
            player: PlayerId(0),
            object: 7,
            from: 42,
            card: cards::get_by_name("Shock").unwrap(),
        };
        let spec = spectator_redact(&ev);
        match spec {
            VisibleEvent::CardDrawn { card, from, .. } => {
                assert!(card.is_none(), "a spectator never learns the drawn card");
                assert!(from.is_none(), "nor the library slot it came from");
            }
            other => panic!("expected CardDrawn, got {other:?}"),
        }
    }

    #[test]
    fn stream_frames_round_trip_as_json_lines() {
        let game = Game::new();
        let snap = StreamFrame::Snapshot {
            seq: 0,
            state: snapshot(&game, PlayerId(0)),
        };
        let delta = StreamFrame::Delta(DeltaEnvelope {
            seq: 1,
            events: vec![VisibleEvent::PriorityPassed { player: 0 }],
            state: snapshot(&game, PlayerId(0)),
            auto_actions: vec!["Discarded Shock (forced)".to_string()],
        });
        for frame in [snap, delta, StreamFrame::Heartbeat] {
            let line = serde_json::to_string(&frame).expect("frame serializes");
            assert!(!line.contains('\n'), "an SSE data line is a single line");
            let back: StreamFrame = serde_json::from_str(&line).expect("frame parses");
            assert_eq!(back, frame);
        }
        // The heartbeat is a bare tag — no payload beyond `frame`.
        assert_eq!(
            serde_json::to_string(&StreamFrame::Heartbeat).unwrap(),
            r#"{"frame":"heartbeat"}"#,
        );
    }

    #[test]
    fn a_delta_envelopes_auto_actions_serialize_and_old_frames_without_it_still_parse() {
        let game = Game::new();
        let with_labels = DeltaEnvelope {
            seq: 1,
            events: vec![],
            state: snapshot(&game, PlayerId(0)),
            auto_actions: vec!["Discarded Shock (forced)".to_string()],
        };
        let line = serde_json::to_string(&with_labels).expect("envelope serializes");
        assert!(line.contains("Discarded Shock (forced)"));
        let back: DeltaEnvelope = serde_json::from_str(&line).expect("envelope parses");
        assert_eq!(back, with_labels);

        // An old frame, persisted or sent before `auto_actions` existed, has no such key —
        // `#[serde(default)]` must still parse it, defaulting to an empty list.
        let old_frame_json = serde_json::to_string(&serde_json::json!({
            "seq": 1,
            "events": [],
            "state": snapshot(&game, PlayerId(0)),
        }))
        .unwrap();
        let parsed: DeltaEnvelope =
            serde_json::from_str(&old_frame_json).expect("a pre-auto_actions frame still parses");
        assert!(
            parsed.auto_actions.is_empty(),
            "a missing auto_actions key defaults to empty",
        );
    }

    #[test]
    fn a_drawn_cards_identity_is_visible_only_to_the_drawer() {
        let alice = PlayerId(0);
        let bob = PlayerId(1);
        let draw = Event::CardDrawn {
            player: alice,
            object: 7,
            from: 3,
            card: cards::get_by_name("Shock").expect("Shock is in the pool"),
        };

        let for_alice = redact(&draw, alice);
        let for_bob = redact(&draw, bob);

        assert_eq!(
            for_alice,
            VisibleEvent::CardDrawn {
                player: 0,
                object: 7,
                from: Some(3),
                card: Some("Shock".to_string())
            },
            "the drawer sees the card identity",
        );
        assert_eq!(
            for_bob,
            VisibleEvent::CardDrawn {
                player: 0,
                object: 7,
                from: None,
                card: None
            },
            "an opponent sees that a draw happened, but not which card — and not the library \
                 object id it came from, which would leak the identity by decklist order",
        );
    }

    #[test]
    fn a_tutored_cards_identity_is_visible_only_to_the_searcher() {
        let alice = PlayerId(0);
        let bob = PlayerId(1);
        let tutor = Event::SearchedToHand {
            player: alice,
            object: 7,
            from: 3,
            card: cards::get_by_name("Shock").expect("Shock is in the pool"),
        };

        let for_alice = redact(&tutor, alice);
        let for_bob = redact(&tutor, bob);

        assert_eq!(
            for_alice,
            VisibleEvent::SearchedToHand {
                player: 0,
                object: 7,
                from: Some(3),
                card: Some("Shock".to_string())
            },
            "the searcher sees the card identity",
        );
        assert_eq!(
            for_bob,
            VisibleEvent::SearchedToHand {
                player: 0,
                object: 7,
                from: None,
                card: None
            },
            "an opponent sees that a tutor happened, but not which card — and not the library \
                 object id it came from, which would leak the identity by decklist order",
        );

        let for_spectator = spectator_redact(&tutor);
        assert_eq!(
            for_spectator,
            VisibleEvent::SearchedToHand {
                player: 0,
                object: 7,
                from: None,
                card: None
            },
            "a spectator gets the same hidden projection as an opponent",
        );
    }
}
