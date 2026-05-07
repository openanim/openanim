"""Abstract backend interface for code-to-video rendering."""
from __future__ import annotations
from abc import ABC, abstractmethod
from pathlib import Path


class Backend(ABC):
    name: str

    @abstractmethod
    def system_prompt(self) -> str:
        """Return the system prompt for code generation."""

    @abstractmethod
    def fix_system_prompt(self) -> str:
        """Return the system prompt for fixing broken code."""

    @abstractmethod
    def validate(self, code: str) -> tuple[bool, str]:
        """Validate generated code. Returns (is_valid, error_message)."""

    @abstractmethod
    def get_output_path(self) -> Path:
        """Return the path to save generated code before rendering."""

    @abstractmethod
    def render(self, script_path: Path, quality: str) -> tuple[int, str]:
        """Render the code. Returns (exit_code, output)."""
