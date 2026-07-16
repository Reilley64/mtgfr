#[cfg(test)]
mod tests {
    //! End-to-end HTTP tests through the real router: the session cookie must survive a full
    //! request/response round-trip (Set-Cookie on signup → Cookie honored on the next request),
    //! and the deck → table → start flow must run over HTTP. Uses in-memory SQLite for the store.

    use std::sync::Arc;
    use std::time::Duration;

    use crate::settings::{self, Settings};
    use crate::{AppState, app, db};
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt; // for `Body::frame`
    use serde_json::{Value, json};
    use tower::ServiceExt; // for `oneshot`

    async fn test_app() -> axum::Router {
        let db = db::connect("sqlite::memory:").await.expect("sqlite");
        app(AppState::for_test(db))
    }

    /// `test_app()`, with `settings` in place of the plain test defaults — for the handful of
    /// tests that exercise deploy-flag-dependent behavior (cookie `Secure`/`Domain`, the admin
    /// token) rather than the happy-path flows every other test in this module covers.
    async fn test_app_with_settings(settings: Settings) -> axum::Router {
        let db = db::connect("sqlite::memory:").await.expect("sqlite");
        app(AppState::new(db, Arc::new(settings)))
    }

    /// The full raw `Set-Cookie` header for the cookie named `name`, so a test can inspect its
    /// attributes (`Secure`, `Domain`) rather than just its value.
    fn find_set_cookie<'a>(res: &'a axum::response::Response, name: &str) -> &'a str {
        res.headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap())
            .find(|c| c.starts_with(&format!("{name}=")))
            .unwrap_or_else(|| panic!("no Set-Cookie header for {name:?}"))
    }

    fn post(uri: &str, cookie: Option<&str>, body: Value) -> Request<Body> {
        let mut b = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(c) = cookie {
            b = b.header("cookie", c);
        }
        b.body(Body::from(body.to_string())).unwrap()
    }

    fn get(uri: &str, cookie: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(uri)
            .header("cookie", cookie)
            .body(Body::empty())
            .unwrap()
    }

    async fn body_json(res: axum::response::Response) -> Value {
        let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// The `session=<token>` pair from a response's Set-Cookie, ready to send back as a Cookie header.
    fn session_cookie(res: &axum::response::Response) -> String {
        let set = res
            .headers()
            .get("set-cookie")
            .expect("signup sets a session cookie")
            .to_str()
            .unwrap();
        set.split(';').next().unwrap().to_string()
    }

    #[tokio::test]
    async fn a_signup_cookie_authenticates_the_next_request() {
        let router = test_app().await;

        let signup = router
            .clone()
            .oneshot(post(
                "/auth/signup/v1",
                None,
                json!({"email": "a@b.c", "password": "pw", "username": "alice"}),
            ))
            .await
            .unwrap();
        assert_eq!(signup.status(), StatusCode::OK);
        let cookie = session_cookie(&signup);

        // /auth/me with no cookie is 401; with the cookie it returns the account.
        let anon = router
            .clone()
            .oneshot(get("/auth/me/v1", "nonsense=1"))
            .await
            .unwrap();
        assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

        let me = router.oneshot(get("/auth/me/v1", &cookie)).await.unwrap();
        assert_eq!(me.status(), StatusCode::OK);
        assert_eq!(body_json(me).await["email"], "a@b.c");
    }

    /// Drive two fresh users through signup → deck save → create/join/ready → start against the
    /// real router, asserting each hop, and return `(host_cookie, guest_cookie, table_id)` for a
    /// running 2-player game. The host claims the first open seat (seat 0, the starting active
    /// player); the guest gets seat 1.
    async fn start_two_player_game(router: &axum::Router) -> (String, String, String) {
        // Two users sign up; keep each one's cookie.
        let host = session_cookie(
            &router
                .clone()
                .oneshot(post(
                    "/auth/signup/v1",
                    None,
                    json!({"email": "h@x.c", "password": "p", "username": "host"}),
                ))
                .await
                .unwrap(),
        );
        let guest = session_cookie(
            &router
                .clone()
                .oneshot(post(
                    "/auth/signup/v1",
                    None,
                    json!({"email": "g@x.c", "password": "p", "username": "guest"}),
                ))
                .await
                .unwrap(),
        );

        // A legal Tajic deck: the pool's RW nonbasics + 93 Plains.
        let deck_body = json!({
            "name": "deck",
            "commander": "Tajic, Legion's Edge",
            "cards": [
                {"name": "Savannah Lions", "count": 1},
                {"name": "Goblin Guide", "count": 1},
                {"name": "Serra Angel", "count": 1},
                {"name": "Glorious Anthem", "count": 1},
                {"name": "Shock", "count": 1},
                {"name": "Brute Force", "count": 1},
                {"name": "Plains", "count": 93}
            ]
        });
        let save = |cookie: &str| {
            router
                .clone()
                .oneshot(post("/decks/v1", Some(cookie), deck_body.clone()))
        };
        let host_deck = body_json(save(&host).await.unwrap()).await["id"]
            .as_i64()
            .expect("host deck id");
        let guest_deck = body_json(save(&guest).await.unwrap()).await["id"]
            .as_i64()
            .expect("guest deck id");

        // Create a table, both join with their deck, both ready, host starts.
        let table = body_json(
            router
                .clone()
                .oneshot(post("/tables/v1", Some(&host), json!({})))
                .await
                .unwrap(),
        )
        .await["table_id"]
            .as_str()
            .unwrap()
            .to_string();

        for (cookie, deck) in [(&host, host_deck), (&guest, guest_deck)] {
            let view = body_json(
                router
                    .clone()
                    .oneshot(post(
                        "/tables/join/v1",
                        Some(cookie),
                        json!({"table_id": table, "deck_id": deck}),
                    ))
                    .await
                    .unwrap(),
            )
            .await;
            assert!(view["you"].is_number(), "join seats the user: {view}");
        }
        for cookie in [&host, &guest] {
            let _ = router
                .clone()
                .oneshot(post(
                    "/tables/ready/v1",
                    Some(cookie),
                    json!({"table_id": table, "ready": true}),
                ))
                .await
                .unwrap();
        }
        let started = body_json(
            router
                .clone()
                .oneshot(post(
                    "/tables/start/v1",
                    Some(&host),
                    json!({"table_id": table}),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            started["started"], true,
            "the host started the game: {started}"
        );
        assert!(started["error"].is_null());

        (host, guest, table)
    }

    #[tokio::test]
    async fn signup_build_a_legal_deck_and_start_a_game_over_http() {
        let router = test_app().await;
        let _ = start_two_player_game(&router).await;
    }

    /// `POST /tables/v1`'s success response also sets the affinity cookie (`mtgfr-instance`) —
    /// a later hop for the same table should have a chance to land back on this instance.
    #[tokio::test]
    async fn create_table_sets_the_affinity_cookie() {
        let router = test_app().await;
        let host = session_cookie(
            &router
                .clone()
                .oneshot(post(
                    "/auth/signup/v1",
                    None,
                    json!({"email": "aff@x.c", "password": "p", "username": "aff"}),
                ))
                .await
                .unwrap(),
        );

        let created = router
            .oneshot(post("/tables/v1", Some(&host), json!({})))
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);
        let set_cookies: Vec<String> = created
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();
        assert!(
            set_cookies.iter().any(|c| c.starts_with("mtgfr-instance=")),
            "the affinity cookie is set on create: {set_cookies:?}"
        );
    }

    /// Deploy PRD §Configuration: `cookie_secure`/`cookie_domain` shape the auth session cookie so
    /// it's shared across `edh.example.com` and `api.edh.example.com` in prod, but the affinity
    /// cookie (`lobby.rs`) stays host-only regardless — it's only meaningful to whichever instance
    /// set it, and `instance_id` (not `cookie_domain`) supplies its value.
    #[tokio::test]
    async fn cookie_secure_and_domain_flags_shape_the_session_and_affinity_cookies() {
        let settings = Settings {
            cookie_secure: true,
            cookie_domain: ".example.com".to_string(),
            instance_id: "edh-api".to_string(),
            ..settings::for_test()
        };
        let router = test_app_with_settings(settings).await;

        let signup = router
            .clone()
            .oneshot(post(
                "/auth/signup/v1",
                None,
                json!({"email": "cookie@x.c", "password": "p", "username": "cookie"}),
            ))
            .await
            .unwrap();
        assert_eq!(signup.status(), StatusCode::OK);
        let session_set_cookie = find_set_cookie(&signup, "session").to_string();
        assert!(
            session_set_cookie.contains("Secure"),
            "the session cookie is Secure: {session_set_cookie}"
        );
        assert!(
            // The `cookie` crate serializes `Domain` without a leading dot (RFC 6265 §5.2.3
            // treats it as a legacy no-op) even though `cookie_domain` is configured with one.
            session_set_cookie.contains("Domain=example.com"),
            "the session cookie is shared across subdomains: {session_set_cookie}"
        );
        let host = session_cookie(&signup);

        let created = router
            .oneshot(post("/tables/v1", Some(&host), json!({})))
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);
        let affinity_set_cookie = find_set_cookie(&created, "mtgfr-instance").to_string();
        assert!(
            affinity_set_cookie.contains("Secure"),
            "the affinity cookie is Secure too: {affinity_set_cookie}"
        );
        assert!(
            !affinity_set_cookie.contains("Domain"),
            "the affinity cookie stays host-only, unlike the session cookie: {affinity_set_cookie}"
        );
        assert_eq!(
            affinity_set_cookie.split(';').next().unwrap(),
            "mtgfr-instance=edh-api",
            "the affinity cookie's value is this instance's id: {affinity_set_cookie}"
        );
    }

    /// Deploy PRD §Admin / drain endpoints: once `admin_token` is configured, `/admin/drain`
    /// rejects requests without a matching credential and accepts either the `Authorization:
    /// Bearer` or the plain `X-Admin-Token` form.
    #[tokio::test]
    async fn admin_drain_requires_the_configured_admin_token_once_set() {
        let settings = Settings {
            admin_token: "topsecret".to_string(),
            ..settings::for_test()
        };
        let router = test_app_with_settings(settings).await;

        let no_credential = router
            .clone()
            .oneshot(post("/admin/drain", None, json!({})))
            .await
            .unwrap();
        assert_eq!(no_credential.status(), StatusCode::UNAUTHORIZED);

        let wrong_bearer = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/drain")
                    .header("authorization", "Bearer nope")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wrong_bearer.status(), StatusCode::UNAUTHORIZED);

        let right_bearer = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/drain")
                    .header("authorization", "Bearer topsecret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(right_bearer.status(), StatusCode::OK);

        let right_plain_header = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/drain")
                    .header("x-admin-token", "topsecret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(right_plain_header.status(), StatusCode::OK);
    }

    /// Deploy PRD §Drain: a draining instance still serves the tables it already owns — lobby
    /// polling, joining, and readying up all keep working — it only rejects *new* tables
    /// (`POST /tables/v1`, 503).
    #[tokio::test]
    async fn draining_still_serves_a_table_it_already_owns() {
        let router = test_app().await;
        let host = session_cookie(
            &router
                .clone()
                .oneshot(post(
                    "/auth/signup/v1",
                    None,
                    json!({"email": "owner@x.c", "password": "p", "username": "owner"}),
                ))
                .await
                .unwrap(),
        );
        let guest = session_cookie(
            &router
                .clone()
                .oneshot(post(
                    "/auth/signup/v1",
                    None,
                    json!({"email": "guest2@x.c", "password": "p", "username": "guest2"}),
                ))
                .await
                .unwrap(),
        );
        let guest_deck = body_json(
            router
                .clone()
                .oneshot(post(
                    "/decks/v1",
                    Some(&guest),
                    json!({
                        "name": "deck",
                        "commander": "Tajic, Legion's Edge",
                        "cards": [
                            {"name": "Savannah Lions", "count": 1},
                            {"name": "Goblin Guide", "count": 1},
                            {"name": "Serra Angel", "count": 1},
                            {"name": "Glorious Anthem", "count": 1},
                            {"name": "Shock", "count": 1},
                            {"name": "Brute Force", "count": 1},
                            {"name": "Plains", "count": 93}
                        ]
                    }),
                ))
                .await
                .unwrap(),
        )
        .await["id"]
            .as_i64()
            .expect("guest deck id");

        let table = body_json(
            router
                .clone()
                .oneshot(post("/tables/v1", Some(&host), json!({})))
                .await
                .unwrap(),
        )
        .await["table_id"]
            .as_str()
            .unwrap()
            .to_string();

        let drained = router
            .clone()
            .oneshot(post("/admin/drain", None, json!({})))
            .await
            .unwrap();
        assert_eq!(drained.status(), StatusCode::OK);

        let lobby = router
            .clone()
            .oneshot(get(&format!("/tables/{table}/lobby/v1"), &host))
            .await
            .unwrap();
        assert_eq!(
            lobby.status(),
            StatusCode::OK,
            "lobby polling for an owned table still works while draining"
        );

        let joined = router
            .clone()
            .oneshot(post(
                "/tables/join/v1",
                Some(&guest),
                json!({"table_id": table, "deck_id": guest_deck}),
            ))
            .await
            .unwrap();
        assert_eq!(
            joined.status(),
            StatusCode::OK,
            "joining an owned table still works while draining"
        );

        let readied = router
            .clone()
            .oneshot(post(
                "/tables/ready/v1",
                Some(&guest),
                json!({"table_id": table, "ready": true}),
            ))
            .await
            .unwrap();
        assert_eq!(
            readied.status(),
            StatusCode::OK,
            "readying up an owned table still works while draining"
        );

        let rejected = router
            .oneshot(post("/tables/v1", Some(&host), json!({})))
            .await
            .unwrap();
        assert_eq!(
            rejected.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "a new table is still rejected while draining"
        );
    }

    /// `POST /admin/drain` flips the live drain flag (no restart), and `GET /health/drain`
    /// reflects both the flag and how many tables the instance still owns. Once draining,
    /// `POST /tables/v1` is rejected with 503 — new tables must land on an instance that will
    /// stick around to run them.
    #[tokio::test]
    async fn admin_drain_flips_the_flag_and_health_drain_reflects_active_tables() {
        let router = test_app().await;
        let (host, _guest, _table) = start_two_player_game(&router).await;

        let before = body_json(
            router
                .clone()
                .oneshot(get("/health/drain", ""))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(before["draining"], false);
        assert_eq!(before["active_tables"], 1, "the started game is active");

        let drained = body_json(
            router
                .clone()
                .oneshot(post("/admin/drain", None, json!({})))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(drained["draining"], true);
        assert_eq!(drained["active_tables"], 1);

        let after = body_json(
            router
                .clone()
                .oneshot(get("/health/drain", ""))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(after["draining"], true, "the flag stuck: {after}");

        let rejected = router
            .clone()
            .oneshot(post("/tables/v1", Some(&host), json!({})))
            .await
            .unwrap();
        assert_eq!(
            rejected.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "a draining instance refuses new tables"
        );
    }

    /// Pull the next SSE `StreamFrame` off a live `/stream` body: accumulate bytes into `buf`, split
    /// on newlines, and parse the first `data:` line's JSON. Blank event separators and `:` keepalive
    /// comments are skipped. Each `body.frame()` await is bounded so a stalled stream fails the test
    /// instead of hanging.
    async fn next_stream_frame(body: &mut Body, buf: &mut Vec<u8>) -> Value {
        loop {
            if let Some(nl) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=nl).collect();
                let line = &line[..line.len() - 1]; // drop the trailing '\n'
                // SSE: only `data:` lines carry a frame; skip blanks, `:` keepalives, and other fields.
                let Some(data) = line.strip_prefix(b"data:") else {
                    continue;
                };
                let data = data.strip_prefix(b" ").unwrap_or(data); // an optional single leading space
                return serde_json::from_slice(data)
                    .expect("a stream data line is StreamFrame JSON");
            }
            let frame = tokio::time::timeout(Duration::from_secs(30), body.frame())
                .await
                .expect("a stream frame arrives within the timeout")
                .expect("the stream has not ended")
                .expect("stream body is not an error");
            if let Ok(data) = frame.into_data() {
                buf.extend_from_slice(&data);
            }
        }
    }

    /// A quiet game (no intents) must still prove the connection is alive: the server emits a
    /// `Heartbeat` data frame every few seconds. `start_paused` advances the virtual clock to the
    /// heartbeat interval automatically once the task parks, so this is deterministic and fast — no
    /// real sleep. Without it a silently-killed backend would leave the client "serenely healthy".
    #[tokio::test(start_paused = true)]
    async fn a_quiet_stream_emits_periodic_heartbeat_frames() {
        let router = test_app().await;
        let (host, _guest, table) = start_two_player_game(&router).await;

        let stream = router
            .clone()
            .oneshot(get(&format!("/tables/{table}/stream/v1"), &host))
            .await
            .unwrap();
        assert_eq!(stream.status(), StatusCode::OK);
        let mut body = stream.into_body();
        let mut buf = Vec::new();

        let opening = next_stream_frame(&mut body, &mut buf).await;
        assert_eq!(opening["frame"], "snapshot");
        // No intent is submitted, so the only thing that can arrive next is a liveness heartbeat.
        let beat = next_stream_frame(&mut body, &mut buf).await;
        assert_eq!(
            beat["frame"], "heartbeat",
            "a quiet stream still beats: {beat}"
        );
    }

    #[tokio::test]
    async fn an_intent_over_http_broadcasts_a_delta_frame_on_the_open_stream() {
        let router = test_app().await;
        let (host, guest, table) = start_two_player_game(&router).await;

        // Ask each seat's opening stream snapshot who may act: a `VisibleState` reports `viewer`
        // (that seat) and `priority` (the seat that holds priority). The seat whose own view holds
        // priority is the one who can legally submit — at game start that's the active player (seat 0).
        let mut actor: Option<(&str, &str, u64)> = None; // (actor cookie, other cookie, seat)
        for (cookie, other) in [(&host, &guest), (&guest, &host)] {
            let mut body = router
                .clone()
                .oneshot(get(&format!("/tables/{table}/stream/v1"), cookie))
                .await
                .unwrap()
                .into_body();
            let snap = next_stream_frame(&mut body, &mut Vec::new()).await;
            assert_eq!(snap["frame"], "snapshot");
            let state = &snap["state"];
            let seat = state["viewer"].as_u64().unwrap();
            if state["priority"].as_u64() == Some(seat) {
                assert_eq!(
                    state["can_act"], true,
                    "the priority holder can act at start"
                );
                actor = Some((cookie, other, seat));
            }
        }
        let (actor_cookie, _other_cookie, actor_seat) = actor.expect("some seat holds priority");

        // Open the actor's stream BEFORE submitting, so the delta can't slip through before we watch.
        // `oneshot` hands back the response with a live streaming body we keep polling.
        let stream = router
            .clone()
            .oneshot(get(&format!("/tables/{table}/stream/v1"), actor_cookie))
            .await
            .unwrap();
        assert_eq!(stream.status(), StatusCode::OK);
        let mut body = stream.into_body();
        let mut buf = Vec::new();

        // First line is the opening snapshot at the current seq.
        let opening = next_stream_frame(&mut body, &mut buf).await;
        assert_eq!(opening["frame"], "snapshot");
        let snapshot_seq = opening["seq"].as_u64().unwrap();

        // The priority holder passing is always legal. Submit it over HTTP with the actor's cookie.
        let ack = body_json(
            router
                .clone()
                .oneshot(post(
                    "/intent/v1",
                    Some(actor_cookie),
                    json!({
                        "table_id": table,
                        "client_seq": 0,
                        "intent": {"kind": "pass_priority", "player": actor_seat},
                    }),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(ack["accepted"], true, "the pass is accepted: {ack}");

        // The pass (and every auto-pass it folds in) arrives as ONE delta frame on the open stream.
        let delta = next_stream_frame(&mut body, &mut buf).await;
        assert_eq!(delta["frame"], "delta", "next frame is a delta: {delta}");
        assert!(
            delta["seq"].as_u64().unwrap() > snapshot_seq,
            "the delta seq advanced past the snapshot: {delta}"
        );
        assert!(
            delta["events"].as_array().is_some_and(|e| !e.is_empty()),
            "the delta carries the pass + auto-passes as events: {delta}"
        );
        assert!(
            delta["state"].is_object(),
            "the delta is self-sufficient (carries full render state): {delta}"
        );

        // Negative half: a signed-in user without a seat cannot submit intents.
        let outsider = session_cookie(
            &router
                .clone()
                .oneshot(post(
                    "/auth/signup/v1",
                    None,
                    json!({"email": "o2@x.c", "password": "p", "username": "o2"}),
                ))
                .await
                .unwrap(),
        );
        let rejected = body_json(
            router
                .clone()
                .oneshot(post(
                    "/intent/v1",
                    Some(&outsider),
                    json!({
                        "table_id": table,
                        "client_seq": 1,
                        "intent": {"kind": "pass_priority", "player": actor_seat},
                    }),
                ))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(rejected["accepted"], false, "outsider is rejected");
        assert_eq!(
            rejected["reason"], "NotSeated",
            "rejection names the seat check: {rejected}"
        );

        // No delta should follow the rejected intent — a bounded wait must time out.
        let none = tokio::time::timeout(
            Duration::from_secs(1),
            next_stream_frame(&mut body, &mut buf),
        )
        .await;
        assert!(none.is_err(), "a rejected intent broadcasts no frame");
    }

    /// C1/6.3: the stream resolves the viewer from the session, never from the client. A signed-in
    /// user with no seat at the table gets the public spectator projection (no private hand view),
    /// and an unknown table is 404. (This rule was previously pinned via the removed `/snapshot/v1`.)
    #[tokio::test]
    async fn the_stream_spectates_outsiders_and_404s_unknown_tables() {
        let router = test_app().await;
        let (host, _guest, table) = start_two_player_game(&router).await;

        // A third signed-up user who holds no seat at the table.
        let outsider = session_cookie(
            &router
                .clone()
                .oneshot(post(
                    "/auth/signup/v1",
                    None,
                    json!({"email": "o@x.c", "password": "p", "username": "outsider"}),
                ))
                .await
                .unwrap(),
        );

        let stream = router
            .clone()
            .oneshot(get(&format!("/tables/{table}/stream/v1"), &outsider))
            .await
            .unwrap();
        assert_eq!(stream.status(), StatusCode::OK);
        let snap = next_stream_frame(&mut stream.into_body(), &mut Vec::new()).await;
        assert_eq!(snap["frame"], "snapshot");
        assert_eq!(
            snap["state"]["viewer"],
            u64::from(schema::SPECTATOR_VIEWER),
            "an outsider watches as a spectator, with no seat"
        );

        let missing = router
            .oneshot(get("/tables/nope/stream/v1", &host))
            .await
            .unwrap();
        assert_eq!(
            missing.status(),
            StatusCode::NOT_FOUND,
            "unknown table is 404"
        );
    }
}
