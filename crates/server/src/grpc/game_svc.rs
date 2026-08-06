//! `mtgfr.v1.GameService` — intents, yield/dwell chrome, and the per-viewer delta stream.
//! `Stream` reuses [`crate::stream::subscribe`] (same heartbeat, seq-dedup, redaction).
#![allow(clippy::result_large_err)] // `tonic::Status` is a large `Err` by design; see auth_ctx.rs.

use std::pin::Pin;

use schema::StreamFrame;
use tonic::{Request, Response, Status};

use crate::AppState;
use crate::game_loop::{
    set_stack_dwell_core, set_turn_yield_core, set_yield_core, submit_intent_core,
};
use crate::grpc::auth_ctx;
use crate::grpc::map;
use crate::grpc::pb;
use crate::stream::{self, TableSubscription};

pub struct GameSvc {
    state: AppState,
}

impl GameSvc {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

fn ack_fields(ack: crate::game_loop::Ack) -> (bool, Option<pb::MessageRef>) {
    (ack.accepted, ack.reject_reason.map(map::message_ref_to_pb))
}

fn submit_intent_response(ack: crate::game_loop::Ack) -> pb::SubmitIntentResponse {
    let (accepted, reject_reason) = ack_fields(ack);
    pb::SubmitIntentResponse {
        accepted,
        reject_reason,
    }
}

fn set_yield_response(ack: crate::game_loop::Ack) -> pb::SetYieldResponse {
    let (accepted, reject_reason) = ack_fields(ack);
    pb::SetYieldResponse {
        accepted,
        reject_reason,
    }
}

fn set_turn_yield_response(ack: crate::game_loop::Ack) -> pb::SetTurnYieldResponse {
    let (accepted, reject_reason) = ack_fields(ack);
    pb::SetTurnYieldResponse {
        accepted,
        reject_reason,
    }
}

fn set_stack_dwell_response(ack: crate::game_loop::Ack) -> pb::SetStackDwellResponse {
    let (accepted, reject_reason) = ack_fields(ack);
    pb::SetStackDwellResponse {
        accepted,
        reject_reason,
    }
}

/// Scrubbed intent discriminant for spans (no payloads).
fn intent_kind_label(intent: &schema::WireIntent) -> String {
    let dbg = format!("{intent:?}");
    dbg.split([' ', '{'])
        .next()
        .unwrap_or("unknown")
        .to_string()
}

#[tonic::async_trait]
impl pb::game_service_server::GameService for GameSvc {
    type StreamStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<pb::StreamResponse, Status>> + Send>>;

    async fn stream(
        &self,
        request: Request<pb::StreamRequest>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let table_id = request.into_inner().table_id;
        let TableSubscription {
            mut rx,
            snapshot_seq,
            snapshot,
            viewer,
            seats,
            prints,
            card_text,
            snapshot_broadcast_seq,
        } = stream::subscribe(&self.state, &table_id, user.id)
            .map_err(|_| Status::not_found("unknown table or game not started"))?;

        let out = async_stream::stream! {
            let mut known_card_text: std::collections::HashSet<(String, String)> = card_text
                .iter()
                .map(|text| (text.card_id.clone(), text.print.clone()))
                .collect();
            yield Ok(map::stream_frame_to_pb(StreamFrame::Snapshot {
                seq: snapshot_seq,
                state: snapshot,
                card_text,
            }));
            let mut heartbeat =
                tokio::time::interval(std::time::Duration::from_secs(crate::HEARTBEAT_SECS));
            heartbeat.tick().await; // first tick fires immediately; skip so it doesn't double the snapshot
            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        let Ok(msg) = msg else { break };
                        if !stream::should_deliver(msg.broadcast_seq, snapshot_broadcast_seq) {
                            continue;
                        }
                        let extras = stream::view_extras(
                            &msg.yields,
                            &msg.turn_yields,
                            &seats,
                            msg.stack_hold_remaining_ms,
                            &prints,
                        );
                        let mut frame = stream::frame_for(
                            viewer,
                            msg.seq,
                            &msg.events,
                            &msg.game,
                            msg.auto_actions.clone(),
                            &extras,
                        );
                        stream::retain_new_card_text(&mut frame, &mut known_card_text);
                        yield Ok(map::stream_frame_to_pb(frame));
                    }
                    _ = heartbeat.tick() => {
                        yield Ok(map::stream_frame_to_pb(StreamFrame::Heartbeat));
                    }
                }
            }
        };
        Ok(Response::new(Box::pin(out) as Self::StreamStream))
    }

    async fn submit_intent(
        &self,
        request: Request<pb::SubmitIntentRequest>,
    ) -> Result<Response<pb::SubmitIntentResponse>, Status> {
        // Parent span is created by `trace::TraceLayer` for every gRPC method.
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let table_id = inner.table_id.clone();
        tracing::Span::current().record(
            crate::otel_semconv::MTGFR_TABLE_ID,
            tracing::field::display(&table_id),
        );
        let envelope = map::intent_envelope_from_pb(
            inner
                .envelope
                .ok_or_else(|| Status::invalid_argument("missing envelope"))?,
        )
        .map_err(Status::invalid_argument)?;
        if envelope.table_id != table_id {
            return Err(Status::invalid_argument(
                "envelope.table_id does not match SubmitIntentRequest.table_id",
            ));
        }
        let intent_kind = intent_kind_label(&envelope.intent);
        tracing::Span::current()
            .record(crate::otel_semconv::MTGFR_INTENT_KIND, intent_kind.as_str());
        let ack = submit_intent_core(&self.state, user.id, &table_id, envelope).await;
        tracing::Span::current().record(crate::otel_semconv::MTGFR_INTENT_ACCEPTED, ack.accepted);
        Ok(Response::new(submit_intent_response(ack)))
    }

    async fn set_yield(
        &self,
        request: Request<pb::SetYieldRequest>,
    ) -> Result<Response<pb::SetYieldResponse>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let ack = set_yield_core(&self.state, user.id, &inner.table_id, inner.enabled).await;
        Ok(Response::new(set_yield_response(ack)))
    }

    async fn set_turn_yield(
        &self,
        request: Request<pb::SetTurnYieldRequest>,
    ) -> Result<Response<pb::SetTurnYieldResponse>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let ack = set_turn_yield_core(&self.state, user.id, &inner.table_id, inner.enabled).await;
        Ok(Response::new(set_turn_yield_response(ack)))
    }

    async fn set_stack_dwell(
        &self,
        request: Request<pb::SetStackDwellRequest>,
    ) -> Result<Response<pb::SetStackDwellResponse>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let ack = set_stack_dwell_core(&self.state, user.id, &inner.table_id, inner.dwelling);
        Ok(Response::new(set_stack_dwell_response(ack)))
    }
}

#[cfg(test)]
mod tests {
    use schema::{MessageParam, MessageRef};

    use super::*;

    #[test]
    fn intent_kind_label_returns_discriminant_only() {
        let intent = schema::WireIntent::Cast {
            player: 0,
            object: 42,
            target: None,
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
            multikicker_count: 0,
            alternative_cost: false,
        };
        let label = intent_kind_label(&intent);
        assert_eq!(label, "Cast");
        assert!(!label.contains("42"));
    }

    #[test]
    fn submit_intent_response_maps_reject_reason_message_ref() {
        let ack = crate::game_loop::Ack {
            accepted: false,
            reject_reason: Some(
                MessageRef::key("reject.engine_error")
                    .with_params(vec![MessageParam::bool("recoverable", false)])
                    .with_children(vec![MessageRef::key("reject.not_helpless")]),
            ),
        };

        let pb = submit_intent_response(ack);
        let reason = pb.reject_reason.expect("reject reason");
        assert_eq!(reason.key, "reject.engine_error");
        assert!(matches!(
            reason.params[0].value.as_ref(),
            Some(pb::message_param::Value::BoolValue(false))
        ));
        assert_eq!(reason.children[0].key, "reject.not_helpless");
    }
}
