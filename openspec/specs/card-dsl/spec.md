# card-dsl Specification

## Purpose

Define how Magic card and token behavior is authored as data, validated, interned for runtime use, and grown only when real cards demand new vocabulary — with explicit fidelity gaps instead of silent mis-modeling.

## Requirements

### Requirement: Cards are authored as structured definition files
The system SHALL represent each deckable card as a structured definition file that loads into a printed card definition. Authors SHALL be able to add or change a card without changing rules-engine code when the required effect vocabulary already exists. Each definition file SHALL open with the card's verbatim oracle text as a leading comment, then identity and rules fields, cost, kind, and ability blocks. Each ability block SHALL be preceded by a comment quoting the oracle sentence(s) it implements.

#### Scenario: Author a simple damage spell
- **WHEN** an author writes a definition for a spell whose effect vocabulary already exists
- **THEN** the definition loads into a printed card definition with the expected cost, kind, and ability effects without a rules-engine code change

#### Scenario: Oracle comment precedes modeling
- **WHEN** a reviewer opens a card definition file
- **THEN** the file begins with verbatim oracle text as a comment before the card name and modeled fields

### Requirement: Printed definitions are shared by stable handles
Printed card definitions SHALL be cloneable shared values, not copy-by-value payloads on every game object. Once a definition enters a game, the system SHALL intern it behind a stable card-id handle and look up the shared definition by that handle. List-like printed fields and nested alternative faces (back, adventure, split halves) SHALL be shared or interned so game snapshots remain cheap to clone.

#### Scenario: Nested face is restored by handle
- **WHEN** a card with a nested back or adventure face is loaded
- **THEN** the nested face is interned as a stable card id and flip or adventure flows restore that face without minting a new definition handle at runtime

#### Scenario: Tests construct definitions without parsing files
- **WHEN** a rules-engine unit test needs a card definition
- **THEN** the test MAY construct the printed definition inline without parsing authoring files

### Requirement: Vocabulary ownership and growth discipline
The card-definition vocabulary (printed definition, abilities, effect families and modes, filters, mana, triggers, conditions, amounts, and the authoring surface) SHALL live in the cards capability. The rules engine SHALL implement behavior over that vocabulary and MUST NOT invent parallel card-behavior enums. The effect vocabulary SHALL grow only when a real card in the pool demands a new leaf. Structural composers (`sequence`, `conditional`, `choose_one`) SHALL be the only effects without a mode; every leaf effect SHALL be authored as family `type` plus leaf `mode`.

#### Scenario: New leaf demanded by a card
- **WHEN** a card cannot be expressed with existing effect family modes
- **THEN** authors add a new leaf mode (and matching rules behavior) demanded by that card rather than contorting the card or anticipating unused vocabulary

#### Scenario: Engine consumes shared vocabulary
- **WHEN** the rules engine evaluates an ability
- **THEN** it uses the cards vocabulary types rather than a separate authored effect language

### Requirement: Ability timing and composition
Each ability SHALL declare a timing and one or more effects. Timings SHALL cover at least spell resolution, enters-the-battlefield triggers, as-enters replacements, activated abilities with costs, static continuous effects, and self-referential or watched triggered timings. Optional intervening-if conditions, optional ("you may") flags, triggers, and ability-level targets SHALL be expressible on ability blocks.

#### Scenario: As-enters is not a stack trigger
- **WHEN** a permanent has an as-enters ability
- **THEN** the effect runs as a replacement at entry rather than as a stack trigger that yields priority between entry and the choice it raises

#### Scenario: Activated ability carries a cost
- **WHEN** an ability uses activated timing
- **THEN** the ability declares an activation cost (tap, mana, life, sacrifice, discard, X, or combination)

### Requirement: Conditions, amounts, and player sets are generic parameters
Numeric magnitudes SHALL use a polymorphic amount type that includes fixed values, cast X, board- and resolution-derived counts, and compositional forms for arithmetic and conditional substitution. Threshold comparisons that compare two magnitudes SHALL use a generic compare condition with inclusive operators rather than one-off named threshold variants whenever a scalar comparison suffices. Effects that land on seats SHALL take a player-set parameter naming the recipient set (you, targeted seats, each opponent/player, trigger-filled seats, and related sets) instead of proliferating per-recipient modes. Multi-seat simultaneous changes SHALL resolve seats in turn order; choice-bearing edicts and multi-seat library searches SHALL proceed in APNAP order.

#### Scenario: Board-count threshold via compare
- **WHEN** a card requires "if you control N or more matching permanents"
- **THEN** the condition is expressed as a compare of a per-permanent amount against N with an inclusive operator

#### Scenario: Life loss fans out by player set
- **WHEN** an effect loses life for each opponent
- **THEN** the life effect uses a single lose mode with `who` naming each opponent rather than a dedicated each-opponent-loses mode

#### Scenario: This-way tally feeds the next step
- **WHEN** an effect removes counters and a following step draws or gains life for each counter removed this way
- **THEN** the removal step records a named this-way tally and the following ordinary effect reads that amount

### Requirement: Costs and kinds cover permanents and spells
Card cost SHALL express generic, colored, colorless, X, hybrid, and Phyrexian mana, plus optional additional costs (kicker, buyback, strive, replicate, discard-land, and similar). Card kind SHALL discriminate creature (with power/toughness), instant/sorcery, enchantment, artifact, planeswalker (loyalty), battle (defense), aura, and land (optional produces, basic, land subtypes). There SHALL be no dedicated token kind tag on deckable cards; tokens use ordinary kinds via token profiles.

#### Scenario: Land without cost is free
- **WHEN** a land definition omits a cost table
- **THEN** the card is treated as having no mana cost

#### Scenario: Dual-typed creature
- **WHEN** a creature is also an artifact
- **THEN** the kind declares creature with an additional type axis and subtypes live on the definition

### Requirement: Token profiles are shared by oracle id
Token profiles SHALL live as separate definition files from deckable cards, authored with the same kind vocabulary. Creating effects SHALL reference a token profile by Scryfall oracle id. Token profiles SHALL be installed before any deckable card that references them is loaded. Token creation SHALL intern the chosen profile into a card-id handle before attaching it to a live object or event.

#### Scenario: Multiple cards share one Pest profile
- **WHEN** two different cards create the same Pest token
- **THEN** both reference the same token profile id and resolve to the same printed characteristics

### Requirement: Fidelity gaps are explicit
When a card diverges from oracle text, the definition SHALL carry a machine-readable approximates note and an inline ponytail comment at the divergence point. Absence of approximates SHALL mean the card is treated as faithful for catalog and audit purposes. When a card needs vocabulary or engine capability the DSL cannot yet express, authors SHALL flag the gap in the active deck's fidelity increments backlog and on the card rather than force-scripting a contorted model.

#### Scenario: Approximated card is visible in catalog
- **WHEN** a card carries an approximates note
- **THEN** the card catalog surfaces that text so deck builders and audits see the same gap the engine runs

#### Scenario: Missing vocabulary is flagged, not forced
- **WHEN** a grind encounters a card that needs an unimplemented effect
- **THEN** the card is noted in the deck's fidelity increments with an approximates note instead of being silently mis-modeled

### Requirement: Catalog metadata is non-rules
Oracle text, flavor text, oracle tags, and set codes SHALL be catalog metadata for hover, card rendering, thematic search, and printing coverage. Flavor text SHALL be the flavor of the card's `default_print`, since flavor is per-printing, and SHALL be absent for a printing that prints none. The rules engine MUST NOT parse oracle or flavor text or read oracle tags or set codes for gameplay. Rules behavior SHALL come only from abilities, keywords, costs, kinds, and related rules fields.

#### Scenario: Search by oracle tag
- **WHEN** a deck builder searches for a thematic tag such as ramp or typal-spirit
- **THEN** matching uses oracle tags even if that tag is not itself a rules keyword

#### Scenario: Flavor rides the catalog, not the rules
- **WHEN** a card definition carries `flavor`
- **THEN** it reaches the client on `CatalogCard` beside `oracle` and no rules decision reads it

### Requirement: Authoring schema and reference stay generated
Committed JSON Schemas for card and token authoring surfaces SHALL be generated from the TOML authoring types. A generated field reference SHALL be produced from the same surface. Structural validation against the schema SHALL catch authoring-shape mistakes (including misspelled effect family tags) with file path, JSON Pointer, and schema message. Rust deserialization into the printed definition SHALL remain authoritative for load. Schema validation MUST NOT encode fidelity judgment, Scryfall freshness, deck legality, or every custom fold that still lives in the loader. Full-pool structural validation SHALL be part of the server verification bar.

#### Scenario: Typo in effect family fails validation
- **WHEN** a card file uses an unknown effect type tag
- **THEN** structural validation fails naming the file, JSON Pointer, and schema message before game load

#### Scenario: Schema drift is rejected
- **WHEN** the committed card or token schema no longer matches the generated authoring surface
- **THEN** the schema drift check fails until the committed artifacts are regenerated

### Requirement: Commander deck legality uses color identity from definitions
A Commander deck SHALL require exactly one legendary creature commander, ninety-nine other cards, singleton except basic lands, and every card's color identity within the commander's. Color identity SHALL be derived from cost pips, explicit color override, hybrid and Phyrexian pips, and extra identity pips. Legality validation on save SHALL return all problems at once. Partner commanders are out of scope: a deck has exactly one commander.

#### Scenario: Off-identity card is rejected with other problems
- **WHEN** a saved deck has an off-identity card and a wrong card count
- **THEN** validation fails listing both problems together

### Requirement: Curated pool and decklists are the fidelity proving ground
The card pool SHALL include every card needed by the curated Commander decklists under the decklists documentation tree (Secrets of Strixhaven precons and the additional closed lists). Presence in the pool is necessary but not sufficient for fidelity: per-deck fidelity reports and increments remain the record of faithful versus residual cards. The north star remains any card authored faithfully; curated lists are proving grounds, not a ceiling.

#### Scenario: Precon fixture validates against the pool
- **WHEN** a Secrets of Strixhaven precon fixture is loaded
- **THEN** every non-basic card resolves in the pool and passes Commander legality validation
