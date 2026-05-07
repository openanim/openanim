"""Config accessor — reads [tool.openanim] from pyproject.toml."""
from __future__ import annotations
import tomllib
from pathlib import Path
from typing import Any

_ROOT = Path(__file__).parent.parent
_PYPROJECT = _ROOT / "pyproject.toml"

_DEFAULTS: dict[str, Any] = {
    "openrouter.base_url": "https://openrouter.ai/api/v1",
    "llm.model": "openrouter/auto",
    "manim.quality": "l",
    "manim.output_file": "generated_scene.py",
    "healing.max_attempts": 3,
}

_openanim: dict[str, Any] = {}

if _PYPROJECT.exists():
    with open(_PYPROJECT, "rb") as f:
        toml = tomllib.load(f)
    _openanim = toml.get("tool", {}).get("openanim", {})


def _walk(root: dict[str, Any], dotpath: str) -> Any:
    result = root
    for key in dotpath.split("."):
        if isinstance(result, dict) and key in result:
            result = result[key]
        else:
            return None
    return result


def get_cfg(key: str, default: Any = None) -> Any:
    val = _walk(_openanim, key)
    if val is not None:
        return val
    return _DEFAULTS.get(key, default)
