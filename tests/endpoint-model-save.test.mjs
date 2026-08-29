import test from "node:test";
import assert from "node:assert/strict";

import { buildTestModelSelectionRequest } from "../frontend/ui/endpointForm.ts";

test("saving test models preserves the existing OpenCode SDK package", () => {
  const endpoint = {
    id: "opencode-001",
    name: "Provider",
    type: "opencode",
    opencode_sdk_package: "@ai-sdk/openai",
    base_url: "https://example.com/v1",
    api_key: "key",
    models: ["model-a"],
  };

  assert.deepEqual(buildTestModelSelectionRequest(endpoint, ["model-b"]), {
    name: "Provider",
    type: "opencode",
    opencode_sdk_package: "@ai-sdk/openai",
    base_url: "https://example.com/v1",
    api_key: "key",
    models: ["model-b"],
    overwrite: true,
  });
});
