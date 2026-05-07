"""Manim backend — generates and renders Manim CE animation code."""
from __future__ import annotations
import sys
import subprocess
from pathlib import Path

from .base import Backend
from ._config import get_cfg


class ManimBackend(Backend):
    name = "manim"

    def __init__(self):
        self.scene_class = "GenScene"

    def system_prompt(self) -> str:
        return (
            f"You are an expert Manim developer. Write a complete, runnable Manim script "
            f"for the requested animation. The script must define a Scene class named "
            f"'{self.scene_class}'. Use ONLY standard Manim Community Edition (v0.19+) "
            f"classes and methods. Output ONLY the python code, no markdown block or "
            f"explanations. Do not use ```python``` or ``````."
        )

    def fix_system_prompt(self) -> str:
        return (
            f"You are an expert Manim developer and debugger. Fix the provided Manim script "
            f"that failed to render. Analyze the traceback carefully for: wrong API usage, "
            f"syntax errors, LaTeX issues, missing imports, or bad animations. The fixed "
            f"script MUST define a Scene class named '{self.scene_class}'. Output ONLY the "
            f"corrected Python code, no markdown blocks or explanations."
        )

    def validate(self, code: str) -> tuple[bool, str]:
        try:
            compile(code, "<generated_scene>", "exec")
        except SyntaxError as e:
            return False, f"SyntaxError at line {e.lineno}: {e.msg}"
        if f"class {self.scene_class}" not in code:
            return False, (
                f"Generated code does not contain a 'class {self.scene_class}' definition."
            )
        return True, ""

    def get_output_path(self) -> Path:
        return Path(get_cfg("manim.output_file", "generated_scene.py"))

    def render(self, script_path: Path, quality: str) -> tuple[int, str]:
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "manim",
                f"-q{quality}",
                str(script_path),
                self.scene_class,
            ],
            capture_output=True,
            text=True,
        )
        full_output = result.stderr + (
            "\n" + result.stdout if result.returncode != 0 else ""
        )
        return result.returncode, full_output.strip()
