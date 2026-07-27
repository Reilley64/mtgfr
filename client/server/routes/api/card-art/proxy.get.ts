import { defineEventHandler, getCookie, getRequestURL, type H3Event } from "nitro/h3";
import { fetchMe } from "../../../../app/domain/api-upstream-auth";
import { fetchProxyCardArt } from "../../../../app/domain/card-art/proxy-fetch";
import { grpcRequestEnv, runTraced } from "../../../../app/domain/otel";
import type { GrpcRequestEnv } from "../../../../app/domain/wire/grpcClient";

const SESSION_COOKIE = "session";
const SUCCESS_CACHE_CONTROL = "private, max-age=300";

type CardArtProxyDeps = {
  buildGrpcEnv?: (sessionToken: string) => Promise<GrpcRequestEnv>;
  fetchMe?: typeof fetchMe;
  fetchProxyArt?: typeof fetchProxyCardArt;
};

function unauthorized(): Response {
  return new Response("Unauthorized", { status: 401 });
}

function badRequest(): Response {
  return new Response("Bad Request", { status: 400 });
}

function badGateway(): Response {
  return new Response("Bad Gateway", { status: 502 });
}

function responseBody(bytes: Uint8Array): Blob {
  const body = new Uint8Array(bytes.byteLength);
  body.set(bytes);
  return new Blob([body]);
}

async function defaultBuildGrpcEnv(sessionToken: string): Promise<GrpcRequestEnv> {
  return runTraced(grpcRequestEnv(sessionToken));
}

export async function handleCardArtProxyRequest(event: H3Event, deps: CardArtProxyDeps = {}): Promise<Response> {
  const sessionToken = getCookie(event, SESSION_COOKIE);
  if (!sessionToken) return unauthorized();

  const buildGrpcEnv = deps.buildGrpcEnv ?? defaultBuildGrpcEnv;
  const fetchMeForRequest = deps.fetchMe ?? fetchMe;
  const fetchProxyArt = deps.fetchProxyArt ?? fetchProxyCardArt;

  const me = await fetchMeForRequest(await buildGrpcEnv(sessionToken));
  if (!me) return unauthorized();

  const rawUrl = getRequestURL(event).searchParams.get("url");
  if (!rawUrl) return badRequest();

  const proxied = await fetchProxyArt(rawUrl);
  if (!proxied.ok) {
    return proxied.status === 400 ? badRequest() : badGateway();
  }

  return new Response(responseBody(proxied.body), {
    status: 200,
    headers: {
      "cache-control": SUCCESS_CACHE_CONTROL,
      "content-type": proxied.contentType,
    },
  });
}

export default defineEventHandler((event) => handleCardArtProxyRequest(event));
