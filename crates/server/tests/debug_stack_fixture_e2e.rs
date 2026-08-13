#![cfg(debug_assertions)]

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use http::uri::PathAndQuery;
use server::grpc::{debug_pb, pb};
use tokio_stream::StreamExt;
use tonic::{Request, metadata::MetadataValue, transport::Channel};

const TABLE_ID: &str = "debug-stack-fixture-e2e";
const PRINTS: [&str; 6] = [
    "11e12a84-e7be-4afc-a230-c2e644743fa8",
    "ba0a14ac-037a-42f2-8fc9-2a41275dc7da",
    "ade7d00d-4e7b-46e9-ace1-63f628a589fc",
    "fc24f763-4c7f-45e4-933b-573d1ace1ddc",
    "1ed2b60b-5f20-4ae6-93fb-51540ad9c9e7",
    "92b8cffe-4466-48b0-836e-51ba5db36696",
];
const NAMES: [&str; 6] = [
    "Dark Ritual",
    "Fog",
    "Time Walk",
    "Tranquility",
    "Night's Whisper",
    "Vision Skeins",
];

fn settings() -> server::settings::Settings {
    server::settings::Settings {
        host: "127.0.0.1".into(),
        port: 0,
        grpc_port: 0,
        database_url: "sqlite::memory:".into(),
        instance_id: "test".into(),
        pod_dns: String::new(),
        drain: false,
        cookie_secure: false,
        cookie_domain: String::new(),
        cors_origin: String::new(),
        version: env!("CARGO_PKG_VERSION").into(),
        master_seed: Some(
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f".into(),
        ),
    }
}

fn authed<T>(message: T, token: &str) -> Request<T> {
    let mut request = Request::new(message);
    request.metadata_mut().insert(
        "x-session-token",
        token
            .parse::<MetadataValue<_>>()
            .expect("session token metadata"),
    );
    request
}

async fn signup(channel: &Channel, email: &str, username: &str) -> (i64, String) {
    let mut grpc = tonic::client::Grpc::new(channel.clone());
    grpc.ready().await.expect("auth client ready");
    let mut request = Request::new(pb::SignupRequest {
        email: email.into(),
        password: "pw".into(),
        username: username.into(),
    });
    request
        .extensions_mut()
        .insert(tonic::GrpcMethod::new("mtgfr.v1.AuthService", "Signup"));
    let response: tonic::Response<pb::SignupResponse> = grpc
        .unary(
            request,
            PathAndQuery::from_static("/mtgfr.v1.AuthService/Signup"),
            tonic::codec::ProstCodec::default(),
        )
        .await
        .expect("signup over bound gRPC");
    let response = response.into_inner();
    (
        response.me.expect("signup identity").id,
        response.session_token,
    )
}

async fn seed(channel: &Channel, request: Request<pb::SeedRequest>) {
    let mut grpc = tonic::client::Grpc::new(channel.clone());
    grpc.ready().await.expect("tables client ready");
    let mut request = request;
    request
        .extensions_mut()
        .insert(tonic::GrpcMethod::new("mtgfr.v1.TablesService", "Seed"));
    let _: tonic::Response<pb::SeedResponse> = grpc
        .unary(
            request,
            PathAndQuery::from_static("/mtgfr.v1.TablesService/Seed"),
            tonic::codec::ProstCodec::default(),
        )
        .await
        .expect("seed real table over bound gRPC");
}

async fn submit(
    channel: &Channel,
    request: Request<pb::SubmitIntentRequest>,
) -> pb::SubmitIntentResponse {
    let mut grpc = tonic::client::Grpc::new(channel.clone());
    grpc.ready().await.expect("game client ready");
    let mut request = request;
    request.extensions_mut().insert(tonic::GrpcMethod::new(
        "mtgfr.v1.GameService",
        "SubmitIntent",
    ));
    let response: tonic::Response<pb::SubmitIntentResponse> = grpc
        .unary(
            request,
            PathAndQuery::from_static("/mtgfr.v1.GameService/SubmitIntent"),
            tonic::codec::ProstCodec::default(),
        )
        .await
        .expect("submit ordinary intent over bound gRPC");
    response.into_inner()
}

async fn open_stream(
    channel: &Channel,
    request: Request<pb::StreamRequest>,
) -> tonic::Streaming<pb::StreamResponse> {
    let mut grpc = tonic::client::Grpc::new(channel.clone());
    grpc.ready().await.expect("stream client ready");
    let mut request = request;
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
        .expect("ordinary stream opens over bound gRPC");
    response.into_inner()
}

fn intent(client_seq: u64, intent: pb::wire_intent::Intent) -> pb::SubmitIntentRequest {
    pb::SubmitIntentRequest {
        table_id: TABLE_ID.into(),
        envelope: Some(pb::IntentEnvelope {
            table_id: TABLE_ID.into(),
            client_seq,
            intent: Some(pb::WireIntent {
                intent: Some(intent),
            }),
        }),
    }
}

async fn next_frame(
    stream: &mut tonic::Streaming<pb::StreamResponse>,
) -> pb::stream_response::Frame {
    tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("ordinary stream frame timeout")
        .expect("ordinary stream remains open")
        .expect("ordinary stream frame")
        .frame
        .expect("ordinary stream frame payload")
}

async fn next_snapshot(stream: &mut tonic::Streaming<pb::StreamResponse>) -> pb::SnapshotFrame {
    let pb::stream_response::Frame::Snapshot(snapshot) = next_frame(stream).await else {
        panic!("expected authoritative replacement snapshot");
    };
    snapshot
}

async fn run_cli(endpoint: &str, args: &[&str]) {
    let executable = env!("CARGO_BIN_EXE_mtgfr-debug");
    let endpoint = endpoint.to_owned();
    let args = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(executable)
            .args(args)
            .args(["--endpoint", &endpoint])
            .output()
            .expect("spawn mtgfr-debug")
    })
    .await
    .expect("join mtgfr-debug subprocess");
    assert!(output.status.success(), "mtgfr-debug failed: {output:?}");
    assert!(output.stderr.is_empty(), "mtgfr-debug stderr: {output:?}");
}

fn assert_private_zones_hidden(
    view: &pb::VisibleState,
    visible_hand_owner: Option<usize>,
    private_players: &[debug_pb::PlayerInspection],
) {
    let visible_ids = view
        .objects
        .iter()
        .map(|object| object.id)
        .collect::<HashSet<_>>();
    for (player, inspection) in private_players.iter().enumerate() {
        let may_see_hand = visible_hand_owner == Some(player);
        for object_id in &inspection.hand {
            assert_eq!(
                visible_ids.contains(object_id),
                may_see_hand,
                "private hand identity visibility differs for player {player}"
            );
        }
        for object_id in &inspection.library {
            assert!(
                !visible_ids.contains(object_id),
                "library identity for player {player} was disclosed"
            );
        }
    }
}

fn assert_fixture_state(state: &pb::VisibleState, expected_entry_ids: &[u64]) {
    assert_eq!(
        state
            .stack
            .iter()
            .map(|entry| entry.entry_id)
            .collect::<Vec<_>>(),
        expected_entry_ids
    );
    assert_eq!(
        state.stack[..6]
            .iter()
            .map(|entry| entry.print.as_str())
            .collect::<Vec<_>>(),
        PRINTS
    );
    assert_eq!(
        state.stack[..6]
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        NAMES
    );
    assert!(state.stack[..6].iter().all(|entry| entry.source.is_some()));

    let ghost = &state.stack[6];
    assert_eq!(ghost.source, None);
    assert_eq!(ghost.name, "Priority Demonstration");
    assert_eq!(
        ghost.label.as_ref().map(|label| label.key.as_str()),
        Some("Debug no-op stack entry")
    );
    assert_eq!(ghost.print, "91fdb56b-54d5-4272-8319-505ff987fe9b");
    assert_eq!(ghost.card_id, "");
    assert_eq!(
        ghost.printed_sentences,
        ["This debug stack entry resolves without an effect."]
    );
}

async fn start_server(
    state: server::AppState,
) -> (
    String,
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral gRPC port");
    let address = listener.local_addr().expect("bound address");
    let endpoint = format!("http://{address}");
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    let server_task = tokio::spawn(async move {
        server::grpc::serve_with_listener(listener, state, async move {
            let _ = shutdown_rx.changed().await;
        })
        .await
    });
    (endpoint, shutdown_tx, server_task)
}

async fn connect_to_server(
    endpoint: &str,
    server_task: &mut tokio::task::JoinHandle<anyhow::Result<()>>,
) -> Channel {
    let endpoint = Channel::from_shared(endpoint.to_owned()).expect("valid endpoint");
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            result = &mut *server_task => {
                panic!("gRPC server exited before accepting connections: {result:?}");
            }
            () = &mut deadline => panic!("gRPC server startup deadline elapsed"),
            result = endpoint.connect() => {
                match result {
                    Ok(channel) => return channel,
                    Err(_) => {
                        tokio::select! {
                            result = &mut *server_task => {
                                panic!("gRPC server exited before accepting connections: {result:?}");
                            }
                            () = &mut deadline => {
                                panic!("gRPC server startup deadline elapsed");
                            }
                            () = tokio::time::sleep(Duration::from_millis(20)) => {}
                        }
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn real_debug_stack_fixture_reaches_private_ordinary_streams_and_restores_after_resolution() {
    let state = server::AppState::new(
        server::db::connect("sqlite::memory:")
            .await
            .expect("sqlite test database"),
        Arc::new(settings()),
    );
    let (endpoint, shutdown_tx, mut server_task) = start_server(state).await;
    let channel = connect_to_server(&endpoint, &mut server_task).await;

    let (host_id, host_token) = signup(&channel, "stack-owner@x.c", "owner").await;
    let (opponent_id, opponent_token) = signup(&channel, "stack-opponent@x.c", "opponent").await;
    let (_spectator_id, spectator_token) =
        signup(&channel, "stack-spectator@x.c", "spectator").await;

    seed(
        &channel,
        authed(
            pb::SeedRequest {
                table_id: TABLE_ID.into(),
                host_user_id: host_id,
                seats: vec![
                    pb::SeedSeat {
                        user_id: host_id,
                        username: "owner".into(),
                        deck_id: -1,
                        gravatar_hash: String::new(),
                    },
                    pb::SeedSeat {
                        user_id: opponent_id,
                        username: "opponent".into(),
                        deck_id: -1,
                        gravatar_hash: String::new(),
                    },
                ],
            },
            &host_token,
        ),
    )
    .await;
    for (player, token) in [(0, &host_token), (1, &opponent_token)] {
        let accepted = submit(
            &channel,
            authed(
                intent(
                    0,
                    pb::wire_intent::Intent::KeepHand(pb::WireIntentKeepHand { player }),
                ),
                token,
            ),
        )
        .await;
        assert!(accepted.accepted);
    }

    let mut owner_stream = open_stream(
        &channel,
        authed(
            pb::StreamRequest {
                table_id: TABLE_ID.into(),
            },
            &host_token,
        ),
    )
    .await;
    let mut opponent_stream = open_stream(
        &channel,
        authed(
            pb::StreamRequest {
                table_id: TABLE_ID.into(),
            },
            &opponent_token,
        ),
    )
    .await;
    let mut spectator_stream = open_stream(
        &channel,
        authed(
            pb::StreamRequest {
                table_id: TABLE_ID.into(),
            },
            &spectator_token,
        ),
    )
    .await;
    let owner_opening_snapshot = next_snapshot(&mut owner_stream).await;
    let opponent_opening_snapshot = next_snapshot(&mut opponent_stream).await;
    let spectator_opening_snapshot = next_snapshot(&mut spectator_stream).await;
    assert_eq!(owner_opening_snapshot.seq, opponent_opening_snapshot.seq);
    assert_eq!(owner_opening_snapshot.seq, spectator_opening_snapshot.seq);
    let opening_seq = owner_opening_snapshot.seq;
    let owner_opening = owner_opening_snapshot.state.expect("owner opening state");
    let opponent_opening = opponent_opening_snapshot
        .state
        .expect("opponent opening state");
    let spectator_opening = spectator_opening_snapshot
        .state
        .expect("spectator opening state");

    let mut debug = debug_pb::debug_service_client::DebugServiceClient::new(channel.clone());
    let before_fixture = debug
        .inspect_table(debug_pb::InspectTableRequest {
            table_id: TABLE_ID.into(),
        })
        .await
        .expect("inspect seeded table")
        .into_inner();
    let game_before_fixture = before_fixture.game.expect("seeded game inspection");
    assert_private_zones_hidden(&owner_opening, Some(0), &game_before_fixture.players);
    assert_private_zones_hidden(&opponent_opening, Some(1), &game_before_fixture.players);
    assert_private_zones_hidden(&spectator_opening, None, &game_before_fixture.players);
    let first_entry_id = game_before_fixture
        .next_stack_entry_id
        .expect("stack frontier");
    let expected_entry_ids = (first_entry_id..first_entry_id + 7).collect::<Vec<_>>();

    run_cli(&endpoint, &["stack-fixture", TABLE_ID]).await;
    let owner_fixture = next_snapshot(&mut owner_stream).await;
    let opponent_fixture = next_snapshot(&mut opponent_stream).await;
    let spectator_fixture = next_snapshot(&mut spectator_stream).await;
    assert_eq!(owner_fixture.seq, opponent_fixture.seq);
    assert_eq!(owner_fixture.seq, spectator_fixture.seq);
    assert_eq!(owner_fixture.seq, opening_seq + 1);
    let owner_fixture_state = owner_fixture.state.expect("owner fixture state");
    let opponent_fixture_state = opponent_fixture.state.expect("opponent fixture state");
    let spectator_fixture_state = spectator_fixture.state.expect("spectator fixture state");
    for fixture_state in [
        &owner_fixture_state,
        &opponent_fixture_state,
        &spectator_fixture_state,
    ] {
        assert_fixture_state(fixture_state, &expected_entry_ids);
    }
    assert_private_zones_hidden(&owner_fixture_state, Some(0), &game_before_fixture.players);
    assert_private_zones_hidden(
        &opponent_fixture_state,
        Some(1),
        &game_before_fixture.players,
    );
    assert_private_zones_hidden(&spectator_fixture_state, None, &game_before_fixture.players);

    let after_fixture = debug
        .inspect_table(debug_pb::InspectTableRequest {
            table_id: TABLE_ID.into(),
        })
        .await
        .expect("inspect fixture")
        .into_inner();
    let inspected_stack = &after_fixture.game.as_ref().expect("fixture game").stack;
    assert_eq!(
        inspected_stack
            .iter()
            .map(|entry| entry.entry_id)
            .collect::<Vec<_>>(),
        expected_entry_ids
    );
    assert_eq!(
        inspected_stack
            .iter()
            .map(|entry| entry.position_from_bottom)
            .collect::<Vec<_>>(),
        (0..7).collect::<Vec<_>>()
    );
    let ghost = inspected_stack.last().expect("ghost on top");
    assert_eq!(ghost.source_object_id, None);
    let ghost_metadata = ghost
        .public_ghost
        .as_ref()
        .expect("explicit ghost metadata");
    assert_eq!(ghost_metadata.name, "Priority Demonstration");
    assert_eq!(ghost_metadata.label, "Debug no-op stack entry");

    run_cli(&endpoint, &["checkpoint", TABLE_ID, "fixture-ready"]).await;
    for (player, token) in [(0, &host_token), (1, &opponent_token)] {
        let accepted = submit(
            &channel,
            authed(
                intent(
                    1,
                    pb::wire_intent::Intent::PassPriority(pb::WireIntentPassPriority { player }),
                ),
                token,
            ),
        )
        .await;
        assert!(accepted.accepted);
    }
    let after_passes = debug
        .inspect_table(debug_pb::InspectTableRequest {
            table_id: TABLE_ID.into(),
        })
        .await
        .expect("inspect resolved ghost")
        .into_inner();
    assert_eq!(after_passes.game.expect("game after ghost").stack.len(), 6);

    // Drain ordinary deltas from both passes before restore's replacement snapshot.
    for stream in [
        &mut owner_stream,
        &mut opponent_stream,
        &mut spectator_stream,
    ] {
        for _ in 0..2 {
            let frame = next_frame(stream).await;
            assert!(matches!(frame, pb::stream_response::Frame::Delta(_)));
        }
    }

    run_cli(&endpoint, &["restore", TABLE_ID, "fixture-ready"]).await;
    let restored_owner = next_snapshot(&mut owner_stream).await;
    let restored_opponent = next_snapshot(&mut opponent_stream).await;
    let restored_spectator = next_snapshot(&mut spectator_stream).await;
    assert!(restored_owner.seq > owner_fixture.seq);
    for snapshot in [restored_owner, restored_opponent, restored_spectator] {
        let restored = snapshot.state.expect("restored fixture state");
        assert_fixture_state(&restored, &expected_entry_ids);
    }

    drop(owner_stream);
    drop(opponent_stream);
    drop(spectator_stream);
    drop(debug);
    drop(channel);
    let _ = shutdown_tx.send(true);
    tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .expect("server shutdown timeout")
        .expect("server task join")
        .expect("server shutdown");
}
