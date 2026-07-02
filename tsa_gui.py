#!/usr/bin/env python3
"""Qt GUI for managing TSA endpoints and testing saved models."""

from __future__ import annotations

import json
import os
import queue
import selectors
import shlex
import shutil
import signal
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Callable

import test_claude_models as claude_core
import test_codex_models as codex_core


try:
    from PyQt6.QtCore import QTimer, Qt
    from PyQt6.QtGui import QFont, QFontDatabase
    from PyQt6.QtWidgets import (
        QApplication,
        QComboBox,
        QCheckBox,
        QDialog,
        QFormLayout,
        QGridLayout,
        QGroupBox,
        QHBoxLayout,
        QHeaderView,
        QLabel,
        QLineEdit,
        QListWidget,
        QListWidgetItem,
        QMainWindow,
        QMessageBox,
        QPushButton,
        QSpinBox,
        QSplitter,
        QTableWidget,
        QTableWidgetItem,
        QTextEdit,
        QVBoxLayout,
        QWidget,
    )
except ModuleNotFoundError:
    print("PyQt6 is required. Install with: python3 -m pip install PyQt6", file=sys.stderr)
    raise SystemExit(1)


ALIGN_CENTER = Qt.AlignmentFlag.AlignCenter
ORIENTATION_HORIZONTAL = Qt.Orientation.Horizontal
ITEM_CHECKED = Qt.CheckState.Checked
USER_ROLE = Qt.ItemDataRole.UserRole


DATA_FILE = Path("tsa_endpoints.json")
PROMPT = "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation."
EXPECTED_OUTPUT = "OKK"
CHINESE_FONT_CANDIDATES = (
    "Noto Sans CJK SC",
    "Noto Sans SC",
    "Source Han Sans SC",
    "WenQuanYi Micro Hei",
    "WenQuanYi Zen Hei",
    "Microsoft YaHei",
    "SimHei",
    "PingFang SC",
    "Heiti SC",
    "Droid Sans Fallback",
)


@dataclass
class SavedEndpoint:
    id: str
    type: str
    base_url: str
    api_key: str
    models: list[str]


@dataclass
class TestResult:
    model: str
    status: str
    seconds: float
    detail: str = ""


@dataclass
class RunningProcess:
    process: subprocess.Popen[str] | None = None
    lock: threading.Lock = field(default_factory=threading.Lock)

    def set(self, process: subprocess.Popen[str] | None) -> None:
        with self.lock:
            self.process = process

    def stop(self) -> None:
        with self.lock:
            process = self.process
        if process is None or process.poll() is not None:
            return
        terminate_process(process)


class EndpointStore:
    def __init__(self, path: Path = DATA_FILE) -> None:
        self.path = path
        self.endpoints: list[SavedEndpoint] = []

    def load(self) -> None:
        if not self.path.exists():
            self.endpoints = []
            return
        with self.path.open("r", encoding="utf-8") as fh:
            payload = json.load(fh)
        raw_endpoints = payload.get("endpoints", [])
        if not isinstance(raw_endpoints, list):
            raise ValueError("tsa_endpoints.json: endpoints must be a list")

        endpoints: list[SavedEndpoint] = []
        for item in raw_endpoints:
            if not isinstance(item, dict):
                continue
            endpoints.append(
                SavedEndpoint(
                    id=str(item.get("id") or self._new_id(str(item.get("type", "endpoint")), endpoints)),
                    type=str(item.get("type", "")),
                    base_url=str(item.get("base_url", "")).rstrip("/"),
                    api_key=str(item.get("api_key", "")),
                    models=[str(model) for model in item.get("models", []) if str(model)],
                )
            )
        self.endpoints = [endpoint for endpoint in endpoints if endpoint.id and endpoint.type and endpoint.base_url]

    def save(self) -> None:
        payload = {"version": 1, "endpoints": [endpoint.__dict__ for endpoint in self.endpoints]}
        self.path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    def add(self, endpoint_type: str, base_url: str, api_key: str, models: list[str]) -> SavedEndpoint:
        endpoint = SavedEndpoint(
            id=self._new_id(endpoint_type, self.endpoints),
            type=endpoint_type,
            base_url=base_url.rstrip("/"),
            api_key=api_key,
            models=models,
        )
        self.endpoints.append(endpoint)
        self.save()
        return endpoint

    def delete(self, endpoint_id: str) -> None:
        self.endpoints = [endpoint for endpoint in self.endpoints if endpoint.id != endpoint_id]
        self.save()

    def get(self, endpoint_id: str) -> SavedEndpoint | None:
        return next((endpoint for endpoint in self.endpoints if endpoint.id == endpoint_id), None)

    def _new_id(self, endpoint_type: str, endpoints: list[SavedEndpoint]) -> str:
        stamp = datetime.now().strftime("%Y%m%d%H%M%S")
        existing = {endpoint.id for endpoint in endpoints}
        index = 1
        while True:
            candidate = f"{endpoint_type}-{stamp}-{index:03d}"
            if candidate not in existing:
                return candidate
            index += 1


def mask_key(api_key: str) -> str:
    if len(api_key) <= 10:
        return "*" * len(api_key)
    return f"{api_key[:7]}...{api_key[-4:]}"


def endpoint_label(endpoint_type: str) -> str:
    return "Codex" if endpoint_type == "codex" else "Claude"


def set_list_checks(widget: QListWidget, checked: bool) -> None:
    state = ITEM_CHECKED if checked else Qt.CheckState.Unchecked
    for index in range(widget.count()):
        widget.item(index).setCheckState(state)


def invert_list_checks(widget: QListWidget) -> None:
    for index in range(widget.count()):
        item = widget.item(index)
        item.setCheckState(Qt.CheckState.Unchecked if item.checkState() == ITEM_CHECKED else ITEM_CHECKED)


def choose_ui_font(app: QApplication) -> str | None:
    families = set(QFontDatabase.families())
    for family in CHINESE_FONT_CANDIDATES:
        if family in families:
            app.setFont(QFont(family, 10))
            return family
    return None


def fetch_endpoint_models(endpoint_type: str, base_url: str, api_key: str, timeout: int) -> list[str]:
    clean_url = base_url.rstrip("/")
    if endpoint_type == "codex":
        return codex_core.fetch_models(codex_core.Endpoint(clean_url, api_key, ()), timeout)
    return claude_core.fetch_models(claude_core.Endpoint(clean_url, api_key, ()), timeout)


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


def run_command(
    command: list[str],
    timeout: int,
    stop_event: threading.Event,
    process_ref: RunningProcess,
    events: queue.Queue[tuple[str, object]],
) -> tuple[str, str]:
    events.put(("log", "running: " + " ".join(shlex.quote(part) for part in command)))
    process = subprocess.Popen(
        command,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        bufsize=1,
        start_new_session=True,
    )
    process_ref.set(process)
    lines: list[str] = []
    deadline = time.monotonic() + timeout
    assert process.stdout is not None
    output_selector = selectors.DefaultSelector()
    output_selector.register(process.stdout, selectors.EVENT_READ)

    try:
        while True:
            if stop_event.is_set():
                terminate_process(process)
                return "STOPPED", "stopped by user"
            if time.monotonic() > deadline:
                terminate_process(process)
                return "UNAVAILABLE", f"timeout after {timeout}s"

            for _key, _mask in output_selector.select(timeout=0.2):
                line = process.stdout.readline()
                if line:
                    clean = line.rstrip()
                    lines.append(clean)
                    events.put(("log", clean))

            if process.poll() is not None:
                for line in process.stdout:
                    clean = line.rstrip()
                    lines.append(clean)
                    events.put(("log", clean))
                break
    finally:
        output_selector.close()
        process_ref.set(None)

    output = "\n".join(lines)
    if process.returncode == 0 and any(line.strip() == EXPECTED_OUTPUT for line in lines):
        return "AVAILABLE", output[-1000:]
    if process.returncode == 0:
        return "UNAVAILABLE", f"command exited 0 but did not return expected '{EXPECTED_OUTPUT}'\n{output[-1000:]}"
    return "UNAVAILABLE", output[-1000:]


def test_codex_model(
    endpoint: SavedEndpoint,
    model: str,
    timeout: int,
    stop_event: threading.Event,
    process_ref: RunningProcess,
    events: queue.Queue[tuple[str, object]],
) -> TestResult:
    start = time.monotonic()
    core_endpoint = codex_core.Endpoint(endpoint.base_url, endpoint.api_key, tuple(endpoint.models))
    auth_path = Path.home() / ".codex" / "auth.json"
    config_path = Path.home() / ".codex" / "config.toml"
    with codex_core.RestorableFile(auth_path), codex_core.RestorableFile(config_path):
        codex_core.write_codex_config(Path.home() / ".codex", core_endpoint, model)
        status, detail = run_command(["codex", "exec", "--skip-git-repo-check", PROMPT], timeout, stop_event, process_ref, events)
    return TestResult(model=model, status=status, seconds=time.monotonic() - start, detail=detail)


def test_claude_model(
    endpoint: SavedEndpoint,
    model: str,
    timeout: int,
    stop_event: threading.Event,
    process_ref: RunningProcess,
    events: queue.Queue[tuple[str, object]],
) -> TestResult:
    start = time.monotonic()
    core_endpoint = claude_core.Endpoint(endpoint.base_url, endpoint.api_key, tuple(endpoint.models))
    settings_path = Path("claude-settings.json")
    command = ["claude", "--debug", "--verbose", "--settings", str(settings_path), "--model", model, "-p", PROMPT]
    with claude_core.RestorableFile(settings_path):
        claude_core.write_claude_settings(settings_path, core_endpoint, model)
        status, detail = run_command(command, timeout, stop_event, process_ref, events)
    return TestResult(model=model, status=status, seconds=time.monotonic() - start, detail=detail)


class MainWindow(QMainWindow):
    def __init__(self, ui_font: str | None) -> None:
        super().__init__()
        self.store = EndpointStore()
        self.events: queue.Queue[tuple[str, object]] = queue.Queue()
        self.fetch_thread: threading.Thread | None = None
        self.ui_font = ui_font

        self.setWindowTitle("TSA GUI")
        self.resize(1120, 760)
        self._build_ui()
        self._load_store()

        if self.ui_font:
            self.log(f"Qt binding: PyQt6; UI font: {self.ui_font}")
        else:
            self.log("Qt binding: PyQt6; using system default font")

        self.timer = QTimer(self)
        self.timer.timeout.connect(self.process_events)
        self.timer.start(100)

    def _build_ui(self) -> None:
        root = QWidget(self)
        self.setCentralWidget(root)
        layout = QVBoxLayout(root)

        splitter = QSplitter(ORIENTATION_HORIZONTAL)
        layout.addWidget(splitter, 3)

        add_group = QGroupBox("添加端点")
        add_layout = QVBoxLayout(add_group)
        form = QFormLayout()
        self.type_combo = QComboBox()
        self.type_combo.addItems(["codex", "claude"])
        self.url_edit = QLineEdit()
        self.url_edit.setPlaceholderText("https://example.com/v1")
        self.key_edit = QLineEdit()
        self.key_edit.setEchoMode(QLineEdit.EchoMode.Password)
        self.timeout_spin = QSpinBox()
        self.timeout_spin.setRange(1, 3600)
        self.timeout_spin.setValue(30)
        self.timeout_spin.setSuffix(" 秒")
        form.addRow("类型", self.type_combo)
        form.addRow("URL", self.url_edit)
        form.addRow("SK", self.key_edit)
        form.addRow("拉取超时", self.timeout_spin)
        add_layout.addLayout(form)

        buttons = QHBoxLayout()
        self.fetch_button = QPushButton("拉取模型")
        self.fetch_button.clicked.connect(self.fetch_models)
        self.save_button = QPushButton("保存端点")
        self.save_button.clicked.connect(self.save_endpoint)
        clear_button = QPushButton("清空")
        clear_button.clicked.connect(self.clear_input)
        buttons.addWidget(self.fetch_button)
        buttons.addWidget(self.save_button)
        buttons.addWidget(clear_button)
        add_layout.addLayout(buttons)

        self.model_list = QListWidget()
        add_layout.addWidget(QLabel("已拉取模型"))
        add_layout.addWidget(self.model_list, 1)
        model_buttons = QHBoxLayout()
        select_all_button = QPushButton("全选")
        select_all_button.clicked.connect(lambda: set_list_checks(self.model_list, True))
        select_none_button = QPushButton("全不选")
        select_none_button.clicked.connect(lambda: set_list_checks(self.model_list, False))
        invert_button = QPushButton("反选")
        invert_button.clicked.connect(lambda: invert_list_checks(self.model_list))
        model_buttons.addWidget(select_all_button)
        model_buttons.addWidget(select_none_button)
        model_buttons.addWidget(invert_button)
        model_buttons.addStretch(1)
        add_layout.addLayout(model_buttons)
        splitter.addWidget(add_group)

        saved_group = QGroupBox("已保存端点")
        saved_layout = QVBoxLayout(saved_group)
        self.endpoint_table = QTableWidget(0, 4)
        self.endpoint_table.setHorizontalHeaderLabels(["类型", "URL", "SK", "模型数"])
        self.endpoint_table.horizontalHeader().setSectionResizeMode(1, QHeaderView.ResizeMode.Stretch)
        self.endpoint_table.setSelectionBehavior(QTableWidget.SelectionBehavior.SelectRows)
        self.endpoint_table.setSelectionMode(QTableWidget.SelectionMode.SingleSelection)
        self.endpoint_table.setEditTriggers(QTableWidget.EditTrigger.NoEditTriggers)
        self.endpoint_table.doubleClicked.connect(self.open_test_dialog)
        saved_layout.addWidget(self.endpoint_table, 1)

        list_buttons = QHBoxLayout()
        test_button = QPushButton("测试")
        test_button.clicked.connect(self.open_test_dialog)
        delete_button = QPushButton("删除")
        delete_button.clicked.connect(self.delete_endpoint)
        refresh_button = QPushButton("刷新")
        refresh_button.clicked.connect(self.refresh_endpoint_list)
        load_button = QPushButton("加载")
        load_button.clicked.connect(self.load_endpoint_to_form)
        copy_url_button = QPushButton("复制 URL")
        copy_url_button.clicked.connect(self.copy_selected_url)
        copy_key_button = QPushButton("复制 KEY")
        copy_key_button.clicked.connect(self.copy_selected_key)
        list_buttons.addWidget(test_button)
        list_buttons.addWidget(delete_button)
        list_buttons.addWidget(refresh_button)
        list_buttons.addWidget(load_button)
        list_buttons.addWidget(copy_url_button)
        list_buttons.addWidget(copy_key_button)
        list_buttons.addStretch(1)
        saved_layout.addLayout(list_buttons)
        splitter.addWidget(saved_group)
        splitter.setSizes([420, 680])

        log_group = QGroupBox("日志")
        log_layout = QVBoxLayout(log_group)
        self.log_text = QTextEdit()
        self.log_text.setReadOnly(True)
        log_layout.addWidget(self.log_text)
        layout.addWidget(log_group, 2)

    def _load_store(self) -> None:
        try:
            self.store.load()
        except Exception as exc:
            result = QMessageBox.question(self, "数据文件读取失败", f"读取 {DATA_FILE} 失败：\n{exc}\n\n是否创建新的空数据？")
            if result != QMessageBox.StandardButton.Yes:
                raise
            self.store.endpoints = []
            self.store.save()
        self.refresh_endpoint_list()

    def fetch_models(self) -> None:
        endpoint_type = self.type_combo.currentText().strip()
        base_url = self.url_edit.text().strip()
        api_key = self.key_edit.text().strip()
        if not base_url:
            QMessageBox.warning(self, "缺少 URL", "请填写端点 URL。")
            return
        if not api_key:
            QMessageBox.warning(self, "缺少 SK", "请填写 API Key。")
            return
        self.fetch_button.setEnabled(False)
        self.log(f"fetching models: type={endpoint_type} url={base_url}")
        self.fetch_thread = threading.Thread(
            target=self._fetch_models_worker,
            args=(endpoint_type, base_url, api_key, int(self.timeout_spin.value())),
            daemon=True,
        )
        self.fetch_thread.start()

    def _fetch_models_worker(self, endpoint_type: str, base_url: str, api_key: str, timeout: int) -> None:
        try:
            models = fetch_endpoint_models(endpoint_type, base_url, api_key, timeout)
        except Exception as exc:
            self.events.put(("fetch_failed", str(exc)))
            return
        self.events.put(("models_fetched", models))

    def save_endpoint(self) -> None:
        endpoint_type = self.type_combo.currentText().strip()
        base_url = self.url_edit.text().strip().rstrip("/")
        api_key = self.key_edit.text().strip()
        models = self.checked_models()
        if not base_url:
            QMessageBox.warning(self, "缺少 URL", "请填写端点 URL。")
            return
        if not api_key:
            QMessageBox.warning(self, "缺少 SK", "请填写 API Key。")
            return
        if not models:
            QMessageBox.warning(self, "未选择模型", "请先拉取模型并选择至少一个模型。")
            return
        try:
            self.store.add(endpoint_type, base_url, api_key, models)
        except Exception as exc:
            QMessageBox.critical(self, "保存失败", str(exc))
            return
        self.log(f"saved endpoint: type={endpoint_type} url={base_url} models={len(models)}")
        self.refresh_endpoint_list()

    def checked_models(self) -> list[str]:
        models: list[str] = []
        for index in range(self.model_list.count()):
            item = self.model_list.item(index)
            if item.checkState() == ITEM_CHECKED:
                models.append(item.text())
        return models

    def clear_input(self) -> None:
        self.url_edit.clear()
        self.key_edit.clear()
        self.model_list.clear()

    def refresh_endpoint_list(self) -> None:
        self.endpoint_table.setRowCount(0)
        for endpoint in self.store.endpoints:
            row = self.endpoint_table.rowCount()
            self.endpoint_table.insertRow(row)
            values = [endpoint_label(endpoint.type), endpoint.base_url, mask_key(endpoint.api_key), str(len(endpoint.models))]
            for column, value in enumerate(values):
                item = QTableWidgetItem(value)
                item.setData(USER_ROLE, endpoint.id)
                if column in (0, 3):
                    item.setTextAlignment(ALIGN_CENTER)
                self.endpoint_table.setItem(row, column, item)

    def selected_endpoint(self) -> SavedEndpoint | None:
        row = self.endpoint_table.currentRow()
        if row < 0:
            QMessageBox.information(self, "未选择端点", "请先选择一个已保存端点。")
            return None
        item = self.endpoint_table.item(row, 0)
        endpoint_id = item.data(USER_ROLE) if item is not None else None
        endpoint = self.store.get(str(endpoint_id)) if endpoint_id else None
        if endpoint is None:
            QMessageBox.critical(self, "端点不存在", "选中的端点不存在，请刷新列表。")
        return endpoint

    def selected_endpoint_id(self) -> str | None:
        row = self.endpoint_table.currentRow()
        if row < 0:
            QMessageBox.information(self, "未选择端点", "请先选择一个已保存端点。")
            return None
        item = self.endpoint_table.item(row, 0)
        endpoint_id = item.data(USER_ROLE) if item is not None else None
        return str(endpoint_id) if endpoint_id else None

    def delete_endpoint(self) -> None:
        endpoint_id = self.selected_endpoint_id()
        if endpoint_id is None:
            return
        endpoint = self.store.get(endpoint_id)
        if endpoint is None:
            return
        result = QMessageBox.question(self, "确认删除", f"确定删除端点？\n{endpoint.base_url}")
        if result != QMessageBox.StandardButton.Yes:
            return
        try:
            self.store.delete(endpoint_id)
        except Exception as exc:
            QMessageBox.critical(self, "删除失败", str(exc))
            return
        self.refresh_endpoint_list()
        self.log(f"deleted endpoint: {endpoint.base_url}")

    def load_endpoint_to_form(self) -> None:
        endpoint = self.selected_endpoint()
        if endpoint is None:
            return
        self.type_combo.setCurrentText(endpoint.type)
        self.url_edit.setText(endpoint.base_url)
        self.key_edit.setText(endpoint.api_key)
        self.model_list.clear()
        for model in endpoint.models:
            item = QListWidgetItem(model)
            item.setCheckState(ITEM_CHECKED)
            self.model_list.addItem(item)
        self.log(f"loaded endpoint into form: {endpoint.base_url}")

    def copy_selected_url(self) -> None:
        endpoint = self.selected_endpoint()
        if endpoint is None:
            return
        QApplication.clipboard().setText(endpoint.base_url)
        self.log(f"copied URL: {endpoint.base_url}")

    def copy_selected_key(self) -> None:
        endpoint = self.selected_endpoint()
        if endpoint is None:
            return
        QApplication.clipboard().setText(endpoint.api_key)
        self.log(f"copied KEY for: {endpoint.base_url}")

    def open_test_dialog(self) -> None:
        endpoint = self.selected_endpoint()
        if endpoint is None:
            return
        dialog = TestDialog(self, endpoint)
        dialog.exec()

    def process_events(self) -> None:
        while True:
            try:
                event, payload = self.events.get_nowait()
            except queue.Empty:
                break
            if event == "models_fetched":
                self.fetch_button.setEnabled(True)
                self.model_list.clear()
                for model in payload:  # type: ignore[union-attr]
                    item = QListWidgetItem(str(model))
                    item.setCheckState(ITEM_CHECKED)
                    self.model_list.addItem(item)
                self.log(f"fetched {self.model_list.count()} models")
            elif event == "fetch_failed":
                self.fetch_button.setEnabled(True)
                self.log(f"fetch failed: {payload}")
                QMessageBox.critical(self, "拉取模型失败", str(payload))

    def log(self, message: str) -> None:
        stamp = datetime.now().strftime("%H:%M:%S")
        self.log_text.append(f"{stamp} {message}")


class TestDialog(QDialog):
    def __init__(self, parent: QWidget, endpoint: SavedEndpoint) -> None:
        super().__init__(parent)
        self.endpoint = endpoint
        self.events: queue.Queue[tuple[str, object]] = queue.Queue()
        self.stop_event = threading.Event()
        self.process_ref = RunningProcess()
        self.worker: threading.Thread | None = None

        self.setWindowTitle(f"测试端点 - {endpoint_label(endpoint.type)}")
        self.resize(900, 650)
        self._build_ui()

        self.timer = QTimer(self)
        self.timer.timeout.connect(self.process_events)
        self.timer.start(100)

    def _build_ui(self) -> None:
        layout = QVBoxLayout(self)
        info = QGroupBox("端点")
        grid = QGridLayout(info)
        grid.setHorizontalSpacing(16)
        grid.setVerticalSpacing(8)
        grid.setColumnMinimumWidth(0, 72)
        grid.setColumnStretch(1, 1)
        grid.setColumnStretch(2, 0)

        type_value = QLabel(endpoint_label(self.endpoint.type))
        url_value = QLabel(self.endpoint.base_url)
        sk_value = QLabel(mask_key(self.endpoint.api_key))
        for value in (type_value, url_value, sk_value):
            value.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)

        grid.addWidget(QLabel("类型"), 0, 0)
        grid.addWidget(type_value, 0, 1)
        grid.addWidget(QLabel("URL"), 1, 0)
        grid.addWidget(url_value, 1, 1)
        copy_url_button = QPushButton("复制 URL")
        copy_url_button.setFixedWidth(86)
        copy_url_button.clicked.connect(lambda: self.copy_text("URL", self.endpoint.base_url))
        grid.addWidget(copy_url_button, 1, 2)
        grid.addWidget(QLabel("SK"), 2, 0)
        grid.addWidget(sk_value, 2, 1)
        copy_key_button = QPushButton("复制 SK")
        copy_key_button.setFixedWidth(86)
        copy_key_button.clicked.connect(lambda: self.copy_text("SK", self.endpoint.api_key))
        grid.addWidget(copy_key_button, 2, 2)
        layout.addWidget(info)

        controls = QHBoxLayout()
        controls.addWidget(QLabel("超时时间"))
        self.timeout_spin = QSpinBox()
        self.timeout_spin.setRange(1, 3600)
        self.timeout_spin.setValue(120)
        self.timeout_spin.setSuffix(" 秒")
        controls.addWidget(self.timeout_spin)
        self.append_1m_check = QCheckBox("模型后追加 1M 上下文 [1m]")
        if self.endpoint.type == "claude":
            controls.addWidget(self.append_1m_check)
        self.start_button = QPushButton("开始测试")
        self.start_button.clicked.connect(self.start_test)
        self.stop_button = QPushButton("停止")
        self.stop_button.setEnabled(False)
        self.stop_button.clicked.connect(self.stop_test)
        close_button = QPushButton("关闭")
        close_button.clicked.connect(self.close)
        controls.addWidget(self.start_button)
        controls.addWidget(self.stop_button)
        controls.addWidget(close_button)
        controls.addStretch(1)
        layout.addLayout(controls)

        splitter = QSplitter(ORIENTATION_HORIZONTAL)
        self.model_list = QListWidget()
        for model in self.endpoint.models:
            item = QListWidgetItem(model)
            item.setCheckState(ITEM_CHECKED)
            self.model_list.addItem(item)
        model_box = QGroupBox("选择模型")
        model_layout = QVBoxLayout(model_box)
        model_layout.addWidget(self.model_list)
        model_buttons = QHBoxLayout()
        select_all_button = QPushButton("全选")
        select_all_button.clicked.connect(lambda: set_list_checks(self.model_list, True))
        select_none_button = QPushButton("全不选")
        select_none_button.clicked.connect(lambda: set_list_checks(self.model_list, False))
        invert_button = QPushButton("反选")
        invert_button.clicked.connect(lambda: invert_list_checks(self.model_list))
        model_buttons.addWidget(select_all_button)
        model_buttons.addWidget(select_none_button)
        model_buttons.addWidget(invert_button)
        model_buttons.addStretch(1)
        model_layout.addLayout(model_buttons)
        splitter.addWidget(model_box)

        result_box = QGroupBox("结果")
        result_layout = QVBoxLayout(result_box)
        self.result_table = QTableWidget(0, 3)
        self.result_table.setHorizontalHeaderLabels(["模型", "状态", "耗时"])
        self.result_table.horizontalHeader().setSectionResizeMode(0, QHeaderView.ResizeMode.Stretch)
        self.result_table.setEditTriggers(QTableWidget.EditTrigger.NoEditTriggers)
        result_layout.addWidget(self.result_table)
        splitter.addWidget(result_box)
        splitter.setSizes([320, 560])
        layout.addWidget(splitter, 2)

        log_box = QGroupBox("日志")
        log_layout = QVBoxLayout(log_box)
        self.log_text = QTextEdit()
        self.log_text.setReadOnly(True)
        log_layout.addWidget(self.log_text)
        layout.addWidget(log_box, 2)

    def selected_models(self) -> list[str]:
        models: list[str] = []
        for index in range(self.model_list.count()):
            item = self.model_list.item(index)
            if item.checkState() == ITEM_CHECKED:
                models.append(item.text())
        return models

    def start_test(self) -> None:
        models = self.selected_models()
        if self.endpoint.type == "claude" and self.append_1m_check.isChecked():
            models = [model + "[1m]" for model in models]
        if not models:
            QMessageBox.warning(self, "未选择模型", "请选择至少一个模型。")
            return
        command_name = "codex" if self.endpoint.type == "codex" else "claude"
        if shutil.which(command_name) is None:
            QMessageBox.critical(self, "命令不存在", f"{command_name} command not found in PATH")
            return

        self.result_table.setRowCount(0)
        self.stop_event.clear()
        self.start_button.setEnabled(False)
        self.stop_button.setEnabled(True)
        self.log(f"starting test: {len(models)} models")
        self.worker = threading.Thread(target=self._test_worker, args=(models, int(self.timeout_spin.value())), daemon=True)
        self.worker.start()

    def copy_text(self, label: str, text: str) -> None:
        QApplication.clipboard().setText(text)
        self.log(f"copied {label}")

    def stop_test(self) -> None:
        self.stop_event.set()
        self.process_ref.stop()
        self.log("stopping test...")

    def closeEvent(self, event) -> None:  # type: ignore[no-untyped-def]
        if self.worker and self.worker.is_alive():
            result = QMessageBox.question(self, "测试仍在运行", "测试仍在运行，是否停止并关闭？")
            if result != QMessageBox.StandardButton.Yes:
                event.ignore()
                return
            self.stop_test()
        event.accept()

    def _test_worker(self, models: list[str], timeout: int) -> None:
        for model in models:
            if self.stop_event.is_set():
                break
            self.events.put(("log", f"testing model: {model}"))
            try:
                runner: Callable[..., TestResult] = test_codex_model if self.endpoint.type == "codex" else test_claude_model
                result = runner(self.endpoint, model, timeout, self.stop_event, self.process_ref, self.events)
            except Exception as exc:
                result = TestResult(model=model, status="UNAVAILABLE", seconds=0.0, detail=str(exc))
            self.events.put(("test_result", result))
            if result.status == "STOPPED":
                break
        self.events.put(("test_finished", None))

    def process_events(self) -> None:
        while True:
            try:
                event, payload = self.events.get_nowait()
            except queue.Empty:
                break
            if event == "log":
                self.log(str(payload))
            elif event == "test_result":
                result = payload
                if isinstance(result, TestResult):
                    self.add_result(result)
                    self.log(f"MODEL_STATUS={result.status} model={result.model} elapsed={result.seconds:.1f}s")
                    if result.detail and result.status != "AVAILABLE":
                        self.log(result.detail)
            elif event == "test_finished":
                self.start_button.setEnabled(True)
                self.stop_button.setEnabled(False)
                self.log("test finished")

    def add_result(self, result: TestResult) -> None:
        row = self.result_table.rowCount()
        self.result_table.insertRow(row)
        values = [result.model, result.status, f"{result.seconds:.1f}s"]
        for column, value in enumerate(values):
            item = QTableWidgetItem(value)
            if column in (1, 2):
                item.setTextAlignment(ALIGN_CENTER)
            self.result_table.setItem(row, column, item)

    def log(self, message: str) -> None:
        stamp = datetime.now().strftime("%H:%M:%S")
        self.log_text.append(f"{stamp} {message}")


def main() -> int:
    app = QApplication(sys.argv)
    ui_font = choose_ui_font(app)
    window = MainWindow(ui_font)
    window.show()
    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
