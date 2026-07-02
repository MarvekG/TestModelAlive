#!/usr/bin/env python3
"""Fetch server models and test them one by one through Codex CLI.

By default this script reads codex endpoints from tsa_endpoints.json, the same
JSON file used by the GUI. Endpoints with type="codex" are selected. If an
endpoint has no saved models, --models is used.

The actual tested set is the intersection of requested models and the server's
/models response only when --models-check is set.

The script temporarily rewrites ~/.codex/auth.json and ~/.codex/config.toml for
each model, runs `codex exec`, and always restores the original files before it
exits.
"""

from __future__ import annotations

import argparse
import gzip
import json
import logging
import os
import shlex
import signal
import shutil
import subprocess
import time
import urllib.error
import urllib.request
import zlib
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


EXPECTED_OUTPUT = "OKK"
DEFAULT_PROMPT = "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation."
DEFAULT_MODELS = "gpt-5.5"
LOG = logging.getLogger("codex-model-test")
LINE = "=" * 72
SUBLINE = "-" * 72
MODEL_SEPARATORS = str.maketrans({"，": ","})
DEFAULT_ENDPOINTS_FILE = Path("tsa_endpoints.json")


@dataclass(frozen=True)
class Endpoint:
    base_url: str
    api_key: str
    models: tuple[str, ...]


@dataclass(frozen=True)
class ModelResult:
    endpoint: str
    api_key: str
    model: str
    ok: bool
    seconds: float
    detail: str


class RestorableFile:
    def __init__(self, path: Path) -> None:
        self.path = path
        self._existed = path.exists()
        self._backup: Path | None = None

    def __enter__(self) -> "RestorableFile":
        if self._existed:
            self.path.parent.mkdir(parents=True, exist_ok=True)
            self._backup = self._next_backup_path()
            shutil.copy2(self.path, self._backup)
        return self

    def __exit__(self, exc_type, exc, tb) -> None:  # type: ignore[no-untyped-def]
        if self._backup is not None:
            self.path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(self._backup, self.path)
            self._backup.unlink(missing_ok=True)
            return

        if not self._existed:
            self.path.unlink(missing_ok=True)

    def _next_backup_path(self) -> Path:
        stamp = int(time.time() * 1000)
        for index in range(1000):
            suffix = f".{stamp}.{index}.bak"
            candidate = self.path.with_name(f"{self.path.name}{suffix}")
            if not candidate.exists():
                return candidate
        raise RuntimeError(f"could not allocate backup path for {self.path}")


def load_endpoints(path: Path, endpoint_type: str = "codex") -> list[Endpoint]:
    with path.open("r", encoding="utf-8") as fh:
        payload = json.load(fh)
    raw_endpoints = payload.get("endpoints")
    if not isinstance(raw_endpoints, list):
        raise ValueError(f"{path}: expected JSON object with endpoints: []")

    endpoints: list[Endpoint] = []
    for index, item in enumerate(raw_endpoints, start=1):
        if not isinstance(item, dict):
            continue
        if str(item.get("type", "")).lower() != endpoint_type:
            continue
        base_url = str(item.get("base_url", "")).strip().rstrip("/")
        api_key = str(item.get("api_key", "")).strip()
        if not base_url or not api_key:
            raise ValueError(f"{path}: endpoint #{index} missing base_url or api_key")
        models = tuple(str(model).strip() for model in item.get("models", []) if str(model).strip())
        endpoints.append(Endpoint(base_url=base_url, api_key=api_key, models=models))

    if not endpoints:
        raise ValueError(f"no {endpoint_type} endpoints found in {path}")
    return endpoints


def select_endpoints(endpoints: list[Endpoint], last_only: bool) -> list[Endpoint]:
    if last_only:
        return endpoints[-1:]
    return endpoints


def filter_endpoints_by_domain(endpoints: list[Endpoint], domain: str | None) -> list[Endpoint]:
    if not domain:
        return endpoints
    needle = domain.strip().lower()
    if not needle:
        return endpoints
    return [endpoint for endpoint in endpoints if needle in endpoint.base_url.lower()]


def fetch_models(endpoint: Endpoint, timeout: int) -> list[str]:
    url = f"{endpoint.base_url}/models"
    request = urllib.request.Request(
        url,
        headers={
            "Authorization": f"Bearer {endpoint.api_key}",
            "Accept": "application/json",
        },
    )

    with urllib.request.urlopen(request, timeout=timeout) as response:
        payload = json.loads(decode_response_body(response.read(), response.headers.get("Content-Encoding")))

    raw_models = payload.get("data")
    if not isinstance(raw_models, list):
        raise ValueError(f"{url} did not return a JSON object with data: []")

    models = []
    for item in raw_models:
        if isinstance(item, dict) and isinstance(item.get("id"), str):
            models.append(item["id"])

    return sorted(set(models))


def decode_response_body(body: bytes, content_encoding: str | None) -> str:
    encoding = (content_encoding or "").lower()
    if "gzip" in encoding or body.startswith(b"\x1f\x8b"):
        body = gzip.decompress(body)
    elif "deflate" in encoding:
        body = zlib.decompress(body)
    return body.decode("utf-8")


def toml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=True)


def write_codex_config(codex_dir: Path, endpoint: Endpoint, model: str) -> None:
    codex_dir.mkdir(parents=True, exist_ok=True)
    auth_path = codex_dir / "auth.json"
    config_path = codex_dir / "config.toml"

    auth_path.write_text(
        json.dumps({"OPENAI_API_KEY": endpoint.api_key}, indent=2) + "\n",
        encoding="utf-8",
    )

    config = f"""model_provider = "custom"
model = {toml_string(model)}
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.custom]
name = "model-test"
base_url = {toml_string(endpoint.base_url)}
wire_api = "responses"
requires_openai_auth = true
"""
    config_path.write_text(config, encoding="utf-8")


def configure_logging(verbose: bool) -> None:
    logging.basicConfig(
        level=logging.DEBUG if verbose else logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
        datefmt="%H:%M:%S",
    )


def run_codex(timeout: int) -> tuple[bool, str]:
    command = ["codex", "exec", "--skip-git-repo-check", DEFAULT_PROMPT]
    LOG.info("running: %s", " ".join(shlex.quote(part) for part in command))
    process = subprocess.Popen(
        command,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        bufsize=1,
        start_new_session=True,
    )

    output_lines: list[str] = []
    deadline = time.monotonic() + timeout
    assert process.stdout is not None

    try:
        while True:
            if time.monotonic() > deadline:
                raise subprocess.TimeoutExpired(command, timeout, output="\n".join(output_lines))

            line = process.stdout.readline()
            if line:
                clean = line.rstrip()
                output_lines.append(clean)
                LOG.info("codex | %s", clean)
                continue

            if process.poll() is not None:
                for line in process.stdout:
                    clean = line.rstrip()
                    output_lines.append(clean)
                    LOG.info("codex | %s", clean)
                break

        output = "\n".join(output_lines)
        has_expected_line = any(line.strip() == EXPECTED_OUTPUT for line in output_lines)
        ok = process.returncode == 0 and has_expected_line
        if process.returncode == 0 and not ok:
            output = f"codex exited 0 but did not return expected '{EXPECTED_OUTPUT}'\n{output}"
        return ok, output[-1000:]
    except subprocess.TimeoutExpired:
        terminate_process(process)
        raise
    except KeyboardInterrupt:
        terminate_process(process)
        raise


def terminate_process(process: subprocess.Popen[str]) -> None:
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return

    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=5)


def iter_selected_models(models: Iterable[str], limit: int | None) -> list[str]:
    selected = list(models)
    if limit is not None:
        selected = selected[:limit]
    return selected


def parse_models(value: str) -> list[str]:
    models = [model.strip() for model in value.translate(MODEL_SEPARATORS).split(",") if model.strip()]
    if not models:
        raise ValueError("--models must contain at least one model id")
    return models


def intersect_models(requested: Iterable[str], available: Iterable[str]) -> list[str]:
    available_set = set(available)
    selected: list[str] = []
    seen: set[str] = set()
    for model in requested:
        if model in available_set and model not in seen:
            selected.append(model)
            seen.add(model)
    return selected


def test_model(endpoint: Endpoint, model: str, codex_dir: Path, timeout: int) -> ModelResult:
    start = time.monotonic()
    LOG.info(SUBLINE)
    LOG.info("testing model: endpoint=%s model=%s", endpoint.base_url, model)
    write_codex_config(codex_dir, endpoint, model)
    LOG.debug("wrote Codex config under %s", codex_dir)
    try:
        ok, detail = run_codex(timeout)
    except subprocess.TimeoutExpired as exc:
        ok = False
        detail = f"timeout after {timeout}s"
        if exc.stdout:
            detail += f"\n{str(exc.stdout)[-1000:]}"
    seconds = time.monotonic() - start
    return ModelResult(
        endpoint=endpoint.base_url,
        api_key=endpoint.api_key,
        model=model,
        ok=ok,
        seconds=seconds,
        detail=detail,
    )


def log_result(result: ModelResult) -> None:
    status = "AVAILABLE" if result.ok else "UNAVAILABLE"
    log = LOG.info if result.ok else LOG.error
    log("MODEL_STATUS=%s endpoint=%s model=%s elapsed=%.1fs", status, result.endpoint, result.model, result.seconds)
    if not result.ok and result.detail:
        LOG.error("failure detail:\n%s", indent(result.detail, "  "))
    LOG.info(SUBLINE)


def indent(text: str, prefix: str) -> str:
    return "\n".join(prefix + line for line in text.splitlines())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--api-file",
        type=Path,
        default=DEFAULT_ENDPOINTS_FILE,
        help="endpoint JSON file from GUI",
    )
    parser.add_argument("--codex-dir", type=Path, default=Path.home() / ".codex", help="Codex config directory")
    parser.add_argument("--fetch-timeout", type=int, default=30, help="seconds for /models requests")
    parser.add_argument("--codex-timeout", type=int, default=120, help="seconds for each codex exec")
    parser.add_argument(
        "--models",
        default=DEFAULT_MODELS,
        help="comma-separated model ids to test when an endpoint has no saved models",
    )
    parser.add_argument(
        "--models-check",
        action="store_true",
        help="call /models and test only the intersection with requested models",
    )
    parser.add_argument("--limit", type=int, help="maximum models to test per endpoint")
    parser.add_argument("--domain", help="only test endpoint URLs containing this text")
    parser.add_argument("--last-only", action="store_true", help="only test the last matching endpoint")
    parser.add_argument(
        "--list-only",
        action="store_true",
        help="only fetch and print models; do not change Codex config",
    )
    parser.add_argument("--list-models", action="store_true", help="fetch and print server models; do not run codex")
    parser.add_argument("-v", "--verbose", action="store_true", help="enable debug logging")
    args = parser.parse_args()
    configure_logging(args.verbose)
    list_models = args.list_only or args.list_models

    if shutil.which("codex") is None and not list_models:
        LOG.error("codex command not found in PATH")
        return 127

    endpoints = select_endpoints(filter_endpoints_by_domain(load_endpoints(args.api_file, "codex"), args.domain), args.last_only)
    if not endpoints:
        LOG.error("no endpoints matched the requested filters")
        return 1
    requested_models = parse_models(args.models)
    auth_path = args.codex_dir / "auth.json"
    config_path = args.codex_dir / "config.toml"
    results: list[ModelResult] = []

    interrupted = False
    try:
        try:
            with RestorableFile(auth_path), RestorableFile(config_path):
                for index, endpoint in enumerate(endpoints, start=1):
                    LOG.info(LINE)
                    LOG.info("ENDPOINT %d/%d: %s", index, len(endpoints), endpoint.base_url)
                    LOG.info(LINE)
                    endpoint_models = list(endpoint.models) if endpoint.models else requested_models

                    if not args.models_check and not list_models:
                        selected = iter_selected_models(endpoint_models, args.limit)
                        LOG.info(
                            "%s: skip /models check, testing %d requested models",
                            endpoint.base_url,
                            len(selected),
                        )
                        if not selected:
                            LOG.error(
                                "ENDPOINT_STATUS=UNAVAILABLE endpoint=%s reason=no_requested_models_selected",
                                endpoint.base_url,
                            )
                        for model in selected:
                            result = test_model(endpoint, model, args.codex_dir, args.codex_timeout)
                            results.append(result)
                            log_result(result)
                        continue

                    try:
                        LOG.info("fetching models from %s", endpoint.base_url)
                        models = fetch_models(endpoint, args.fetch_timeout)
                    except (urllib.error.URLError, TimeoutError, ValueError, json.JSONDecodeError) as exc:
                        LOG.error(
                            "ENDPOINT_STATUS=UNAVAILABLE endpoint=%s reason=fetch_models_failed detail=%s",
                            endpoint.base_url,
                            exc,
                        )
                        continue

                    if list_models:
                        selected = iter_selected_models(models, args.limit)
                        LOG.info("%s: fetched %d models, showing %d", endpoint.base_url, len(models), len(selected))
                    else:
                        selected = intersect_models(endpoint_models, models)
                        missing = [model for model in endpoint_models if model not in set(models)]
                        selected = iter_selected_models(selected, args.limit)
                        LOG.info(
                            "%s: fetched %d models, requested %d, testing intersection %d",
                            endpoint.base_url,
                            len(models),
                            len(endpoint_models),
                            len(selected),
                        )
                        if missing:
                            LOG.warning("%s: requested models not available: %s", endpoint.base_url, ",".join(missing))
                        if not selected:
                            LOG.error(
                                "ENDPOINT_STATUS=UNAVAILABLE endpoint=%s reason=no_requested_models_available",
                                endpoint.base_url,
                            )

                    for model in selected:
                        if list_models:
                            LOG.info("model: %s", model)
                            continue

                        result = test_model(endpoint, model, args.codex_dir, args.codex_timeout)
                        results.append(result)
                        log_result(result)
        except KeyboardInterrupt:
            interrupted = True
            LOG.warning("interrupted by Ctrl+C; stopping after restoring Codex config")
    finally:
        LOG.info("restored %s and %s", auth_path, config_path)

    if list_models:
        return 130 if interrupted else 0

    ok_count = sum(1 for result in results if result.ok)
    fail_count = len(results) - ok_count
    LOG.info("summary: %d ok, %d failed", ok_count, fail_count)
    available_results = [result for result in results if result.ok]
    if available_results:
        LOG.info(LINE)
        LOG.info("AVAILABLE_CONFIGS")
    for result in results:
        if not result.ok:
            continue
        LOG.info(LINE)
        LOG.info("%s", result.endpoint)
        LOG.info("%s", result.api_key)
        LOG.info("%s", result.model)
    if interrupted:
        return 130
    return 0 if fail_count == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
