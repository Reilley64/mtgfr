## ADDED Requirements

### Requirement: Object views carry the facts a rendered card face needs

An object view SHALL carry, alongside its identity, whether the object is a token, whether it is legendary, and the card's colours, so a client can render a card face without a second lookup. The legendary flag and the colours SHALL follow the same redaction as every other card identity: a face-down permanent SHALL report neither. The token flag SHALL be reported regardless of face-down state, because a face-down permanent's back looks the same whether or not it is a token.

#### Scenario: Face-up permanent carries its face facts
- **WHEN** a viewer sees a face-up legendary permanent
- **THEN** its object view reports it as legendary, reports whether it is a token, and lists its colours

#### Scenario: Face-down permanent hides its identity
- **WHEN** a viewer sees a face-down permanent
- **THEN** its object view carries no legendary flag and no colours
