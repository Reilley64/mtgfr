//! Integration tests for the gRPC services: drive the real service impls without a transport.
//! A smoke test at the bottom proves `grpc::serve` binds and accepts connections.

use tonic::{Request, Status};

use super::*;
use crate::db;
use crate::decks::keep_all_hands;
use crate::test_support::seat_deck;

async fn test_state() -> AppState {
    AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"))
}

/// A request carrying `token` as the `x-session-token` metadata.
fn authed<T>(msg: T, token: &str) -> Request<T> {
    let mut req = Request::new(msg);
    req.metadata_mut()
        .insert(auth_ctx::SESSION_METADATA_KEY, token.parse().unwrap());
    req
}

#[tokio::test]
async fn signup_mints_a_session_that_get_me_resolves_over_metadata() {
    use pb::auth_server::Auth;

    let state = test_state().await;
    let auth = auth_svc::AuthSvc::new(state.clone());

    let session = auth
        .signup(Request::new(pb::SignupCredentials {
            email: "a@b.c".to_string(),
            password: "hunter2".to_string(),
            username: "alice".to_string(),
        }))
        .await
        .expect("signup")
        .into_inner();
    let me = session.me.expect("signup returns the new account");
    assert_eq!(me.email, "a@b.c");
    assert!(!session.session_token.is_empty(), "a raw token is minted");

    let resolved = auth
        .get_me(authed(pb::Empty {}, &session.session_token))
        .await
        .expect("the minted token authenticates GetMe")
        .into_inner();
    assert_eq!(resolved.id, me.id);
    assert_eq!(resolved.username, "alice");
}

#[tokio::test]
async fn get_me_without_a_token_is_unauthenticated() {
    use pb::auth_server::Auth;

    let state = test_state().await;
    let auth = auth_svc::AuthSvc::new(state);

    let err = auth
        .get_me(Request::new(pb::Empty {}))
        .await
        .expect_err("no x-session-token metadata");
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn login_rejects_a_wrong_password() {
    use pb::auth_server::Auth;

    let state = test_state().await;
    let auth = auth_svc::AuthSvc::new(state);
    auth.signup(Request::new(pb::SignupCredentials {
        email: "w@b.c".to_string(),
        password: "right".to_string(),
        username: "wpass".to_string(),
    }))
    .await
    .expect("signup");

    let err = auth
        .login(Request::new(pb::Credentials {
            email: "w@b.c".to_string(),
            password: "wrong".to_string(),
        }))
        .await
        .expect_err("wrong password is rejected");
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn login_then_logout_revokes_the_session() {
    use pb::auth_server::Auth;

    let state = test_state().await;
    let auth = auth_svc::AuthSvc::new(state);
    let signup = auth
        .signup(Request::new(pb::SignupCredentials {
            email: "l@b.c".to_string(),
            password: "pw".to_string(),
            username: "lee".to_string(),
        }))
        .await
        .expect("signup")
        .into_inner();

    let login = auth
        .login(Request::new(pb::Credentials {
            email: "l@b.c".to_string(),
            password: "pw".to_string(),
        }))
        .await
        .expect("login with the right password")
        .into_inner();

    auth.logout(authed(pb::Empty {}, &login.session_token))
        .await
        .expect("logout");

    // The signup session is untouched; the logged-in-then-out session no longer authenticates.
    let still_signed_in = auth
        .get_me(authed(pb::Empty {}, &signup.session_token))
        .await;
    assert!(still_signed_in.is_ok());
    let logged_out = auth
        .get_me(authed(pb::Empty {}, &login.session_token))
        .await;
    assert!(logged_out.is_err());
}

/// Sign up a fresh user and mint a session token, for tests that need an authenticated caller.
async fn signed_up(state: &AppState, email: &str, username: &str) -> (i64, String) {
    use pb::auth_server::Auth;

    let auth = auth_svc::AuthSvc::new(state.clone());
    let session = auth
        .signup(Request::new(pb::SignupCredentials {
            email: email.to_string(),
            password: "pw".to_string(),
            username: username.to_string(),
        }))
        .await
        .expect("signup")
        .into_inner();
    let me = session.me.expect("signup returns the new account");
    (me.id, session.session_token)
}

#[tokio::test]
async fn decks_round_trip_create_list_get_update_delete() {
    use pb::decks_server::Decks;

    let state = test_state().await;
    let (_uid, token) = signed_up(&state, "d@b.c", "deckbuilder").await;
    let decks_svc = decks_svc::DecksSvc::new(state.clone());

    let tajic = cards::get_by_name("Tajic, Legion's Edge").unwrap();
    let plains = cards::get_by_name("Plains").unwrap();
    let save = pb::SaveDeckRequest {
        name: "My Deck".to_string(),
        commander: tajic.id.to_string(),
        commander_print: tajic.default_print.to_string(),
        cards: vec![pb::DeckCardEntry {
            id: plains.id.to_string(),
            count: 99,
            print: plains.default_print.to_string(),
        }],
    };

    let deck = decks_svc
        .create(authed(save.clone(), &token))
        .await
        .expect("create")
        .into_inner();
    assert_eq!(deck.name, "My Deck");

    let list = decks_svc
        .list(authed(pb::Empty {}, &token))
        .await
        .expect("list")
        .into_inner();
    assert!(list.decks.iter().any(|d| d.id == deck.id));

    let got = decks_svc
        .get(authed(pb::DeckId { id: deck.id }, &token))
        .await
        .expect("get")
        .into_inner();
    assert_eq!(got.id, deck.id);

    let renamed = pb::SaveDeckRequest {
        name: "Renamed".to_string(),
        ..save
    };
    let updated = decks_svc
        .update(authed(
            pb::UpdateDeckRequest {
                id: deck.id,
                request: Some(renamed),
            },
            &token,
        ))
        .await
        .expect("update")
        .into_inner();
    assert_eq!(updated.name, "Renamed");

    decks_svc
        .delete(authed(pb::DeckId { id: deck.id }, &token))
        .await
        .expect("delete");
    let err = decks_svc
        .get(authed(pb::DeckId { id: deck.id }, &token))
        .await
        .expect_err("deleted deck is gone");
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn cards_catalog_and_search_are_public() {
    use pb::cards_server::Cards;

    let state = test_state().await;
    let cards_svc = cards_svc::CardsSvc::new(state);

    let catalog = cards_svc
        .catalog(Request::new(pb::Empty {}))
        .await
        .expect("catalog needs no auth")
        .into_inner();
    assert!(!catalog.cards.is_empty(), "the pool is non-empty");
}

/// Seed a two-player table over `Tables.Seed`, then submit the priority holder's pass over
/// `Game.SubmitIntent` — exercising the same live-registry path the HTTP routes drive.
#[tokio::test]
async fn tables_seed_and_game_submit_intent_round_trip() {
    use pb::game_server::Game;
    use pb::tables_server::Tables;

    let state = test_state().await;
    let (host_id, host_token) = signed_up(&state, "host@x.c", "host").await;
    let (guest_id, _guest_token) = signed_up(&state, "guest@x.c", "guest").await;

    let host_deck_id = deck_row(&state, host_id).await;
    let guest_deck_id = deck_row(&state, guest_id).await;

    let seed_req = pb::SeedRequest {
        table_id: "grpc-tbl".to_string(),
        host_user_id: host_id,
        seats: vec![
            pb::SeedSeat {
                user_id: host_id,
                username: "host".to_string(),
                deck_id: host_deck_id,
            },
            pb::SeedSeat {
                user_id: guest_id,
                username: "guest".to_string(),
                deck_id: guest_deck_id,
            },
        ],
    };
    let tables_svc = tables_svc::TablesSvc::new(state.clone());
    let resp = tables_svc
        .seed(authed(seed_req, &host_token))
        .await
        .expect("seed")
        .into_inner();
    assert_eq!(resp.table_id, "grpc-tbl");
    keep_table_hands(&state, "grpc-tbl");

    let game_svc = game_svc::GameSvc::new(state.clone());
    let envelope = map::intent_envelope_to_pb(schema::IntentEnvelope {
        table_id: "grpc-tbl".to_string(),
        client_seq: 0,
        intent: schema::WireIntent::PassPriority { player: 0 },
    });
    let ack = game_svc
        .submit_intent(authed(
            pb::IntentRequest {
                table_id: "grpc-tbl".to_string(),
                envelope: Some(envelope),
            },
            &host_token,
        ))
        .await
        .expect("submit_intent")
        .into_inner();
    assert!(ack.accepted, "the active player's pass is legal: {ack:?}");
}

#[tokio::test]
async fn submit_intent_rejects_mismatched_envelope_table_id() {
    use pb::game_server::Game;
    use pb::tables_server::Tables;

    let state = test_state().await;
    let (host_id, host_token) = signed_up(&state, "mismatch-host@x.c", "host").await;
    let (guest_id, _guest_token) = signed_up(&state, "mismatch-guest@x.c", "guest").await;

    let host_deck_id = deck_row(&state, host_id).await;
    let guest_deck_id = deck_row(&state, guest_id).await;

    let tables_svc = tables_svc::TablesSvc::new(state.clone());
    tables_svc
        .seed(authed(
            pb::SeedRequest {
                table_id: "match-tbl".to_string(),
                host_user_id: host_id,
                seats: vec![
                    pb::SeedSeat {
                        user_id: host_id,
                        username: "host".to_string(),
                        deck_id: host_deck_id,
                    },
                    pb::SeedSeat {
                        user_id: guest_id,
                        username: "guest".to_string(),
                        deck_id: guest_deck_id,
                    },
                ],
            },
            &host_token,
        ))
        .await
        .expect("seed");

    let game_svc = game_svc::GameSvc::new(state);
    let envelope = map::intent_envelope_to_pb(schema::IntentEnvelope {
        table_id: "other-tbl".to_string(),
        client_seq: 0,
        intent: schema::WireIntent::PassPriority { player: 0 },
    });
    let err = game_svc
        .submit_intent(authed(
            pb::IntentRequest {
                table_id: "match-tbl".to_string(),
                envelope: Some(envelope),
            },
            &host_token,
        ))
        .await
        .expect_err("mismatched envelope.table_id must be rejected");
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(
        err.message().contains("table_id"),
        "error names the mismatch: {}",
        err.message()
    );
}

/// Seed a running two-player table under `table_id` with the given host/guest, returning the
/// host's user id. Shared setup for the `Game.Stream` coverage below.
async fn seed_two_player_table(state: &AppState, table_id: &str) -> (i64, String) {
    use pb::tables_server::Tables;

    let (host_id, host_token) = signed_up(state, &format!("{table_id}-host@x.c"), "host").await;
    let (guest_id, _guest_token) =
        signed_up(state, &format!("{table_id}-guest@x.c"), "guest").await;
    let host_deck_id = deck_row(state, host_id).await;
    let guest_deck_id = deck_row(state, guest_id).await;

    let tables_svc = tables_svc::TablesSvc::new(state.clone());
    tables_svc
        .seed(authed(
            pb::SeedRequest {
                table_id: table_id.to_string(),
                host_user_id: host_id,
                seats: vec![
                    pb::SeedSeat {
                        user_id: host_id,
                        username: "host".to_string(),
                        deck_id: host_deck_id,
                    },
                    pb::SeedSeat {
                        user_id: guest_id,
                        username: "guest".to_string(),
                        deck_id: guest_deck_id,
                    },
                ],
            },
            &host_token,
        ))
        .await
        .expect("seed");
    keep_table_hands(state, table_id);
    (host_id, host_token)
}

fn keep_table_hands(state: &AppState, table_id: &str) {
    let mut reg = crate::lock(&state.reg);
    let table = reg.get_mut(table_id).expect("seeded table exists");
    let game = table.game.as_mut().expect("seeded table has a game");
    keep_all_hands(game);
}

/// Pull the next decoded `StreamFrame` off a live `Game.Stream` response, bounded so a stalled
/// stream fails the test instead of hanging.
async fn next_frame(
    stream: &mut (impl tokio_stream::Stream<Item = Result<pb::StreamFrame, Status>> + Unpin),
) -> pb::stream_frame::Frame {
    use tokio_stream::StreamExt;
    let msg = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .expect("a frame arrives within the timeout")
        .expect("the stream has not ended")
        .expect("stream item is not an error");
    msg.frame.expect("frame payload")
}

/// The delta from a submitted intent reaches the same `Game.Stream` connection that was already
/// open before the intent was submitted.
#[tokio::test]
async fn game_stream_emits_snapshot_then_a_delta_on_intent() {
    use pb::game_server::Game;

    let state = test_state().await;
    let (_host_id, host_token) = seed_two_player_table(&state, "gs-tbl").await;

    let game_svc = game_svc::GameSvc::new(state.clone());
    let mut stream = game_svc
        .stream(authed(
            pb::StreamRequest {
                table_id: "gs-tbl".to_string(),
            },
            &host_token,
        ))
        .await
        .expect("stream opens")
        .into_inner();

    let opening = next_frame(&mut stream).await;
    assert!(
        matches!(opening, pb::stream_frame::Frame::Snapshot(_)),
        "first frame is a snapshot: {opening:?}"
    );

    let envelope = map::intent_envelope_to_pb(schema::IntentEnvelope {
        table_id: "gs-tbl".to_string(),
        client_seq: 0,
        intent: schema::WireIntent::PassPriority { player: 0 },
    });
    let ack = game_svc
        .submit_intent(authed(
            pb::IntentRequest {
                table_id: "gs-tbl".to_string(),
                envelope: Some(envelope),
            },
            &host_token,
        ))
        .await
        .expect("submit_intent")
        .into_inner();
    assert!(ack.accepted, "the host's pass is legal: {ack:?}");

    let delta = next_frame(&mut stream).await;
    assert!(
        matches!(delta, pb::stream_frame::Frame::Delta(_)),
        "the pass broadcasts a delta on the open stream: {delta:?}"
    );
}

/// A quiet game (no intents) still proves the connection alive with a periodic `Heartbeat`
/// frame — mirrors the removed HTTP integration test. `start_paused` advances virtual time to
/// the heartbeat interval automatically once the task parks, so this is deterministic and fast.
#[tokio::test(start_paused = true)]
async fn game_stream_emits_a_heartbeat_on_a_quiet_table() {
    use pb::game_server::Game;

    let state = test_state().await;
    let (_host_id, host_token) = seed_two_player_table(&state, "hb-tbl").await;

    let game_svc = game_svc::GameSvc::new(state.clone());
    let mut stream = game_svc
        .stream(authed(
            pb::StreamRequest {
                table_id: "hb-tbl".to_string(),
            },
            &host_token,
        ))
        .await
        .expect("stream opens")
        .into_inner();

    let opening = next_frame(&mut stream).await;
    assert!(matches!(opening, pb::stream_frame::Frame::Snapshot(_)));

    let beat = next_frame(&mut stream).await;
    assert!(
        matches!(beat, pb::stream_frame::Frame::Heartbeat(_)),
        "a quiet stream still beats: {beat:?}"
    );
}

/// C1/6.3: the stream resolves the viewer from the session, never from the client. A signed-in
/// user with no seat at the table gets the public spectator projection (no private hand view),
/// and an unknown table is `NOT_FOUND`.
#[tokio::test]
async fn game_stream_spectates_outsiders_and_errors_on_an_unknown_table() {
    use pb::game_server::Game;

    let state = test_state().await;
    let (_host_id, host_token) = seed_two_player_table(&state, "spec-tbl").await;
    let (_outsider_id, outsider_token) =
        signed_up(&state, "spec-tbl-outsider@x.c", "outsider").await;

    let game_svc = game_svc::GameSvc::new(state.clone());
    let mut stream = game_svc
        .stream(authed(
            pb::StreamRequest {
                table_id: "spec-tbl".to_string(),
            },
            &outsider_token,
        ))
        .await
        .expect("an outsider may still watch as a spectator")
        .into_inner();
    let opening = next_frame(&mut stream).await;
    let pb::stream_frame::Frame::Snapshot(snapshot) = opening else {
        panic!("expected a snapshot frame");
    };
    let view = snapshot.state.expect("snapshot carries a state");
    assert_eq!(
        view.viewer,
        u32::from(schema::SPECTATOR_VIEWER),
        "an outsider watches as a spectator, with no seat"
    );

    let err = game_svc
        .stream(authed(
            pb::StreamRequest {
                table_id: "nope".to_string(),
            },
            &host_token,
        ))
        .await
        .map(|_| ())
        .expect_err("unknown table");
    assert_eq!(err.code(), tonic::Code::NotFound);
}

/// Deploy PRD §Drain: a draining instance still serves a `Game.Stream` for a table it already
/// owns, and rejects *new* `Tables.Seed` calls.
#[tokio::test]
async fn draining_still_serves_an_owned_stream_but_rejects_new_seeds() {
    use pb::game_server::Game;
    use pb::tables_server::Tables;

    let state = test_state().await;
    let (host_id, host_token) = seed_two_player_table(&state, "drain-tbl").await;
    state
        .draining
        .store(true, std::sync::atomic::Ordering::Relaxed);

    let game_svc = game_svc::GameSvc::new(state.clone());
    let mut stream = game_svc
        .stream(authed(
            pb::StreamRequest {
                table_id: "drain-tbl".to_string(),
            },
            &host_token,
        ))
        .await
        .expect("a draining instance still streams a table it already owns")
        .into_inner();
    let opening = next_frame(&mut stream).await;
    assert!(matches!(opening, pb::stream_frame::Frame::Snapshot(_)));

    // Draining is checked before anything else in `seed_table_core`, so this seat list need not
    // be otherwise valid.
    let deck_id = deck_row(&state, host_id).await;
    let tables_svc = tables_svc::TablesSvc::new(state.clone());
    let err = tables_svc
        .seed(authed(
            pb::SeedRequest {
                table_id: "drain-tbl-2".to_string(),
                host_user_id: host_id,
                seats: vec![
                    pb::SeedSeat {
                        user_id: host_id,
                        username: "host".to_string(),
                        deck_id,
                    },
                    pb::SeedSeat {
                        user_id: host_id,
                        username: "host2".to_string(),
                        deck_id,
                    },
                ],
            },
            &host_token,
        ))
        .await
        .expect_err("a new seed is rejected while draining");
    assert_eq!(err.code(), tonic::Code::Unavailable);
}

/// Save a legal deck for `user_id` via the durable store directly (the gRPC `Decks` service is
/// covered by its own round-trip test above) and return its id.
async fn deck_row(state: &AppState, user_id: i64) -> i64 {
    let deck = seat_deck();
    let cards: Vec<schema::DeckCardEntry> = deck
        .cards
        .iter()
        .map(|(def, count)| schema::DeckCardEntry {
            id: def.id.to_string(),
            count: *count as u32,
            print: def.default_print.to_string(),
        })
        .collect();
    let mut db = state.db.clone();
    crate::db::Deck::create()
        .user_id(user_id)
        .name("deck")
        .commander(deck.commander.id)
        .commander_print(deck.commander.default_print)
        .cards(serde_json::to_string(&cards).unwrap())
        .exec(&mut db)
        .await
        .expect("create deck row")
        .id
}

/// Bind-and-accept smoke test for `grpc::serve` (no generated client required).
#[tokio::test]
async fn serve_binds_and_accepts_a_connection() {
    let state = test_state().await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral port");
    let addr = listener.local_addr().unwrap();
    drop(listener); // release the port; `serve` rebinds it (tonic wants the SocketAddr, not the listener)

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    let server = tokio::spawn(async move {
        super::serve(addr, state, async move {
            let _ = shutdown_rx.changed().await;
        })
        .await
    });

    // Give the server a moment to bind, then prove the port accepts a raw TCP connection
    // (enough to confirm the listener is live — a full gRPC handshake needs a generated client).
    let connected = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if tokio::net::TcpStream::connect(addr).await.is_ok() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(connected.is_ok(), "the gRPC listener accepts connections");

    let _ = shutdown_tx.send(true);
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .expect("server shuts down promptly")
        .expect("server task did not panic")
        .expect("serve returns Ok on graceful shutdown");
}
