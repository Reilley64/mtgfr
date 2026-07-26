/// <reference lib="webworker" />

export {};

declare const self: ServiceWorkerGlobalScope;

self.addEventListener("install", (event: ExtendableEvent) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener("activate", (event: ExtendableEvent) => {
  event.waitUntil(self.clients.claim());
});

// Network-only service worker: installability without offline caching.
self.addEventListener("fetch", (event: FetchEvent) => {
  event.respondWith(fetch(event.request));
});
