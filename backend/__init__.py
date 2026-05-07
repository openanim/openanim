"""Backend registry for OpenAnim."""
from __future__ import annotations

from ._config import get_cfg
from .base import Backend
from .manim import ManimBackend

_BACKENDS: dict[str, Backend] = {
    "manim": ManimBackend(),
}


def get_backend(name: str) -> Backend:
    if name not in _BACKENDS:
        raise ValueError(f"Unknown backend '{name}'. Available: {list(_BACKENDS)}")
    return _BACKENDS[name]


def list_backends() -> list[str]:
    return list(_BACKENDS)
