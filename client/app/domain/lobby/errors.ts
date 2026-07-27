import * as Schema from "effect/Schema";

export class LobbyUnauthorized extends Schema.TaggedErrorClass<LobbyUnauthorized>()("LobbyUnauthorized", {}) {}

export class LobbyNotFound extends Schema.TaggedErrorClass<LobbyNotFound>()("LobbyNotFound", {}) {}

export class LobbyBadRequest extends Schema.TaggedErrorClass<LobbyBadRequest>()("LobbyBadRequest", {
  message: Schema.optional(Schema.String),
}) {}

export class LobbyHttpError extends Schema.TaggedErrorClass<LobbyHttpError>()("LobbyHttpError", {
  status: Schema.NullOr(Schema.Number),
  description: Schema.String,
}) {}

export class LobbyDecodeError extends Schema.TaggedErrorClass<LobbyDecodeError>()("LobbyDecodeError", {
  message: Schema.String,
}) {}

export type LobbyClientError = LobbyUnauthorized | LobbyNotFound | LobbyBadRequest | LobbyHttpError | LobbyDecodeError;
