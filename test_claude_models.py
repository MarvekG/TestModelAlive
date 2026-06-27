#!/usr/bin/env python3
"""Test Claude CLI models from endpoint credentials.

Input file format, one endpoint per line:
    https://example.com;sk-...;claude-sonnet-4-5,claude-opus-4-5

The third column is optional. If omitted, --models is used. Separators support
both English and Chinese punctuation: ';'/'；' for fields and ','/'，' for models.
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import selectors
import shlex
import shutil
import signal
import subprocess
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


EXPECTED_OUTPUT = "OKK"
DEFAULT_PROMPT = "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation."
DEFAULT_MODELS = "claude-opus-4-6"
LOG = logging.getLogger("claude-model-test")
LINE = "=" * 72
SUBLINE = "-" * 72
SETTINGS_PATH = Path("claude-settings.json")
FIELD_SEPARATORS = str.maketrans({"；": ";"})
MODEL_SEPARATORS = str.maketrans({"，": ","})


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


def parse_endpoint_line(line: str, line_no: int) -> Endpoint | None:
    clean = line.strip()
    if not clean or clean.startswith("#"):
        return None

    parts = [part.strip() for part in clean.translate(FIELD_SEPARATORS).split(";")]
    if len(parts) not in (2, 3) or not parts[0] or not parts[1]:
        raise ValueError(f"claude file line {line_no}: expected 'base_url;api_key;model1,model2'")
    models = tuple(parse_models(parts[2])) if len(parts) == 3 and parts[2] else ()
    return Endpoint(base_url=parts[0].rstrip("/"), api_key=parts[1], models=models)


def load_endpoints(path: Path) -> list[Endpoint]:
    endpoints: list[Endpoint] = []
    with path.open("r", encoding="utf-8") as fh:
        for line_no, line in enumerate(fh, start=1):
            endpoint = parse_endpoint_line(line, line_no)
            if endpoint is not None:
                endpoints.append(endpoint)

    if not endpoints:
        raise ValueError(f"no endpoints found in {path}")
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


def configure_logging(verbose: bool) -> None:
    logging.basicConfig(
        level=logging.DEBUG if verbose else logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
        datefmt="%H:%M:%S",
    )


def models_url(base_url: str) -> str:
    clean = base_url.rstrip("/")
    if clean.endswith("/v1"):
        return f"{clean}/models"
    return f"{clean}/v1/models"


def fetch_models(endpoint: Endpoint, timeout: int) -> list[str]:
    url = models_url(endpoint.base_url)
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/json",
            "Authorization": f"Bearer {endpoint.api_key}",
            "X-Api-Key": endpoint.api_key,
            "Anthropic-Version": "2023-06-01",
        },
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        payload = json.loads(response.read().decode("utf-8"))

    raw_models = payload.get("data") or payload.get("models")
    if not isinstance(raw_models, list):
        raise ValueError(f"{url} did not return a JSON object with data/models: []")

    models: list[str] = []
    for item in raw_models:
        if isinstance(item, str):
            models.append(item)
        elif isinstance(item, dict) and isinstance(item.get("id"), str):
            models.append(item["id"])

    return sorted(set(models))


def write_claude_settings(settings_path: Path, endpoint: Endpoint, model: str) -> None:
    settings_path.parent.mkdir(parents=True, exist_ok=True)
    settings = {
        "env": {
            "ANTHROPIC_BASE_URL": endpoint.base_url,
            "ANTHROPIC_AUTH_TOKEN": endpoint.api_key,
        }
    }
    settings_path.write_text(json.dumps(settings, indent=2) + "\n", encoding="utf-8")


def run_claude(endpoint: Endpoint, model: str, timeout: int, settings_path: Path) -> tuple[bool, str]:
    write_claude_settings(settings_path, endpoint, model)
    command = [
        "claude",
        "--debug",
        "--verbose",
        "--settings",
        str(settings_path),
        "--model",
        model,
        "-p",
        DEFAULT_PROMPT,
    ]
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
    output_selector = selectors.DefaultSelector()
    output_selector.register(process.stdout, selectors.EVENT_READ)

    try:
        while True:
            if time.monotonic() > deadline:
                raise subprocess.TimeoutExpired(command, timeout, output="\n".join(output_lines))

            for _key, _mask in output_selector.select(timeout=0.2):
                line = process.stdout.readline()
                if line:
                    clean = line.rstrip()
                    output_lines.append(clean)
                    LOG.info("claude | %s", clean)

            if process.poll() is not None:
                for line in process.stdout:
                    clean = line.rstrip()
                    output_lines.append(clean)
                    LOG.info("claude | %s", clean)
                break

        output = "\n".join(output_lines)
        has_expected_line = any(line.strip() == EXPECTED_OUTPUT for line in output_lines)
        ok = process.returncode == 0 and has_expected_line
        if process.returncode == 0 and not ok:
            output = f"claude exited 0 but did not return expected '{EXPECTED_OUTPUT}'\n{output}"
        return ok, output[-1000:]
    except subprocess.TimeoutExpired:
        terminate_process(process)
        raise
    except KeyboardInterrupt:
        terminate_process(process)
        raise
    finally:
        output_selector.close()


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


def test_model(endpoint: Endpoint, model: str, timeout: int, settings_path: Path) -> ModelResult:
    start = time.monotonic()
    LOG.info(SUBLINE)
    LOG.info("testing model: endpoint=%s model=%s", endpoint.base_url, model)
    try:
        ok, detail = run_claude(endpoint, model, timeout, settings_path)
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


def indent(text: str, prefix: str) -> str:
    return "\n".join(prefix + line for line in text.splitlines())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--claude-file",
        type=Path,
        default=Path("claude.txt"),
        help="file containing base_url;api_key;model1,model2 lines",
    )
    parser.add_argument("--timeout", type=int, default=120, help="seconds for each claude test")
    parser.add_argument(
        "--models",
        default=DEFAULT_MODELS,
        help="comma-separated model ids to test when a line has no third column",
    )
    parser.add_argument(
        "--models-check",
        action="store_true",
        help="call /models and test only the intersection with requested models",
    )
    parser.add_argument("--limit", type=int, help="maximum models to test per endpoint")
    parser.add_argument("--domain", help="only test endpoint URLs containing this text")
    parser.add_argument("--last-only", action="store_true", help="only test the last valid line in the claude file")
    parser.add_argument("--list-models", action="store_true", help="fetch and print server models; do not run claude")
    parser.add_argument("-v", "--verbose", action="store_true", help="enable debug logging")
    args = parser.parse_args()
    configure_logging(args.verbose)

    if not args.list_models and shutil.which("claude") is None:
        LOG.error("claude command not found in PATH")
        return 127

    endpoints = select_endpoints(
        filter_endpoints_by_domain(load_endpoints(args.claude_file), args.domain),
        args.last_only,
    )
    if not endpoints:
        LOG.error("no endpoints matched the requested filters")
        return 1
    requested_models = parse_models(args.models)
    settings_path = SETTINGS_PATH
    results: list[ModelResult] = []
    interrupted = False

    if args.list_models:
        for index, endpoint in enumerate(endpoints, start=1):
            LOG.info(LINE)
            LOG.info("ENDPOINT %d/%d: %s", index, len(endpoints), endpoint.base_url)
            LOG.info(LINE)
            try:
                models = fetch_models(endpoint, args.timeout)
            except (urllib.error.URLError, TimeoutError, ValueError, json.JSONDecodeError) as exc:
                LOG.error(
                    "ENDPOINT_STATUS=UNAVAILABLE endpoint=%s reason=fetch_models_failed detail=%s",
                    endpoint.base_url,
                    exc,
                )
                continue
            LOG.info("%s: fetched %d models", endpoint.base_url, len(models))
            for model in models:
                LOG.info("model: %s", model)
        return 0

    try:
        with RestorableFile(settings_path):
            for index, endpoint in enumerate(endpoints, start=1):
                LOG.info(LINE)
                LOG.info("ENDPOINT %d/%d: %s", index, len(endpoints), endpoint.base_url)
                LOG.info(LINE)
                endpoint_models = list(endpoint.models) if endpoint.models else requested_models
                if args.models_check:
                    try:
                        available_models = fetch_models(endpoint, args.timeout)
                    except (urllib.error.URLError, TimeoutError, ValueError, json.JSONDecodeError) as exc:
                        LOG.error(
                            "ENDPOINT_STATUS=UNAVAILABLE endpoint=%s reason=fetch_models_failed detail=%s",
                            endpoint.base_url,
                            exc,
                        )
                        continue
                    selected = intersect_models(endpoint_models, available_models)
                    missing = [model for model in endpoint_models if model not in set(available_models)]
                    selected = iter_selected_models(selected, args.limit)
                    LOG.info(
                        "%s: fetched %d models, requested %d, testing intersection %d",
                        endpoint.base_url,
                        len(available_models),
                        len(endpoint_models),
                        len(selected),
                    )
                    if missing:
                        LOG.warning("%s: requested models not available: %s", endpoint.base_url, ",".join(missing))
                else:
                    selected = iter_selected_models(endpoint_models, args.limit)
                    LOG.info("%s: testing %d requested models", endpoint.base_url, len(selected))
                if not selected:
                    LOG.error(
                        "ENDPOINT_STATUS=UNAVAILABLE endpoint=%s reason=no_requested_models_selected",
                        endpoint.base_url,
                    )

                for model in selected:
                    result = test_model(endpoint, model, args.timeout, settings_path)
                    results.append(result)
                    log_result(result)
    except KeyboardInterrupt:
        interrupted = True
        LOG.warning("interrupted by Ctrl+C; stopping")
    finally:
        LOG.info("restored %s", settings_path)

    ok_count = sum(1 for result in results if result.ok)
    fail_count = len(results) - ok_count
    LOG.info("summary: %d ok, %d failed", ok_count, fail_count)
    available_results = [result for result in results if result.ok]
    if available_results:
        LOG.info(LINE)
        LOG.info("AVAILABLE_CONFIGS")
    for result in available_results:
        LOG.info(LINE)
        LOG.info("%s", result.endpoint)
        LOG.info("%s", result.api_key)
        LOG.info("%s", result.model)

    if interrupted:
        return 130
    return 0 if fail_count == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
