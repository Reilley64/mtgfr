import { Router } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import { Suspense } from "solid-js";
import "~/global.css";
import { PortraitGate } from "~/PortraitGate";

export default function App() {
  return (
    <Router
      root={(props) => (
        <>
          <PortraitGate />
          <Suspense>{props.children}</Suspense>
        </>
      )}
    >
      <FileRoutes />
    </Router>
  );
}
