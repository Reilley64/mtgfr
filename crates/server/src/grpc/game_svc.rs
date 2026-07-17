//! `mtgfr.v1.Game` — intents, yield/dwell chrome, and the per-viewer delta stream.
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

fn ack_msg(ack: crate::game_loop::Ack) -> pb::Ack {
    pb::Ack {
        accepted: ack.accepted,
        reason: ack.reason,
    }
}

#[tonic::async_trait]
impl pb::game_server::Game for GameSvc {
    type StreamStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<pb::StreamFrame, Status>> + Send>>;

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
            snapshot_broadcast_seq,
        } = stream::subscribe(&self.state, &table_id, user.id)
            .map_err(|_| Status::not_found("unknown table or game not started"))?;

        let out = async_stream::stream! {
            yield Ok(map::stream_frame_to_pb(StreamFrame::Snapshot { seq: snapshot_seq, state: snapshot }));
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
                        yield Ok(map::stream_frame_to_pb(stream::frame_for(
                            viewer,
                            msg.seq,
                            &msg.events,
                            &msg.game,
                            msg.auto_actions.clone(),
                            &msg.yields,
                            &msg.turn_yields,
                            &seats,
                            msg.stack_hold_remaining_ms,
                            &prints,
                        )));
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
        request: Request<pb::IntentRequest>,
    ) -> Result<Response<pb::Ack>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let envelope = map::intent_envelope_from_pb(
            inner
                .envelope
                .ok_or_else(|| Status::invalid_argument("missing envelope"))?,
        )
        .map_err(Status::invalid_argument)?;
        if envelope.table_id != inner.table_id {
            return Err(Status::invalid_argument(
                "envelope.table_id does not match IntentRequest.table_id",
            ));
        }
        let ack = submit_intent_core(&self.state, user.id, &inner.table_id, envelope).await;
        Ok(Response::new(ack_msg(ack)))
    }

    async fn set_yield(
        &self,
        request: Request<pb::YieldRequest>,
    ) -> Result<Response<pb::Ack>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let ack = set_yield_core(&self.state, user.id, &inner.table_id, inner.enabled).await;
        Ok(Response::new(ack_msg(ack)))
    }

    async fn set_turn_yield(
        &self,
        request: Request<pb::YieldRequest>,
    ) -> Result<Response<pb::Ack>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let ack = set_turn_yield_core(&self.state, user.id, &inner.table_id, inner.enabled).await;
        Ok(Response::new(ack_msg(ack)))
    }

    async fn set_stack_dwell(
        &self,
        request: Request<pb::StackDwellRequest>,
    ) -> Result<Response<pb::Ack>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let ack = set_stack_dwell_core(&self.state, user.id, &inner.table_id, inner.dwelling);
        Ok(Response::new(ack_msg(ack)))
    }
}
