import { Router } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import { Suspense } from "solid-js";
import { PortraitGate } from "~/components/molecules/portrait-gate";
import "~/global.css";

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
