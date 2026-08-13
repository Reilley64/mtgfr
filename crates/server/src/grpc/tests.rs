//! Integration tests for the gRPC services: drive the real service impls without a transport.
//! A smoke test at the bottom proves `grpc::serve` binds and accepts connections.

use tonic::{Request, Status};

use super::*;
use crate::db;
use crate::decks::keep_all_hands;
use crate::elo::STARTING_RATING;
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
    use pb::auth_service_server::AuthService;

    let state = test_state().await;
    let auth = auth_svc::AuthSvc::new(state.clone());

    let session = auth
        .signup(Request::new(pb::SignupRequest {
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
        .get_me(authed(pb::GetMeRequest {}, &session.session_token))
        .await
        .expect("the minted token authenticates GetMe")
        .into_inner();
    assert_eq!(resolved.id, me.id);
    assert_eq!(resolved.username, "alice");

    let mut db = state.db.clone();
    let created_user = db::User::filter_by_id(me.id)
        .get(&mut db)
        .await
        .expect("signup persists the user row");
    assert_eq!(created_user.rating, STARTING_RATING);
    assert!(
        created_user.rating_set_at > 0,
        "signup stamps when the starting rating was set"
    );
}

#[tokio::test]
async fn get_me_without_a_token_is_unauthenticated() {
    use pb::auth_service_server::AuthService;

    let state = test_state().await;
    let auth = auth_svc::AuthSvc::new(state);

    let err = auth
        .get_me(Request::new(pb::GetMeRequest {}))
        .await
        .expect_err("no x-session-token metadata");
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn login_rejects_a_wrong_password() {
    use pb::auth_service_server::AuthService;

    let state = test_state().await;
    let auth = auth_svc::AuthSvc::new(state);
    auth.signup(Request::new(pb::SignupRequest {
        email: "w@b.c".to_string(),
        password: "right".to_string(),
        username: "wpass".to_string(),
    }))
    .await
    .expect("signup");

    let err = auth
        .login(Request::new(pb::LoginRequest {
            email: "w@b.c".to_string(),
            password: "wrong".to_string(),
        }))
        .await
        .expect_err("wrong password is rejected");
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn login_then_logout_revokes_the_session() {
    use pb::auth_service_server::AuthService;

    let state = test_state().await;
    let auth = auth_svc::AuthSvc::new(state);
    let signup = auth
        .signup(Request::new(pb::SignupRequest {
            email: "l@b.c".to_string(),
            password: "pw".to_string(),
            username: "lee".to_string(),
        }))
        .await
        .expect("signup")
        .into_inner();

    let login = auth
        .login(Request::new(pb::LoginRequest {
            email: "l@b.c".to_string(),
            password: "pw".to_string(),
        }))
        .await
        .expect("login with the right password")
        .into_inner();

    auth.logout(authed(pb::LogoutRequest {}, &login.session_token))
        .await
        .expect("logout");

    // The signup session is untouched; the logged-in-then-out session no longer authenticates.
    let still_signed_in = auth
        .get_me(authed(pb::GetMeRequest {}, &signup.session_token))
        .await;
    assert!(still_signed_in.is_ok());
    let logged_out = auth
        .get_me(authed(pb::GetMeRequest {}, &login.session_token))
        .await;
    assert!(logged_out.is_err());
}

/// Sign up a fresh user and mint a session token, for tests that need an authenticated caller.
async fn signed_up(state: &AppState, email: &str, username: &str) -> (i64, String) {
    use pb::auth_service_server::AuthService;

    let auth = auth_svc::AuthSvc::new(state.clone());
    let session = auth
        .signup(Request::new(pb::SignupRequest {
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

async fn set_user_rating(state: &AppState, user_id: i64, rating: i32, rating_set_at: i64) {
    let mut db = state.db.clone();
    let mut user = db::User::filter_by_id(user_id)
        .get(&mut db)
        .await
        .expect("user exists");
    user.update()
        .rating(rating)
        .rating_set_at(rating_set_at)
        .exec(&mut db)
        .await
        .expect("update user rating");
}

#[tokio::test]
async fn ratings_get_leaderboard_requires_authentication() {
    use pb::ratings_service_server::RatingsService;

    let state = test_state().await;
    let ratings_svc = ratings_svc::RatingsSvc::new(state);

    let err = ratings_svc
        .get_leaderboard(Request::new(pb::GetLeaderboardRequest {
            limit: 0,
            offset: 0,
        }))
        .await
        .expect_err("leaderboard should require auth");
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn ratings_get_leaderboard_orders_stably_and_pages() {
    use pb::ratings_service_server::RatingsService;

    let state = test_state().await;
    let (alice_id, alice_token) = signed_up(&state, "lb-alice@x.c", "alice").await;
    let (bob_id, _bob_token) = signed_up(&state, "lb-bob@x.c", "bob").await;
    let (cara_id, _cara_token) = signed_up(&state, "lb-cara@x.c", "cara").await;
    let (dax_id, _dax_token) = signed_up(&state, "lb-dax@x.c", "dax").await;

    set_user_rating(&state, alice_id, 1000, 300).await;
    set_user_rating(&state, bob_id, 1200, 200).await;
    set_user_rating(&state, cara_id, 1200, 100).await;
    set_user_rating(&state, dax_id, 1200, 100).await;

    let ratings_svc = ratings_svc::RatingsSvc::new(state.clone());

    let paged = ratings_svc
        .get_leaderboard(authed(
            pb::GetLeaderboardRequest {
                limit: 2,
                offset: 1,
            },
            &alice_token,
        ))
        .await
        .expect("leaderboard page")
        .into_inner();
    assert_eq!(paged.total, 4);
    assert_eq!(paged.entries.len(), 2);
    assert_eq!(paged.entries[0].user_id, dax_id);
    assert_eq!(paged.entries[0].username, "dax");
    assert_eq!(paged.entries[0].rating, 1200);
    assert_eq!(paged.entries[0].rank, 2);
    assert_eq!(paged.entries[1].user_id, bob_id);
    assert_eq!(paged.entries[1].username, "bob");
    assert_eq!(paged.entries[1].rating, 1200);
    assert_eq!(paged.entries[1].rank, 3);

    let default_limit = ratings_svc
        .get_leaderboard(authed(
            pb::GetLeaderboardRequest {
                limit: 0,
                offset: 0,
            },
            &alice_token,
        ))
        .await
        .expect("leaderboard default limit")
        .into_inner();
    assert_eq!(default_limit.total, 4);
    assert_eq!(default_limit.entries.len(), 4);
    assert_eq!(default_limit.entries[0].user_id, cara_id);
    assert_eq!(default_limit.entries[0].rank, 1);
    assert_eq!(default_limit.entries[1].user_id, dax_id);
    assert_eq!(default_limit.entries[1].rank, 2);
    assert_eq!(default_limit.entries[2].user_id, bob_id);
    assert_eq!(default_limit.entries[2].rank, 3);
    assert_eq!(default_limit.entries[3].user_id, alice_id);
    assert_eq!(default_limit.entries[3].rank, 4);
}

#[tokio::test]
async fn decks_round_trip_create_list_get_update_delete() {
    use pb::decks_service_server::DecksService;

    let state = test_state().await;
    let (_uid, token) = signed_up(&state, "d@b.c", "deckbuilder").await;
    let decks_svc = decks_svc::DecksSvc::new(state.clone());

    let tajic = cards::get_by_name("Tajic, Legion's Edge").unwrap();
    let plains = cards::get_by_name("Plains").unwrap();
    let save = pb::CreateRequest {
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
        .list(authed(pb::ListRequest {}, &token))
        .await
        .expect("list")
        .into_inner();
    assert!(list.decks.iter().any(|d| d.id == deck.id));

    let got = decks_svc
        .get(authed(pb::GetRequest { id: deck.id }, &token))
        .await
        .expect("get")
        .into_inner();
    assert_eq!(got.id, deck.id);

    let renamed = pb::DeckSaveBody {
        name: "Renamed".to_string(),
        commander: save.commander,
        commander_print: save.commander_print,
        cards: save.cards,
    };
    let updated = decks_svc
        .update(authed(
            pb::UpdateRequest {
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
        .delete(authed(pb::DeleteRequest { id: deck.id }, &token))
        .await
        .expect("delete");
    let err = decks_svc
        .get(authed(pb::GetRequest { id: deck.id }, &token))
        .await
        .expect_err("deleted deck is gone");
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn cards_catalog_and_search_are_public() {
    use pb::cards_service_server::CardsService;

    let state = test_state().await;
    let cards_svc = cards_svc::CardsSvc::new(state);

    let catalog = cards_svc
        .catalog(Request::new(pb::CatalogRequest {}))
        .await
        .expect("catalog needs no auth")
        .into_inner();
    assert!(!catalog.cards.is_empty(), "the pool is non-empty");
}

/// Seed a two-player table over `Tables.Seed`, then submit the priority holder's pass over
/// `Game.SubmitIntent` — exercising the same live-registry path the HTTP routes drive.
#[tokio::test]
async fn tables_seed_and_game_submit_intent_round_trip() {
    use pb::game_service_server::GameService;
    use pb::tables_service_server::TablesService;

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
                gravatar_hash: String::new(),
            },
            pb::SeedSeat {
                user_id: guest_id,
                username: "guest".to_string(),
                deck_id: guest_deck_id,
                gravatar_hash: String::new(),
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
            pb::SubmitIntentRequest {
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
async fn conceding_a_seeded_table_persists_elo_ratings() {
    use pb::game_service_server::GameService;

    let state = test_state().await;
    let (alice_id, bob_id, alice_token) =
        seed_two_player_table_with_players(&state, "elo-concede-tbl").await;

    let game_svc = game_svc::GameSvc::new(state.clone());
    let envelope = map::intent_envelope_to_pb(schema::IntentEnvelope {
        table_id: "elo-concede-tbl".to_string(),
        client_seq: 0,
        intent: schema::WireIntent::Concede { player: 0 },
    });
    let ack = game_svc
        .submit_intent(authed(
            pb::SubmitIntentRequest {
                table_id: "elo-concede-tbl".to_string(),
                envelope: Some(envelope),
            },
            &alice_token,
        ))
        .await
        .expect("submit concede")
        .into_inner();
    assert!(ack.accepted, "concede is accepted: {ack:?}");

    let mut db = state.db.clone();
    let alice = db::User::filter_by_id(alice_id)
        .get(&mut db)
        .await
        .expect("alice still exists");
    let bob = db::User::filter_by_id(bob_id)
        .get(&mut db)
        .await
        .expect("bob still exists");
    assert_eq!(alice.rating, 984);
    assert_eq!(bob.rating, 1016);
}

#[tokio::test]
async fn submit_intent_rejects_mismatched_envelope_table_id() {
    use pb::game_service_server::GameService;
    use pb::tables_service_server::TablesService;

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
                        gravatar_hash: String::new(),
                    },
                    pb::SeedSeat {
                        user_id: guest_id,
                        username: "guest".to_string(),
                        deck_id: guest_deck_id,
                        gravatar_hash: String::new(),
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
            pb::SubmitIntentRequest {
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
    let (host_id, _guest_id, host_token) =
        seed_two_player_table_with_players(state, table_id).await;
    (host_id, host_token)
}

/// Seed a running two-player table and return both account ids plus the host token.
async fn seed_two_player_table_with_players(
    state: &AppState,
    table_id: &str,
) -> (i64, i64, String) {
    use pb::tables_service_server::TablesService;

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
                        gravatar_hash: String::new(),
                    },
                    pb::SeedSeat {
                        user_id: guest_id,
                        username: "guest".to_string(),
                        deck_id: guest_deck_id,
                        gravatar_hash: String::new(),
                    },
                ],
            },
            &host_token,
        ))
        .await
        .expect("seed");
    keep_table_hands(state, table_id);
    (host_id, guest_id, host_token)
}

fn keep_table_hands(state: &AppState, table_id: &str) {
    let mut reg = crate::lock(&state.reg);
    let table = reg.get_mut(table_id).expect("seeded table exists");
    let game = table.game.as_mut().expect("seeded table has a game");
    keep_all_hands(game);
}

/// Pull the next decoded `StreamResponse` off a live `Game.Stream` response, bounded so a stalled
/// stream fails the test instead of hanging.
async fn next_frame(
    stream: &mut (impl tokio_stream::Stream<Item = Result<pb::StreamResponse, Status>> + Unpin),
) -> pb::stream_response::Frame {
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
    use pb::game_service_server::GameService;

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
        matches!(opening, pb::stream_response::Frame::Snapshot(_)),
        "first frame is a snapshot: {opening:?}"
    );

    let envelope = map::intent_envelope_to_pb(schema::IntentEnvelope {
        table_id: "gs-tbl".to_string(),
        client_seq: 0,
        intent: schema::WireIntent::PassPriority { player: 0 },
    });
    let ack = game_svc
        .submit_intent(authed(
            pb::SubmitIntentRequest {
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
        matches!(delta, pb::stream_response::Frame::Delta(_)),
        "the pass broadcasts a delta on the open stream: {delta:?}"
    );
}

/// An already-connected authenticated gRPC stream receives an authoritative replacement as a
/// snapshot and maps it with the publication's current seats and print preferences.
#[tokio::test]
async fn grpc_stream_delivers_midstream_snapshot() {
    use crate::session::{PublishedState, PublishedUpdate};
    use http::uri::PathAndQuery;

    let state = test_state().await;
    let (_host_id, host_token) = seed_two_player_table(&state, "midstream-tbl").await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("ephemeral port");
    let addr = listener.local_addr().expect("listener address");
    drop(listener);

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    let server_state = state.clone();
    let server = tokio::spawn(async move {
        super::serve(addr, server_state, async move {
            let _ = shutdown_rx.changed().await;
        })
        .await
    });

    let endpoint =
        tonic::transport::Endpoint::from_shared(format!("http://{addr}")).expect("valid endpoint");
    let channel = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match endpoint.connect().await {
                Ok(channel) => return channel,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("server accepts a gRPC connection");
    let mut grpc = tonic::client::Grpc::new(channel);
    grpc.ready().await.expect("gRPC client becomes ready");
    let mut request = authed(
        pb::StreamRequest {
            table_id: "midstream-tbl".into(),
        },
        &host_token,
    );
    request
        .extensions_mut()
        .insert(tonic::GrpcMethod::new("mtgfr.v1.GameService", "Stream"));
    let response: tonic::Response<tonic::Streaming<pb::StreamResponse>> = grpc
        .server_streaming(
            request,
            PathAndQuery::from_static("/mtgfr.v1.GameService/Stream"),
            tonic::codec::ProstCodec::default(),
        )
        .await
        .expect("stream opens over the bound gRPC server");
    let mut stream = response.into_inner();
    let opening = stream
        .message()
        .await
        .expect("opening frame decodes")
        .expect("opening frame exists")
        .frame
        .expect("opening frame payload");
    assert!(matches!(opening, pb::stream_response::Frame::Snapshot(_)));

    let (publication, hand_card) = {
        let mut reg = crate::lock(&state.reg);
        let table = reg.get_mut("midstream-tbl").expect("seeded table");
        table.seq += 1;
        table.broadcast_seq += 1;
        table.seats[0].username = Some("fresh-alice".into());
        let game = table.game.as_ref().expect("running game");
        let hand_card = game.hand(engine::PlayerId(0))[0];
        let card_id = game.def_of(hand_card).id.to_string();
        table.prints[0].insert(card_id, "fresh-print".into());
        (
            PublishedState {
                seq: table.seq,
                broadcast_seq: table.broadcast_seq,
                game: game.clone(),
                yields: *table.chrome.yields(),
                turn_yields: *table.chrome.turn_yields(),
                stack_hold_remaining_ms: table.stack_hold_remaining_ms(),
                seats: table.seats.clone(),
                prints: table.prints.clone(),
            },
            hand_card,
        )
    };
    let expected_seq = publication.seq;
    {
        let reg = crate::lock(&state.reg);
        let table = reg.get("midstream-tbl").expect("seeded table");
        assert!(
            table
                .tx
                .send(std::sync::Arc::new(PublishedUpdate::Snapshot(publication)))
                .is_ok(),
            "connected stream receives publication",
        );
    }

    let replacement = tokio::time::timeout(std::time::Duration::from_secs(5), stream.message())
        .await
        .expect("midstream frame arrives")
        .expect("midstream frame decodes")
        .expect("midstream frame exists")
        .frame
        .expect("midstream frame payload");
    let pb::stream_response::Frame::Snapshot(snapshot) = replacement else {
        panic!("authoritative replacement must not be encoded as a delta");
    };
    assert_eq!(snapshot.seq, expected_seq);
    let view = snapshot.state.expect("snapshot state");
    assert_eq!(view.players[0].username, "fresh-alice");
    let visible_hand = view
        .objects
        .iter()
        .find(|object| object.id == hand_card)
        .expect("the authenticated owner sees their hand card");
    assert_eq!(visible_hand.print, "fresh-print");

    drop(stream);
    drop(grpc);
    let _ = shutdown_tx.send(true);
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .expect("server shuts down promptly")
        .expect("server task does not panic")
        .expect("server exits cleanly");
}

/// A quiet game (no intents) still proves the connection alive with a periodic `Heartbeat`
/// frame — mirrors the removed HTTP integration test. `start_paused` advances virtual time to
/// the heartbeat interval automatically once the task parks, so this is deterministic and fast.
#[tokio::test(start_paused = true)]
async fn game_stream_emits_a_heartbeat_on_a_quiet_table() {
    use pb::game_service_server::GameService;

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
    assert!(matches!(opening, pb::stream_response::Frame::Snapshot(_)));

    let beat = next_frame(&mut stream).await;
    assert!(
        matches!(beat, pb::stream_response::Frame::Heartbeat(_)),
        "a quiet stream still beats: {beat:?}"
    );
}

/// C1/6.3: the stream resolves the viewer from the session, never from the client. A signed-in
/// user with no seat at the table gets the public spectator projection (no private hand view),
/// and an unknown table is `NOT_FOUND`.
#[tokio::test]
async fn game_stream_spectates_outsiders_and_errors_on_an_unknown_table() {
    use pb::game_service_server::GameService;

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
    let pb::stream_response::Frame::Snapshot(snapshot) = opening else {
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
    use pb::game_service_server::GameService;
    use pb::tables_service_server::TablesService;

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
    assert!(matches!(opening, pb::stream_response::Frame::Snapshot(_)));

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
                        gravatar_hash: String::new(),
                    },
                    pb::SeedSeat {
                        user_id: host_id,
                        username: "host2".to_string(),
                        deck_id,
                        gravatar_hash: String::new(),
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

#[cfg(debug_assertions)]
#[test]
fn debug_stack_controller_preserves_presence() {
    let stack = super::debug_pb::StackInspection {
        controller: None,
        ..Default::default()
    };
    let _: Option<u32> = stack.controller;
}

#[cfg(debug_assertions)]
fn insert_debug_game(
    state: &AppState,
    table_id: &str,
    game: engine::Game,
    debug_revision: u64,
    table_seq: u64,
) {
    let mut table = crate::Table::empty();
    table.game = Some(game);
    table.debug.revision = debug_revision;
    table.seq = table_seq;
    assert!(crate::lock(&state.reg).try_insert(table_id.to_string(), table));
}

#[cfg(debug_assertions)]
fn debug_set_life(player: u32, life: i32) -> debug_pb::Mutation {
    debug_pb::Mutation {
        operation: Some(debug_pb::mutation::Operation::SetLife(debug_pb::SetLife {
            player,
            life,
        })),
    }
}

#[cfg(debug_assertions)]
fn decode_debug_detail(status: &Status) -> debug_pb::ErrorDetail {
    use prost::Message;

    debug_pb::ErrorDetail::decode(status.details()).expect("one protobuf ErrorDetail")
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_service_list_is_sorted_and_content_free_and_inspect_is_unfiltered() {
    use debug_pb::debug_service_server::DebugService;
    use prost::Message;

    let state = test_state().await;
    let secret = cards::get_by_name("Island").expect("test card");
    let secret_id = secret.id.to_string();
    let mut game = engine::Game::with_players(2, 0);
    let hand_object = game.spawn_in_hand(engine::PlayerId(0), secret);
    insert_debug_game(&state, "z-table", game, 4, 9);
    insert_debug_game(&state, "a-table", engine::Game::with_players(2, 0), 2, 7);
    crate::lock(&state.reg)
        .get_mut("z-table")
        .expect("inserted table")
        .debug
        .debug_mutated = true;

    let service = debug_svc::DebugSvc::new(state);
    let listed = service
        .list_tables(Request::new(debug_pb::ListTablesRequest {}))
        .await
        .expect("plain request lists tables")
        .into_inner();
    assert_eq!(
        listed.tables,
        vec![
            debug_pb::TableSummary {
                table_id: "a-table".into(),
                debug_revision: 2,
                table_seq: 7,
            },
            debug_pb::TableSummary {
                table_id: "z-table".into(),
                debug_revision: 4,
                table_seq: 9,
            },
        ]
    );
    let listed_wire = listed.encode_to_vec();
    assert!(
        !listed_wire
            .windows(secret_id.len())
            .any(|window| window == secret_id.as_bytes()),
        "ListTables never carries card contents"
    );

    let inspected = service
        .inspect_table(Request::new(debug_pb::InspectTableRequest {
            table_id: "z-table".into(),
        }))
        .await
        .expect("plain request inspects the authoritative game")
        .into_inner();
    assert_eq!(inspected.table_id, "z-table");
    assert_eq!(inspected.debug_revision, 4);
    assert_eq!(inspected.table_seq, 9);
    assert!(inspected.debug_mutated);
    let game = inspected.game.expect("inspection payload");
    assert!(game.players[0].hand.contains(&hand_object));
    assert!(game.objects.iter().any(|object| {
        matches!(
            object.state.as_ref(),
            Some(debug_pb::object_inspection::State::Card(card)) if card.card_id == secret_id
        )
    }));
    assert!(game.stack.is_empty());
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_service_inspect_rejects_corrupt_card_stack_zone_without_leaking_payload() {
    use debug_pb::debug_service_server::DebugService;

    let state = test_state().await;
    insert_debug_game(
        &state,
        "corrupt-table",
        engine::Game::with_players(2, 0),
        3,
        4,
    );
    let mut inspection = empty_debug_inspection();
    inspection
        .objects
        .push(engine::debug::ObjectInspection::Card {
            object_id: 7,
            card_id: "private-card-id".into(),
            owner: engine::PlayerId(0),
            zone: engine::Zone::Stack,
            commander: false,
            face_down: false,
        });
    let service = debug_svc::DebugSvc::with_inspection_for_test(state, inspection);

    let error = service
        .inspect_table(Request::new(debug_pb::InspectTableRequest {
            table_id: "corrupt-table".into(),
        }))
        .await
        .expect_err("corrupt authoritative inspection is rejected");

    assert_eq!(error.code(), tonic::Code::Internal);
    assert_eq!(error.message(), "debug request rejected");
    let detail = decode_debug_detail(&error);
    assert_eq!(detail.operation_index, None);
    assert_eq!(
        detail.reason,
        debug_pb::DebugErrorReason::InvalidValue as i32
    );
    assert!(!String::from_utf8_lossy(error.details()).contains("private-card-id"));
}

#[cfg(debug_assertions)]
#[test]
fn debug_service_rejects_card_inspection_in_stack_zone_without_leaking_payload() {
    let error = debug_svc::map_inspection(engine::debug::Inspection {
        players: vec![],
        objects: vec![engine::debug::ObjectInspection::Card {
            object_id: 7,
            card_id: "private-card-id".into(),
            owner: engine::PlayerId(0),
            zone: engine::Zone::Stack,
            commander: false,
            face_down: false,
        }],
        stack: vec![],
        active_player: engine::PlayerId(0),
        step: engine::Step::Upkeep,
        priority_player: engine::PlayerId(0),
        consecutive_passes: 0,
        has_pending_choice: false,
        has_deferred_resume: false,
    })
    .expect_err("a card object cannot successfully map from the stack zone");

    assert_eq!(error.code(), tonic::Code::Internal);
    assert_eq!(error.message(), "debug request rejected");
    let detail = decode_debug_detail(&error);
    assert_eq!(detail.operation_index, None);
    assert_eq!(
        detail.reason,
        debug_pb::DebugErrorReason::InvalidValue as i32
    );
    assert!(!String::from_utf8_lossy(error.details()).contains("private-card-id"));
}

#[cfg(debug_assertions)]
fn empty_debug_inspection() -> engine::debug::Inspection {
    engine::debug::Inspection {
        players: vec![],
        objects: vec![],
        stack: vec![],
        active_player: engine::PlayerId(0),
        step: engine::Step::Untap,
        priority_player: engine::PlayerId(0),
        consecutive_passes: 0,
        has_pending_choice: false,
        has_deferred_resume: false,
    }
}

#[cfg(debug_assertions)]
#[test]
fn debug_service_maps_every_inspection_field_and_object_variant() {
    use debug_pb::object_inspection::State;
    use engine::debug::{ObjectInspection, PlayerInspection, StackInspection};

    let mut inspection = empty_debug_inspection();
    inspection.players = vec![PlayerInspection {
        player_id: engine::PlayerId(3),
        life: i32::MIN,
        poison: 5,
        rad: 6,
        library: vec![1, 2],
        hand: vec![3],
        graveyard: vec![4],
        exile: vec![5],
        command: vec![6],
    }];
    inspection.objects = vec![
        ObjectInspection::Card {
            object_id: 10,
            card_id: "card".into(),
            owner: engine::PlayerId(1),
            zone: engine::Zone::Exile,
            commander: true,
            face_down: false,
        },
        ObjectInspection::Permanent {
            object_id: 11,
            card_id: "permanent".into(),
            owner: engine::PlayerId(1),
            controller: engine::PlayerId(2),
            tapped: true,
            marked_damage: -7,
            plus_one_counters: 8,
            attached_to: Some(10),
            commander: false,
            token: false,
            face_down: false,
        },
        ObjectInspection::Spell {
            object_id: 12,
            card_id: "spell".into(),
            controller: engine::PlayerId(3),
        },
        ObjectInspection::Moved {
            object_id: 13,
            to_object_id: 14,
        },
        ObjectInspection::Removed {
            object_id: 15,
            card_id: "removed".into(),
            owner: engine::PlayerId(2),
        },
        ObjectInspection::Card {
            object_id: 16,
            card_id: "face-down-card".into(),
            owner: engine::PlayerId(2),
            zone: engine::Zone::Hand,
            commander: false,
            face_down: true,
        },
        ObjectInspection::Permanent {
            object_id: 17,
            card_id: "commander-permanent".into(),
            owner: engine::PlayerId(2),
            controller: engine::PlayerId(3),
            tapped: false,
            marked_damage: 1,
            plus_one_counters: 2,
            attached_to: None,
            commander: true,
            token: false,
            face_down: false,
        },
        ObjectInspection::Permanent {
            object_id: 18,
            card_id: "token-permanent".into(),
            owner: engine::PlayerId(3),
            controller: engine::PlayerId(1),
            tapped: false,
            marked_damage: 3,
            plus_one_counters: 4,
            attached_to: None,
            commander: false,
            token: true,
            face_down: false,
        },
        ObjectInspection::Permanent {
            object_id: 19,
            card_id: "face-down-permanent".into(),
            owner: engine::PlayerId(1),
            controller: engine::PlayerId(3),
            tapped: false,
            marked_damage: 5,
            plus_one_counters: 6,
            attached_to: None,
            commander: false,
            token: false,
            face_down: true,
        },
    ];
    inspection.stack = vec![
        StackInspection {
            position_from_bottom: 0,
            kind: "spell",
            source_object_id: Some(12),
            controller: Some(engine::PlayerId(3)),
            label: "Secret Spell".into(),
        },
        StackInspection {
            position_from_bottom: 1,
            kind: "ability",
            source_object_id: None,
            controller: None,
            label: "Secret Ability".into(),
        },
    ];
    inspection.active_player = engine::PlayerId(2);
    inspection.step = engine::Step::Cleanup;
    inspection.priority_player = engine::PlayerId(1);
    inspection.consecutive_passes = u8::MAX;
    inspection.has_pending_choice = true;
    inspection.has_deferred_resume = false;

    let mapped = debug_svc::map_inspection(inspection).expect("complete inspection maps");

    assert_eq!(
        mapped.players,
        vec![debug_pb::PlayerInspection {
            player_id: 3,
            life: i32::MIN,
            poison: 5,
            rad: 6,
            library: vec![1, 2],
            hand: vec![3],
            graveyard: vec![4],
            exile: vec![5],
            command: vec![6],
        }]
    );
    assert_eq!(
        mapped.objects[0],
        debug_pb::ObjectInspection {
            object_id: 10,
            state: Some(State::Card(debug_pb::CardInspection {
                card_id: "card".into(),
                owner: 1,
                zone: debug_pb::Zone::Exile as i32,
                commander: true,
                face_down: false,
            })),
        }
    );
    assert_eq!(
        mapped.objects[1],
        debug_pb::ObjectInspection {
            object_id: 11,
            state: Some(State::Permanent(debug_pb::PermanentInspection {
                card_id: "permanent".into(),
                owner: 1,
                controller: 2,
                tapped: true,
                marked_damage: -7,
                plus_one_counters: 8,
                attached_to: Some(10),
                commander: false,
                token: false,
                face_down: false,
            })),
        }
    );
    assert_eq!(
        mapped.objects[2],
        debug_pb::ObjectInspection {
            object_id: 12,
            state: Some(State::Spell(debug_pb::SpellInspection {
                card_id: "spell".into(),
                controller: 3
            })),
        }
    );
    assert_eq!(
        mapped.objects[3],
        debug_pb::ObjectInspection {
            object_id: 13,
            state: Some(State::Moved(debug_pb::MovedInspection { to_object_id: 14 })),
        }
    );
    assert_eq!(
        mapped.objects[4],
        debug_pb::ObjectInspection {
            object_id: 15,
            state: Some(State::Removed(debug_pb::RemovedInspection {
                card_id: "removed".into(),
                owner: 2
            })),
        }
    );
    assert_eq!(
        mapped.stack,
        vec![
            debug_pb::StackInspection {
                position_from_bottom: 0,
                kind: "spell".into(),
                source_object_id: Some(12),
                controller: Some(3),
                label: "Secret Spell".into(),
            },
            debug_pb::StackInspection {
                position_from_bottom: 1,
                kind: "ability".into(),
                source_object_id: None,
                controller: None,
                label: "Secret Ability".into(),
            },
        ]
    );
    assert_eq!(mapped.active_player, 2);
    assert_eq!(mapped.step, debug_pb::Step::Cleanup as i32);
    assert_eq!(mapped.priority_player, 1);
    assert_eq!(mapped.consecutive_passes, u32::from(u8::MAX));
    assert!(mapped.has_pending_choice);
    assert!(!mapped.has_deferred_resume);

    assert_eq!(
        mapped.objects[5],
        debug_pb::ObjectInspection {
            object_id: 16,
            state: Some(State::Card(debug_pb::CardInspection {
                card_id: "face-down-card".into(),
                owner: 2,
                zone: debug_pb::Zone::Hand as i32,
                commander: false,
                face_down: true,
            })),
        }
    );
    assert_eq!(
        mapped.objects[6],
        debug_pb::ObjectInspection {
            object_id: 17,
            state: Some(State::Permanent(debug_pb::PermanentInspection {
                card_id: "commander-permanent".into(),
                owner: 2,
                controller: 3,
                tapped: false,
                marked_damage: 1,
                plus_one_counters: 2,
                attached_to: None,
                commander: true,
                token: false,
                face_down: false,
            })),
        }
    );
    assert_eq!(
        mapped.objects[7],
        debug_pb::ObjectInspection {
            object_id: 18,
            state: Some(State::Permanent(debug_pb::PermanentInspection {
                card_id: "token-permanent".into(),
                owner: 3,
                controller: 1,
                tapped: false,
                marked_damage: 3,
                plus_one_counters: 4,
                attached_to: None,
                commander: false,
                token: true,
                face_down: false,
            })),
        }
    );
    assert_eq!(
        mapped.objects[8],
        debug_pb::ObjectInspection {
            object_id: 19,
            state: Some(State::Permanent(debug_pb::PermanentInspection {
                card_id: "face-down-permanent".into(),
                owner: 1,
                controller: 3,
                tapped: false,
                marked_damage: 5,
                plus_one_counters: 6,
                attached_to: None,
                commander: false,
                token: false,
                face_down: true,
            })),
        }
    );

    let mut reversed_game_flags = empty_debug_inspection();
    reversed_game_flags.has_pending_choice = false;
    reversed_game_flags.has_deferred_resume = true;
    let reversed_game_flags =
        debug_svc::map_inspection(reversed_game_flags).expect("reversed game flags map");
    assert!(!reversed_game_flags.has_pending_choice);
    assert!(reversed_game_flags.has_deferred_resume);
}

#[cfg(debug_assertions)]
#[test]
fn debug_service_maps_every_step_zone_and_player_counter() {
    use debug_pb::mutation::Operation;
    use engine::debug::{DebugZone, Mutation, ObjectInspection};
    use engine::{PlayerCounterKind, Step, Zone};

    let steps = [
        (debug_pb::Step::Untap, Step::Untap),
        (debug_pb::Step::Upkeep, Step::Upkeep),
        (debug_pb::Step::Draw, Step::Draw),
        (debug_pb::Step::Main1, Step::Main1),
        (debug_pb::Step::BeginCombat, Step::BeginCombat),
        (debug_pb::Step::DeclareAttackers, Step::DeclareAttackers),
        (debug_pb::Step::DeclareBlockers, Step::DeclareBlockers),
        (
            debug_pb::Step::FirstStrikeCombatDamage,
            Step::FirstStrikeCombatDamage,
        ),
        (debug_pb::Step::CombatDamage, Step::CombatDamage),
        (debug_pb::Step::EndCombat, Step::EndCombat),
        (debug_pb::Step::Main2, Step::Main2),
        (debug_pb::Step::End, Step::End),
        (debug_pb::Step::Cleanup, Step::Cleanup),
    ];
    for (wire, domain) in steps {
        let mapped = debug_svc::map_mutation(debug_pb::Mutation {
            operation: Some(Operation::SetTurnState(debug_pb::SetTurnState {
                step: wire as i32,
                ..Default::default()
            })),
        })
        .expect("known step maps");
        assert!(matches!(mapped, Mutation::SetTurnState { step, .. } if step == domain));

        let mut inspection = empty_debug_inspection();
        inspection.step = domain;
        assert_eq!(
            debug_svc::map_inspection(inspection).unwrap().step,
            wire as i32
        );
    }

    let zones = [
        (debug_pb::Zone::Library, DebugZone::Library, Zone::Library),
        (debug_pb::Zone::Hand, DebugZone::Hand, Zone::Hand),
        (
            debug_pb::Zone::Battlefield,
            DebugZone::Battlefield,
            Zone::Battlefield,
        ),
        (
            debug_pb::Zone::Graveyard,
            DebugZone::Graveyard,
            Zone::Graveyard,
        ),
        (debug_pb::Zone::Exile, DebugZone::Exile, Zone::Exile),
        (debug_pb::Zone::Command, DebugZone::Command, Zone::Command),
    ];
    for (wire, debug_zone, zone) in zones {
        let mapped = debug_svc::map_mutation(debug_pb::Mutation {
            operation: Some(Operation::CreateCard(debug_pb::CreateCard {
                card_id: "card".into(),
                destination: wire as i32,
                ..Default::default()
            })),
        })
        .expect("known debug zone maps");
        assert!(
            matches!(mapped, Mutation::CreateCard { destination, .. } if destination == debug_zone)
        );

        let mut inspection = empty_debug_inspection();
        inspection.objects.push(ObjectInspection::Card {
            object_id: 1,
            card_id: "card".into(),
            owner: engine::PlayerId(0),
            zone,
            commander: false,
            face_down: false,
        });
        let state = debug_svc::map_inspection(inspection)
            .unwrap()
            .objects
            .remove(0)
            .state
            .unwrap();
        assert!(
            matches!(state, debug_pb::object_inspection::State::Card(card) if card.zone == wire as i32)
        );
    }

    for (wire, domain) in [
        (debug_pb::PlayerCounter::Poison, PlayerCounterKind::Poison),
        (debug_pb::PlayerCounter::Rad, PlayerCounterKind::Rad),
    ] {
        let mapped = debug_svc::map_mutation(debug_pb::Mutation {
            operation: Some(Operation::SetPlayerCounter(debug_pb::SetPlayerCounter {
                counter: wire as i32,
                ..Default::default()
            })),
        })
        .expect("known player counter maps");
        assert!(matches!(mapped, Mutation::SetPlayerCounter { counter, .. } if counter == domain));
    }
}

#[cfg(debug_assertions)]
#[test]
fn debug_service_inspection_preserves_absent_stack_controller() {
    let mapped = debug_svc::map_inspection(engine::debug::Inspection {
        players: vec![],
        objects: vec![],
        stack: vec![engine::debug::StackInspection {
            position_from_bottom: 0,
            kind: "spell",
            source_object_id: Some(u32::MAX),
            controller: None,
            label: "corrupt stack entry".into(),
        }],
        active_player: engine::PlayerId(0),
        step: engine::Step::Upkeep,
        priority_player: engine::PlayerId(0),
        consecutive_passes: 0,
        has_pending_choice: false,
        has_deferred_resume: false,
    })
    .expect("inspection values fit the protobuf");

    assert_eq!(mapped.stack[0].source_object_id, Some(u32::MAX));
    assert_eq!(mapped.stack[0].controller, None, "absence is preserved");
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_journal_maps_all_eleven_domain_mutations_with_exact_wire_fields() {
    use crate::debug::{JournalKind, JournalRecord};
    use debug_pb::debug_service_server::DebugService;
    use debug_pb::mutation::Operation;
    use engine::debug::{DebugZone, Mutation};
    use engine::{PlayerCounterKind, PlayerId, Step};

    let operations = vec![
        Mutation::SetLife {
            player: PlayerId(3),
            life: -41,
        },
        Mutation::SetPlayerCounter {
            player: PlayerId(2),
            counter: PlayerCounterKind::Rad,
            value: 17,
        },
        Mutation::SetTurnState {
            active_player: PlayerId(1),
            step: Step::FirstStrikeCombatDamage,
            priority_player: PlayerId(3),
            consecutive_passes: 2,
        },
        Mutation::SetPermanentState {
            object_id: 101,
            tapped: Some(false),
            marked_damage: Some(-17),
            plus_one_counters: None,
        },
        Mutation::SetController {
            object_id: 102,
            controller: PlayerId(2),
        },
        Mutation::SetAttachment {
            object_id: 103,
            attached_to: Some(203),
        },
        Mutation::CreateCard {
            object_id: 104,
            card_id: "created-card".into(),
            owner: PlayerId(1),
            controller: PlayerId(3),
            destination: DebugZone::Exile,
            commander: true,
            face_down: false,
        },
        Mutation::MoveCard {
            object_id: 105,
            new_object_id: 205,
            destination: DebugZone::Command,
            controller: PlayerId(2),
            face_down: true,
        },
        Mutation::SetLibraryOrder {
            player: PlayerId(3),
            object_ids: vec![301, 302, 303],
        },
        Mutation::RemoveCard { object_id: 106 },
        Mutation::ClearPendingOrchestration {
            clear_queued_triggers: true,
        },
    ];
    let expected = vec![
        Operation::SetLife(debug_pb::SetLife {
            player: 3,
            life: -41,
        }),
        Operation::SetPlayerCounter(debug_pb::SetPlayerCounter {
            player: 2,
            counter: debug_pb::PlayerCounter::Rad as i32,
            value: 17,
        }),
        Operation::SetTurnState(debug_pb::SetTurnState {
            active_player: 1,
            step: debug_pb::Step::FirstStrikeCombatDamage as i32,
            priority_player: 3,
            consecutive_passes: 2,
        }),
        Operation::SetPermanentState(debug_pb::SetPermanentState {
            object_id: 101,
            tapped: Some(false),
            marked_damage: Some(-17),
            plus_one_counters: None,
        }),
        Operation::SetController(debug_pb::SetController {
            object_id: 102,
            controller: 2,
        }),
        Operation::SetAttachment(debug_pb::SetAttachment {
            object_id: 103,
            attached_to: Some(203),
        }),
        Operation::CreateCard(debug_pb::CreateCard {
            object_id: 104,
            card_id: "created-card".into(),
            owner: 1,
            controller: 3,
            destination: debug_pb::Zone::Exile as i32,
            commander: true,
            face_down: false,
        }),
        Operation::MoveCard(debug_pb::MoveCard {
            object_id: 105,
            new_object_id: 205,
            destination: debug_pb::Zone::Command as i32,
            controller: 2,
            face_down: true,
        }),
        Operation::SetLibraryOrder(debug_pb::SetLibraryOrder {
            player: 3,
            object_ids: vec![301, 302, 303],
        }),
        Operation::RemoveCard(debug_pb::RemoveCard { object_id: 106 }),
        Operation::ClearPendingOrchestration(debug_pb::ClearPendingOrchestration {
            clear_queued_triggers: true,
        }),
    ];

    let state = test_state().await;
    insert_debug_game(&state, "table", engine::Game::with_players(4, 0), 0, 0);
    crate::lock(&state.reg)
        .get_mut("table")
        .unwrap()
        .debug
        .journal
        .push_back(JournalRecord {
            ordinal: 7,
            timestamp_unix_ms: 1_700_000_123_456,
            debug_revision: 11,
            table_seq: 13,
            encoded_request_bytes: 1_337,
            kind: JournalKind::MutationCommitted { operations },
        });

    let response = debug_svc::DebugSvc::new(state)
        .get_debug_journal(Request::new(debug_pb::GetDebugJournalRequest {
            table_id: "table".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.records.len(), 1);
    let record = &response.records[0];
    assert_eq!(
        (
            record.ordinal,
            record.timestamp_unix_ms,
            record.debug_revision,
            record.table_seq,
            record.encoded_request_bytes
        ),
        (7, 1_700_000_123_456, 11, 13, 1_337),
    );
    let actual = match record.kind.as_ref().unwrap() {
        debug_pb::debug_journal_record::Kind::MutationCommitted(committed) => &committed.operations,
        other => panic!("expected mutation_committed oneof, got {other:?}"),
    };
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.operation.as_ref(), Some(&expected));
    }
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_inspection_maps_distinguishable_chrome_pending_and_phase_a_flags() {
    use debug_pb::debug_service_server::DebugService;

    let state = test_state().await;
    insert_debug_game(&state, "table", engine::Game::with_players(4, 0), 21, 34);
    let mut inspection = empty_debug_inspection();
    inspection.has_pending_choice = true;
    inspection.has_deferred_resume = true;
    let service = debug_svc::DebugSvc::with_inspection_state_for_test(
        state,
        inspection,
        crate::chrome::DebugChromeSnapshot {
            yields: [true, false, true, false],
            turn_yields: [false, true, false, true],
            hold_requested: true,
        },
        engine::debug::PendingOrchestrationInspection {
            has_pending_choice: true,
            has_resume: true,
            has_resolution_frame: true,
            has_resolution_finish: true,
            pending_enter_bonus_counters: 2,
            pending_trigger_groups: 3,
            pending_obligations: 4,
        },
    );

    let response = service
        .inspect_table(Request::new(debug_pb::InspectTableRequest {
            table_id: "table".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    let game = response.game.unwrap();
    assert!(game.has_pending_choice);
    assert!(game.has_deferred_resume);
    let chrome = response.chrome.unwrap();
    assert_eq!(chrome.yields, [true, false, true, false]);
    assert_eq!(chrome.turn_yields, [false, true, false, true]);
    assert!(chrome.hold_requested);
    let pending = response.pending_orchestration.unwrap();
    assert!(pending.has_pending_choice);
    assert!(pending.has_resume);
    assert!(pending.has_resolution_frame);
    assert!(pending.has_resolution_finish);
    assert_eq!(pending.pending_enter_bonus_counters, 2);
    assert_eq!(pending.pending_trigger_groups, 3);
    assert_eq!(pending.pending_obligations, 4);
}

#[cfg(debug_assertions)]
#[test]
fn debug_service_maps_all_eleven_mutations_without_truncation() {
    use debug_pb::mutation::Operation;
    use engine::debug::{DebugZone, Mutation};
    use engine::{PlayerCounterKind, PlayerId, Step};

    let cases = vec![
        (
            Operation::SetLife(debug_pb::SetLife {
                player: 3,
                life: i32::MIN,
            }),
            Mutation::SetLife {
                player: PlayerId(3),
                life: i32::MIN,
            },
        ),
        (
            Operation::SetPlayerCounter(debug_pb::SetPlayerCounter {
                player: 2,
                counter: debug_pb::PlayerCounter::Rad as i32,
                value: u8::MAX.into(),
            }),
            Mutation::SetPlayerCounter {
                player: PlayerId(2),
                counter: PlayerCounterKind::Rad,
                value: u8::MAX,
            },
        ),
        (
            Operation::SetTurnState(debug_pb::SetTurnState {
                active_player: 1,
                step: debug_pb::Step::FirstStrikeCombatDamage as i32,
                priority_player: 2,
                consecutive_passes: u8::MAX.into(),
            }),
            Mutation::SetTurnState {
                active_player: PlayerId(1),
                step: Step::FirstStrikeCombatDamage,
                priority_player: PlayerId(2),
                consecutive_passes: u8::MAX,
            },
        ),
        (
            Operation::SetPermanentState(debug_pb::SetPermanentState {
                object_id: u32::MAX,
                tapped: Some(false),
                marked_damage: Some(i32::MAX),
                plus_one_counters: None,
            }),
            Mutation::SetPermanentState {
                object_id: u32::MAX,
                tapped: Some(false),
                marked_damage: Some(i32::MAX),
                plus_one_counters: None,
            },
        ),
        (
            Operation::SetController(debug_pb::SetController {
                object_id: u32::MAX,
                controller: 3,
            }),
            Mutation::SetController {
                object_id: u32::MAX,
                controller: PlayerId(3),
            },
        ),
        (
            Operation::SetAttachment(debug_pb::SetAttachment {
                object_id: u32::MAX,
                attached_to: Some(u32::MAX - 1),
            }),
            Mutation::SetAttachment {
                object_id: u32::MAX,
                attached_to: Some(u32::MAX - 1),
            },
        ),
        (
            Operation::CreateCard(debug_pb::CreateCard {
                object_id: u32::MAX,
                card_id: "card-id".into(),
                owner: 1,
                controller: 2,
                destination: debug_pb::Zone::Battlefield as i32,
                commander: true,
                face_down: false,
            }),
            Mutation::CreateCard {
                object_id: u32::MAX,
                card_id: "card-id".into(),
                owner: PlayerId(1),
                controller: PlayerId(2),
                destination: DebugZone::Battlefield,
                commander: true,
                face_down: false,
            },
        ),
        (
            Operation::CreateCard(debug_pb::CreateCard {
                object_id: u32::MAX - 1,
                card_id: "face-down-card-id".into(),
                owner: 2,
                controller: 3,
                destination: debug_pb::Zone::Exile as i32,
                commander: false,
                face_down: true,
            }),
            Mutation::CreateCard {
                object_id: u32::MAX - 1,
                card_id: "face-down-card-id".into(),
                owner: PlayerId(2),
                controller: PlayerId(3),
                destination: DebugZone::Exile,
                commander: false,
                face_down: true,
            },
        ),
        (
            Operation::MoveCard(debug_pb::MoveCard {
                object_id: u32::MAX - 1,
                new_object_id: u32::MAX,
                destination: debug_pb::Zone::Command as i32,
                controller: 3,
                face_down: false,
            }),
            Mutation::MoveCard {
                object_id: u32::MAX - 1,
                new_object_id: u32::MAX,
                destination: DebugZone::Command,
                controller: PlayerId(3),
                face_down: false,
            },
        ),
        (
            Operation::SetLibraryOrder(debug_pb::SetLibraryOrder {
                player: 3,
                object_ids: vec![0, u32::MAX],
            }),
            Mutation::SetLibraryOrder {
                player: PlayerId(3),
                object_ids: vec![0, u32::MAX],
            },
        ),
        (
            Operation::RemoveCard(debug_pb::RemoveCard {
                object_id: u32::MAX,
            }),
            Mutation::RemoveCard {
                object_id: u32::MAX,
            },
        ),
        (
            Operation::ClearPendingOrchestration(debug_pb::ClearPendingOrchestration {
                clear_queued_triggers: true,
            }),
            Mutation::ClearPendingOrchestration {
                clear_queued_triggers: true,
            },
        ),
    ];

    for (operation, expected) in cases {
        let actual = debug_svc::map_mutation(debug_pb::Mutation {
            operation: Some(operation),
        })
        .expect("valid mutation maps");
        assert_eq!(actual, expected);
    }
}

#[cfg(debug_assertions)]
#[test]
fn debug_service_rejects_missing_unknown_and_narrowing_inputs() {
    use debug_pb::mutation::Operation;

    let invalid = [
        debug_pb::Mutation { operation: None },
        debug_pb::Mutation {
            operation: Some(Operation::SetLife(debug_pb::SetLife {
                player: 256,
                life: 0,
            })),
        },
        debug_pb::Mutation {
            operation: Some(Operation::SetPlayerCounter(debug_pb::SetPlayerCounter {
                player: 0,
                counter: debug_pb::PlayerCounter::Unspecified as i32,
                value: 0,
            })),
        },
        debug_pb::Mutation {
            operation: Some(Operation::SetPlayerCounter(debug_pb::SetPlayerCounter {
                player: 0,
                counter: 99,
                value: 0,
            })),
        },
        debug_pb::Mutation {
            operation: Some(Operation::SetPlayerCounter(debug_pb::SetPlayerCounter {
                player: 0,
                counter: debug_pb::PlayerCounter::Poison as i32,
                value: 256,
            })),
        },
        debug_pb::Mutation {
            operation: Some(Operation::SetTurnState(debug_pb::SetTurnState {
                active_player: 0,
                step: debug_pb::Step::Unspecified as i32,
                priority_player: 0,
                consecutive_passes: 0,
            })),
        },
        debug_pb::Mutation {
            operation: Some(Operation::SetTurnState(debug_pb::SetTurnState {
                active_player: 0,
                step: 99,
                priority_player: 0,
                consecutive_passes: 0,
            })),
        },
        debug_pb::Mutation {
            operation: Some(Operation::SetTurnState(debug_pb::SetTurnState {
                active_player: 0,
                step: debug_pb::Step::Upkeep as i32,
                priority_player: 0,
                consecutive_passes: 256,
            })),
        },
        debug_pb::Mutation {
            operation: Some(Operation::CreateCard(debug_pb::CreateCard {
                card_id: "Shock".into(),
                owner: 1,
                controller: 2,
                destination: debug_pb::Zone::Unspecified as i32,
                ..Default::default()
            })),
        },
        debug_pb::Mutation {
            operation: Some(Operation::MoveCard(debug_pb::MoveCard {
                destination: 99,
                ..Default::default()
            })),
        },
    ];

    for mutation in invalid {
        let error = debug_svc::map_mutation(mutation).expect_err("malformed input is rejected");
        assert_eq!(error.code(), tonic::Code::InvalidArgument);
    }
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_mutate_journal_accounts_for_the_exact_protobuf_request_bytes() {
    use debug_pb::debug_service_server::DebugService;

    let state = test_state().await;
    insert_debug_game(&state, "table", engine::Game::with_players(2, 0), 0, 0);
    let service = debug_svc::DebugSvc::new(state.clone());
    let request = debug_pb::MutateTableRequest {
        table_id: "table".into(),
        expected_debug_revision: Some(0),
        expected_table_seq: Some(0),
        operations: vec![debug_set_life(0, 19)],
    };
    let encoded_len = prost::Message::encoded_len(&request);

    service
        .mutate_table(Request::new(request))
        .await
        .expect("valid mutation commits");

    let registry = crate::lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!(table.debug.journal.len(), 1);
    assert_eq!(table.debug.journal_request_bytes, encoded_len);
    assert_eq!(
        table.debug.journal.front().unwrap().encoded_request_bytes,
        encoded_len
    );
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_service_maps_domain_failures_to_bounded_typed_status_details() {
    use debug_pb::debug_service_server::DebugService;
    use debug_pb::mutation::Operation;

    let state = test_state().await;
    let mut game = engine::Game::with_players(2, 0);
    game.spawn_in_hand(
        engine::PlayerId(0),
        cards::get_by_name("Shock").expect("test card"),
    );
    insert_debug_game(&state, "table", game, 0, 0);
    let service = debug_svc::DebugSvc::new(state.clone());

    let success = service
        .mutate_table(Request::new(debug_pb::MutateTableRequest {
            table_id: "table".into(),
            expected_debug_revision: Some(0),
            expected_table_seq: Some(0),
            operations: vec![debug_set_life(0, 19)],
        }))
        .await
        .expect("both matching guards commit")
        .into_inner();
    assert_eq!((success.debug_revision, success.table_seq), (1, 1));
    assert_eq!(success.applied_operation_count, 1);

    let checks = vec![
        (
            debug_pb::MutateTableRequest {
                table_id: "missing-table".into(),
                operations: vec![debug_set_life(0, 1)],
                ..Default::default()
            },
            tonic::Code::NotFound,
            None,
            debug_pb::DebugErrorReason::UnknownEntity,
        ),
        (
            debug_pb::MutateTableRequest {
                table_id: "table".into(),
                operations: vec![debug_set_life(9, 1)],
                ..Default::default()
            },
            tonic::Code::NotFound,
            Some(0),
            debug_pb::DebugErrorReason::UnknownEntity,
        ),
        (
            debug_pb::MutateTableRequest {
                table_id: "table".into(),
                operations: vec![debug_pb::Mutation {
                    operation: Some(Operation::CreateCard(debug_pb::CreateCard {
                        object_id: 0,
                        card_id: cards::get_by_name("Plains").unwrap().id.to_string(),
                        owner: 0,
                        controller: 0,
                        destination: debug_pb::Zone::Hand as i32,
                        commander: false,
                        face_down: false,
                    })),
                }],
                ..Default::default()
            },
            tonic::Code::AlreadyExists,
            Some(0),
            debug_pb::DebugErrorReason::DuplicateId,
        ),
        (
            debug_pb::MutateTableRequest {
                table_id: "table".into(),
                operations: vec![],
                ..Default::default()
            },
            tonic::Code::InvalidArgument,
            None,
            debug_pb::DebugErrorReason::EmptyBatch,
        ),
        (
            debug_pb::MutateTableRequest {
                table_id: "table".into(),
                expected_debug_revision: Some(0),
                operations: vec![debug_set_life(0, 1)],
                ..Default::default()
            },
            tonic::Code::Aborted,
            None,
            debug_pb::DebugErrorReason::StaleDebugRevision,
        ),
        (
            debug_pb::MutateTableRequest {
                table_id: "table".into(),
                expected_table_seq: Some(0),
                operations: vec![debug_set_life(0, 1)],
                ..Default::default()
            },
            tonic::Code::Aborted,
            None,
            debug_pb::DebugErrorReason::StaleTableSeq,
        ),
    ];

    for (request, code, operation_index, reason) in checks {
        let error = service
            .mutate_table(Request::new(request))
            .await
            .expect_err("domain failure becomes tonic status");
        assert_eq!(error.code(), code);
        assert!(error.message().len() <= 64);
        let detail = decode_debug_detail(&error);
        assert_eq!(detail.operation_index, operation_index);
        assert_eq!(detail.reason, reason as i32);
        assert!(detail.violations.len() <= 16);
        let wire = error.details();
        assert!(!String::from_utf8_lossy(wire).contains("Plains"));
        assert!(!String::from_utf8_lossy(wire).contains("operations"));
        assert!(!error.message().contains("table"));
    }

    let invalid_mapping = service
        .mutate_table(Request::new(debug_pb::MutateTableRequest {
            table_id: "table".into(),
            operations: vec![debug_pb::Mutation {
                operation: Some(Operation::SetPlayerCounter(debug_pb::SetPlayerCounter {
                    player: 0,
                    counter: debug_pb::PlayerCounter::Poison as i32,
                    value: 256,
                })),
            }],
            ..Default::default()
        }))
        .await
        .expect_err("checked narrowing is rejected before the transaction");
    assert_eq!(invalid_mapping.code(), tonic::Code::InvalidArgument);
    let detail = decode_debug_detail(&invalid_mapping);
    assert_eq!(detail.operation_index, Some(0));
    assert_eq!(
        detail.reason,
        debug_pb::DebugErrorReason::InvalidValue as i32
    );

    let projected_state = test_state().await;
    insert_debug_game(
        &projected_state,
        "projection",
        engine::Game::with_players(5, 0),
        0,
        0,
    );
    let projected_service = debug_svc::DebugSvc::new(projected_state);
    let projection_error = projected_service
        .mutate_table(Request::new(debug_pb::MutateTableRequest {
            table_id: "projection".into(),
            operations: vec![debug_set_life(0, 5)],
            ..Default::default()
        }))
        .await
        .expect_err("unsafe projection is rejected");
    assert_eq!(projection_error.code(), tonic::Code::FailedPrecondition);
    let detail = decode_debug_detail(&projection_error);
    assert_eq!(detail.operation_index, None);
    assert_eq!(
        detail.reason,
        debug_pb::DebugErrorReason::ProjectionFailed as i32
    );
    assert_eq!(detail.violations.len(), 1);
    assert!(detail.violations[0].message.len() <= 64);
}

#[cfg(debug_assertions)]
#[test]
fn debug_service_maps_every_domain_failure_reason_and_operation_index() {
    use engine::debug::ErrorReason;

    let invalid_reasons = [
        (
            ErrorReason::EmptyBatch,
            debug_pb::DebugErrorReason::EmptyBatch,
        ),
        (
            ErrorReason::UnknownEntity,
            debug_pb::DebugErrorReason::UnknownEntity,
        ),
        (
            ErrorReason::DuplicateId,
            debug_pb::DebugErrorReason::DuplicateId,
        ),
        (
            ErrorReason::InvalidValue,
            debug_pb::DebugErrorReason::InvalidValue,
        ),
        (
            ErrorReason::WrongObjectKind,
            debug_pb::DebugErrorReason::WrongObjectKind,
        ),
        (
            ErrorReason::ZoneDisagreement,
            debug_pb::DebugErrorReason::ZoneDisagreement,
        ),
        (
            ErrorReason::ReferencedObject,
            debug_pb::DebugErrorReason::ReferencedObject,
        ),
        (
            ErrorReason::AttachmentCycle,
            debug_pb::DebugErrorReason::AttachmentCycle,
        ),
    ];
    for (reason, wire_reason) in invalid_reasons {
        let error = debug_svc::status(crate::debug::DebugFailure::Invalid {
            operation_index: Some(11),
            reason,
        });
        assert_eq!(error.code(), tonic::Code::InvalidArgument);
        assert_eq!(error.message(), "debug request rejected");
        let detail = decode_debug_detail(&error);
        assert_eq!(detail.operation_index, Some(11));
        assert_eq!(detail.reason, wire_reason as i32);
        assert!(detail.violations.is_empty());
    }

    for (violations, expected_reason) in [
        (
            vec![engine::debug::Violation {
                code: "zone_index_mismatch",
                message: "private".into(),
            }],
            debug_pb::DebugErrorReason::InvalidValue,
        ),
        (
            vec![engine::debug::Violation {
                code: "projection_failed",
                message: "private".into(),
            }],
            debug_pb::DebugErrorReason::ProjectionFailed,
        ),
    ] {
        let error = debug_svc::status(crate::debug::DebugFailure::FailedPrecondition {
            operation_index: Some(12),
            violations,
        });
        assert_eq!(error.code(), tonic::Code::FailedPrecondition);
        assert_eq!(error.message(), "debug request rejected");
        let detail = decode_debug_detail(&error);
        assert_eq!(detail.operation_index, Some(12));
        assert_eq!(detail.reason, expected_reason as i32);
        assert_eq!(
            detail.violations[0].message,
            "candidate violates a structural invariant"
        );
        assert!(!String::from_utf8_lossy(error.details()).contains("private"));
    }
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_service_direct_requests_reject_empty_ids_and_missing_inspection_without_secrets() {
    use debug_pb::debug_service_server::DebugService;

    let state = test_state().await;
    let mut no_game = crate::Table::empty();
    no_game.debug.revision = 99;
    assert!(crate::lock(&state.reg).try_insert("no-game".into(), no_game));
    let service = debug_svc::DebugSvc::new(state);

    let empty_inspect = service
        .inspect_table(Request::new(debug_pb::InspectTableRequest {
            table_id: String::new(),
        }))
        .await
        .expect_err("empty inspect table id is invalid");
    let missing_inspect = service
        .inspect_table(Request::new(debug_pb::InspectTableRequest {
            table_id: "private-missing-id".into(),
        }))
        .await
        .expect_err("missing inspected table is not found");
    let no_game_inspect = service
        .inspect_table(Request::new(debug_pb::InspectTableRequest {
            table_id: "no-game".into(),
        }))
        .await
        .expect_err("table without a game is not inspectable");
    let empty_mutate = service
        .mutate_table(Request::new(debug_pb::MutateTableRequest {
            table_id: String::new(),
            operations: vec![debug_set_life(0, 1)],
            ..Default::default()
        }))
        .await
        .expect_err("empty mutate table id is invalid");

    for (error, code, reason) in [
        (
            empty_inspect,
            tonic::Code::InvalidArgument,
            debug_pb::DebugErrorReason::InvalidValue,
        ),
        (
            missing_inspect,
            tonic::Code::NotFound,
            debug_pb::DebugErrorReason::UnknownEntity,
        ),
        (
            no_game_inspect,
            tonic::Code::NotFound,
            debug_pb::DebugErrorReason::UnknownEntity,
        ),
        (
            empty_mutate,
            tonic::Code::InvalidArgument,
            debug_pb::DebugErrorReason::InvalidValue,
        ),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.message(), "debug request rejected");
        let detail = decode_debug_detail(&error);
        assert_eq!(detail.operation_index, None);
        assert_eq!(detail.reason, reason as i32);
        assert!(!String::from_utf8_lossy(error.details()).contains("private"));
        assert!(!error.message().contains("private"));
    }
}

#[cfg(debug_assertions)]
#[test]
fn debug_service_bounds_and_sanitizes_structural_failure_details() {
    let violations = (0..20)
        .map(|_| engine::debug::Violation {
            code: "structural_code_that_is_intentionally_longer_than_sixty_four_ascii_bytes_for_bounding",
            message: "Plains and the submitted operation must never reach status details".into(),
        })
        .collect();
    let error = debug_svc::status(crate::debug::DebugFailure::FailedPrecondition {
        operation_index: Some(7),
        violations,
    });

    assert_eq!(error.code(), tonic::Code::FailedPrecondition);
    let detail = decode_debug_detail(&error);
    assert_eq!(detail.operation_index, Some(7));
    assert_eq!(detail.violations.len(), 16);
    assert!(detail.violations.iter().all(|violation| {
        violation.code.len() <= 64
            && violation.message.len() <= 64
            && !violation.message.contains("Plains")
            && !violation.message.contains("operation")
    }));
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_service_is_registered_without_auth() {
    let state = test_state().await;
    insert_debug_game(
        &state,
        "bound-table",
        engine::Game::with_players(2, 0),
        0,
        0,
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("ephemeral port");
    let addr = listener.local_addr().expect("listener address");
    drop(listener);

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    let server = tokio::spawn(async move {
        super::serve(addr, state, async move {
            let _ = shutdown_rx.changed().await;
        })
        .await
    });
    let endpoint = format!("http://{addr}");
    let mut debug = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match debug_pb::debug_service_client::DebugServiceClient::connect(endpoint.clone())
                .await
            {
                Ok(client) => return client,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("debug client connects");

    let listed = debug
        .list_tables(debug_pb::ListTablesRequest {})
        .await
        .expect("ListTables needs no metadata")
        .into_inner();
    assert_eq!(listed.tables.len(), 1);
    debug
        .inspect_table(debug_pb::InspectTableRequest {
            table_id: "bound-table".into(),
        })
        .await
        .expect("InspectTable needs no metadata");
    debug
        .checkpoint_table(debug_pb::CheckpointTableRequest {
            table_id: "bound-table".into(),
            name: "baseline".into(),
            expected_table_seq: Some(0),
            ..Default::default()
        })
        .await
        .expect("CheckpointTable needs no metadata");
    debug
        .mutate_table(debug_pb::MutateTableRequest {
            table_id: "bound-table".into(),
            expected_debug_revision: Some(0),
            expected_table_seq: Some(0),
            operations: vec![debug_set_life(0, 18)],
        })
        .await
        .expect("MutateTable needs no metadata");
    debug
        .restore_checkpoint(debug_pb::RestoreCheckpointRequest {
            table_id: "bound-table".into(),
            name: "baseline".into(),
            expected_debug_revision: Some(1),
            expected_table_seq: Some(1),
        })
        .await
        .expect("RestoreCheckpoint needs no metadata");
    let journal = debug
        .get_debug_journal(debug_pb::GetDebugJournalRequest {
            table_id: "bound-table".into(),
        })
        .await
        .expect("GetDebugJournal needs no metadata")
        .into_inner();
    assert_eq!(journal.records.len(), 3);
    assert_eq!(
        journal
            .records
            .iter()
            .map(|record| record.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        journal
            .records
            .iter()
            .map(|record| record.debug_revision)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        journal
            .records
            .iter()
            .map(|record| record.table_seq)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(
        journal
            .records
            .iter()
            .all(|record| record.encoded_request_bytes > 0)
    );
    assert!(matches!(
        journal.records[0].kind,
        Some(debug_pb::debug_journal_record::Kind::CheckpointCreated(_))
    ));
    assert!(matches!(
        journal.records[1].kind,
        Some(debug_pb::debug_journal_record::Kind::MutationCommitted(_))
    ));
    assert!(matches!(
        journal.records[2].kind,
        Some(debug_pb::debug_journal_record::Kind::CheckpointRestored(_))
    ));

    let channel = tonic::transport::Endpoint::from_shared(endpoint)
        .expect("valid endpoint")
        .connect()
        .await
        .expect("ordinary client connects");
    let mut auth = tonic::client::Grpc::new(channel);
    auth.ready().await.expect("ordinary gRPC client ready");
    let mut request = Request::new(pb::GetMeRequest {});
    request
        .extensions_mut()
        .insert(tonic::GrpcMethod::new("mtgfr.v1.AuthService", "GetMe"));
    let auth_result: Result<tonic::Response<pb::Me>, Status> = auth
        .unary(
            request,
            http::uri::PathAndQuery::from_static("/mtgfr.v1.AuthService/GetMe"),
            tonic::codec::ProstCodec::default(),
        )
        .await;
    let auth_error = auth_result.expect_err("ordinary authenticated service remains protected");
    assert_eq!(auth_error.code(), tonic::Code::Unauthenticated);

    let _ = shutdown_tx.send(true);
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .expect("server shuts down")
        .expect("server task does not panic")
        .expect("server exits cleanly");
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_checkpoint_restore_and_journal_are_plain_requests_with_exact_wire_facts() {
    use debug_pb::debug_service_server::DebugService;
    use prost::Message;

    let state = test_state().await;
    let mut game = engine::Game::with_players(2, 0);
    game.spawn_in_hand(engine::PlayerId(0), cards::get_by_name("Island").unwrap());
    insert_debug_game(&state, "table", game, 0, 0);
    let service = debug_svc::DebugSvc::new(state.clone());

    let checkpoint = debug_pb::CheckpointTableRequest {
        table_id: "table".into(),
        name: "z-last".into(),
        replace_existing: false,
        expected_table_seq: Some(0),
    };
    let checkpoint_bytes = checkpoint.encoded_len();
    let created = service
        .checkpoint_table(Request::new(checkpoint))
        .await
        .unwrap()
        .into_inner();
    assert_eq!((created.debug_revision, created.table_seq), (0, 0));
    assert_eq!(created.object_slots, 1);
    assert!(!created.replaced);

    let second_checkpoint = debug_pb::CheckpointTableRequest {
        table_id: "table".into(),
        name: "a-first".into(),
        ..Default::default()
    };
    let second_checkpoint_bytes = second_checkpoint.encoded_len();
    service
        .checkpoint_table(Request::new(second_checkpoint))
        .await
        .unwrap();

    let replacement = debug_pb::CheckpointTableRequest {
        table_id: "table".into(),
        name: "z-last".into(),
        replace_existing: true,
        expected_table_seq: Some(0),
    };
    let replacement_bytes = replacement.encoded_len();
    let replaced = service
        .checkpoint_table(Request::new(replacement))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(replaced.object_slots, 1);
    assert!(replaced.replaced);

    let mutation = debug_pb::MutateTableRequest {
        table_id: "table".into(),
        expected_debug_revision: Some(0),
        expected_table_seq: Some(0),
        operations: vec![debug_set_life(0, 7)],
    };
    let mutation_bytes = mutation.encoded_len();
    service.mutate_table(Request::new(mutation)).await.unwrap();

    let restore = debug_pb::RestoreCheckpointRequest {
        table_id: "table".into(),
        name: "z-last".into(),
        expected_debug_revision: Some(1),
        expected_table_seq: Some(1),
    };
    let restore_bytes = restore.encoded_len();
    let restored = service
        .restore_checkpoint(Request::new(restore))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        (
            restored.debug_revision,
            restored.table_seq,
            restored.restored_source_table_seq
        ),
        (2, 2, 0)
    );

    let inspected = service
        .inspect_table(Request::new(debug_pb::InspectTableRequest {
            table_id: "table".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(inspected.checkpoint_names, vec!["a-first", "z-last"]);

    let journal = service
        .get_debug_journal(Request::new(debug_pb::GetDebugJournalRequest {
            table_id: "table".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(journal.records.len(), 5);
    assert_eq!(
        journal
            .records
            .iter()
            .map(|record| (
                record.ordinal,
                record.debug_revision,
                record.table_seq,
                record.encoded_request_bytes,
            ))
            .collect::<Vec<_>>(),
        vec![
            (0, 0, 0, u64::try_from(checkpoint_bytes).unwrap()),
            (1, 0, 0, u64::try_from(second_checkpoint_bytes).unwrap()),
            (2, 0, 0, u64::try_from(replacement_bytes).unwrap()),
            (3, 1, 1, u64::try_from(mutation_bytes).unwrap()),
            (4, 2, 2, u64::try_from(restore_bytes).unwrap()),
        ]
    );
    assert!(
        journal
            .records
            .iter()
            .all(|record| record.timestamp_unix_ms > 0)
    );
    assert_eq!(
        journal.records[0].kind,
        Some(debug_pb::debug_journal_record::Kind::CheckpointCreated(
            debug_pb::CheckpointCreated {
                name: "z-last".into(),
                replaced: false
            }
        ))
    );
    assert_eq!(
        journal.records[1].kind,
        Some(debug_pb::debug_journal_record::Kind::CheckpointCreated(
            debug_pb::CheckpointCreated {
                name: "a-first".into(),
                replaced: false
            }
        ))
    );
    assert_eq!(
        journal.records[2].kind,
        Some(debug_pb::debug_journal_record::Kind::CheckpointCreated(
            debug_pb::CheckpointCreated {
                name: "z-last".into(),
                replaced: true
            }
        ))
    );
    let operations = match journal.records[3].kind.as_ref().unwrap() {
        debug_pb::debug_journal_record::Kind::MutationCommitted(record) => &record.operations,
        other => panic!("expected mutation_committed, got {other:?}"),
    };
    assert_eq!(operations, &[debug_set_life(0, 7)]);
    assert_eq!(
        journal.records[4].kind,
        Some(debug_pb::debug_journal_record::Kind::CheckpointRestored(
            debug_pb::CheckpointRestored {
                name: "z-last".into(),
                source_table_seq: 0
            }
        ))
    );

    let before = {
        let registry = crate::lock(&state.reg);
        let debug = &registry.get("table").unwrap().debug;
        (debug.journal.len(), debug.journal_request_bytes)
    };
    service
        .get_debug_journal(Request::new(debug_pb::GetDebugJournalRequest {
            table_id: "table".into(),
        }))
        .await
        .unwrap();
    let registry = crate::lock(&state.reg);
    let debug = &registry.get("table").unwrap().debug;
    assert_eq!(
        (debug.journal.len(), debug.journal_request_bytes),
        before,
        "journal reads are neither journaled nor charged request capacity"
    );
}

#[cfg(debug_assertions)]
#[test]
fn debug_checkpoint_failure_statuses_use_exact_additive_reasons() {
    use crate::debug::{DebugFailure, ResourceLimit};
    let cases = [
        (
            DebugFailure::CheckpointNotFound,
            tonic::Code::NotFound,
            debug_pb::DebugErrorReason::CheckpointNotFound,
        ),
        (
            DebugFailure::CheckpointAlreadyExists,
            tonic::Code::AlreadyExists,
            debug_pb::DebugErrorReason::CheckpointExists,
        ),
        (
            DebugFailure::InvalidCheckpointName,
            tonic::Code::InvalidArgument,
            debug_pb::DebugErrorReason::CheckpointNameInvalid,
        ),
        (
            DebugFailure::ResourceExhausted {
                reason: ResourceLimit::CheckpointCount,
            },
            tonic::Code::ResourceExhausted,
            debug_pb::DebugErrorReason::CheckpointCountLimit,
        ),
        (
            DebugFailure::ResourceExhausted {
                reason: ResourceLimit::CheckpointObjectSlots,
            },
            tonic::Code::ResourceExhausted,
            debug_pb::DebugErrorReason::CheckpointObjectLimit,
        ),
        (
            DebugFailure::ResourceExhausted {
                reason: ResourceLimit::JournalRecords,
            },
            tonic::Code::ResourceExhausted,
            debug_pb::DebugErrorReason::JournalLimit,
        ),
        (
            DebugFailure::ResourceExhausted {
                reason: ResourceLimit::JournalRequestBytes,
            },
            tonic::Code::ResourceExhausted,
            debug_pb::DebugErrorReason::JournalLimit,
        ),
    ];
    for (failure, code, reason) in cases {
        let status = debug_svc::status(failure);
        assert_eq!(status.code(), code);
        let detail = decode_debug_detail(&status);
        assert_eq!(detail.reason, reason as i32);
        assert_eq!(detail.operation_index, None);
    }
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn debug_checkpoint_and_restore_direct_error_matrix_is_stable() {
    use crate::debug::{MAX_CHECKPOINTS_PER_TABLE, MAX_JOURNAL_REQUEST_BYTES};
    use debug_pb::debug_service_server::DebugService;

    fn assert_status(error: Status, code: tonic::Code, reason: debug_pb::DebugErrorReason) {
        assert_eq!(error.code(), code);
        assert_eq!(error.message(), "debug request rejected");
        let detail = decode_debug_detail(&error);
        assert_eq!(detail.reason, reason as i32);
        assert_eq!(detail.operation_index, None);
    }

    let state = test_state().await;
    let mut game = engine::Game::with_players(2, 0);
    game.spawn_in_hand(engine::PlayerId(0), cards::get_by_name("Island").unwrap());
    insert_debug_game(&state, "table", game, 0, 0);
    let service = debug_svc::DebugSvc::new(state.clone());

    let invalid = service
        .checkpoint_table(Request::new(debug_pb::CheckpointTableRequest {
            table_id: "table".into(),
            name: "bad/name".into(),
            ..Default::default()
        }))
        .await
        .unwrap_err();
    assert_status(
        invalid,
        tonic::Code::InvalidArgument,
        debug_pb::DebugErrorReason::CheckpointNameInvalid,
    );

    service
        .checkpoint_table(Request::new(debug_pb::CheckpointTableRequest {
            table_id: "table".into(),
            name: "saved".into(),
            ..Default::default()
        }))
        .await
        .unwrap();
    let duplicate = service
        .checkpoint_table(Request::new(debug_pb::CheckpointTableRequest {
            table_id: "table".into(),
            name: "saved".into(),
            ..Default::default()
        }))
        .await
        .unwrap_err();
    assert_status(
        duplicate,
        tonic::Code::AlreadyExists,
        debug_pb::DebugErrorReason::CheckpointExists,
    );
    let replaced = service
        .checkpoint_table(Request::new(debug_pb::CheckpointTableRequest {
            table_id: "table".into(),
            name: "saved".into(),
            replace_existing: true,
            ..Default::default()
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(replaced.replaced);

    let stale_checkpoint = service
        .checkpoint_table(Request::new(debug_pb::CheckpointTableRequest {
            table_id: "table".into(),
            name: "stale".into(),
            expected_table_seq: Some(9),
            ..Default::default()
        }))
        .await
        .unwrap_err();
    assert_status(
        stale_checkpoint,
        tonic::Code::Aborted,
        debug_pb::DebugErrorReason::StaleTableSeq,
    );
    let stale_restore = service
        .restore_checkpoint(Request::new(debug_pb::RestoreCheckpointRequest {
            table_id: "table".into(),
            name: "saved".into(),
            expected_table_seq: Some(9),
            ..Default::default()
        }))
        .await
        .unwrap_err();
    assert_status(
        stale_restore,
        tonic::Code::Aborted,
        debug_pb::DebugErrorReason::StaleTableSeq,
    );
    let missing = service
        .restore_checkpoint(Request::new(debug_pb::RestoreCheckpointRequest {
            table_id: "table".into(),
            name: "absent".into(),
            ..Default::default()
        }))
        .await
        .unwrap_err();
    assert_status(
        missing,
        tonic::Code::NotFound,
        debug_pb::DebugErrorReason::CheckpointNotFound,
    );

    {
        let mut registry = crate::lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        let template = table.debug.checkpoints.get("saved").unwrap().clone();
        table.debug.checkpoints.clear();
        for index in 0..MAX_CHECKPOINTS_PER_TABLE {
            table
                .debug
                .checkpoints
                .insert(format!("cp-{index}"), template.clone());
        }
    }
    let count = service
        .checkpoint_table(Request::new(debug_pb::CheckpointTableRequest {
            table_id: "table".into(),
            name: "overflow".into(),
            ..Default::default()
        }))
        .await
        .unwrap_err();
    assert_status(
        count,
        tonic::Code::ResourceExhausted,
        debug_pb::DebugErrorReason::CheckpointCountLimit,
    );

    {
        let mut registry = crate::lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        table.debug.checkpoints.clear();
        table.debug.checkpoint_object_slots = crate::debug::MAX_OBJECT_SLOTS_ACROSS_CHECKPOINTS;
    }
    let objects = service
        .checkpoint_table(Request::new(debug_pb::CheckpointTableRequest {
            table_id: "table".into(),
            name: "objects".into(),
            ..Default::default()
        }))
        .await
        .unwrap_err();
    assert_status(
        objects,
        tonic::Code::ResourceExhausted,
        debug_pb::DebugErrorReason::CheckpointObjectLimit,
    );

    {
        let mut registry = crate::lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        table.debug.checkpoint_object_slots = 0;
        table.debug.journal_request_bytes = MAX_JOURNAL_REQUEST_BYTES;
    }
    let journal = service
        .checkpoint_table(Request::new(debug_pb::CheckpointTableRequest {
            table_id: "table".into(),
            name: "journal".into(),
            ..Default::default()
        }))
        .await
        .unwrap_err();
    assert_status(
        journal,
        tonic::Code::ResourceExhausted,
        debug_pb::DebugErrorReason::JournalLimit,
    );

    let empty_journal = service
        .get_debug_journal(Request::new(debug_pb::GetDebugJournalRequest {
            table_id: String::new(),
        }))
        .await
        .unwrap_err();
    assert_status(
        empty_journal,
        tonic::Code::InvalidArgument,
        debug_pb::DebugErrorReason::InvalidValue,
    );
}
