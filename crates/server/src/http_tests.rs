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
        test_app_with_state().await.0
    }

    async fn test_app_with_state() -> (axum::Router, AppState) {
        let db = db::connect("sqlite::memory:").await.expect("sqlite");
        let state = AppState::for_test(db);
        (app(state.clone()), state)
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

    /// Drive two fresh users through signup → deck save → seed against the real router.
    /// Returns `(host_cookie, guest_cookie, table_id)` for a running 2-player game.
    async fn start_two_player_game(router: &axum::Router) -> (String, String, String) {
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

        let host_me = body_json(
            router
                .clone()
                .oneshot(get("/auth/me/v1", &host))
                .await
                .unwrap(),
        )
        .await;
        let guest_me = body_json(
            router
                .clone()
                .oneshot(get("/auth/me/v1", &guest))
                .await
                .unwrap(),
        )
        .await;
        let host_uid = host_me["id"].as_i64().expect("host id");
        let guest_uid = guest_me["id"].as_i64().expect("guest id");
        let host_name = host_me["username"].as_str().unwrap().to_string();
        let guest_name = guest_me["username"].as_str().unwrap().to_string();

        let deck_body = json!({
            "name": "deck",
            "commander": "ae6f21a2-e6b6-4793-8343-e27310c0bea1",
            "commander_print": "45c4c3b3-be18-4d74-99d8-f137498673d7",
            "cards": [
                {"id": "60ba93eb-39e6-4af2-9c66-cd38f72daff2", "count": 1, "print": "9c9ac1bc-cdf3-4fa6-8319-a7ea164e9e47"},
                {"id": "51d9564b-44fc-4de1-9119-09d7b4089378", "count": 1, "print": "3c0f5411-1940-410f-96ce-6f92513f753a"},
                {"id": "4b7ac066-e5c7-43e6-9e7e-2739b24a905d", "count": 1, "print": "b8c5e74c-96e7-4a1f-93b7-14d776fe4b2d"},
                {"id": "e3886fe8-9b76-4613-8891-4ec74657c087", "count": 1, "print": "17d154d3-7ae5-43ff-9978-d974285e2c89"},
                {"id": "a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d", "count": 1, "print": "b23900fb-efe9-43ab-9f67-4545dd01fb9c"},
                {"id": "9880ba09-d5b8-4675-bfb4-2161d86d2d41", "count": 1, "print": "89db7256-3bd0-4c1d-9c6f-de81f7d3c1a2"},
                {"id": "bc71ebf6-2056-41f7-be35-b2e5c34afa99", "count": 93, "print": "5f9b6584-ad27-410d-b6f1-c25c91630aea"}
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

        let table = "HTTP01".to_string();
        let seeded = router
            .clone()
            .oneshot(post(
                "/tables/seed/v1",
                Some(&host),
                json!({
                    "table_id": table,
                    "host_user_id": host_uid,
                    "seats": [
                        {"user_id": host_uid, "username": host_name, "deck_id": host_deck},
                        {"user_id": guest_uid, "username": guest_name, "deck_id": guest_deck}
                    ]
                }),
            ))
            .await
            .unwrap();
        assert_eq!(seeded.status(), StatusCode::OK, "seed starts the game");
        let body = body_json(seeded).await;
        assert_eq!(body["table_id"], table);

        (host, guest, table)
    }

    #[tokio::test]
    async fn signup_build_a_legal_deck_and_seed_a_game_over_http() {
        let router = test_app().await;
        let _ = start_two_player_game(&router).await;
    }

    /// Session cookie still honors Secure / Domain from settings (affinity cookie is gone).
    #[tokio::test]
    async fn cookie_secure_and_domain_flags_shape_the_session_cookie() {
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
            session_set_cookie.contains("Domain=example.com"),
            "the session cookie is shared across subdomains: {session_set_cookie}"
        );
    }

    /// Deploy PRD: once `admin_token` is configured, `/health/drain` rejects requests without a
    /// matching credential and accepts either `Authorization: Bearer` or `X-Admin-Token`.
    #[tokio::test]
    async fn health_drain_requires_the_configured_admin_token_once_set() {
        let settings = Settings {
            admin_token: "topsecret".to_string(),
            ..settings::for_test()
        };
        let router = test_app_with_settings(settings).await;

        let no_credential = router
            .clone()
            .oneshot(get("/health/drain", ""))
            .await
            .unwrap();
        assert_eq!(no_credential.status(), StatusCode::UNAUTHORIZED);

        let wrong_bearer = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health/drain")
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
                    .method("GET")
                    .uri("/health/drain")
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
                    .method("GET")
                    .uri("/health/drain")
                    .header("x-admin-token", "topsecret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(right_plain_header.status(), StatusCode::OK);
    }

    /// Deploy PRD §Drain: a draining instance still serves games it already owns (stream/intent),
    /// and rejects *new* seeds with 503.
    #[tokio::test]
    async fn draining_still_serves_a_table_it_already_owns() {
        use std::sync::atomic::Ordering;

        let (router, state) = test_app_with_state().await;
        let (host, _guest, table) = start_two_player_game(&router).await;

        state.draining.store(true, Ordering::Relaxed);

        let stream = router
            .clone()
            .oneshot(get(&format!("/tables/{table}/stream/v1"), &host))
            .await
            .unwrap();
        assert_eq!(
            stream.status(),
            StatusCode::OK,
            "stream for an owned table still works while draining"
        );

        let host_me = body_json(
            router
                .clone()
                .oneshot(get("/auth/me/v1", &host))
                .await
                .unwrap(),
        )
        .await;
        let rejected = router
            .oneshot(post(
                "/tables/seed/v1",
                Some(&host),
                json!({
                    "table_id": "NEWONE",
                    "host_user_id": host_me["id"],
                    "seats": [
                        {"user_id": host_me["id"], "username": "a", "deck_id": 1},
                        {"user_id": host_me["id"], "username": "b", "deck_id": 1}
                    ]
                }),
            ))
            .await
            .unwrap();
        assert_eq!(
            rejected.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "a new seed is rejected while draining"
        );
    }

    /// `GET /health/drain` reflects tables; flipping the drain flag (as SIGTERM would) makes
    /// `POST /tables/seed/v1` return 503.
    #[tokio::test]
    async fn health_drain_reflects_active_tables_and_drain_flag() {
        use std::sync::atomic::Ordering;

        let (router, state) = test_app_with_state().await;
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

        state.draining.store(true, Ordering::Relaxed);

        let after = body_json(
            router
                .clone()
                .oneshot(get("/health/drain", ""))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(after["draining"], true, "the flag stuck: {after}");
        assert_eq!(after["active_tables"], 1);

        let host_me = body_json(
            router
                .clone()
                .oneshot(get("/auth/me/v1", &host))
                .await
                .unwrap(),
        )
        .await;
        let rejected = router
            .clone()
            .oneshot(post(
                "/tables/seed/v1",
                Some(&host),
                json!({
                    "table_id": "DRAINX",
                    "host_user_id": host_me["id"],
                    "seats": [
                        {"user_id": host_me["id"], "username": "a", "deck_id": 1},
                        {"user_id": host_me["id"], "username": "b", "deck_id": 1}
                    ]
                }),
            ))
            .await
            .unwrap();
        assert_eq!(
            rejected.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "a draining instance refuses new seeds"
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
                    &format!("/tables/{table}/intent/v1"),
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
                    &format!("/tables/{table}/intent/v1"),
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
